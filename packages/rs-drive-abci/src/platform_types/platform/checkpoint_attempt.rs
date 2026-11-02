//! The block time of the last checkpoint attempt, kept on disk beside the
//! checkpoints so that a restart does not forget it.
//!
//! Every node checkpoints at the first block after a checkpoint interval boundary,
//! and a failed attempt is skipped for the rest of its interval rather than retried
//! at a later block (see `should_checkpoint`). A node restarted inside that interval
//! would otherwise checkpoint at its next block, at a height the rest of the network
//! does not have.

use crate::platform_types::platform::Platform;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

/// File under the database path holding the block time, in milliseconds, of the
/// last checkpoint attempt as decimal text. It sits beside the `checkpoints`
/// directory rather than inside it so that failing to create that directory
/// cannot also lose the record.
const LAST_CHECKPOINT_ATTEMPT_FILE: &str = "last_checkpoint_attempt";

fn last_checkpoint_attempt_path(db_path: &Path) -> PathBuf {
    db_path.join(LAST_CHECKPOINT_ATTEMPT_FILE)
}

/// The persisted block time of the last checkpoint attempt, or 0 when there is
/// none. An unreadable or malformed record is logged and treated as none.
pub(crate) fn load_last_checkpoint_attempt(db_path: &Path) -> u64 {
    let path = last_checkpoint_attempt_path(db_path);
    match std::fs::read_to_string(&path) {
        Ok(contents) => match contents.trim().parse::<u64>() {
            Ok(block_time_ms) => block_time_ms,
            Err(error) => {
                tracing::warn!(
                    ?error,
                    ?path,
                    "ignoring malformed last checkpoint attempt record"
                );
                0
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
        Err(error) => {
            tracing::warn!(
                ?error,
                ?path,
                "failed to read last checkpoint attempt record; treating it as none"
            );
            0
        }
    }
}

/// Persists `block_time_ms` as the last checkpoint attempt, replacing the record
/// atomically so a crash mid-write cannot leave a torn one behind.
pub(crate) fn store_last_checkpoint_attempt(
    db_path: &Path,
    block_time_ms: u64,
) -> std::io::Result<()> {
    let path = last_checkpoint_attempt_path(db_path);
    let temporary_path = path.with_extension("tmp");
    std::fs::write(&temporary_path, block_time_ms.to_string())?;
    std::fs::rename(&temporary_path, &path)
}

impl<C> Platform<C> {
    /// Records a checkpoint attempt at `block_time_ms`, in memory and on disk.
    ///
    /// The in-memory record is what `should_checkpoint` reads. The on-disk copy only
    /// matters across a restart, so failing to write it is logged rather than failing
    /// the attempt.
    pub(crate) fn record_checkpoint_attempt(&self, block_time_ms: u64) {
        self.last_checkpoint_attempt_block_time_ms
            .store(block_time_ms, Ordering::Relaxed);

        if let Err(error) = store_last_checkpoint_attempt(&self.config.db_path, block_time_ms) {
            tracing::warn!(
                ?error,
                block_time_ms,
                "failed to persist the checkpoint attempt record; a restart inside this \
                 interval may checkpoint off the network's schedule"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_record_means_no_attempt() {
        let directory = tempfile::tempdir().expect("temporary directory");

        assert_eq!(load_last_checkpoint_attempt(directory.path()), 0);
    }

    #[test]
    fn stored_record_round_trips_and_overwrites() {
        let directory = tempfile::tempdir().expect("temporary directory");

        store_last_checkpoint_attempt(directory.path(), 1_700_000_000_123).expect("store");
        assert_eq!(
            load_last_checkpoint_attempt(directory.path()),
            1_700_000_000_123
        );

        store_last_checkpoint_attempt(directory.path(), 1_700_000_600_456).expect("overwrite");
        assert_eq!(
            load_last_checkpoint_attempt(directory.path()),
            1_700_000_600_456
        );
        assert!(
            !directory
                .path()
                .join("last_checkpoint_attempt.tmp")
                .exists(),
            "the temporary file is renamed into place"
        );
    }

    #[test]
    fn malformed_record_means_no_attempt() {
        let directory = tempfile::tempdir().expect("temporary directory");
        std::fs::write(
            last_checkpoint_attempt_path(directory.path()),
            "not a block time",
        )
        .expect("write");

        assert_eq!(load_last_checkpoint_attempt(directory.path()), 0);
    }
}
