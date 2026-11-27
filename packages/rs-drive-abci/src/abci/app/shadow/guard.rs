//! What keeps a failure of the shadow from being a failure of the node, and where the
//! shadow's stop and the node's end are held.

use crate::error::Error;
use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Mutex, OnceLock};

thread_local! {
    /// How many calls of [`contained`] this thread is inside.
    static CONTAINED: Cell<usize> = const { Cell::new(0) };
}

struct Inside;

impl Inside {
    fn enter() -> Self {
        CONTAINED.with(|contained| contained.set(contained.get() + 1));
        Self
    }
}

impl Drop for Inside {
    fn drop(&mut self) {
        CONTAINED.with(|contained| contained.set(contained.get().saturating_sub(1)));
    }
}

/// Whether a panic on this thread, now, is one that shadow mode catches: the thread is
/// doing the shadow's work, and the node goes on after it by its configuration.
///
/// The panic hook of the node asks it. A hook that ends the node at every panic would end
/// it at a panic of the shadow too, before the panic is caught.
pub fn a_panic_here_is_contained() -> bool {
    CONTAINED.with(|contained| contained.get() > 0)
}

pub(super) fn panic_text(panic: Box<dyn std::any::Any + Send>) -> String {
    let text = panic
        .downcast_ref::<&str>()
        .map(|text| text.to_string())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "a panic that is not text".to_string());
    format!("panic: {text}")
}

/// Runs what the shadow does and gives its panic as text.
pub(super) fn contained<T>(run: impl FnOnce() -> T) -> Result<T, String> {
    let inside = Inside::enter();
    let returned = catch_unwind(AssertUnwindSafe(run));
    drop(inside);
    returned.map_err(panic_text)
}

/// Runs what the shadow does and gives its error or its panic as text.
pub(super) fn guarded<T>(run: impl FnOnce() -> Result<T, Error>) -> Result<T, String> {
    contained(run)?.map_err(|error| error.to_string())
}

/// Whether the shadow applies blocks, and a stop that is not written down yet.
#[derive(Default)]
struct Stop {
    stopped: bool,
    /// Why another thread stopped the shadow, until the thread that executes blocks has
    /// written it down with the height it then is at.
    to_record: Option<String>,
}

/// The shadow's stop and the node's end. The thread that executes blocks and the thread
/// that compares stored records both set them, and the first reads them once at every
/// block.
pub(super) struct Latches {
    stop: Mutex<Stop>,
    /// The height the one that ended the node gave: that of the block a divergence was
    /// found at, or the one a comparison of stored records was of, which the node can be
    /// beyond. It is set once.
    ended_at: OnceLock<u64>,
    end_the_node: Arc<dyn Fn() + Send + Sync>,
}

impl Latches {
    pub(super) fn new(stopped: bool, end_the_node: Arc<dyn Fn() + Send + Sync>) -> Self {
        Self {
            stop: Mutex::new(Stop {
                stopped,
                to_record: None,
            }),
            ended_at: OnceLock::new(),
            end_the_node,
        }
    }

    fn stop_state(&self) -> std::sync::MutexGuard<'_, Stop> {
        self.stop
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Whether the shadow applies blocks.
    pub(super) fn applies(&self) -> bool {
        !self.stop_state().stopped
    }

    /// What the thread that executes blocks goes by for one block, read at once: whether
    /// the shadow applies the block, and the reason of a stop another thread made that
    /// is not written down yet, which is given once. A stop that another thread makes
    /// after this holds from the block after.
    pub(super) fn at_a_block(&self) -> (bool, Option<String>) {
        let mut stop = self.stop_state();
        (!stop.stopped, stop.to_record.take())
    }

    /// The shadow applies no more blocks. The caller, which is the thread that executes
    /// blocks, writes it down, and a stop of another thread that is not written down yet
    /// is not written down after it.
    pub(super) fn stop(&self) {
        let mut stop = self.stop_state();
        stop.stopped = true;
        stop.to_record = None;
    }

    /// The shadow applies no more blocks from the next one on, for `reason`: said by a
    /// thread that does not know the height the node is at. Nothing where the shadow is
    /// stopped already.
    #[cfg_attr(not(feature = "shadow-store-compare"), allow(dead_code))]
    pub(super) fn stop_from_the_next_block(&self, reason: String) {
        let mut stop = self.stop_state();
        if !stop.stopped {
            stop.stopped = true;
            stop.to_record = Some(reason);
        }
    }

    /// Ends the node for a divergence that is of `height`, which the node can be beyond:
    /// the hook is called, once however often and from however many threads the node is
    /// ended. Returns the height that the call that ended the node gave.
    pub(super) fn end(&self, height: u64) -> u64 {
        if self.ended_at.set(height).is_ok() {
            (self.end_the_node)();
        }
        *self.ended_at.get().unwrap_or(&height)
    }

    /// The height that the call that ended the node gave, where the node was ended.
    pub(super) fn ended(&self) -> Option<u64> {
        self.ended_at.get().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::execution::ExecutionError;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn a_panic_and_an_error_of_the_shadow_are_given_as_text() {
        assert_eq!(guarded(|| Ok(5)), Ok(5));
        let failed: Result<(), String> = guarded(|| {
            Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "an error",
            )))
        });
        assert!(failed.expect_err("an error").contains("an error"));
        let with_text: Result<(), String> = guarded(|| panic!("a {}", "panic"));
        assert_eq!(with_text, Err("panic: a panic".to_string()));
        let with_a_literal: Result<(), String> = guarded(|| panic!("a literal"));
        assert_eq!(with_a_literal, Err("panic: a literal".to_string()));
        let with_a_value: Result<(), String> = guarded(|| std::panic::panic_any(5u8));
        assert_eq!(
            with_a_value,
            Err("panic: a panic that is not text".to_string())
        );
    }

    #[test]
    fn a_panic_is_contained_only_inside_the_shadows_work() {
        assert!(!a_panic_here_is_contained());
        let inside = contained(|| {
            let outer = a_panic_here_is_contained();
            let inner = contained(a_panic_here_is_contained).expect("no panic");
            (outer, inner, a_panic_here_is_contained())
        });
        assert_eq!(inside, Ok((true, true, true)));
        assert!(!a_panic_here_is_contained());
        // After a panic of the shadow the thread is outside again.
        assert!(contained(|| panic!("inside")).is_err());
        assert!(!a_panic_here_is_contained());
        // Another thread is not inside because this one is.
        let elsewhere = contained(|| {
            std::thread::scope(|scope| {
                scope
                    .spawn(a_panic_here_is_contained)
                    .join()
                    .expect("the other thread")
            })
        });
        assert_eq!(elsewhere, Ok(false));
    }

    #[test]
    fn the_node_is_ended_once_and_at_the_first_height() {
        let ended = Arc::new(AtomicUsize::new(0));
        let counts = Arc::clone(&ended);
        let latches = Latches::new(
            false,
            Arc::new(move || {
                counts.fetch_add(1, Ordering::AcqRel);
            }),
        );
        assert_eq!(latches.ended(), None);
        assert_eq!(latches.end(12), 12);
        assert_eq!(latches.end(15), 12);
        assert_eq!(latches.ended(), Some(12));
        assert_eq!(ended.load(Ordering::Acquire), 1);
    }

    /// The node is ended by several threads at once, each for a height of its own: the
    /// hook is called once, and every one of them, and every reader after, is given the
    /// one height the node ended at.
    #[test]
    fn threads_that_end_the_node_at_once_end_it_once_at_one_height() {
        for _ in 0..200 {
            let ended = Arc::new(AtomicUsize::new(0));
            let counts = Arc::clone(&ended);
            let latches = Latches::new(
                false,
                Arc::new(move || {
                    counts.fetch_add(1, Ordering::AcqRel);
                }),
            );
            let start = std::sync::Barrier::new(4);
            let said: Vec<(u64, Option<u64>)> = std::thread::scope(|scope| {
                let threads: Vec<_> = (1..=4u64)
                    .map(|height| {
                        let (latches, start) = (&latches, &start);
                        scope.spawn(move || {
                            start.wait();
                            let at = latches.end(height);
                            (at, latches.ended())
                        })
                    })
                    .collect();
                threads
                    .into_iter()
                    .map(|thread| thread.join().expect("a thread"))
                    .collect()
            });
            let at = latches.ended().expect("ended");
            assert!((1..=4).contains(&at));
            assert_eq!(said, vec![(at, Some(at)); 4]);
            assert_eq!(ended.load(Ordering::Acquire), 1);
        }
    }

    #[test]
    fn a_stop_from_another_thread_is_recorded_once_by_the_one_that_executes_blocks() {
        let latches = Latches::new(false, Arc::new(|| ()));
        assert!(latches.applies());
        assert_eq!(latches.at_a_block(), (true, None));
        latches.stop_from_the_next_block("the stored records differ".to_string());
        assert!(!latches.applies());
        // A second one changes nothing: the shadow is stopped already.
        latches.stop_from_the_next_block("again".to_string());
        // The block after it: the shadow does not apply it, and the reason is given once.
        assert_eq!(
            latches.at_a_block(),
            (false, Some("the stored records differ".to_string()))
        );
        assert_eq!(latches.at_a_block(), (false, None));

        // A stop that the block thread made itself leaves nothing to record, whether
        // another thread stopped the shadow before it in the same block or after.
        let latches = Latches::new(false, Arc::new(|| ()));
        latches.stop_from_the_next_block("before".to_string());
        latches.stop();
        latches.stop_from_the_next_block("after".to_string());
        assert_eq!(latches.at_a_block(), (false, None));
        assert!(!Latches::new(true, Arc::new(|| ())).applies());
    }
}
