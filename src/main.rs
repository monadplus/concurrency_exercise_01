mod try_all;
#[allow(dead_code)]
mod try_at_most_one;

use thiserror::Error;
use tokio::time;
use try_all::try_all;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Server {0}: abruptly disconnected")]
    Disconnected(String),
}

#[derive(Debug)]
struct Binary {
    #[allow(dead_code)]
    from: String,
}

async fn download(server_name: impl Into<String>) -> Result<Binary, ServerError> {
    let mut interval = time::interval(time::Duration::from_millis(100));
    for _i in 0..5 {
        interval.tick().await;
        if rand::random() {
            return Err(ServerError::Disconnected(server_name.into()));
        }
    }
    Ok(Binary {
        from: server_name.into(),
    })
}

const REPOSITORIES: [&str; 10] = [
    "Uno", "Dos", "Tres", "Quatro", "Cinco", "Seis", "Siete", "Ocho", "Nueve", "Diez",
];

#[tokio::main]
async fn main() {
    let v = REPOSITORIES.into_iter().map(download);
    match try_all(v).await {
        None => println!("All downloads failed!"),
        Some(binary) => println!("Binary {binary:?} downloaded"),
    }
}
