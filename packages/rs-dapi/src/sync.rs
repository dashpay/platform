use std::future::Future;
use std::sync::Mutex;
use std::{fmt::Debug, sync::Arc};
use tokio::task::{AbortHandle, JoinSet};

use crate::DapiError;

#[derive(Clone, Default)]
pub struct Workers {
    pub(crate) inner: Arc<Mutex<JoinSet<Result<(), DapiError>>>>,
}

impl Debug for Workers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let workers = self.inner.try_lock().map(|j| j.len() as i64).unwrap_or(-1);
        write!(f, "Workers {{ num_workers: {workers} }}")
    }
}

impl Workers {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(JoinSet::new())),
        }
    }

    /// Spawn a new task into the join set.
    pub fn spawn<F, O, E>(&self, fut: F) -> AbortHandle
    where
        F: Future<Output = Result<O, E>> + Send + 'static,
        E: Debug + Into<DapiError>,
    {
        let mut join_set = match self.inner.lock() {
            Ok(guard) => guard,
            Err(_poisoned) => {
                tracing::error!("Workers join set mutex poisoned, terminating process");
                std::process::exit(1);
            }
        };
        join_set.spawn(async move {
            match fut.await {
                Ok(_) => Ok(()),
                Err(e) => {
                    tracing::error!(error=?e, "Worker task failed");
                    Err(e.into())
                }
            }
        })
    }
}
