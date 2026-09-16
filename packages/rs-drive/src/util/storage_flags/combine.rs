//! Combining storage flags when an element is replaced.
//!
//! The epoch arithmetic is the crate's. The typed owner of each side is
//! replaced by its carrier key for the duration of the crate call and mapped
//! back through the inputs afterwards, so the crate's owner comparison
//! (equality of 32-byte ids) keeps two different owners apart and the kind
//! of the winning owner is read from the input that supplied it, never from
//! the key.

use super::{CrateStorageFlags, MergingOwnersStrategy, StorageFlags};
use dpp::fee::refund_owner::RefundOwner;
use grovedb_costs::storage_cost::removal::StorageRemovedBytes;
use grovedb_epoch_based_storage_flags::error::StorageFlagsError;

impl StorageFlags {
    /// Optional combine added bytes
    pub fn optional_combine_added_bytes(
        ours: Option<Self>,
        theirs: Self,
        added_bytes: u32,
        merging_owners_strategy: MergingOwnersStrategy,
    ) -> Result<Self, StorageFlagsError> {
        match ours {
            None => Ok(theirs),
            Some(ours) => ours.combine_added_bytes(theirs, added_bytes, merging_owners_strategy),
        }
    }

    /// Optional combine removed bytes
    pub fn optional_combine_removed_bytes(
        ours: Option<Self>,
        theirs: Self,
        removed_bytes: &StorageRemovedBytes,
        merging_owners_strategy: MergingOwnersStrategy,
    ) -> Result<Self, StorageFlagsError> {
        match ours {
            None => Ok(theirs),
            Some(ours) => {
                ours.combine_removed_bytes(theirs, removed_bytes, merging_owners_strategy)
            }
        }
    }

    /// Combine added bytes
    pub fn combine_added_bytes(
        self,
        rhs: Self,
        added_bytes: u32,
        merging_owners_strategy: MergingOwnersStrategy,
    ) -> Result<Self, StorageFlagsError> {
        let ours_owner = self.refund_owner();
        let theirs_owner = rhs.refund_owner();
        Self::reject_colliding_owners(ours_owner, theirs_owner)?;
        let combined = self
            .to_crate_flags_keyed_by_removal_key()
            .combine_added_bytes(
                rhs.to_crate_flags_keyed_by_removal_key(),
                added_bytes,
                merging_owners_strategy,
            )?;
        Self::from_crate_combined(combined, ours_owner, theirs_owner)
    }

    /// Combine removed bytes
    pub fn combine_removed_bytes(
        self,
        rhs: Self,
        removed_bytes: &StorageRemovedBytes,
        merging_owners_strategy: MergingOwnersStrategy,
    ) -> Result<Self, StorageFlagsError> {
        let ours_owner = self.refund_owner();
        let theirs_owner = rhs.refund_owner();
        Self::reject_colliding_owners(ours_owner, theirs_owner)?;
        let combined = self
            .to_crate_flags_keyed_by_removal_key()
            .combine_removed_bytes(
                rhs.to_crate_flags_keyed_by_removal_key(),
                removed_bytes,
                merging_owners_strategy,
            )?;
        Self::from_crate_combined(combined, ours_owner, theirs_owner)
    }

    /// Two different typed owners with one carrier key can never be told
    /// apart by the crate, so the merge is refused before it starts.
    fn reject_colliding_owners(
        ours: Option<RefundOwner>,
        theirs: Option<RefundOwner>,
    ) -> Result<(), StorageFlagsError> {
        if let (Some(ours), Some(theirs)) = (ours, theirs) {
            if ours != theirs && ours.removal_key() == theirs.removal_key() {
                return Err(StorageFlagsError::MergingStorageFlagsFromDifferentOwners(
                    "two different refund owners share one storage removal carrier key".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Maps the crate's combined flags back to the typed owner that supplied
    /// the winning carrier key.
    fn from_crate_combined(
        combined: CrateStorageFlags,
        ours: Option<RefundOwner>,
        theirs: Option<RefundOwner>,
    ) -> Result<Self, StorageFlagsError> {
        let Some(key) = combined.owner_id().copied() else {
            return Ok(Self::from(combined));
        };
        let owner = [ours, theirs]
            .into_iter()
            .flatten()
            .find(|owner| owner.removal_key() == key)
            .ok_or_else(|| {
                StorageFlagsError::MergingStorageFlagsFromDifferentOwners(
                    "combined storage flags name an owner that neither side supplied".to_string(),
                )
            })?;
        match owner {
            RefundOwner::Identity(_) => Ok(Self::from(combined)),
            RefundOwner::ContractBucket {
                contract_id,
                position,
            } => {
                let base_epoch = *combined.base_epoch();
                Ok(match combined.epoch_index_map() {
                    None => StorageFlags::SingleEpochContractBucket(
                        base_epoch,
                        contract_id.to_buffer(),
                        position,
                    ),
                    Some(epochs) => StorageFlags::MultiEpochContractBucket(
                        base_epoch,
                        epochs.clone(),
                        contract_id.to_buffer(),
                        position,
                    ),
                })
            }
        }
    }
}
