use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::block::epoch::Epoch;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::IdentityPublicKey;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use integer_encoding::VarInt;
use std::collections::HashMap;

impl Drive {
    /// Insert a new non unique key into an identity operations
    pub(super) fn insert_new_non_unique_key_operations_v0(
        &self,
        identity_id: [u8; 32],
        identity_key: IdentityPublicKey,
        with_references: bool,
        epoch: &Epoch,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        drive_operations.append(&mut self.insert_reference_to_non_unique_key_operations(
            identity_id,
            &identity_key,
            estimated_costs_only_with_layer_info,
            transaction,
            drive_version,
        )?);

        let key_id_bytes = identity_key.id().encode_var_vec();

        self.insert_key_to_storage_operations(
            identity_id,
            &identity_key,
            key_id_bytes.as_slice(),
            drive_operations,
            drive_version,
        )?;

        // if there are contract bounds we need to insert them
        self.add_potential_contract_info_for_contract_bounded_key(
            identity_id,
            &identity_key,
            epoch,
            estimated_costs_only_with_layer_info,
            transaction,
            drive_operations,
        )?;

        // if we set that we wanted to add references we should construct those

        if with_references
            && matches!(
                identity_key.purpose(),
                Purpose::AUTHENTICATION | Purpose::WITHDRAW
            )
        {
            self.insert_key_searchable_references_operations(
                identity_id,
                &identity_key,
                key_id_bytes.as_slice(),
                estimated_costs_only_with_layer_info,
                transaction,
                drive_operations,
                drive_version,
            )?;
        }
        Ok(())
    }
}
