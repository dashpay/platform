//! Updating the flags of an element that is replaced in a batch.

use super::{MergingOwnersStrategy, StorageFlags};
use crate::error::Error;
use grovedb::ElementFlags;
use grovedb_costs::storage_cost::transition::OperationStorageTransitionType;
use grovedb_costs::storage_cost::StorageCost;
use grovedb_epoch_based_storage_flags::error::StorageFlagsError;

impl StorageFlags {
    /// The batch flag update closure for typed owners: the crate's rule
    /// expressed over the typed flags.
    ///
    /// A replace merges the old flags into the new ones with `UseTheirs`, so
    /// new flags naming a different owner transfer the bytes to that owner.
    /// That holds across kinds: an identity can hand bytes to a contract
    /// bucket and a bucket to an identity, and it holds for every replace
    /// shape, including one that moves no bytes (the crate keeps the old
    /// owner there; this generation transfers, so that a replace GroveDB
    /// first priced as a shrink resolves the same way when it is priced
    /// again as same size).
    ///
    /// GroveDB prices a replace with the old flags attached and re-prices
    /// only when this closure reports a change, so a change of header width
    /// (35 bytes for an identity owner, 37 for a bucket owner) is reported
    /// even when the merged flags already equal the proposed ones.
    pub fn update_element_flags_typed(
        cost: &StorageCost,
        old_flags: Option<ElementFlags>,
        new_flags: &mut ElementFlags,
    ) -> Result<bool, Error> {
        // if there were no flags before then the new flags are used
        let Some(old_flags) = old_flags else {
            return Ok(false);
        };

        let maybe_old_storage_flags = Self::from_element_flags_ref(&old_flags)?;
        let new_storage_flags = Self::from_element_flags_ref(new_flags)?.ok_or(
            StorageFlagsError::RemovingFlagsError(
                "removing flags from an item with flags is not allowed".to_string(),
            ),
        )?;
        let Some(old_storage_flags) = maybe_old_storage_flags else {
            return Err(StorageFlagsError::RemovingFlagsError(
                "old storage flags missing during update".to_string(),
            )
            .into());
        };

        match &cost.transition_type() {
            OperationStorageTransitionType::OperationUpdateBiggerSize => {
                let combined_storage_flags = old_storage_flags.combine_added_bytes(
                    new_storage_flags,
                    cost.added_bytes,
                    MergingOwnersStrategy::UseTheirs,
                )?;
                Ok(Self::replace_and_report(
                    &old_flags,
                    combined_storage_flags,
                    new_flags,
                ))
            }
            OperationStorageTransitionType::OperationUpdateSmallerSize => {
                let combined_storage_flags = old_storage_flags.combine_removed_bytes(
                    new_storage_flags,
                    &cost.removed_bytes,
                    MergingOwnersStrategy::UseTheirs,
                )?;
                Ok(Self::replace_and_report(
                    &old_flags,
                    combined_storage_flags,
                    new_flags,
                ))
            }
            OperationStorageTransitionType::OperationUpdateSameSize => {
                let combined_storage_flags = old_storage_flags
                    .combine_same_size(new_storage_flags, MergingOwnersStrategy::UseTheirs)?;
                Ok(Self::replace_and_report(
                    &old_flags,
                    combined_storage_flags,
                    new_flags,
                ))
            }
            _ => Ok(false),
        }
    }

    /// Writes the combined flags over the proposed ones and reports whether
    /// GroveDB has to price the element again: when the bytes changed, or
    /// when the flags kept their bytes but differ in width from the old
    /// flags the replace was priced with.
    fn replace_and_report(
        old_flags: &ElementFlags,
        combined: StorageFlags,
        new_flags: &mut ElementFlags,
    ) -> bool {
        let combined_flags = combined.to_element_flags();
        let changed = combined_flags != *new_flags;
        if changed {
            *new_flags = combined_flags;
        }
        changed || old_flags.len() != new_flags.len()
    }
}
