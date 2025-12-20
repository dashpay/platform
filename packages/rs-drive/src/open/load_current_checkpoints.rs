use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use arc_swap::ArcSwap;
use dpp::prelude::{BlockHeight, TimestampMillis};
use grovedb::GroveDb;

use crate::drive::Checkpoint;
use crate::error::Error;

/// Loads existing checkpoints from the checkpoints directory.
///
/// This function scans the `<db_path>/checkpoints/` directory for existing checkpoint
/// subdirectories (named by block height), opens each one as a GroveDb, and returns
/// an ArcSwap containing the loaded checkpoints.
///
/// # Arguments
/// * `db_path` - The path to the database directory (parent of the checkpoints directory)
///
/// # Returns
/// * An `ArcSwap` containing a `BTreeMap` of checkpoints keyed by block height
pub fn load_current_checkpoints<P: AsRef<Path>>(
    db_path: P,
) -> Result<ArcSwap<BTreeMap<BlockHeight, (TimestampMillis, Arc<Checkpoint>)>>, Error> {
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
        let grove_db = match GroveDb::open(&path) {
            Ok(db) => db,
            Err(_) => continue, // Skip corrupted or invalid checkpoints
        };

        // Get the modification time as the checkpoint timestamp
        // If we can't get the metadata, use 0 as a fallback
        let timestamp_ms: TimestampMillis = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as TimestampMillis)
            .unwrap_or(0);

        let checkpoint = Checkpoint::new(grove_db, path);
        checkpoints.insert(block_height, (timestamp_ms, Arc::new(checkpoint)));
    }

    Ok(ArcSwap::from_pointee(checkpoints))
}
