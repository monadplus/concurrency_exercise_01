use futures::{
    future::{IntoFuture, TryMaybeDone},
    Future, TryFuture, TryFutureExt,
};
use std::{
    mem,
    pin::Pin,
    task::{Context, Poll},
};

enum FinalState {
    Pending,
    AllError,
    Done(usize),
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct TryAll<F>
where
    F: TryFuture,
{
    elems: Pin<Box<[TryMaybeDone<IntoFuture<F>>]>>,
}

fn iter_pin_mut<T>(slice: Pin<&mut [T]>) -> impl Iterator<Item = Pin<&mut T>> {
    unsafe { slice.get_unchecked_mut() }
        .iter_mut()
        .map(|t| unsafe { Pin::new_unchecked(t) })
}

pub fn try_all<I>(iter: I) -> TryAll<I::Item>
where
    I: IntoIterator,
    I::Item: TryFuture,
{
    let iter = iter.into_iter().map(TryFutureExt::into_future);
    TryAll {
        elems: iter.map(TryMaybeDone::Future).collect::<Box<[_]>>().into(),
    }
}

impl<F> Future for TryAll<F>
where
    F: TryFuture,
{
    type Output = Option<F::Ok>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = FinalState::AllError;

        for (i, elem) in iter_pin_mut(self.elems.as_mut()).enumerate() {
            let x = elem.try_poll(cx);
            match x {
                Poll::Pending => state = FinalState::Pending,
                Poll::Ready(Ok(())) => {
                    state = FinalState::Done(i);
                    break;
                }
                Poll::Ready(Err(_)) => {}
            }
        }

        match state {
            FinalState::Pending => Poll::Pending,
            FinalState::Done(idx) => {
                let mut elems = mem::replace(&mut self.elems, Box::pin([]));
                let mut r: Option<F::Ok> = None;
                for (i, elem) in iter_pin_mut(elems.as_mut()).enumerate() {
                    if i == idx {
                        r = Some(elem.take_output().unwrap());
                        break;
                    }
                }
                Poll::Ready(r)
            }
            FinalState::AllError => {
                let _ = mem::replace(&mut self.elems, Box::pin([]));
                Poll::Ready(None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    fn assert_future<T, F>(future: F) -> F
    where
        F: Future<Output = T>,
    {
        future
    }

    #[tokio::test]
    async fn test_try_all() {
        let mut v = Vec::new();
        for idx in 1..=10 {
            let fut = assert_future::<Result<usize, String>, _>(async move {
                sleep(Duration::from_millis(100)).await;
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
