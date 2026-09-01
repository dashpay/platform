//! Shared TTL semantics for time-range indexes — the walker-facing half
//! of `book/src/drive/time-range-ttl.md`.
//!
//! Three rules, one definition each:
//!
//! - **Writes never target expired buckets.** The insert path cannot
//!   produce one by construction (`$createdAt` &co. are consensus-assigned
//!   and validation requires `ttl >= range`), and the update path filters
//!   its new entry keys through [`live_time_range_entry_keys`] — an update
//!   of a document whose windows have all expired simply leaves it with no
//!   entries under the TTL'd index, and never resurrects a dropped bucket.
//! - **Removals touch an expired bucket only while it still stands.** The
//!   TTL drop is bucket-granular and lazy, so between a bucket's expiry
//!   and its drop a delete (or key-changing update) of one of its
//!   documents must still remove that document's entries — otherwise the
//!   bucket would carry dangling references until the drop. Once the
//!   bucket is gone the entries are gone with it, and per-entry removal
//!   must skip rather than fail. [`Drive::time_range_entry_is_removable`]
//!   is that check: live bucket ⇒ always removable; expired bucket ⇒
//!   removable exactly when it still exists. The existence read is
//!   deterministic — it reads consensus state.
//! - **Expiry has one definition**:
//!   [`TimeRangeTransform::expiry_horizon_ms`], shared by these helpers
//!   and the bucket-drop cleanup.

use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::push_drive_operation_result;
use crate::util::grove_operations::DirectQueryType;
use dpp::data_contract::document_type::{DocumentPropertyType, TimeRangeTransform};
use dpp::version::PlatformVersion;
use grovedb::operations::delete::DeleteOptions;
use grovedb::query_result_type::QueryResultType;
use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};
use grovedb_path::SubtreePath;

/// The bucket start a stored time-range entry key encodes, when it
/// encodes one. Mirrors the gate in
/// [`TimeRangeTransform::entry_keys_for_raw`]: only an exactly-8-byte key
/// is a bucket start — the null entry (empty key) and raw non-timestamp
/// keys have no expiry semantics and live until their document goes.
pub(crate) fn entry_key_bucket_start(entry_key: &[u8]) -> Option<u64> {
    (entry_key.len() == 8)
        .then(|| DocumentPropertyType::decode_date_timestamp(entry_key))
        .flatten()
}

/// Filter a derived time-range entry-key set down to the keys whose
/// bucket has not expired at `block_time_ms`. Keys without bucket-start
/// semantics (the null entry, raw keys) always pass; everything passes
/// when the transform declares no TTL.
pub(crate) fn live_time_range_entry_keys(
    transform: &TimeRangeTransform,
    entry_keys: Vec<Vec<u8>>,
    block_time_ms: u64,
) -> Vec<Vec<u8>> {
    let Some(horizon) = transform.expiry_horizon_ms(block_time_ms) else {
        return entry_keys;
    };
    entry_keys
        .into_iter()
        .filter(|key| entry_key_bucket_start(key).is_none_or(|start| start >= horizon))
        .collect()
}

impl Drive {
    /// Whether a removal walker should process the time-range entry at
    /// `entry_key` under the grid level at `level_path`.
    ///
    /// `true` for every live (or non-bucket) key with no read performed;
    /// for an expired key, `true` exactly when the bucket value tree still
    /// exists — the window between expiry and its lazy drop, where the
    /// document's entries are still on disk and must still be removed.
    /// Callers in estimation mode must not call this (state reads have no
    /// place in a dry run); they process every key, which keeps the dry
    /// run an upper bound.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn time_range_entry_is_removable(
        &self,
        transform: &TimeRangeTransform,
        entry_key: &[u8],
        block_time_ms: u64,
        level_path: &[Vec<u8>],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        let Some(horizon) = transform.expiry_horizon_ms(block_time_ms) else {
            return Ok(true);
        };
        let Some(start) = entry_key_bucket_start(entry_key) else {
            return Ok(true);
        };
        if start >= horizon {
            return Ok(true);
        }
        let path_refs: Vec<&[u8]> = level_path
            .iter()
            .map(|segment| segment.as_slice())
            .collect();
        self.grove_has_raw(
            SubtreePath::from(path_refs.as_slice()),
            entry_key,
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }

    /// Drop up to `max_drops` expired buckets from the grid level at
    /// `level_path` — the lazy cleanup a bucket-creating write triggers.
    ///
    /// Expired children are found oldest-first with a bounded range read
    /// below the TTL horizon; the null entry (empty key) sorts below every
    /// bucket start and is excluded — null entries are not windowed and
    /// live until their document goes. Everything here is deterministic:
    /// the horizon derives from block time, the read and the drops act on
    /// consensus state under the same transaction as the triggering write.
    ///
    /// PLACEHOLDER COST CLASS — grovedb#848: the drop currently runs
    /// grovedb's recursive element delete (correct: it removes the bucket
    /// subtree with everything nested, and deleting an indexed tree sweeps
    /// its per-axis secondaries), whose cost scales with the bucket's
    /// contents. The detach-and-sweep primitive specified in
    /// <https://github.com/dashpay/grovedb/issues/848> replaces the call
    /// below with an O(1) detach plus budgeted background reclamation;
    /// nothing else in this function changes.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn drop_expired_time_range_buckets(
        &self,
        transform: &TimeRangeTransform,
        level_path: &[Vec<u8>],
        block_time_ms: u64,
        max_drops: u16,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let Some(horizon) = transform.expiry_horizon_ms(block_time_ms) else {
            return Ok(());
        };
        if max_drops == 0 {
            return Ok(());
        }
        let horizon_key = DocumentPropertyType::encode_date_timestamp(horizon);
        let mut below_horizon = Query::new();
        below_horizon.insert_range_to(..horizon_key);
        let path_query = PathQuery::new(
            level_path.to_vec(),
            SizedQuery::new(below_horizon, Some(max_drops), None),
        );
        let (results, _) = self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            drive_operations,
            &platform_version.drive,
        )?;
        let path_refs: Vec<&[u8]> = level_path
            .iter()
            .map(|segment| segment.as_slice())
            .collect();
        for (key, _element) in results.to_key_elements() {
            // The horizon range read can only return keys below the first
            // bucket start when the level holds a null entry (empty key,
            // which sorts first) — skip anything that is not an expired
            // bucket start, defensively re-checking the decode.
            let Some(start) = entry_key_bucket_start(&key) else {
                continue;
            };
            if start >= horizon {
                continue;
            }
            let options = DeleteOptions {
                allow_deleting_non_empty_trees: true,
                deleting_non_empty_trees_returns_error: false,
                base_root_storage_is_free: true,
                validate_tree_at_path_exists: false,
            };
            let cost_context = self.grove.delete(
                SubtreePath::from(path_refs.as_slice()),
                key.as_slice(),
                Some(options),
                transaction,
                &platform_version.drive.grove_version,
            );
            push_drive_operation_result(cost_context, drive_operations)?;
        }
        Ok(())
    }
}
