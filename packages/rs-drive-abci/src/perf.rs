//! Lightweight per-block phase timing.
//!
//! Enabled only when `DRIVE_BLOCK_PERF=1` is set in the environment. Phases are
//! accumulated in memory and reported as means every `DRIVE_BLOCK_PERF_EVERY`
//! blocks (default 500), so the measurement does not pay for a log line inside
//! the very spans it is measuring.

use std::sync::{Mutex, OnceLock};
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
        if !self.on {
            return;
        }
        let now = Instant::now();
        self.buf
            .push((name, now.duration_since(self.last).as_micros() as u64));
        self.last = now;
    }

    /// True when perf logging is enabled.
    pub fn on(&self) -> bool {
        self.on
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
        let mut totals = totals().lock().expect("block perf totals poisoned");
        for (name, micros) in self.buf.drain(..) {
            totals.add(name, micros);
        }
    }
}

/// Record a non-timing value (e.g. a byte count) under `name`.
pub fn value(name: &'static str, v: u64) {
    if !enabled() {
        return;
    }
    totals()
        .lock()
        .expect("block perf totals poisoned")
        .add(name, v);
}

/// Called once per finalized block. Emits the means and resets every
/// `DRIVE_BLOCK_PERF_EVERY` blocks.
pub fn end_block(height: u64) {
    if !enabled() {
        return;
    }
    let every = report_every();
    let report = {
        let mut totals = totals().lock().expect("block perf totals poisoned");
        totals.blocks += 1;
        if totals.blocks < every {
            None
        } else {
            let blocks = totals.blocks;
            let mut line = String::with_capacity(totals.phases.len() * 20);
            for (name, sum, samples) in &totals.phases {
                if !line.is_empty() {
                    line.push(' ');
                }
                // mean over blocks, not over samples: a phase that only runs on
                // some blocks should show its share of the per-block cost
                line.push_str(name);
                line.push('=');
                line.push_str(&(*sum / blocks).to_string());
                line.push('/');
                line.push_str(&samples.to_string());
            }
            totals.phases.clear();
            totals.blocks = 0;
            Some((blocks, line))
        }
    };
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
