use std::{fmt::Debug, future::Future, sync::Arc, time::Duration};

use async_channel::{Sender, unbounded};
use tokio::task::JoinHandle;
use tracing::Instrument;

pub struct WorkerPool<T> {
    sender: Sender<T>,
    workers: Vec<JoinHandle<()>>,
    pub timeout: Duration,
}

impl<T> WorkerPool<T>
where
    T: Send + Debug + 'static,
{
    pub fn new<F, Fut>(count: usize, timeout: Duration, f: F) -> Self
    where
        F: Fn(T) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = anyhow::Result<()>> + Send,
    {
        assert!(count > 0, "worker pool must have at least one worker");

        let (sender, receiver) = unbounded();

        let fa = Arc::new(f);
        let mut workers = Vec::new();

        for _ in 0..count {
            let receiver = receiver.clone();
            let fw = fa.clone();

            let w = tokio::spawn(async move {
                while let Ok(message) = receiver.recv().await {
                    let span = tracing::error_span!("worker_pool.process", ?message);
                    let fm = fw.clone();

                    async move {
                        let result = tokio::time::timeout(timeout, fm(message)).await;
                        match result {
                            Ok(Ok(())) => {},
                            Ok(Err(err)) => {
                                tracing::error!(error = %err, "failed to process worker pool message");
                            },
                            Err(_) => {
                                tracing::error!("worker pool message processing timed out");
                            }
                        }
                    }
                    .instrument(span)
                    .await;
                }
            });
            workers.push(w);
        }

        Self {
            sender,
            workers,
            timeout,
        }
    }

    pub fn sender(&self) -> Sender<T> {
        self.sender.clone()
    }

    pub async fn shutdown(self) {
        self.sender().close();

        for w in self.workers {
            if let Err(err) = w.await {
                tracing::error!(
                    error = %err,
                    "worker task terminated unexpectedly"
                );
            }
        }
    }
}
