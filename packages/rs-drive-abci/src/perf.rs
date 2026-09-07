//! Lightweight per-block phase timing.
//!
//! Enabled only when `DRIVE_BLOCK_PERF=1` is set in the environment. Phases are
//! accumulated in memory and reported as means every `DRIVE_BLOCK_PERF_EVERY`
//! blocks (default 500), so the measurement does not pay for a log line inside
//! the very spans it is measuring.
//!
//! This is read straight from the environment rather than through
//! `PlatformConfig` on purpose: it is a developer switch for replay benchmarks,
//! it must cost nothing when off, and it should not need a config change to be
//! flipped on a node under investigation.
//!
//! Phases nest where a handler times a call whose body is itself timed: the
//! `fb_proposal` lap in `finalize_block` covers all of the `fbp_*` laps taken
//! inside `finalize_block_proposal`. Add up laps from one level only.

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

#[derive(Default)]
struct Totals {
    blocks: u64,
    /// (name, summed microseconds, samples), in first-seen order
    phases: Vec<(&'static str, u64, u64)>,
}

impl Totals {
    fn add(&mut self, name: &'static str, micros: u64) {
        if let Some(entry) = self.phases.iter_mut().find(|(n, _, _)| *n == name) {
            entry.1 += micros;
            entry.2 += 1;
        } else {
            self.phases.push((name, micros, 1));
        }
    }

    /// One `name=mean/samples` term per phase, space separated. The mean is
    /// over blocks, not over samples: a phase that only runs on some blocks
    /// shows its share of the per-block cost, and the sample count shows how
    /// often it ran. Only called from `end_block`, after a block was counted.
    fn report_line(&self) -> String {
        debug_assert!(self.blocks > 0, "report_line before any block was counted");
        let mut line = String::with_capacity(self.phases.len() * 20);
        for (name, sum, samples) in &self.phases {
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(name);
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

/// Accumulates the elapsed time of successive phases of block execution.
///
/// Timings are merged into the process-wide totals when the value is dropped.
pub struct Laps {
    last: Instant,
    on: bool,
    buf: Vec<(&'static str, u64)>,
}

impl Laps {
    /// Start a new lap sequence. Cheap and inert when perf logging is off.
    pub fn new() -> Self {
        let on = enabled();
        Laps {
            last: Instant::now(),
            on,
            buf: if on {
                Vec::with_capacity(32)
            } else {
                Vec::new()
            },
        }
    }

    /// Record the time since the previous lap under `name`.
    pub fn lap(&mut self, name: &'static str) {
        self.lap_if(true, name);
    }

    /// Like [`lap`](Self::lap), but only records a sample when `ran` is true.
    /// Use it after work that runs on some blocks only, so the sample count in
    /// the report is the number of blocks the work actually ran on. The lap
    /// boundary moves either way.
    pub fn lap_if(&mut self, ran: bool, name: &'static str) {
        if !self.on {
            return;
        }
        let now = Instant::now();
        if ran {
            self.buf
                .push((name, now.duration_since(self.last).as_micros() as u64));
        }
        self.last = now;
    }
}

impl Default for Laps {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Laps {
    fn drop(&mut self) {
        if !self.on || self.buf.is_empty() {
            return;
        }
        // Telemetry only: a panic elsewhere while the lock was held must not
        // turn into a second panic here, least of all during unwinding.
        let mut totals = totals().lock().unwrap_or_else(PoisonError::into_inner);
        for (name, micros) in self.buf.drain(..) {
            totals.add(name, micros);
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

    #[test]
    fn phases_keep_first_seen_order_and_sum_samples() {
        let mut totals = Totals::default();
        totals.add("b", 10);
        totals.add("a", 5);
        totals.add("b", 20);

        assert_eq!(totals.phases, vec![("b", 30, 2), ("a", 5, 1)]);
    }

    #[test]
    fn report_means_over_blocks_not_over_samples() {
        let mut totals = Totals::default();
        // Ran on one block out of four, costing 400 µs that time.
        totals.add("rare", 400);
        // Ran on every block.
        for _ in 0..4 {
            totals.add("common", 10);
        }
        totals.blocks = 4;

        assert_eq!(totals.report_line(), "rare=100/1 common=10/4");
    }

    #[test]
    fn end_block_reports_and_resets_at_the_interval() {
        let mut totals = Totals::default();
        totals.add("x", 30);
        assert_eq!(totals.end_block(3), None);
        totals.add("x", 30);
        assert_eq!(totals.end_block(3), None);
        totals.add("x", 30);

        assert_eq!(totals.end_block(3), Some((3, "x=30/3".to_string())));
        assert_eq!(totals.blocks, 0);
        assert!(totals.phases.is_empty());
    }

    #[test]
    fn a_lap_that_did_not_run_moves_the_boundary_without_a_sample() {
        let mut laps = Laps {
            last: Instant::now(),
            on: true,
            buf: Vec::new(),
        };
        let before = laps.last;
        laps.lap_if(false, "skipped");
        assert!(laps.buf.is_empty());
        assert!(laps.last >= before);

        laps.lap_if(true, "ran");
        assert_eq!(laps.buf.len(), 1);
        assert_eq!(laps.buf[0].0, "ran");
        // Drop must not merge test laps into the process-wide totals.
        laps.buf.clear();
    }

    #[test]
    fn report_line_is_empty_when_nothing_was_recorded() {
        let mut totals = Totals::default();
        assert_eq!(totals.end_block(1), Some((1, String::new())));
    }
}
