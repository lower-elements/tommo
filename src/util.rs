use std::future::Future;

use tokio::{
    io::{BufReader, BufStream, BufWriter},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::{broadcast, Mutex, MutexGuard},
};

pub type BufReadWrite<R, W> = ReadWrite<BufReader<R>, BufWriter<W>>;
pub type BroadcastReadWrite<T> = ReadWrite<broadcast::Receiver<T>, broadcast::Sender<T>>;

#[derive(Debug)]
pub struct ReadWrite<R, W> {
    pub rx: Mutex<R>,
    pub tx: Mutex<W>,
}

impl<R, W> ReadWrite<R, W> {
    pub fn new(rx: R, tx: W) -> Self {
        Self {
            rx: Mutex::new(rx),
            tx: Mutex::new(tx),
        }
    }

    pub async fn read(&self) -> MutexGuard<'_, R> {
        self.rx.lock().await
    }

    pub async fn write(&self) -> MutexGuard<'_, W> {
        self.tx.lock().await
    }

    pub async fn with_rx<'a, F, Fut, Res>(&'a self, f: F) -> Res
        where
        F: FnOnce(MutexGuard<'a, R>) -> Fut,
        Fut: Future<Output = Res> + 'a {
            let guard = self.read().await;
            f(guard).await
        }

    pub async fn with_tx<'a, F, Fut, Res>(&'a self, f: F) -> Res
        where
        F: FnOnce(MutexGuard<'a, W>) -> Fut,
        Fut: Future<Output = Res> + 'a {
            let guard = self.write().await;
            f(guard).await
        }
}

impl<T: Clone> ReadWrite<broadcast::Receiver<T>, broadcast::Sender<T>> {
    fn channel(queue_size: usize) -> Self {
        let (tx, rx) = broadcast::channel(queue_size);
        Self::new(rx, tx)
    }
}

impl<T: Clone> From<broadcast::Sender<T>>
    for ReadWrite<broadcast::Receiver<T>, broadcast::Sender<T>>
{
    fn from(tx: broadcast::Sender<T>) -> Self {
        let rx = tx.subscribe();
        Self::new(rx, tx)
    }
}

impl ReadWrite<BufReader<OwnedReadHalf>, BufWriter<OwnedWriteHalf>> {
    pub fn buffered(sock: TcpStream) -> Self {
        let (rx, tx) = sock.into_split();
        Self::new(BufReader::new(rx), BufWriter::new(tx))
    }
}

impl From<TcpStream> for ReadWrite<OwnedReadHalf, OwnedWriteHalf> {
    fn from(sock: TcpStream) -> Self {
        let (rx, tx) = sock.into_split();
        Self::new(rx, tx)
    }
}

impl From<BufStream<TcpStream>> for ReadWrite<BufReader<OwnedReadHalf>, BufWriter<OwnedWriteHalf>> {
    fn from(sock: BufStream<TcpStream>) -> Self {
        let (rx, tx) = sock.into_inner().into_split();
        Self::new(BufReader::new(rx), BufWriter::new(tx))
    }
}
