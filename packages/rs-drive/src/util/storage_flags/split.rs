//! Splitting removed bytes across epochs and owners when an element shrinks
//! or is deleted.

use super::StorageFlags;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::fee::refund_owner::{RefundOwner, RefundOwnersByIdentifier, SYSTEM_REFUND_CARRIER_KEY};
use grovedb::ElementFlags;
use grovedb_costs::storage_cost::removal::StorageRemovedBytes;
use grovedb_costs::storage_cost::removal::StorageRemovedBytes::BasicStorageRemoval;

impl StorageFlags {
    /// Sections removed bytes per epoch, taking from the latest epochs
    /// first, under the owner's carrier key (or the system key when the
    /// bytes are unowned).
    ///
    /// The carrier key alone does not say what kind of owner it names; use
    /// [`Self::split_removal_bytes_typed`] on a path that must route the
    /// refund, so that the owner is recorded alongside.
    pub fn split_storage_removed_bytes(
        &self,
        removed_key_bytes: u32,
        removed_value_bytes: u32,
    ) -> (StorageRemovedBytes, StorageRemovedBytes) {
        self.to_crate_flags_keyed_by_removal_key()
            .split_storage_removed_bytes(removed_key_bytes, removed_value_bytes)
    }

    /// The batch split closure for typed owners.
    ///
    /// Decodes all six flag types, sections the removed bytes under the
    /// owner's carrier key and records `(carrier key, owner)` in
    /// `refund_owners` for every owned removal, identities included. Within
    /// one batch a carrier key may name only one owner; a second owner for
    /// the same key, or a bucket whose key equals the system key, fails the
    /// batch. Nothing is guessed and nothing is re-derived.
    pub fn split_removal_bytes_typed(
        flags: &mut ElementFlags,
        removed_key_bytes: u32,
        removed_value_bytes: u32,
        refund_owners: &mut RefundOwnersByIdentifier,
    ) -> Result<(StorageRemovedBytes, StorageRemovedBytes), Error> {
        let maybe_storage_flags = Self::from_element_flags_ref(flags)?;
        match maybe_storage_flags {
            None => Ok((
                BasicStorageRemoval(removed_key_bytes),
                BasicStorageRemoval(removed_value_bytes),
            )),
            Some(storage_flags) => {
                if let Some(owner) = storage_flags.refund_owner() {
                    Self::record_refund_owner(owner, refund_owners)?;
                }
                Ok(storage_flags
                    .into_crate_flags_keyed_by_removal_key()
                    .split_storage_removed_bytes(removed_key_bytes, removed_value_bytes))
            }
        }
    }

    fn record_refund_owner(
        owner: RefundOwner,
        refund_owners: &mut RefundOwnersByIdentifier,
    ) -> Result<(), Error> {
        let key = owner.removal_key();
        if key == SYSTEM_REFUND_CARRIER_KEY {
            return match owner {
                // The all-zero identity has always been sectioned as system
                // bytes and never refunded; recording nothing keeps that.
                RefundOwner::Identity(_) => Ok(()),
                RefundOwner::ContractBucket { .. } => {
                    Err(Error::Drive(DriveError::CorruptedCodeExecution(
                        "a contract bucket refund owner derived the system carrier key",
                    )))
                }
            };
        }
        match refund_owners.get(&key) {
            Some(existing) if *existing != owner => {
                Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "two different refund owners share one storage removal carrier key within a batch",
                )))
            }
            Some(_) => Ok(()),
            None => {
                refund_owners.insert(key, owner);
                Ok(())
            }
        }
    }
}
