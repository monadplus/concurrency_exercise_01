use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Future, TryFuture, TryFutureExt};

pub struct TryAtMostOne<A> {
    inner: Option<(A, A)>,
}

impl<A: Unpin> Unpin for TryAtMostOne<A> {}

pub fn try_at_most_one<A>(future1: A, future2: A) -> TryAtMostOne<A>
where
    A: TryFuture + Unpin,
{
    TryAtMostOne {
        inner: Some((future1, future2)),
    }
}

impl<A: Unpin> Future for TryAtMostOne<A>
where
    A: TryFuture,
{
    type Output = Result<A::Ok, A::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let (mut a, mut b) = self.inner.take().expect("cannot poll AtLeastOnce twice");

        match a.try_poll_unpin(cx) {
            Poll::Ready(Ok(x)) => Poll::Ready(Ok(x)),
            Poll::Ready(Err(x)) => match b.try_poll_unpin(cx) {
                Poll::Ready(Ok(x)) => Poll::Ready(Ok(x)),
                Poll::Ready(Err(_)) => {
                    Poll::Ready(Err(x))
                }
                Poll::Pending => {
                    self.inner = Some((a, b));
                    Poll::Pending
                }
            },
            Poll::Pending => match b.try_poll_unpin(cx) {
                Poll::Ready(Ok(x)) => Poll::Ready(Ok(x)),
                Poll::Ready(Err(_)) => {
                    self.inner = Some((a, b));
                    Poll::Pending
                }
                Poll::Pending => {
                    self.inner = Some((a, b));
                    Poll::Pending
                }
            },
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use futures::future::{err, ok};

    #[tokio::test]
    async fn test_try_at_most_one() {
        let r: Result<usize, String> =
            try_at_most_one(ok(0), ok(1)).await;
        assert_eq!(r, Ok(0));

        let r: Result<usize, String> =
            try_at_most_one(err("Bad".to_string()), ok(0)).await;
        assert_eq!(r, Ok(0));

        let r: Result<usize, String> =
            try_at_most_one(ok(0), err("Bad".to_string())).await;
        assert_eq!(r, Ok(0));

        let r: Result<usize, String> =
            try_at_most_one(err("Bad".to_string()), err("Really bad".to_string())).await;
        assert_eq!(r, Err("Bad".to_string()));
    }
}
