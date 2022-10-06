use crate::statement::*;
use crate::try_all::try_all;
use async_trait::async_trait;

#[async_trait]
pub trait Solution {
    async fn solve(repositories: Vec<ServerName>) -> Option<Binary>;
}

pub struct Solution0;

#[async_trait]
impl Solution for Solution0 {
    async fn solve(repositories: Vec<ServerName>) -> Option<Binary> {
        try_all(repositories.into_iter().map(download)).await
    }
}
