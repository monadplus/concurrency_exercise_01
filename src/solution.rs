use crate::statement::*;
use async_trait::async_trait;
use tokio::task::JoinSet;

#[async_trait]
pub trait Solution {
    async fn solve(repositories: Vec<ServerName>) -> Option<Binary>;
}

pub struct Solution0;

#[async_trait]
impl Solution for Solution0 {
    async fn solve(repositories: Vec<ServerName>) -> Option<Binary> {
        let mut set = JoinSet::new();
        for repository in repositories {
            set.spawn(download(repository));
        }

        while let Some(task) = set.join_next().await {
            match task.unwrap() {
                Ok(binary) => return Some(binary),
                Err(err) => {
                    println!("{err}");
                    match err {
                        ServerError::Disconnected(server_name) => set.spawn(download(server_name)),
                    };
                }
            }
        }

        return None;
    }
}
