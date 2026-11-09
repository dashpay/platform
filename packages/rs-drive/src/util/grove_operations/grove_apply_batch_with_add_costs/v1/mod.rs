use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::query::GroveError;
use crate::util::batch::GroveDbOpBatch;
use crate::util::storage_flags::StorageFlags;
use grovedb::{ElementFlags, TransactionArg};
use grovedb_costs::storage_cost::transition::ElementFlagsUpdate;
use grovedb_costs::storage_cost::StorageCost;
use grovedb_epoch_based_storage_flags::error::StorageFlagsError;
use platform_version::version::drive_versions::DriveVersion;
use std::collections::BTreeSet;

impl Drive {
    /// Generation 0 with a flags update that follows the owner the written flags name: see
    /// [`update_element_flags_for_owner_change`].
    pub(super) fn grove_apply_batch_with_add_costs_v1(
        &self,
        ops: GroveDbOpBatch,
        validate: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let full_handover_owners = ops.full_handover_owners.clone();
        self.grove_apply_batch_with_flags_update(
            ops,
            validate,
            |cost, old_flags, new_flags| {
                update_element_flags_for_owner_change(
                    cost,
                    old_flags,
                    new_flags,
                    &full_handover_owners,
                )
                .map_err(|e| GroveError::JustInTimeElementFlagsClientError(e.to_string()))
            },
            transaction,
            drive_operations,
            drive_version,
        )
    }
}

/// The flags update of a stored element whose new flags may name a new owner.
///
/// An owner change involving one of `full_handover_owners` refunds the old owner for the whole
/// element and charges the payer for all of it, so a document's owner cannot keep storage a
/// gas sponsor paid for, nor a sponsor what the owner paid for. Any other owner change keeps
/// the stored bytes and moves them to the new owner, as a transfer or a purchase does.
fn update_element_flags_for_owner_change(
    cost: &StorageCost,
    old_flags: Option<ElementFlags>,
    new_flags: &mut ElementFlags,
    full_handover_owners: &BTreeSet<[u8; 32]>,
) -> Result<ElementFlagsUpdate, StorageFlagsError> {
    let names_full_handover_owner = |flags: &ElementFlags| -> Result<bool, StorageFlagsError> {
        Ok(
            StorageFlags::from_element_flags_ref(flags)?.is_some_and(|flags| {
                flags
                    .owner_id()
                    .is_some_and(|owner| full_handover_owners.contains(owner))
            }),
        )
    };
    let is_full_handover = !full_handover_owners.is_empty()
        && (names_full_handover_owner(new_flags)?
            || old_flags
                .as_ref()
                .map(names_full_handover_owner)
                .transpose()?
                .unwrap_or(false));
    if is_full_handover {
        StorageFlags::update_element_flags_replacing_on_owner_change(cost, old_flags, new_flags)
    } else {
        StorageFlags::update_element_flags_with_written_owner(cost, old_flags, new_flags)
            .map(ElementFlagsUpdate::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use grovedb_costs::storage_cost::removal::StorageRemovedBytes::NoStorageRemoval;

    const OLD_OWNER: [u8; 32] = [1; 32];
    const NEW_OWNER: [u8; 32] = [2; 32];
    const SPONSOR: [u8; 32] = [3; 32];

    fn flags(epoch: u16, owner: [u8; 32]) -> ElementFlags {
        StorageFlags::new_single_epoch(epoch, Some(owner)).to_element_flags()
    }

    fn update(old_owner: [u8; 32], new_owner: [u8; 32]) -> (ElementFlagsUpdate, StorageFlags) {
        let same_size = StorageCost {
            added_bytes: 0,
            replaced_bytes: 10,
            removed_bytes: NoStorageRemoval,
        };
        let mut new_flags = flags(2, new_owner);
        let answer = update_element_flags_for_owner_change(
            &same_size,
            Some(flags(1, old_owner)),
            &mut new_flags,
            &BTreeSet::from([SPONSOR]),
        )
        .expect("expected the flags to update");
        let written = StorageFlags::from_element_flags_ref(&new_flags)
            .expect("expected flags that decode")
            .expect("expected flags");
        (answer, written)
    }

    #[test]
    fn should_keep_the_stored_bytes_and_change_their_owner_when_no_sponsor_is_involved() {
        let (answer, written) = update(OLD_OWNER, NEW_OWNER);

        assert_eq!(answer, ElementFlagsUpdate::Changed);
        assert_eq!(written.owner_id(), Some(&NEW_OWNER));
        assert_eq!(
            written.base_epoch(),
            &1,
            "the stored bytes keep their epoch"
        );
    }

    #[test]
    fn should_account_a_replacement_when_the_old_owner_is_a_sponsor() {
        let (answer, written) = update(SPONSOR, NEW_OWNER);

        assert_eq!(answer, ElementFlagsUpdate::AccountAsReplacement);
        assert_eq!(written.owner_id(), Some(&NEW_OWNER));
        assert_eq!(written.base_epoch(), &2, "the new owner's bytes start now");
    }

    #[test]
    fn should_account_a_replacement_when_the_new_owner_is_a_sponsor() {
        let (answer, written) = update(OLD_OWNER, SPONSOR);

        assert_eq!(answer, ElementFlagsUpdate::AccountAsReplacement);
        assert_eq!(written.owner_id(), Some(&SPONSOR));
    }

    #[test]
    fn should_not_account_a_replacement_when_a_sponsor_keeps_its_element() {
        let (answer, written) = update(SPONSOR, SPONSOR);

        assert_ne!(answer, ElementFlagsUpdate::AccountAsReplacement);
        assert_eq!(written.owner_id(), Some(&SPONSOR));
        assert_eq!(written.base_epoch(), &1);
    }
}
