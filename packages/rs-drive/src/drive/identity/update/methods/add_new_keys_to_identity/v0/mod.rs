use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
use dpp::identity::IdentityPublicKey;

use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// The operations for adding new keys to an identity
    /// This should not be called for adding new keys to a masternode
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_new_keys_to_identity_operations_v0(
        &self,
        identity_id: [u8; 32],
        unique_keys_to_add: Vec<IdentityPublicKey>,
        non_unique_keys_to_add: Vec<IdentityPublicKey>,
        with_searchable_inner_references: bool,
        epoch: &Epoch,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_keys_for_identity_id(
                identity_id,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        for key in unique_keys_to_add {
            self.insert_new_unique_key_operations(
                identity_id,
                key,
                with_searchable_inner_references,
                epoch,
                estimated_costs_only_with_layer_info,
                transaction,
                &mut drive_operations,
                platform_version,
            )?;
        }

        for key in non_unique_keys_to_add {
            self.insert_new_non_unique_key_operations(
                identity_id,
                key,
                true,
                with_searchable_inner_references,
                epoch,
                estimated_costs_only_with_layer_info,
                transaction,
                &mut drive_operations,
                platform_version,
            )?;
        }
        Ok(drive_operations)
    }

    /// The operations for adding new non-unique keys to a masternode identity
    #[inline(always)]
    pub(super) fn add_new_keys_to_masternode_identity_operations_v0(
        &self,
        identity_id: [u8; 32],
        non_unique_keys_to_add: Vec<IdentityPublicKey>,
        epoch: &Epoch,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_keys_for_identity_id(
                identity_id,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        for key in non_unique_keys_to_add {
            // When estimating costs the reference is assumed, as by `AddNewKeysToIdentity`.
            // Otherwise a key whose hash is already a unique key hash is added without a
            // non-unique key hash reference. The probe records no cost operation, so every
            // other key gets exactly the operations of `AddNewKeysToIdentity`.
            let with_reference_to_non_unique_key = estimated_costs_only_with_layer_info.is_some()
                || !self.has_unique_public_key_hash(
                    key.public_key_hash()?,
                    transaction,
                    &platform_version.drive,
                )?;

            self.insert_new_non_unique_key_operations(
                identity_id,
                key,
                with_reference_to_non_unique_key,
                true,
                epoch,
                estimated_costs_only_with_layer_info,
                transaction,
                &mut drive_operations,
                platform_version,
            )?;
        }
        Ok(drive_operations)
    }
}
