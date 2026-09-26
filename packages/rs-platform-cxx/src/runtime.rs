// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! The one tokio runtime this crate owns. Every SDK request is spawned on
//! its worker threads and the calling thread blocks on the join handle; the
//! handle is kept so `shutdown` can abort whatever is in flight (nothing in
//! `dash-sdk` observes a cancellation token, so aborting the task is the
//! only way to interrupt a request) before the runtime is stopped with a
//! bounded timeout.

use std::future::Future;
use std::sync::Mutex;
use std::time::Duration;

use tokio::runtime::Handle;
use tokio::task::AbortHandle;

use crate::sync::lock;

/// Worker threads: one request at a time from the embedder's serial worker,
/// plus the SDK's own background work.
const WORKER_THREADS: usize = 2;
/// GroveDB proof replay recurses deeper than a default thread stack allows.
const WORKER_STACK_SIZE: usize = 16 * 1024 * 1024;
/// How long `shutdown` waits for aborted tasks to unwind.
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

/// Why a request did not complete.
#[derive(Debug)]
pub enum RunError {
    /// The runtime was shut down before or while the request ran.
    ShutDown,
    /// The request panicked (a bug, or hostile input reaching a panic).
    Panicked(String),
}

pub struct Runtime {
    inner: Mutex<Option<tokio::runtime::Runtime>>,
    in_flight: Mutex<Vec<AbortHandle>>,
}

impl Runtime {
    pub fn new() -> Result<Self, String> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(WORKER_THREADS)
            .thread_name("dash-platform-sdk")
            .thread_stack_size(WORKER_STACK_SIZE)
            .enable_all()
            .build()
            .map_err(|e| format!("unable to start the Platform SDK runtime: {e}"))?;
        Ok(Runtime {
            inner: Mutex::new(Some(runtime)),
            in_flight: Mutex::new(Vec::new()),
        })
    }

    /// The runtime's handle, `None` once shut down.
    pub fn handle(&self) -> Option<Handle> {
        lock(&self.inner)
            .as_ref()
            .map(|runtime| runtime.handle().clone())
    }

    /// Runs `future` on the worker threads, blocking the calling thread
    /// until it completes, is aborted by [`Self::shutdown`], or panics.
    pub fn run<F, T>(&self, future: F) -> Result<T, RunError>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let handle = self.handle().ok_or(RunError::ShutDown)?;
        let task = handle.spawn(future);
        {
            let mut in_flight = lock(&self.in_flight);
            in_flight.retain(|task| !task.is_finished());
            in_flight.push(task.abort_handle());
        }
        match handle.block_on(task) {
            Ok(value) => Ok(value),
            Err(join) if join.is_cancelled() => Err(RunError::ShutDown),
            Err(join) => Err(RunError::Panicked(join.to_string())),
        }
    }

    /// Aborts every in-flight request and stops the runtime, waiting at
    /// most [`SHUTDOWN_TIMEOUT`] for the tasks to unwind. Idempotent.
    pub fn shutdown(&self) {
        for task in lock(&self.in_flight).drain(..) {
            task.abort();
        }
        // Taken out from under the lock first: a blocking teardown while
        // holding it would stall every concurrent `run`.
        let runtime = lock(&self.inner).take();
        if let Some(runtime) = runtime {
            runtime.shutdown_timeout(SHUTDOWN_TIMEOUT);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_and_shuts_down() {
        let runtime = Runtime::new().expect("runtime");
        assert_eq!(runtime.run(async { 41 + 1 }).unwrap(), 42);
        runtime.shutdown();
        assert!(matches!(runtime.run(async { 1 }), Err(RunError::ShutDown)));
        runtime.shutdown();
    }

    #[test]
    fn a_panicking_request_is_reported_not_propagated() {
        let runtime = Runtime::new().expect("runtime");
        let error = runtime.run(async { panic!("hostile bytes") }).unwrap_err();
        assert!(matches!(error, RunError::Panicked(message) if message.contains("hostile bytes")));
    }

    #[test]
    fn shutdown_interrupts_an_in_flight_request() {
        let runtime = std::sync::Arc::new(Runtime::new().expect("runtime"));
        let worker = {
            let runtime = std::sync::Arc::clone(&runtime);
            std::thread::spawn(move || {
                runtime.run(async {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                })
            })
        };
        std::thread::sleep(Duration::from_millis(100));
        let started = std::time::Instant::now();
        runtime.shutdown();
        assert!(matches!(worker.join().unwrap(), Err(RunError::ShutDown)));
        assert!(started.elapsed() < Duration::from_secs(30));
    }
}
