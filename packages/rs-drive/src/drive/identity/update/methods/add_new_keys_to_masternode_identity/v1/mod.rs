use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::estimated_layers::EstimatedLayers;
use dpp::block::epoch::Epoch;
use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
use dpp::identity::IdentityPublicKey;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::BTreeSet;

impl Drive {
    /// Adds keys to a masternode identity like version 0, except that a key whose public key
    /// hash the identity already holds a non-unique key hash reference for, or that an earlier
    /// key of the same call has, is added without a second reference.
    ///
    /// The reference records that an identity holds a key with a hash, once per hash and
    /// identity; version 0 fails the batch with `IdentityKeyAlreadyExists` when asked for a
    /// second one. A masternode identity's keys come from Core, which accepts any 20 bytes as an
    /// operator payout or a platform node id, so an operator identity can need two keys of
    /// different kinds with one hash, such as a payout equal to a platform node id it had
    /// before. Each key is still stored under its own id with its own purpose.
    #[inline(always)]
    pub(super) fn add_new_keys_to_masternode_identity_operations_v1(
        &self,
        identity_id: [u8; 32],
        non_unique_keys_to_add: Vec<IdentityPublicKey>,
        epoch: &Epoch,
        estimated_costs_only_with_layer_info: &mut Option<EstimatedLayers>,
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

        let mut referenced_hashes = BTreeSet::new();
        for key in non_unique_keys_to_add {
            // When estimating costs the reference is assumed, as by version 0, once per hash of
            // the call. The probes record no cost operation, as version 0's does not.
            let with_reference_to_non_unique_key = if estimated_costs_only_with_layer_info.is_some()
            {
                referenced_hashes.insert(key.public_key_hash()?)
            } else {
                let public_key_hash = key.public_key_hash()?;
                !self.has_unique_public_key_hash(
                    public_key_hash,
                    transaction,
                    &platform_version.drive,
                )? && !self.has_non_unique_public_key_hash_already_for_identity_operations(
                    public_key_hash,
                    identity_id,
                    transaction,
                    &mut vec![],
                    &platform_version.drive,
                )?
                    // False for a hash an earlier key of this call references: the batch holds
                    // that reference already, and a second insert of it would be a duplicate
                    // operation, which a batch checked for consistency refuses.
                    && referenced_hashes.insert(public_key_hash)
            };

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
