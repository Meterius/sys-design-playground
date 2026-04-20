use std::num::NonZeroUsize;
use std::sync::{mpsc, oneshot};
use std::{collections::BinaryHeap, fs, path::PathBuf, thread};

#[derive(Debug)]
struct Request {
    path: PathBuf,
    resp: oneshot::Sender<Result<Vec<u8>, std::io::Error>>,
}

impl Ord for Request {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.path.cmp(&self.path)
    }
}
impl PartialOrd for Request {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for Request {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}
impl Eq for Request {}

pub struct QueuedReader {
    tx: mpsc::Sender<Request>,
}

const BATCH_SIZE: usize = 128;

impl QueuedReader {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || worker(rx));
        Self { tx }
    }

    pub fn blocking_read<P: Into<PathBuf>>(&self, path: P) -> Result<Vec<u8>, std::io::Error> {
        let (resp_tx, resp_rx) = oneshot::channel();

        let req = Request {
            path: path.into(),
            resp: resp_tx,
        };

        self.tx.send(req).unwrap();
        resp_rx.recv().unwrap()
    }
}

fn worker(rx: mpsc::Receiver<Request>) {
    let mut heap = BinaryHeap::with_capacity(BATCH_SIZE);
    let mut buffer = Vec::with_capacity(BATCH_SIZE);
    let mut cache = lru::LruCache::<PathBuf, Vec<u8>>::new(NonZeroUsize::new(64).unwrap());

    loop {
        heap.push(match rx.recv() {
            Ok(req) => req,
            Err(mpsc::RecvError) => break,
        });

        while heap.len() < BATCH_SIZE
            && let Some(req) = rx
                .try_recv()
                .map(Some)
                .or_else(|err| {
                    if matches!(err, mpsc::TryRecvError::Empty) {
                        Ok(None)
                    } else {
                        Err(err)
                    }
                })
                .unwrap()
        {
            heap.push(req);
        }

        for req in buffer.drain(..) {
            heap.push(req);
        }

        while let Some(req) = heap.pop() {
            if let Some(bytes) = cache.get(&req.path) {
                let _ = req.resp.send(Ok(bytes.clone()));
                continue;
            }

            let result = fs::read(&req.path);

            if let Ok(bytes) = result.as_ref() {
                cache.push(req.path.clone(), bytes.clone());
            }

            let _ = req.resp.send(result);
        }
    }
}
