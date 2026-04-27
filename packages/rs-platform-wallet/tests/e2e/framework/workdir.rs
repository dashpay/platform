//! Cross-process workdir slot selection via `flock`.
//!
//! Mirrors the `dash-evo-tool` pattern: walk slots `0..MAX_SLOTS`,
//! return the first whose `.lock` file is exclusively claimable. The
//! returned `File` MUST stay open for the slot's lifetime — dropping
//! it releases the lock and lets a sibling test process grab it.
//!
//! Cross-environment isolation is the operator's responsibility
//! (set distinct `PLATFORM_WALLET_E2E_BANK_MNEMONIC` per env);
//! same-machine concurrency is handled here.
//!
//! Wave 2 stub. Wave 3 wires `fs2::FileExt::try_lock_exclusive` and
//! the slot-fallback loop.

use std::fs::File;
use std::path::{Path, PathBuf};

use super::{FrameworkError, FrameworkResult};

/// Maximum number of concurrent test processes per machine.
///
/// Beyond this count [`pick_available_workdir`] errors rather than
/// queueing — running more than `MAX_SLOTS` concurrent test
/// processes on one machine is an operator concern (raise the
/// constant, or partition workloads across machines).
pub const MAX_SLOTS: u32 = 10;

/// Acquire an exclusive workdir slot under `base`.
///
/// Returns `(slot_dir, lock_file)` where `slot_dir` is `base` for
/// slot 0 and `base-1`, `base-2`, … for higher slots, and
/// `lock_file` is the open `flock`-held lock that the caller must
/// keep alive for as long as the slot is in use.
///
/// Wave 2 stub: returns `NotImplemented` immediately. Wave 3
/// implements the real loop.
pub fn pick_available_workdir(_base: &Path) -> FrameworkResult<(PathBuf, File)> {
    Err(FrameworkError::NotImplemented(
        "workdir::pick_available_workdir — wired in Wave 3",
    ))
}
