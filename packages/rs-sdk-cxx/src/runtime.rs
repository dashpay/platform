// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! The tokio runtime the binding owns. Every SDK call is spawned on its
//! workers while the calling thread blocks on the join handle, which `shutdown`
//! aborts: the SDK does not observe its cancellation token on every path. A
//! task's future, and the C++ callbacks it owns, is dropped before its join
//! handle resolves, and `shutdown` returns only once every [`Call`] has ended,
//! so no callback runs after it.

use std::future::Future;
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use tokio::runtime::Handle;
use tokio::task::AbortHandle;

/// Calls are async; two workers carry any number of them, plus the SDK's
/// own background work. Callbacks run through [`blocking`].
const WORKER_THREADS: usize = 2;
/// GroveDB proof verification recurses deeper than a default thread stack.
const WORKER_STACK_SIZE: usize = 16 * 1024 * 1024;
/// How long `shutdown` waits for the SDK's own background tasks, which hold
/// no callbacks, once every call has ended.
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug)]
pub enum RunError {
    /// The call was aborted by `shutdown`.
    ShutDown,
    /// The call panicked.
    Panicked(String),
}

#[derive(Default)]
struct Calls {
    closing: bool,
    active: usize,
    tasks: Vec<AbortHandle>,
}

pub struct Runtime {
    /// Held for the whole of `shutdown`, so a second caller returns only once
    /// the first has finished; `None` once stopped.
    owned: Mutex<Option<tokio::runtime::Runtime>>,
    /// Kept for spawning and entering: `shutdown` stops the runtime only
    /// after every call has ended.
    pub handle: Handle,
    calls: Mutex<Calls>,
    idle: Condvar,
}

/// Every lock here guards plain data with no invariant spanning a write, so
/// a poisoned one is still served.
pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Runs a C++ callback, which may block on the embedder's locks, without
/// stalling the other tasks on its worker.
pub fn blocking<T>(callback: impl FnOnce() -> T) -> T {
    tokio::task::block_in_place(callback)
}

/// One call in progress; `shutdown` waits until it is dropped.
pub struct Call<'a>(&'a Runtime);

impl Drop for Call<'_> {
    fn drop(&mut self) {
        let mut calls = lock(&self.0.calls);
        calls.active -= 1;
        if calls.active == 0 {
            self.0.idle.notify_all();
        }
    }
}

impl Call<'_> {
    /// Runs `future` on the workers, blocking the calling thread until it
    /// completes, is aborted by [`Runtime::shutdown`], or panics.
    pub fn run<F, T>(&self, future: F) -> Result<T, RunError>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let task = {
            let mut calls = lock(&self.0.calls);
            if calls.closing {
                return Err(RunError::ShutDown);
            }
            let task = self.0.handle.spawn(future);
            calls.tasks.retain(|task| !task.is_finished());
            calls.tasks.push(task.abort_handle());
            task
        };
        match self.0.handle.block_on(task) {
            Ok(value) => Ok(value),
            Err(join) if join.is_cancelled() => Err(RunError::ShutDown),
            Err(join) => Err(RunError::Panicked(join.to_string())),
        }
    }
}

impl Runtime {
    pub fn new() -> Result<Self, String> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(WORKER_THREADS)
            .thread_name("dash-sdk-cxx")
            .thread_stack_size(WORKER_STACK_SIZE)
            .enable_all()
            .build()
            .map_err(|e| format!("unable to start the SDK runtime: {e}"))?;
        Ok(Runtime {
            handle: runtime.handle().clone(),
            owned: Mutex::new(Some(runtime)),
            calls: Mutex::new(Calls::default()),
            idle: Condvar::new(),
        })
    }

    /// Starts a call, `None` once shutdown has begun.
    pub fn call(&self) -> Option<Call<'_>> {
        let mut calls = lock(&self.calls);
        if calls.closing {
            return None;
        }
        calls.active += 1;
        Some(Call(self))
    }

    /// Refuses new calls, aborts the running ones and waits for every call to
    /// end: a call blocked in a C++ callback ends when the callback returns.
    /// Then runs `before_stop` inside the runtime context and stops the
    /// runtime. Idempotent.
    pub fn shutdown(&self, before_stop: impl FnOnce()) {
        let mut owned = lock(&self.owned);
        let mut calls = lock(&self.calls);
        calls.closing = true;
        for task in calls.tasks.drain(..) {
            task.abort();
        }
        while calls.active > 0 {
            calls = self
                .idle
                .wait(calls)
                .unwrap_or_else(PoisonError::into_inner);
        }
        drop(calls);
        if let Some(runtime) = owned.take() {
            {
                let _context = runtime.enter();
                before_stop();
            }
            runtime.shutdown_timeout(SHUTDOWN_TIMEOUT);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn a_panic_is_reported_and_shutdown_interrupts_a_call() {
        let runtime = Arc::new(Runtime::new().expect("runtime"));
        let call = runtime.call().expect("open");
        assert_eq!(call.run(async { 41 + 1 }).unwrap(), 42);
        let error = call.run(async { panic!("hostile bytes") }).unwrap_err();
        assert!(matches!(error, RunError::Panicked(m) if m.contains("hostile bytes")));
        drop(call);

        let worker = {
            let runtime = Arc::clone(&runtime);
            std::thread::spawn(move || {
                let call = runtime.call().expect("open");
                call.run(async { tokio::time::sleep(Duration::from_secs(60)).await })
            })
        };
        std::thread::sleep(Duration::from_millis(100));
        let started = std::time::Instant::now();
        runtime.shutdown(|| ());
        assert!(matches!(worker.join().unwrap(), Err(RunError::ShutDown)));
        assert!(started.elapsed() < Duration::from_secs(30));
        assert!(runtime.call().is_none());
        runtime.shutdown(|| ());
    }

    /// A call blocked in a callback keeps `shutdown` waiting until it ends.
    #[test]
    fn shutdown_waits_for_a_call_blocked_in_a_callback() {
        let runtime = Arc::new(Runtime::new().expect("runtime"));
        let (entered, entered_rx) = std::sync::mpsc::channel();
        let finished = Arc::new(AtomicBool::new(false));
        let worker = {
            let (runtime, finished) = (Arc::clone(&runtime), Arc::clone(&finished));
            std::thread::spawn(move || {
                let call = runtime.call().expect("open");
                let _ = call.run(async move {
                    blocking(|| {
                        entered.send(()).unwrap();
                        std::thread::sleep(Duration::from_millis(300));
                    })
                });
                finished.store(true, Ordering::SeqCst);
            })
        };
        entered_rx.recv().unwrap();
        runtime.shutdown(|| ());
        assert!(
            finished.load(Ordering::SeqCst),
            "shutdown returned before the call ended"
        );
        worker.join().unwrap();
    }
}
