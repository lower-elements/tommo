use tokio::io::{self, AsyncWriteExt};
#[tokio::main]
async fn main() {
    let mut stdout = io::stdout();
    stdout.write_all("Hello, world!\n".as_bytes()).await.unwrap();
    stdout.flush().await.unwrap();
}
