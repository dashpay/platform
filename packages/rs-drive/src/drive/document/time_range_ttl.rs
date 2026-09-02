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
//! itself is bounded (a capped count of O(1) operations plus a handful of
//! bounded reads per write) and is system maintenance of state nobody
//! holds refunds against; the planned ephemeral-bytes fee rate is where
//! TTL writers pre-pay it in aggregate.

use crate::drive::document::index_level_tree_types::index_level_tree_types_with_continuation_demotion;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::push_drive_operation_result;
use crate::util::grove_operations::DirectQueryType;
use dpp::data_contract::document_type::{DocumentPropertyType, IndexLevel, TimeRangeTransform};
use dpp::version::PlatformVersion;
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
    pub(crate) fn time_range_entry_is_removable(
        &self,
        transform: &TimeRangeTransform,
        entry_key: &[u8],
        block_time_ms: u64,
        level_path: &[Vec<u8>],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        // Unbilled bookkeeping read — see the module's Billing section.
        let mut scratch_operations: Vec<LowLevelDriveOperation> = vec![];
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
            &mut scratch_operations,
            &platform_version.drive,
        )
    }

    /// Whether every segment of `path_segments` beyond the first
    /// `known_prefix_len` (a prefix known to exist — the contract's
    /// document-type path) resolves, walked one `has_raw` at a time so a
    /// missing intermediate subtree answers `false` instead of erroring.
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
            let parent_refs: Vec<&[u8]> = path_segments[..depth]
                .iter()
                .map(|segment| segment.as_slice())
                .collect();
            if !self.grove_has_raw(
                SubtreePath::from(parent_refs.as_slice()),
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

    /// Drain expired buckets from the grid level at `level_path` — the
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
    /// Every step is a deterministic function of consensus state and block
    /// time, and every step is O(1) — the *number* of steps is what scales
    /// with user data (one per group, per level, per bucket), and that is
    /// exactly what `max_operations` bounds per write. A bucket drains
    /// across as many writes as it needs; between writes it stands
    /// partially drained, which TTL semantics allow (entries live *at
    /// most* `ttl`) and which the removal walkers handle at full-path
    /// granularity.
    ///
    /// The dropped paths embed their window start, so they are never
    /// re-created before their redo records drain (writes never target
    /// expired buckets) — the flat-drop path-reuse contract holds by
    /// construction. The host completes reclamation by calling
    /// `GroveDb::flush_pending_prefix_drops` after committing the block's
    /// transaction (and once at startup).
    /// One budgeted drainage pass for every TTL'd time-range level of a
    /// document type. Levels are keyed by their grid-qualified storage key,
    /// so indexes sharing a grid share one level and drain exactly once per
    /// write — draining per *index* would multiply the per-write budget and,
    /// worse, interleave direct grovedb drops with already-queued batch
    /// mutations (a later index's drain can remove a path an earlier
    /// index's pending operation targets). Callers must therefore run this
    /// sweep BEFORE queuing any batch mutations, so every queued operation
    /// describes post-drain state. Stateful only — never call from an
    /// estimation dry run.
    pub(crate) fn drain_expired_time_range_levels(
        &self,
        index_level: &IndexLevel,
        contract_document_type_path: &[Vec<u8>],
        block_time_ms: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let Some(max_operations) = platform_version
            .system_limits
            .max_time_range_ttl_drop_operations_per_write
        else {
            return Ok(());
        };
        for (name, sub_level) in index_level.sub_levels() {
            if let Some(transform) = sub_level.time_range() {
                if transform.ttl_seconds.is_some() {
                    let mut level_path = contract_document_type_path.to_vec();
                    level_path.push(name.as_bytes().to_vec());
                    self.drain_expired_time_range_buckets(
                        transform,
                        sub_level,
                        &level_path,
                        block_time_ms,
                        max_operations,
                        transaction,
                        platform_version,
                    )?;
                }
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn drain_expired_time_range_buckets(
        &self,
        transform: &TimeRangeTransform,
        bucket_level: &IndexLevel,
        level_path: &[Vec<u8>],
        block_time_ms: u64,
        max_operations: u16,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let Some(horizon) = transform.expiry_horizon_ms(block_time_ms) else {
            return Ok(());
        };
        // Unbilled system maintenance — see the module's Billing section.
        let drive_operations: &mut Vec<LowLevelDriveOperation> = &mut vec![];
        let mut budget = max_operations;
        while budget > 0 {
            // Oldest expired bucket first. The null entry (empty key)
            // sorts below every bucket start and is excluded — null
            // entries are not windowed and live until their document goes.
            let horizon_key = DocumentPropertyType::encode_date_timestamp(horizon);
            let mut below_horizon = Query::new();
            // Only 8-byte keys carry bucket semantics. With today's grammar
            // no other key can sort below the horizon anyway — the source
            // is a required system timestamp, so the level holds 8-byte
            // bucket starts plus at most the single null entry (empty key)
            // — but the range start and the wider limit keep the finder
            // live even if a future grammar admits raw (non-timestamp)
            // keys: without them, low-sorting raw keys could fill every
            // result slot and stall drainage forever.
            below_horizon.insert_range(vec![0u8; 8]..horizon_key);
            let path_query = PathQuery::new(
                level_path.to_vec(),
                SizedQuery::new(below_horizon, Some(8), None),
            );
            let (results, _) = self.grove_get_raw_path_query(
                &path_query,
                transaction,
                QueryResultType::QueryKeyElementPairResultType,
                drive_operations,
                &platform_version.drive,
            )?;
            let Some(bucket_key) = results
                .to_key_elements()
                .into_iter()
                .map(|(key, _)| key)
                .find(|key| entry_key_bucket_start(key).is_some_and(|start| start < horizon))
            else {
                return Ok(());
            };
            let mut bucket_path = level_path.to_vec();
            bucket_path.push(bucket_key.clone());
            let fully_drained = self.drain_expired_node(
                &bucket_path,
                bucket_level,
                level_path,
                &bucket_key,
                // The grid level is never an indexed primary — ranking the
                // bucketed level is rejected at contract validation.
                TreeType::NormalTree,
                &mut budget,
                transaction,
                drive_operations,
                platform_version,
            )?;
            if !fully_drained {
                return Ok(());
            }
        }
        Ok(())
    }

    /// Drain one tree of an expired bucket, deepest-first, then drop the
    /// tree itself. Returns whether the tree was fully removed (`false` ⇒
    /// the budget ran out mid-way; the next write resumes exactly here,
    /// because every completed step is a real removal).
    ///
    /// `level` describes the merged contract-known structure below this
    /// tree (property-name children by level key); the value-tree children
    /// under each property-name tree are user data, enumerated one at a
    /// time.
    #[allow(clippy::too_many_arguments)]
    fn drain_expired_node(
        &self,
        node_path: &[Vec<u8>],
        level: &IndexLevel,
        parent_path: &[Vec<u8>],
        node_key: &[u8],
        parent_tree_type: TreeType,
        budget: &mut u16,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        let drive_version = &platform_version.drive;
        // 1) Contract-known property-name children.
        for (level_key, sub_level) in level.sub_levels() {
            let level_key_bytes = level_key.as_bytes();
            let node_path_refs: Vec<&[u8]> =
                node_path.iter().map(|segment| segment.as_slice()).collect();
            let pn_element = self.grove_get_raw_optional(
                SubtreePath::from(node_path_refs.as_slice()),
                level_key_bytes,
                DirectQueryType::StatefulDirectQuery,
                transaction,
                drive_operations,
                drive_version,
            )?;
            if pn_element.is_none() {
                continue;
            }
            let pn_tree_type = index_level_tree_types_with_continuation_demotion(sub_level)?
                .property_name_tree_type;
            let mut pn_path = node_path.to_vec();
            pn_path.push(level_key_bytes.to_vec());
            // 1a) User-data value-tree children, one at a time.
            loop {
                if *budget == 0 {
                    return Ok(false);
                }
                let mut all = Query::new();
                all.insert_all();
                let path_query =
                    PathQuery::new(pn_path.clone(), SizedQuery::new(all, Some(1), None));
                let (results, _) = self.grove_get_raw_path_query(
                    &path_query,
                    transaction,
                    QueryResultType::QueryKeyElementPairResultType,
                    drive_operations,
                    drive_version,
                )?;
                let Some((value_key, _)) = results.to_key_elements().into_iter().next() else {
                    break;
                };
                let mut value_path = pn_path.clone();
                value_path.push(value_key.clone());
                if !self.drain_expired_node(
                    &value_path,
                    sub_level,
                    &pn_path,
                    &value_key,
                    pn_tree_type,
                    budget,
                    transaction,
                    drive_operations,
                    platform_version,
                )? {
                    return Ok(false);
                }
            }
            // 1b) The drained property-name tree — flat drop, which also
            // dooms its per-axis secondary prefixes when it was indexed.
            if *budget == 0 {
                return Ok(false);
            }
            self.grove_drop_flat_subtree(
                node_path,
                level_key_bytes,
                transaction,
                drive_operations,
                platform_version,
            )?;
            *budget -= 1;
        }
        // 2) The terminal `[0]` reference tree, when this level hosts one
        // (non-unique / indexOnly layouts; the unique layout stores the
        // reference AT key `[0]` as a bare element, which the flat drop of
        // this node covers).
        let node_path_refs: Vec<&[u8]> =
            node_path.iter().map(|segment| segment.as_slice()).collect();
        let zero_element = self.grove_get_raw_optional(
            SubtreePath::from(node_path_refs.as_slice()),
            &[0],
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
            drive_version,
        )?;
        if let Some(element) = zero_element {
            if element.is_any_tree() {
                if *budget == 0 {
                    return Ok(false);
                }
                self.grove_drop_flat_subtree(
                    node_path,
                    &[0],
                    transaction,
                    drive_operations,
                    platform_version,
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
        let parent_path_refs: Vec<&[u8]> = parent_path
            .iter()
            .map(|segment| segment.as_slice())
            .collect();
        match parent_tree_type {
            TreeType::ProvableCountIndexedTree => {
                push_drive_operation_result(
                    self.grove.delete_from_count_indexed_tree(
                        SubtreePath::from(parent_path_refs.as_slice()),
                        node_key,
                        transaction,
                        &platform_version.drive.grove_version,
                    ),
                    drive_operations,
                )?;
            }
            TreeType::ProvableSumIndexedTree => {
                push_drive_operation_result(
                    self.grove.delete_from_provable_sum_indexed_tree(
                        SubtreePath::from(parent_path_refs.as_slice()),
                        node_key,
                        transaction,
                        &platform_version.drive.grove_version,
                    ),
                    drive_operations,
                )?;
            }
            TreeType::ProvableCountProvableSumIndexedTree => {
                push_drive_operation_result(
                    self.grove
                        .delete_from_provable_count_provable_sum_indexed_tree(
                            SubtreePath::from(parent_path_refs.as_slice()),
                            node_key,
                            transaction,
                            &platform_version.drive.grove_version,
                        ),
                    drive_operations,
                )?;
            }
            _ => {
                self.grove_drop_flat_subtree(
                    parent_path,
                    node_key,
                    transaction,
                    drive_operations,
                    platform_version,
                )?;
            }
        }
        *budget -= 1;
        Ok(true)
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
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let path_refs: Vec<&[u8]> = path.iter().map(|segment| segment.as_slice()).collect();
        let cost_context = self.grove.drop_flat_subtree(
            SubtreePath::from(path_refs.as_slice()),
            key,
            transaction,
            &platform_version.drive.grove_version,
        );
        push_drive_operation_result(cost_context, drive_operations)
    }
}
