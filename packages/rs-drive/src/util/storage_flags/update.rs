//! Updating the flags of an element that is replaced in a batch.

use super::{CrateStorageFlags, MergingOwnersStrategy, StorageFlags};
use crate::error::Error;
use grovedb::ElementFlags;
use grovedb_costs::storage_cost::transition::OperationStorageTransitionType;
use grovedb_costs::storage_cost::StorageCost;
use grovedb_epoch_based_storage_flags::error::StorageFlagsError;

impl StorageFlags {
    /// The batch flag update closure that shipped with identity-only owners.
    ///
    /// Hands the raw bytes to the crate, so a contract bucket type byte is
    /// rejected as an unknown flags type.
    pub fn update_element_flags(
        cost: &StorageCost,
        old_flags: Option<ElementFlags>,
        new_flags: &mut ElementFlags,
    ) -> Result<bool, StorageFlagsError> {
        CrateStorageFlags::update_element_flags(cost, old_flags, new_flags)
    }

    /// The batch flag update closure for typed owners: the crate's rule
    /// expressed over the typed flags.
    ///
    /// A replace merges the old flags into the new ones with `UseTheirs`, so
    /// new flags naming a different owner transfer the bytes to that owner.
    /// That holds across kinds: an identity can hand bytes to a contract
    /// bucket and a bucket to an identity.
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
                Ok(Self::replace_if_changed(combined_storage_flags, new_flags))
            }
            OperationStorageTransitionType::OperationUpdateSmallerSize => {
                let combined_storage_flags = old_storage_flags.combine_removed_bytes(
                    new_storage_flags,
                    &cost.removed_bytes,
                    MergingOwnersStrategy::UseTheirs,
                )?;
                Ok(Self::replace_if_changed(combined_storage_flags, new_flags))
            }
            OperationStorageTransitionType::OperationUpdateSameSize => {
                // a same-size update keeps the old flags
                *new_flags = old_storage_flags.to_element_flags();
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn replace_if_changed(combined: StorageFlags, new_flags: &mut ElementFlags) -> bool {
        let combined_flags = combined.to_element_flags();
        if combined_flags == *new_flags {
            false
        } else {
            *new_flags = combined_flags;
            true
        }
    }
}
