//! Reads of how a document's index entry is stored, for the walkers that
//! remove or refresh an entry an earlier protocol version may have laid out
//! otherwise than the current rule.
//!
//! The rule (protocol version 14 on): a unique index holds a document's
//! entry as the bare reference at key `[0]` of its last value tree when none
//! of the index's own values is missing, and in the `[0]` tree, keyed by
//! document id, when one is; a `nullSearchable: false` index holds no entry
//! for a document missing all of its values. Earlier versions wrote entries
//! that disagree with it:
//!
//! - the index-level insert walkers carried the null flags from one sibling
//!   sub-level into the next, so a missing value in one index's branch put a
//!   later sibling's unique entry in the `[0]` tree, and gave a
//!   `nullSearchable: false` index an entry because a sibling had a value;
//! - the replace walker (update v0) wrote a unique index's entry as the bare
//!   reference unless all of its values were missing, and wrote an entry for
//!   a `nullSearchable: false` index whatever its values.
//!
//! # Billing
//!
//! These reads are bookkeeping, unbilled like the time-range walkers'
//! removability reads (see `time_range_ttl.rs`): their costs go to a scratch
//! vector that is dropped. A dry run cannot read state, so billing them on
//! execution only would let a transition pass fee validation and then
//! overdraw on apply.

use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use grovedb::TransactionArg;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// Whether the entry of an index under `value_path` (the value tree of
    /// the index's last property) is the `[0]` tree holding references by
    /// document id rather than the bare reference at `[0]`. `rule` is what
    /// the current rule says, answered when nothing is stored at `[0]`.
    pub(crate) fn stored_index_entry_is_tree(
        &self,
        value_path: &[Vec<u8>],
        rule: bool,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        let mut scratch_operations: Vec<LowLevelDriveOperation> = vec![];
        Ok(self
            .grove_get_raw_optional(
                value_path.into(),
                &[0],
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut scratch_operations,
                drive_version,
            )?
            .map_or(rule, |element| element.is_any_tree()))
    }

    /// Whether the `[0]` tree under `value_path` holds an entry for
    /// `document_id`: for an index the rule gives no entry, whether an
    /// earlier version wrote one anyway. A bare reference or nothing at
    /// `[0]` holds none.
    pub(crate) fn index_entry_in_tree_exists(
        &self,
        value_path: &[Vec<u8>],
        document_id: &[u8],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        if !self.stored_index_entry_is_tree(value_path, false, transaction, drive_version)? {
            return Ok(false);
        }
        let mut entry_path = value_path.to_vec();
        entry_path.push(vec![0]);
        let mut scratch_operations: Vec<LowLevelDriveOperation> = vec![];
        Ok(self
            .grove_get_raw_optional(
                entry_path.as_slice().into(),
                document_id,
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut scratch_operations,
                drive_version,
            )?
            .is_some())
    }
}
