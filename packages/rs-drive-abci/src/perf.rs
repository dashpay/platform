//! Lightweight per-block phase timing for debug builds.
//!
//! This module and all timing call sites are compiled only with debug assertions
//! enabled. Standard release builds exclude the instrumentation entirely, even
//! when `DRIVE_BLOCK_PERF=1` is set.
//!
//! In debug builds, enabled only when `DRIVE_BLOCK_PERF=1` is set in the
//! environment. Phases are accumulated in memory and reported as means every
//! `DRIVE_BLOCK_PERF_EVERY` blocks (default 500), so the measurement does not
//! pay for a log line inside the very spans it is measuring.
//!
//! This is read straight from the environment rather than through
//! `PlatformConfig` on purpose: it is a developer switch for replay benchmarks,
//! it must cost nothing when off, and it should not need a config change to be
//! flipped on a node under investigation.
//!
//! Each [`PhaseTimer`] belongs to a scope, normally the function it lives in,
//! and every phase is reported as `scope.phase`. Phases are named after the
//! call they time, so a term in the report can be grepped straight to the code.
//!
//! Phases nest where a handler times a call whose body is itself timed:
//! `finalize_block.finalize_block_proposal` covers all of the
//! `finalize_block_proposal.*` phases. Add up phases from one level only.

use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::Instant;

fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var("DRIVE_BLOCK_PERF").as_deref() == Ok("1"))
}

fn report_every() -> u64 {
    static EVERY: OnceLock<u64> = OnceLock::new();
    *EVERY.get_or_init(|| {
        std::env::var("DRIVE_BLOCK_PERF_EVERY")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(500)
    })
}

/// A phase's identity in the report: the timer's scope and the phase name.
type PhaseKey = (&'static str, &'static str);

#[derive(Default)]
struct Totals {
    blocks: u64,
    /// (scope, phase, summed microseconds, samples), in first-seen order
    phases: Vec<(&'static str, &'static str, u64, u64)>,
}

impl Totals {
    fn add(&mut self, (scope, phase): PhaseKey, micros: u64) {
        if let Some(entry) = self
            .phases
            .iter_mut()
            .find(|(s, p, _, _)| *s == scope && *p == phase)
        {
            entry.2 += micros;
            entry.3 += 1;
        } else {
            self.phases.push((scope, phase, micros, 1));
        }
    }

    /// One `scope.phase=mean/samples` term per phase, space separated. The mean
    /// is over blocks, not over samples: a phase that only runs on some blocks
    /// shows its share of the per-block cost, and the sample count shows how
    /// often it ran. Only called from `end_block`, after a block was counted.
    fn report_line(&self) -> String {
        debug_assert!(self.blocks > 0, "report_line before any block was counted");
        let mut line = String::with_capacity(self.phases.len() * 48);
        for (scope, phase, sum, samples) in &self.phases {
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(scope);
            line.push('.');
            line.push_str(phase);
            line.push('=');
            line.push_str(&(*sum / self.blocks.max(1)).to_string());
            line.push('/');
            line.push_str(&samples.to_string());
        }
        line
    }

    /// Counts a finished block. Returns the report and resets when the
    /// reporting interval is reached. An interval of zero reports every block.
    fn end_block(&mut self, every: u64) -> Option<(u64, String)> {
        self.blocks += 1;
        if self.blocks < every {
            return None;
        }
        let report = (self.blocks, self.report_line());
        self.phases.clear();
        self.blocks = 0;
        Some(report)
    }
}

fn totals() -> &'static Mutex<Totals> {
    static TOTALS: OnceLock<Mutex<Totals>> = OnceLock::new();
    TOTALS.get_or_init(|| Mutex::new(Totals::default()))
}

/// Times the successive phases of one function during block execution.
///
/// A phase is the time between the previous [`end_phase`](Self::end_phase)
/// (or construction) and this one. Timings are merged into the process-wide
/// totals when the timer is dropped, under `scope.phase`.
pub struct PhaseTimer {
    scope: &'static str,
    phase_start: Instant,
    on: bool,
    buf: Vec<(PhaseKey, u64)>,
}

impl PhaseTimer {
    /// Start timing under `scope`, normally the name of the enclosing function.
    /// Cheap and inert when perf logging is off.
    pub fn new(scope: &'static str) -> Self {
        let on = enabled();
        PhaseTimer {
            scope,
            phase_start: Instant::now(),
            on,
            buf: if on {
                Vec::with_capacity(32)
            } else {
                Vec::new()
            },
        }
    }

    /// Record the time since the previous phase ended under `phase`.
    pub fn end_phase(&mut self, phase: &'static str) {
        self.end_phase_if(true, phase);
    }

    /// Like [`end_phase`](Self::end_phase), but only records a sample when
    /// `ran` is true. Use it after work that runs on some blocks only, so the
    /// sample count in the report is the number of blocks the work actually
    /// ran on. The next phase starts now either way.
    pub fn end_phase_if(&mut self, ran: bool, phase: &'static str) {
        if !self.on {
            return;
        }
        let now = Instant::now();
        if ran {
            self.buf.push((
                (self.scope, phase),
                now.duration_since(self.phase_start).as_micros() as u64,
            ));
        }
        self.phase_start = now;
    }
}

impl Drop for PhaseTimer {
    fn drop(&mut self) {
        if !self.on || self.buf.is_empty() {
            return;
        }
        // Telemetry only: a panic elsewhere while the lock was held must not
        // turn into a second panic here, least of all during unwinding.
        let mut totals = totals().lock().unwrap_or_else(PoisonError::into_inner);
        for (key, micros) in self.buf.drain(..) {
            totals.add(key, micros);
        }
    }
}

/// Called once per finalized block. Emits the means and resets every
/// `DRIVE_BLOCK_PERF_EVERY` blocks.
pub fn end_block(height: u64) {
    if !enabled() {
        return;
    }
    let report = totals()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .end_block(report_every());
    if let Some((blocks, line)) = report {
        tracing::info!(
            block_perf = "agg",
            height,
            blocks,
            phases = line,
            "block perf"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn phases_keep_first_seen_order_and_sum_samples() {
        let mut totals = Totals::default();
        totals.add(("s", "b"), 10);
        totals.add(("s", "a"), 5);
        totals.add(("s", "b"), 20);
        // Same phase name in another scope is a different phase.
        totals.add(("t", "b"), 1);

        assert_eq!(
            totals.phases,
            vec![("s", "b", 30, 2), ("s", "a", 5, 1), ("t", "b", 1, 1)]
        );
    }

    #[test]
    fn report_means_over_blocks_not_over_samples() {
        let mut totals = Totals::default();
        // Ran on one block out of four, costing 400 µs that time.
        totals.add(("run", "rare"), 400);
        // Ran on every block.
        for _ in 0..4 {
            totals.add(("run", "common"), 10);
        }
        totals.blocks = 4;

        assert_eq!(totals.report_line(), "run.rare=100/1 run.common=10/4");
    }

    #[test]
    fn end_block_reports_and_resets_at_the_interval() {
        let mut totals = Totals::default();
        totals.add(("s", "x"), 30);
        assert_eq!(totals.end_block(3), None);
        totals.add(("s", "x"), 30);
        assert_eq!(totals.end_block(3), None);
        totals.add(("s", "x"), 30);

        assert_eq!(totals.end_block(3), Some((3, "s.x=30/3".to_string())));
        assert_eq!(totals.blocks, 0);
        assert!(totals.phases.is_empty());
    }

    #[test]
    fn a_phase_that_did_not_run_starts_the_next_phase_without_a_sample() {
        let mut timer = PhaseTimer {
            scope: "test",
            phase_start: Instant::now(),
            on: true,
            buf: Vec::new(),
        };
        // Put the running phase's start in the past: the skipped phase must
        // still move the start forward, or the next phase would absorb it.
        let before = Instant::now();
        timer.phase_start = before - Duration::from_secs(1);
        timer.end_phase_if(false, "skipped");
        assert!(timer.buf.is_empty());
        assert!(timer.phase_start >= before);

        timer.end_phase_if(true, "ran");
        assert_eq!(timer.buf.len(), 1);
        assert_eq!(timer.buf[0].0, ("test", "ran"));
        assert!(
            timer.buf[0].1 < 1_000_000,
            "must not include the skipped second"
        );
        // Drop must not merge test phases into the process-wide totals.
        timer.buf.clear();
    }

    #[test]
    fn report_line_is_empty_when_nothing_was_recorded() {
        let mut totals = Totals::default();
        assert_eq!(totals.end_block(1), Some((1, String::new())));
    }
}
