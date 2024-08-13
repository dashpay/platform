use crate::drive::{identity_tree_path, Drive};
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::PathKeyInfo::PathFixedSizeKey;
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;

use crate::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDIdentityPublicKeyPairBTreeMap, KeyRequestType,
};
use crate::error::drive::DriveError;
use dpp::identity::identity_public_key::accessors::v0::{
    IdentityPublicKeyGettersV0, IdentityPublicKeySettersV0,
};
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use itertools::Itertools;
use std::collections::{BTreeSet, HashMap};

impl Drive {
    /// Adds a identity by inserting a new identity subtree structure to the `Identities` subtree.
    pub(super) fn add_new_identity_v0(
        &self,
        identity: Identity,
        is_masternode_identity: bool,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.add_new_identity_add_to_operations_v0(
            identity,
            is_masternode_identity,
            block_info,
            apply,
            &mut None,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let fees = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )?;
        Ok(fees)
    }

    /// Adds identity creation operations to drive operations
    pub(super) fn add_new_identity_add_to_operations_v0(
        &self,
        identity: Identity,
        is_masternode_identity: bool,
        block_info: &BlockInfo,
        apply: bool,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let batch_operations = self.add_new_identity_operations(
            identity,
            is_masternode_identity,
            block_info,
            previous_batch_operations,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;

        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            drive_operations,
            &platform_version.drive,
        )
    }

    /// The operations needed to create an identity
    pub(super) fn add_new_identity_operations_v0(
        &self,
        identity: Identity,
        is_masternode_identity: bool,
        block_info: &BlockInfo,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];

        // There is no reason to store the owner id as we always know the owner id of identity information.
        let storage_flags = StorageFlags::new_single_epoch(block_info.epoch.index, None);

        let identity_tree_path = identity_tree_path();

        let id = identity.id();
        let revision = identity.revision();
        let balance = identity.balance();
        let public_keys = identity.public_keys_owned();

        let apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_using_sums: false,
                is_sum_tree: false,
                flags_len: storage_flags.serialized_size(),
            }
        };

        // We insert the identity tree
        let inserted = self.batch_insert_empty_tree_if_not_exists(
            PathFixedSizeKey((identity_tree_path, id.to_vec())),
            false,
            Some(&storage_flags),
            apply_type,
            transaction,
            previous_batch_operations,
            &mut batch_operations,
            &platform_version.drive,
        )?;

        if !inserted {
            return if is_masternode_identity {
                // there could be a situation where we are trying to reenable a masternode identity that already existed
                // In this case we should reenable it's keys

                //we need to know what keys existed before
                let key_request = IdentityKeysRequest {
                    identity_id: id.to_buffer(),
                    request_type: KeyRequestType::AllKeys,
                    limit: None,
                    offset: None,
                };

                let old_masternode_identity_keys = self
                    .fetch_identity_keys::<KeyIDIdentityPublicKeyPairBTreeMap>(
                        key_request,
                        transaction,
                        platform_version,
                    )?;

                let mut last_key_id = *old_masternode_identity_keys.keys().max().unwrap();

                if old_masternode_identity_keys.is_empty() {
                    return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                        "expected old keys to exist if the masternode identity {} used to exist",
                        id
                    ))));
                }

                // we need the old key ids, this is why we do things like this

                let mut old_masternode_identity_keys_to_reenable =
                    old_masternode_identity_keys.values().collect::<Vec<_>>();

                old_masternode_identity_keys_to_reenable.retain(|old_public_key| {
                    public_keys
                        .values()
                        .map(|key| key.data())
                        .contains(old_public_key.data())
                });

                // we should find what keys should be re-enabled first
                batch_operations.append(
                    &mut self.re_enable_identity_keys_operations(
                        id.to_buffer(),
                        old_masternode_identity_keys_to_reenable
                            .iter()
                            .map(|identity_public_key| identity_public_key.id())
                            .collect(),
                        &block_info.epoch,
                        estimated_costs_only_with_layer_info,
                        transaction,
                        platform_version,
                    )?,
                );

                let old_masternode_identity_keys_to_reenable_data =
                    old_masternode_identity_keys_to_reenable
                        .iter()
                        .map(|key| key.data())
                        .collect::<BTreeSet<_>>();

                //we might also need to add new keys (in the case of an operator)

                for mut identity_public_key in public_keys.into_values() {
                    if old_masternode_identity_keys_to_reenable_data
                        .contains(identity_public_key.data())
                    {
                        // this was reenabled
                        continue;
                    }
                    last_key_id += 1;
                    identity_public_key.set_id(last_key_id);
                    self.insert_new_non_unique_key_operations(
                        id.to_buffer(),
                        identity_public_key,
                        false,
                        true,
                        &block_info.epoch,
                        estimated_costs_only_with_layer_info,
                        transaction,
                        &mut batch_operations,
                        platform_version,
                    )?;
                }
                Ok(batch_operations)
            } else {
                Err(Error::Identity(IdentityError::IdentityAlreadyExists(
                    "trying to insert an identity that already exists",
                )))
            };
        }

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_balances(
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        // We insert the balance
        batch_operations.push(self.insert_identity_balance_operation_v0(id.to_buffer(), balance)?);

        batch_operations
            .push(self.initialize_negative_identity_balance_operation_v0(id.to_buffer()));

        // We insert the revision
        batch_operations
            .push(self.initialize_identity_revision_operation_v0(id.to_buffer(), revision));

        // We insert a nonce of 0, nonces are used to prevent replay attacks, and should not be confused
        // revisions
        batch_operations.push(self.initialize_identity_nonce_operation_v0(id.to_buffer(), 0));

        let mut create_tree_keys_operations = self.create_key_tree_with_keys_operations(
            id.to_buffer(),
            public_keys.into_values().collect(),
            // if we are a masternode identity, we want to register all keys as non unique
            is_masternode_identity,
            &block_info.epoch,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;
        // We insert the key tree and keys
        batch_operations.append(&mut create_tree_keys_operations);

        Ok(batch_operations)
    }
}

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::{setup_drive, setup_drive_with_initial_state_structure};
    use dpp::identity::Identity;

    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::IdentityGettersV0;

    use dpp::version::PlatformVersion;

    #[test]
    fn test_insert_and_fetch_identity_v0() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::first();

        let transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&transaction), platform_version)
            .expect("expected to create root tree successfully");

        let identity = Identity::random_identity(5, Some(12345), platform_version)
            .expect("expected a random identity");

        drive
            .add_new_identity_v0(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to insert identity");

        let fetched_identity = drive
            .fetch_full_identity(
                identity.id().to_buffer(),
                Some(&transaction),
                platform_version,
            )
            .expect("should fetch an identity")
            .expect("should have an identity");

        assert_eq!(identity, fetched_identity);
    }

    #[test]
    fn test_insert_identity_v0() {
        let drive = setup_drive_with_initial_state_structure();

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let identity = Identity::random_identity(5, Some(12345), platform_version)
            .expect("expected a random identity");

        drive
            .add_new_identity_v0(
                identity,
                false,
                &BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
            )
            .expect("expected to insert identity");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("expected to be able to commit a transaction");
    }
}
