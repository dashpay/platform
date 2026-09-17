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
            .into_crate_flags_keyed_by_removal_key()
            .combine_added_bytes(
                rhs.into_crate_flags_keyed_by_removal_key(),
                added_bytes,
                merging_owners_strategy,
            )?;
        Self::from_crate_combined(combined, ours_owner, theirs_owner)
    }

    /// Combine removed bytes
    ///
    /// When a single epoch element shrinks in a later epoch there is no epoch
    /// map to subtract from and the crate returns the old flags untouched,
    /// before it looks at the merging strategy. The typed path resolves the
    /// owner itself in that case so that `UseTheirs` transfers ownership on a
    /// shrinking replace exactly as it does on a growing one.
    pub fn combine_removed_bytes(
        self,
        rhs: Self,
        removed_bytes: &StorageRemovedBytes,
        merging_owners_strategy: MergingOwnersStrategy,
    ) -> Result<Self, StorageFlagsError> {
        let ours_owner = self.refund_owner();
        let theirs_owner = rhs.refund_owner();
        Self::reject_colliding_owners(ours_owner, theirs_owner)?;
        if self.epoch_index_map().is_none() && self.base_epoch() < rhs.base_epoch() {
            let owner = Self::resolve_owner(ours_owner, theirs_owner, merging_owners_strategy)?;
            return Ok(Self::new_single_epoch_for_owner(*self.base_epoch(), owner));
        }
        let combined = self
            .into_crate_flags_keyed_by_removal_key()
            .combine_removed_bytes(
                rhs.into_crate_flags_keyed_by_removal_key(),
                removed_bytes,
                merging_owners_strategy,
            )?;
        Self::from_crate_combined(combined, ours_owner, theirs_owner)
    }

    /// Combine for a replace that moved no bytes: the epochs stay ours and
    /// the owner follows the merging strategy.
    ///
    /// This is the branch GroveDB reaches when a replace nets to the same
    /// size. It must resolve ownership exactly as the added and removed
    /// branches do, because GroveDB may price one replace as a shrink first
    /// and as same size after the flags changed width; two different answers
    /// would never converge.
    pub fn combine_same_size(
        self,
        rhs: Self,
        merging_owners_strategy: MergingOwnersStrategy,
    ) -> Result<Self, StorageFlagsError> {
        let ours_owner = self.refund_owner();
        let theirs_owner = rhs.refund_owner();
        Self::reject_colliding_owners(ours_owner, theirs_owner)?;
        let owner = Self::resolve_owner(ours_owner, theirs_owner, merging_owners_strategy)?;
        let (base_epoch, epochs, _) = self.into_parts();
        Ok(Self::from_parts(base_epoch, epochs, owner))
    }

    /// The crate's owner rule over typed owners: a side without an owner
    /// yields to the other, equal owners keep, different owners follow the
    /// strategy.
    fn resolve_owner(
        ours: Option<RefundOwner>,
        theirs: Option<RefundOwner>,
        merging_owners_strategy: MergingOwnersStrategy,
    ) -> Result<Option<RefundOwner>, StorageFlagsError> {
        match (ours, theirs) {
            (None, theirs) => Ok(theirs),
            (ours, None) => Ok(ours),
            (Some(ours), Some(theirs)) if ours == theirs => Ok(Some(ours)),
            (Some(ours), Some(theirs)) => match merging_owners_strategy {
                MergingOwnersStrategy::RaiseIssue => {
                    Err(StorageFlagsError::MergingStorageFlagsFromDifferentOwners(
                        "can not merge from different owners".to_string(),
                    ))
                }
                MergingOwnersStrategy::UseOurs => Ok(Some(ours)),
                MergingOwnersStrategy::UseTheirs => Ok(Some(theirs)),
            },
        }
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
        let (base_epoch, epochs) = match combined {
            CrateStorageFlags::SingleEpoch(base_epoch)
            | CrateStorageFlags::SingleEpochOwned(base_epoch, _) => (base_epoch, None),
            CrateStorageFlags::MultiEpoch(base_epoch, epochs)
            | CrateStorageFlags::MultiEpochOwned(base_epoch, epochs, _) => {
                (base_epoch, Some(epochs))
            }
        };
        Ok(Self::from_parts(base_epoch, epochs, Some(owner)))
    }
}
