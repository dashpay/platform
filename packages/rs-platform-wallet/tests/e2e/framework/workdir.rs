//! Cross-process workdir slot selection via `flock`. Walks
//! `0..MAX_SLOTS` and returns the first slot whose `.lock` file is
//! exclusively claimable. The returned `File` MUST stay open for
//! the slot's lifetime — dropping it releases the lock.

use std::fs::{self, File, OpenOptions};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use fs2::FileExt;

use super::{FrameworkError, FrameworkResult};

/// Maximum concurrent test processes per machine; beyond this
/// [`pick_available_workdir`] errors rather than queueing.
pub const MAX_SLOTS: u32 = 10;

/// Acquire an exclusive workdir slot under `base`.
///
/// Returns `(slot_dir, lock_file)` — slot 0 is `base` itself,
/// higher slots are `<base>-N`. The caller MUST keep `lock_file`
/// alive for the slot's lifetime; dropping it releases the lock.
pub fn pick_available_workdir(base: &Path) -> FrameworkResult<(PathBuf, File)> {
    for slot in 0..MAX_SLOTS {
        let dir = slot_dir(base, slot);
        fs::create_dir_all(&dir).map_err(|err| {
            FrameworkError::Io(format!("creating workdir {}: {err}", dir.display()))
        })?;

        let lock_path = dir.join(".lock");
        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|err| {
                FrameworkError::Io(format!("opening lock file {}: {err}", lock_path.display()))
            })?;

        match FileExt::try_lock_exclusive(&lock_file) {
            Ok(()) => {
                tracing::info!(
                    target: "platform_wallet::e2e::workdir",
                    slot,
                    dir = %dir.display(),
                    "acquired workdir slot"
                );
                return Ok((dir, lock_file));
            }
            // `WouldBlock` is the only "slot is held by another
            // process" outcome. Anything else (permission denied,
            // unsupported filesystem, EIO, etc.) is propagated so
            // operators see the real cause instead of a misleading
            // "no available workdir slots" message after the loop.
            Err(err) if err.kind() == ErrorKind::WouldBlock => {
                tracing::debug!(
                    target: "platform_wallet::e2e::workdir",
                    slot,
                    dir = %dir.display(),
                    error = %err,
                    "workdir slot busy, trying next"
                );
                // Dropping `lock_file` here releases the would-be
                // lock without affecting the existing holder.
                continue;
            }
            Err(err) => {
                return Err(FrameworkError::Io(format!(
                    "locking {} failed (kind={:?}): {err}",
                    lock_path.display(),
                    err.kind()
                )));
            }
        }
    }

    Err(FrameworkError::Io(format!(
        "no available workdir slots (tried {} under {})",
        MAX_SLOTS,
        base.display()
    )))
}

/// Slot 0 is `base`; higher slots append `-N`. Matches the DET
/// convention so on-disk artifacts from concurrent runs are
/// recognisable at a glance.
fn slot_dir(base: &Path, slot: u32) -> PathBuf {
    if slot == 0 {
        return base.to_path_buf();
    }
    let parent = base.parent().unwrap_or_else(|| Path::new("."));
    let name = base
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "dash-platform-wallet-e2e".to_string());
    parent.join(format!("{name}-{slot}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_call_takes_slot_zero_second_falls_through() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("e2e");

        let (slot0_dir, _lock0) = pick_available_workdir(&base).unwrap();
        assert_eq!(slot0_dir, base);

        // With `_lock0` held, the next caller falls through to slot 1.
        let (slot1_dir, _lock1) = pick_available_workdir(&base).unwrap();
        assert!(
            slot1_dir.ends_with("e2e-1"),
            "expected slot 1 to be `<base>-1`, got {}",
            slot1_dir.display()
        );

        drop(_lock0);
        // After release slot 0 is reclaimable.
        let (slot0_again, _lock0_again) = pick_available_workdir(&base).unwrap();
        assert_eq!(slot0_again, base);
    }
}
