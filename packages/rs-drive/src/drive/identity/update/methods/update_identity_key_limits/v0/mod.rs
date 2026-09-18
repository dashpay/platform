use crate::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDIdentityPublicKeyPairVec, KeyRequestType,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::accessors::v1::IdentityPublicKeyGettersV1;
use dpp::identity::{IdentityPublicKey, KeyID, TimestampMillis};
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use integer_encoding::VarInt;
use std::collections::HashMap;

impl Drive {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn update_identity_key_limits_v0(
        &self,
        identity_id: [u8; 32],
        key_id: KeyID,
        total_budget: Option<Credits>,
        expires_at: Option<TimestampMillis>,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        let key_request = IdentityKeysRequest {
            identity_id,
            request_type: KeyRequestType::SpecificKeys(vec![key_id]),
            limit: Some(1),
            offset: None,
        };

        let mut keys: KeyIDIdentityPublicKeyPairVec = self.fetch_identity_keys_operations(
            key_request,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;

        let Some((_, key)) = keys.pop() else {
            return Err(Error::Drive(DriveError::UpdatingDocumentThatDoesNotExist(
                "key whose limits are being raised is not found",
            )));
        };

        let batch_operations = self.update_identity_key_limits_operations_v0(
            identity_id,
            key,
            total_budget,
            expires_at,
            &block_info.epoch,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;

        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )
    }

    /// Rewrites the key with its raised limits, refreshes the references to it (they carry the
    /// value hash of the key), and raises the remaining budget by the amount the total budget
    /// grew. The key comes in as stored, read when the transition was validated, so the byte
    /// delta of the rewrite (a bigger varint, or an expiry that was absent) is priced from it
    /// without reading it again.
    ///
    /// Drive only loosens what consensus already checked: a limit is raised, never added or
    /// lowered. A caller that asks for anything else corrupts the invariant that the remaining
    /// budget never exceeds the total, so it is refused as a corrupted state.
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub(super) fn update_identity_key_limits_operations_v0(
        &self,
        identity_id: [u8; 32],
        key: IdentityPublicKey,
        total_budget: Option<Credits>,
        expires_at: Option<TimestampMillis>,
        epoch: &Epoch,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];

        let drive_version = &platform_version.drive;

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_keys_for_identity_id(
                identity_id,
                estimated_costs_only_with_layer_info,
                drive_version,
            )?;
            Self::add_estimation_costs_for_root_key_reference_tree(
                identity_id,
                estimated_costs_only_with_layer_info,
                drive_version,
            )?;
            Self::add_estimation_costs_for_key_budgets(
                identity_id,
                estimated_costs_only_with_layer_info,
                drive_version,
            )?;
        }

        let key_id = key.id();
        let previous_total_budget = key.total_budget();
        let previous_expires_at = key.expires_at();
        let previous_serialized_size = key.serialize_to_bytes()?.len();

        let added_budget = match (total_budget, previous_total_budget) {
            (None, _) => None,
            (Some(_), None) => {
                return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                    "key {} has no budget to raise",
                    key_id
                ))));
            }
            (Some(total_budget), Some(previous_total_budget)) => Some(
                total_budget
                    .checked_sub(previous_total_budget)
                    .filter(|added| *added > 0)
                    .ok_or_else(|| {
                        Error::Drive(DriveError::CorruptedDriveState(format!(
                            "the total budget of key {} is not raised from {} to {}",
                            key_id, previous_total_budget, total_budget
                        )))
                    })?,
            ),
        };

        if let Some(expires_at) = expires_at {
            match previous_expires_at {
                None => {
                    return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                        "key {} has no expiry to extend",
                        key_id
                    ))));
                }
                Some(previous_expires_at) if expires_at <= previous_expires_at => {
                    return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                        "the expiry of key {} is not moved later from {} to {}",
                        key_id, previous_expires_at, expires_at
                    ))));
                }
                Some(_) => {}
            }
        }

        let key = key.with_limits(
            total_budget.or(previous_total_budget),
            expires_at.or(previous_expires_at),
        );

        let serialized_size = key.serialize_to_bytes()?.len();
        let change_in_bytes = i32::try_from(
            serialized_size as i64 - previous_serialized_size as i64,
        )
        .map_err(|_| {
            Error::Drive(DriveError::CorruptedDriveState(format!(
                "the rewrite of key {} changes its size by more than an i32",
                key_id
            )))
        })?;

        let key_id_bytes = key_id.encode_var_vec();

        self.replace_key_in_storage_operations(
            identity_id.as_slice(),
            &key,
            &key_id_bytes,
            change_in_bytes,
            &mut drive_operations,
            drive_version,
        )?;

        self.refresh_identity_key_reference_operations(
            identity_id,
            &key,
            epoch,
            estimated_costs_only_with_layer_info,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;

        if let Some(added_budget) = added_budget {
            let (budget_operations, _) = self.add_to_identity_key_budget_operations(
                identity_id,
                key_id,
                added_budget,
                transaction,
                platform_version,
            )?;
            drive_operations.extend(budget_operations);
        }

        Ok(drive_operations)
    }
}
