//! Shared TTL semantics for time-range indexes — the walker-facing half
//! of `book/src/drive/time-range-ttl.md`.
//!
//! Three rules, one definition each:
//!
//! - **Writes never target expired buckets.** The insert walker filters
//!   its bucket keys through [`live_time_range_index_keys`] and the update
//!   walker its new entry keys through [`live_time_range_entry_keys`]: a
//!   document whose windows have all expired simply gets no entries under
//!   the TTL'd index, and never resurrects a dropped bucket. Consensus
//!   assigns `$createdAt` &co. from block time, so with `ttl >= range`
//!   only a re-inserted document with a historical timestamp (a contested
//!   document awarded after its window expired) ever hits the filter.
//! - **Removals touch an expired bucket only while it still stands.** The
//!   TTL drop is bucket-granular and lazy, so between a bucket's expiry
//!   and its drop a delete (or key-changing update) of one of its
//!   documents must still remove that document's entries — otherwise the
//!   bucket would carry dangling references until the drop. Once the
//!   bucket is gone the entries are gone with it, and per-entry removal
//!   must skip rather than fail. [`Drive::time_range_entry_state`] is that
//!   check: live bucket ⇒ always removable; expired bucket ⇒ removable
//!   exactly when it still exists. The existence read is deterministic —
//!   it reads consensus state.
//! - **Expiry has one definition**:
//!   [`TimeRangeTransform::bucket_expired`], shared by these helpers and
//!   the bucket-drop cleanup.
//!
//! # Timing
//!
//! Drainage never runs while a transition's operations are still queued:
//! the walkers emit a [`TimeRangeTtlDrainRequest`] per write into a TTL'd
//! level, and `apply_batch_low_level_drive_operations` runs the requests —
//! once per level — after the batch is applied. Draining mid-conversion
//! could remove subtrees that queued removals (an earlier document of the
//! same transition, or an earlier index sharing the level) still targeted.
//!
//! # Billing
//!
//! Nothing in this module bills the triggering user: drainage operations
//! and the walkers' TTL bookkeeping reads accumulate their costs into
//! local scratch vectors that are dropped, never into the caller's fee
//! operations. This is load-bearing for the `estimated >= actual` fee
//! invariant — the estimation dry run cannot read state, so it cannot
//! price state-dependent drainage, and billing it on execution only would
//! let a transition pass validation and then overdraw on apply. The work
//! itself is bounded (a capped count of drop operations applied as one
//! batch, plus a handful of bounded reads per write) and is system
//! maintenance of state nobody holds refunds against; the ephemeral-bytes
//! fee rate is where TTL writers pre-pay it in aggregate.

use crate::drive::document::index_level_tree_types::{
    index_level_tree_types_with_continuation_demotion, terminal_member_tree_type,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::{LowLevelDriveOperation, TimeRangeTtlDrainRequest};
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::util::batch::GroveDbOpBatch;
use crate::util::grove_operations::push_drive_operation_result;
use crate::util::grove_operations::DirectQueryType;
use crate::util::object_size_info::DriveKeyInfo;
use dpp::data_contract::document_type::{DocumentPropertyType, IndexLevel, TimeRangeTransform};
use dpp::version::drive_versions::DriveVersion;
use dpp::version::PlatformVersion;
use grovedb::batch::{QualifiedGroveDbOp, SubelementsDeletionBehavior};
use grovedb::query_result_type::QueryResultType;
use grovedb::{PathQuery, Query, SizedQuery, TransactionArg, TreeType};
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

/// Whether a stored entry key names an expired bucket at `block_time_ms`.
/// Keys without bucket-start semantics never expire.
fn entry_key_expired(transform: &TimeRangeTransform, entry_key: &[u8], block_time_ms: u64) -> bool {
    entry_key_bucket_start(entry_key)
        .is_some_and(|start| transform.bucket_expired(start, block_time_ms))
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
    entry_keys
        .into_iter()
        .filter(|key| !entry_key_expired(transform, key, block_time_ms))
        .collect()
}

/// [`live_time_range_entry_keys`] for the insert walker's key infos.
/// Size-only keys (the estimation dry run) always pass.
pub(crate) fn live_time_range_index_keys<'a>(
    transform: &TimeRangeTransform,
    index_keys: Vec<DriveKeyInfo<'a>>,
    block_time_ms: u64,
) -> Vec<DriveKeyInfo<'a>> {
    index_keys
        .into_iter()
        .filter(|key| match key {
            DriveKeyInfo::Key(key) => !entry_key_expired(transform, key, block_time_ms),
            DriveKeyInfo::KeyRef(key) => !entry_key_expired(transform, key, block_time_ms),
            DriveKeyInfo::KeySize(_) => true,
        })
        .collect()
}

/// What a removal walker finds for a time-range entry key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TimeRangeEntryState {
    /// A live (or non-bucket) key: remove as usual, no read performed.
    Live,
    /// The bucket is expired but still on disk — the window between
    /// expiry and its lazy drop, where the entry must still be removed,
    /// at full-path granularity (drainage may have taken its deeper
    /// trees already).
    ExpiredStanding,
    /// The bucket is gone, and the entry with it: nothing to remove.
    ExpiredGone,
}

impl Drive {
    /// Classifies the time-range entry at `entry_key` under the grid level
    /// at `level_path` for a removal walker — see [`TimeRangeEntryState`].
    /// Callers in estimation mode must not call this (state reads have no
    /// place in a dry run); they process every key, which keeps the dry
    /// run an upper bound.
    pub(crate) fn time_range_entry_state(
        &self,
        transform: &TimeRangeTransform,
        entry_key: &[u8],
        block_time_ms: u64,
        level_path: &[Vec<u8>],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<TimeRangeEntryState, Error> {
        if !entry_key_expired(transform, entry_key, block_time_ms) {
            return Ok(TimeRangeEntryState::Live);
        }
        // Unbilled bookkeeping read — see the module's Billing section.
        let mut scratch_operations: Vec<LowLevelDriveOperation> = vec![];
        let standing = self.grove_has_raw(
            SubtreePath::from(level_path),
            entry_key,
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut scratch_operations,
            &platform_version.drive,
        )?;
        Ok(if standing {
            TimeRangeEntryState::ExpiredStanding
        } else {
            TimeRangeEntryState::ExpiredGone
        })
    }

    /// Whether every segment of `path_segments` from index
    /// `known_prefix_len` on resolves, walked one `has_raw` at a time so a
    /// missing intermediate subtree answers `false` instead of erroring.
    /// The prefix below `known_prefix_len` is known to exist — the callers
    /// pass the index just past the bucket key, whose existence
    /// [`Self::time_range_entry_state`] already established.
    ///
    /// The removal walkers use this at full-path granularity for entries
    /// in expired-but-standing buckets: TTL drainage removes whole `[0]`
    /// trees and group value trees before the bucket itself goes, so an
    /// entry's deeper path can be gone while the bucket still stands.
    pub(crate) fn expired_entry_path_exists(
        &self,
        path_segments: &[Vec<u8>],
        known_prefix_len: usize,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        // Unbilled bookkeeping reads — see the module's Billing section.
        let mut scratch_operations: Vec<LowLevelDriveOperation> = vec![];
        let mut depth = known_prefix_len;
        while depth < path_segments.len() {
            if !self.grove_has_raw(
                SubtreePath::from(&path_segments[..depth]),
                path_segments[depth].as_slice(),
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut scratch_operations,
                &platform_version.drive,
            )? {
                return Ok(false);
            }
            depth += 1;
        }
        Ok(true)
    }

    /// Drain expired buckets from the grid level a request names — the
    /// lazy, budgeted cleanup every write into a TTL'd index continues.
    ///
    /// The drop primitive is grovedb's flat-subtree drop (grovedb#848 /
    /// PR #849): O(1) consensus removal of a subtree **declared to hold no
    /// child subtrees**, with its storage prefixes reclaimed outside
    /// consensus via range tombstones. A time-range bucket is NOT flat —
    /// it nests one property-name tree per remaining index level, value
    /// trees per distinct group, and `[0]` reference trees — so the drain
    /// works **deepest-first**, exactly one flat unit at a time:
    ///
    /// - a `[0]` reference tree holds only reference/Item elements → flat
    ///   drop (this is where the mass lives);
    /// - a value tree whose property-name children and `[0]` subtree are
    ///   gone holds at most bare elements → flat drop — except under a
    ///   ranked (indexed-primary) property-name tree, where grovedb
    ///   rightly refuses a generic child removal and the (by then empty)
    ///   tree leaves through the ordinary delete, which mirrors the
    ///   secondary;
    /// - a drained property-name tree → flat drop, which also dooms its
    ///   per-axis secondary prefixes when it was an indexed primary;
    /// - the emptied bucket itself → flat drop.
    ///
    /// The flat drops are batched: they are collected while the bucket is
    /// walked and applied as ONE grovedb batch at the end, so a drain
    /// costs a single root-hash propagation however many units it drops.
    /// Only the indexed-tree deletes have no batched form; a node under a
    /// ranked parent, and everything below it, is removed immediately
    /// instead (the batch applies afterwards, so a deferred ancestor never
    /// drops before an immediately removed descendant is gone).
    ///
    /// Every step is a deterministic function of consensus state and block
    /// time, and every step is O(1) in the subtree it drops — the *number*
    /// of steps is what scales with user data (one per group, per level,
    /// per bucket), and that is exactly what the request's
    /// `max_operations` bounds per write and level. A bucket drains across
    /// as many writes as it needs; between writes it stands partially
    /// drained, which TTL semantics allow (entries live *at most* `ttl`)
    /// and which the removal walkers handle at full-path granularity.
    ///
    /// The dropped paths embed their window start, so they are never
    /// re-created before their redo records drain (writes never target
    /// expired buckets) — the flat-drop path-reuse contract holds by
    /// construction. The host completes reclamation by calling
    /// `GroveDb::flush_pending_prefix_drops` after committing the block's
    /// transaction (and once at startup).
    pub(crate) fn drain_expired_time_range_buckets(
        &self,
        request: &TimeRangeTtlDrainRequest,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let TimeRangeTtlDrainRequest {
            transform,
            bucket_level,
            level_path,
            block_time_ms,
            max_operations,
        } = request;
        let Some(horizon) = transform.expiry_horizon_ms(*block_time_ms) else {
            return Ok(());
        };
        // Unbilled system maintenance — see the module's Billing section.
        let drive_operations: &mut Vec<LowLevelDriveOperation> = &mut vec![];
        let bucket_tree_type =
            index_level_tree_types_with_continuation_demotion(bucket_level)?.value_tree_type;
        let mut deferred_drops: Vec<QualifiedGroveDbOp> = vec![];
        let mut drained_buckets: Vec<Vec<u8>> = vec![];
        let mut budget = *max_operations;

        // Oldest expired bucket first. The null entry (empty key) sorts
        // below every bucket start and is excluded — null entries are not
        // windowed and live until their document goes. Only 8-byte keys
        // carry bucket semantics. With today's grammar no other key can
        // sort below the horizon anyway — the source is a required system
        // timestamp, so the level holds 8-byte bucket starts plus at most
        // the single null entry (empty key) — but the range start and the
        // wider limit keep the finder live even if a future grammar admits
        // raw (non-timestamp) keys: without them, low-sorting raw keys
        // could fill every result slot and stall drainage forever.
        let horizon_key = DocumentPropertyType::encode_date_timestamp(horizon);
        let mut below_horizon = Query::new();
        below_horizon.insert_range(vec![0u8; 8]..horizon_key);
        let path_query = PathQuery::new(
            level_path.clone(),
            SizedQuery::new(
                below_horizon,
                Some(max_operations.saturating_add(1).max(8)),
                None,
            ),
        );
        while budget > 0 {
            let (results, _) = self.grove_get_raw_path_query(
                &path_query,
                transaction,
                QueryResultType::QueryKeyElementPairResultType,
                drive_operations,
                drive_version,
            )?;
            // Deferred drops leave a fully drained bucket on disk until the
            // batch applies, so skip the ones this drain already took.
            let Some(bucket_key) = results.to_keys().into_iter().find(|key| {
                entry_key_bucket_start(key).is_some_and(|start| start < horizon)
                    && !drained_buckets.contains(key)
            }) else {
                break;
            };
            let mut bucket_path = level_path.clone();
            bucket_path.push(bucket_key.clone());
            let fully_drained = self.drain_expired_node(
                &bucket_path,
                bucket_level,
                bucket_tree_type,
                // The grid level is never an indexed primary — ranking the
                // bucketed level is rejected at contract validation.
                TreeType::NormalTree,
                false,
                &mut budget,
                &mut deferred_drops,
                transaction,
                drive_operations,
                drive_version,
            )?;
            if !fully_drained {
                break;
            }
            drained_buckets.push(bucket_key);
        }
        if !deferred_drops.is_empty() {
            self.apply_batch_grovedb_operations(
                None,
                transaction,
                GroveDbOpBatch::from_operations(deferred_drops),
                drive_operations,
                drive_version,
            )?;
        }
        Ok(())
    }

    /// Drain one tree of an expired bucket, deepest-first, then drop the
    /// tree itself. Returns whether the tree was fully removed (`false` ⇒
    /// the budget ran out mid-way; the next write resumes exactly here,
    /// because every completed step is a real removal once the deferred
    /// batch applies).
    ///
    /// `level` describes the merged contract-known structure below this
    /// tree (property-name children by level key); the value-tree children
    /// under each property-name tree are user data, enumerated once per
    /// visit up to the remaining budget.
    #[allow(clippy::too_many_arguments)]
    fn drain_expired_node(
        &self,
        node_path: &[Vec<u8>],
        level: &IndexLevel,
        node_tree_type: TreeType,
        parent_tree_type: TreeType,
        parent_removed_immediately: bool,
        budget: &mut u16,
        deferred_drops: &mut Vec<QualifiedGroveDbOp>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        let Some((node_key, parent_path)) = node_path.split_last() else {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "a drained time-range node always has a parent",
            )));
        };
        // A node under an indexed-primary parent leaves through grovedb's
        // dedicated indexed-tree delete, which has no batched form and must
        // find the node already empty — so it, and everything below it, is
        // removed immediately. Every other unit is a deferred flat drop.
        let removed_immediately =
            parent_removed_immediately || is_indexed_primary(parent_tree_type);

        // 1) Contract-known property-name children.
        for (level_key, sub_level) in level.sub_levels() {
            let level_key_bytes = level_key.as_bytes();
            let pn_element = self.grove_get_raw_optional(
                SubtreePath::from(node_path),
                level_key_bytes,
                DirectQueryType::StatefulDirectQuery,
                transaction,
                drive_operations,
                drive_version,
            )?;
            if pn_element.is_none() {
                continue;
            }
            let sub_tree_types = index_level_tree_types_with_continuation_demotion(sub_level)?;
            let mut pn_path = node_path.to_vec();
            pn_path.push(level_key_bytes.to_vec());
            // 1a) User-data value-tree children, enumerated once up to the
            // remaining budget: each costs at least one operation, so a
            // full page means the budget is spent before the page is, and
            // the next drain re-enumerates.
            if *budget == 0 {
                return Ok(false);
            }
            let mut all = Query::new();
            all.insert_all();
            let path_query =
                PathQuery::new(pn_path.clone(), SizedQuery::new(all, Some(*budget), None));
            let (results, _) = self.grove_get_raw_path_query(
                &path_query,
                transaction,
                QueryResultType::QueryKeyElementPairResultType,
                drive_operations,
                drive_version,
            )?;
            let value_keys = results.to_keys();
            let page_full = value_keys.len() == usize::from(*budget);
            for value_key in value_keys {
                let mut value_path = pn_path.clone();
                value_path.push(value_key);
                if !self.drain_expired_node(
                    &value_path,
                    sub_level,
                    sub_tree_types.value_tree_type,
                    sub_tree_types.property_name_tree_type,
                    removed_immediately,
                    budget,
                    deferred_drops,
                    transaction,
                    drive_operations,
                    drive_version,
                )? {
                    return Ok(false);
                }
            }
            if page_full {
                return Ok(false);
            }
            // 1b) The drained property-name tree — flat drop, which also
            // dooms its per-axis secondary prefixes when it was indexed.
            if *budget == 0 {
                return Ok(false);
            }
            self.drop_flat_unit(
                node_path,
                level_key_bytes,
                sub_tree_types.property_name_tree_type,
                removed_immediately,
                deferred_drops,
                transaction,
                drive_operations,
                drive_version,
            )?;
            *budget -= 1;
        }
        // 2) The terminal `[0]` reference tree, when an index terminates at
        // this level (non-unique / indexOnly layouts; the unique layout
        // stores the reference AT key `[0]` as a bare element, which the
        // flat drop of this node covers).
        if let Some(index_type) = level.has_index_with_type() {
            let zero_element = self.grove_get_raw_optional(
                SubtreePath::from(node_path),
                &[0],
                DirectQueryType::StatefulDirectQuery,
                transaction,
                drive_operations,
                drive_version,
            )?;
            if zero_element.is_some_and(|element| element.is_any_tree()) {
                if *budget == 0 {
                    return Ok(false);
                }
                self.drop_flat_unit(
                    node_path,
                    &[0],
                    terminal_member_tree_type(index_type),
                    removed_immediately,
                    deferred_drops,
                    transaction,
                    drive_operations,
                    drive_version,
                )?;
                *budget -= 1;
            }
        }
        // 3) The node itself. Under an indexed-primary parent grovedb
        // refuses every generic child removal (it could not mirror the
        // secondary), so the (by now drained) node leaves through the
        // dedicated indexed-tree delete matching the parent's tree type —
        // which mirrors the ordering value out of the secondary. Under a
        // plain parent the flat drop covers any remaining bare elements.
        if *budget == 0 {
            return Ok(false);
        }
        match parent_tree_type {
            TreeType::ProvableCountIndexedTree => {
                push_drive_operation_result(
                    self.grove.delete_from_count_indexed_tree(
                        SubtreePath::from(parent_path),
                        node_key,
                        transaction,
                        &drive_version.grove_version,
                    ),
                    drive_operations,
                )?;
            }
            TreeType::ProvableSumIndexedTree => {
                push_drive_operation_result(
                    self.grove.delete_from_provable_sum_indexed_tree(
                        SubtreePath::from(parent_path),
                        node_key,
                        transaction,
                        &drive_version.grove_version,
                    ),
                    drive_operations,
                )?;
            }
            TreeType::ProvableCountProvableSumIndexedTree => {
                push_drive_operation_result(
                    self.grove
                        .delete_from_provable_count_provable_sum_indexed_tree(
                            SubtreePath::from(parent_path),
                            node_key,
                            transaction,
                            &drive_version.grove_version,
                        ),
                    drive_operations,
                )?;
            }
            _ => {
                self.drop_flat_unit(
                    parent_path,
                    node_key,
                    node_tree_type,
                    removed_immediately,
                    deferred_drops,
                    transaction,
                    drive_operations,
                    drive_version,
                )?;
            }
        }
        *budget -= 1;
        Ok(true)
    }

    /// One flat-drop unit of a drain: executed right away when the unit
    /// sits under an immediately removed ancestor, deferred into the
    /// drain's single batch otherwise.
    #[allow(clippy::too_many_arguments)]
    fn drop_flat_unit(
        &self,
        path: &[Vec<u8>],
        key: &[u8],
        tree_type: TreeType,
        immediately: bool,
        deferred_drops: &mut Vec<QualifiedGroveDbOp>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        if immediately {
            return self.grove_drop_flat_subtree(
                path,
                key,
                transaction,
                drive_operations,
                drive_version,
            );
        }
        deferred_drops.push(QualifiedGroveDbOp::delete_tree_op(
            path.to_vec(),
            key.to_vec(),
            tree_type,
            SubelementsDeletionBehavior::DropFlat,
        ));
        Ok(())
    }

    /// Cost-pushing wrapper over [`GroveDb::drop_flat_subtree`] — the O(1)
    /// consensus detach of a flat subtree with staged prefix reclamation
    /// (grovedb#848). Version-gated inside grovedb itself: fail-closed
    /// below GROVE_V4, which platform reaches exactly when the TTL grammar
    /// exists (protocol v14's drive version carries GROVE_V4).
    pub(crate) fn grove_drop_flat_subtree(
        &self,
        path: &[Vec<u8>],
        key: &[u8],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let cost_context = self.grove.drop_flat_subtree(
            SubtreePath::from(path),
            key,
            transaction,
            &drive_version.grove_version,
        );
        push_drive_operation_result(cost_context, drive_operations)
    }
}

/// The three indexed-primary (ranked) tree types — the parents whose
/// children grovedb only removes through the dedicated indexed deletes.
fn is_indexed_primary(tree_type: TreeType) -> bool {
    matches!(
        tree_type,
        TreeType::ProvableCountIndexedTree
            | TreeType::ProvableSumIndexedTree
            | TreeType::ProvableCountProvableSumIndexedTree
    )
}
