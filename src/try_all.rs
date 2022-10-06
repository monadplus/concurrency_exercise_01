use futures::{
    future::{Fuse, FusedFuture, IntoFuture},
    Future, FutureExt, TryFuture, TryFutureExt,
};
use std::{
    mem,
    pin::Pin,
    task::{Context, Poll},
};

trait FusedTryFuture: FusedFuture<Output = Result<Self::Ok, Self::Err>> {
    type Ok;
    type Err;
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct TryAll<F>
where
    F: TryFuture,
{
    elems: Pin<Box<[Fuse<IntoFuture<F>>]>>,
}

pub fn try_all<I>(iter: I) -> TryAll<I::Item>
where
    I: IntoIterator,
    I::Item: TryFuture,
{
    let x = iter
        .into_iter()
        .map(TryFutureExt::into_future)
        .map(|fut| fut.fuse())
        .collect::<Box<[_]>>();
    TryAll { elems: x.into() }
}

fn iter_pin_mut<T>(slice: Pin<&mut [T]>) -> impl Iterator<Item = Pin<&mut T>> {
    unsafe { slice.get_unchecked_mut() }
        .iter_mut()
        .map(|t| unsafe { Pin::new_unchecked(t) })
}

enum FinalState<T> {
    Pending,
    Done(T),
    AllError,
}

impl<F> Future for TryAll<F>
where
    F: TryFuture,
{
    type Output = Option<F::Ok>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = FinalState::AllError;

        for elem in iter_pin_mut(self.elems.as_mut()) {
            if !elem.is_terminated() {
                let x = elem.try_poll(cx);
                match x {
                    Poll::Pending => state = FinalState::Pending,
                    Poll::Ready(Ok(a)) => {
                        state = FinalState::Done(a);
                        break;
                    }
                    Poll::Ready(Err(_)) => {}
                }
            }
        }
        let output = match state {
            FinalState::Pending => return Poll::Pending,
            FinalState::Done(a) => Some(a),
            FinalState::AllError => None,
        };
        let _ = mem::replace(&mut self.elems, Box::pin([]));
        Poll::Ready(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time;

    fn assert_future<T, F>(future: F) -> F
    where
        F: Future<Output = T>,
    {
        future
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn test_try_all_all_fail() {
        let mut v = Vec::new();
        for i in 1..=10 {
            let fut = assert_future::<Result<usize, String>, _>(async move {
                let mut interval = time::interval(time::Duration::from_millis(10));
                for _ in i..=10 {
                    interval.tick().await;
                }
                Err("Bad".to_string())
            });
            v.push(fut);
        }
        let r = try_all(v).await;
        assert_eq!(r, None);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn test_try_all_once_success() {
        let mut v = Vec::new();
        for idx in 1..=10 {
            let fut = assert_future::<Result<usize, String>, _>(async move {
                let mut interval = time::interval(time::Duration::from_millis(10));
                for _i in 0..5 {
                    interval.tick().await;
                }
                if idx == 10 {
                    Ok(0)
                } else {
                    Err("Bad".to_string())
                }
            });
            v.push(fut);
        }
        let r = try_all(v).await;
        assert_eq!(r, Some(0_usize));
    }
}
