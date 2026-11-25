use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use arc_swap::ArcSwap;
use dpp::prelude::{BlockHeight, TimestampMillis};

use super::storage_engine::{
    no_entry_says_a_database, open_grove, remove_checkpoint_leftover, Role,
};
use crate::config::DriveConfig;
use crate::drive::{Checkpoint, CheckpointInfo, CheckpointsMap};
use crate::error::Error;

/// Loads existing checkpoints from the checkpoints directory.
///
/// This function scans the `<db_path>/checkpoints/` directory for existing checkpoint
/// subdirectories (named by block height), opens each one as a checkpoint on the storage
/// engine of `config`, and returns an ArcSwap containing the loaded checkpoints. A directory
/// that holds no database of that engine is skipped, and none is made in it. Each skip is
/// logged with the directory: as a warning with the error of the open, or as information
/// where no entry of the directory says a database, which every start meets again.
///
/// On LigninDB, what a creation or a deletion of a checkpoint that was cut short left in
/// the directory is removed first, by the engine.
///
/// # Arguments
/// * `db_path` - The path to the database directory (parent of the checkpoints directory)
/// * `config` - The configuration the database was opened with
///
/// # Returns
/// * An `ArcSwap` containing a `BTreeMap` of checkpoints keyed by block height
pub fn load_current_checkpoints<P: AsRef<Path>>(
    db_path: P,
    config: &DriveConfig,
) -> Result<CheckpointsMap, Error> {
    let checkpoints_dir = db_path.as_ref().join("checkpoints");

    let mut checkpoints = BTreeMap::new();

    // If the checkpoints directory doesn't exist, return an empty map
    if !checkpoints_dir.exists() {
        return Ok(ArcSwap::from_pointee(checkpoints));
    }

    // Read the checkpoints directory
    let entries = match std::fs::read_dir(&checkpoints_dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(ArcSwap::from_pointee(checkpoints)),
    };

    for entry in entries.flatten() {
        let path = entry.path();

        // Skip if not a directory
        if !path.is_dir() {
            continue;
        }

        if let Some(removed) = remove_checkpoint_leftover(&path, config) {
            match removed {
                Ok(()) => tracing::info!(
                    "removed the directory {}, left by a creation or a deletion of a \
                     checkpoint that was cut short",
                    path.display()
                ),
                Err(error) => tracing::warn!(
                    %error,
                    "the directory {}, left by a creation or a deletion of a checkpoint that \
                     was cut short, could not be removed",
                    path.display()
                ),
            }
            continue;
        }

        // Try to parse the directory name as a block height
        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => continue,
        };

        let block_height: BlockHeight = match file_name.parse() {
            Ok(height) => height,
            Err(_) => continue,
        };

        // Try to open the checkpoint as a GroveDb
        let grove_db = match open_grove(&path, config, Role::Checkpoint) {
            Ok(db) => db,
            Err(error) => {
                if no_entry_says_a_database(&path) {
                    tracing::info!(
                        "the checkpoint directory {} holds no database and is skipped",
                        path.display()
                    );
                } else {
                    tracing::warn!(
                        %error,
                        "the checkpoint directory {} could not be opened as a checkpoint and \
                         is skipped",
                        path.display()
                    );
                }
                continue;
            }
        };

        // Get the modification time as the checkpoint timestamp
        // If we can't get the metadata, use 0 as a fallback. The node replaces it with the
        // time of the checkpoint's block where it has the state saved with the checkpoint.
        let timestamp_ms: TimestampMillis = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as TimestampMillis)
            .unwrap_or(0);

        let checkpoint = Checkpoint::new(grove_db, path);
        checkpoints.insert(
            block_height,
            CheckpointInfo::new(timestamp_ms, Arc::new(checkpoint)),
        );
    }

    Ok(ArcSwap::from_pointee(checkpoints))
}
