use crate::drive::tokens::paths::token_statuses_root_path;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::PathKeyElementInfo;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::serialization::PlatformSerializable;
use dpp::tokens::status::TokenStatus;
use dpp::version::PlatformVersion;
use grovedb::{batch::KeyInfoPath, Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    pub(super) fn token_apply_status_v0(
        &self,
        token_id: [u8; 32],
        status: TokenStatus,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations = vec![];

        self.token_apply_status_add_to_operations_v0(
            token_id,
            status,
            apply,
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

    pub(super) fn token_apply_status_add_to_operations_v0(
        &self,
        token_id: [u8; 32],
        status: TokenStatus,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info =
            if apply { None } else { Some(HashMap::new()) };

        let batch_operations = self.token_apply_status_operations_v0(
            token_id,
            status,
            &mut estimated_costs_only_with_layer_info,
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

    pub(super) fn token_apply_status_operations_v0(
        &self,
        token_id: [u8; 32],
        status: TokenStatus,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];

        let token_status_bytes = status.serialize_consume_to_bytes()?;

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_token_status_infos(
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        self.batch_insert(
            PathKeyElementInfo::PathFixedSizeKeyRefElement::<2>((
                token_statuses_root_path(),
                token_id.as_slice(),
                Element::Item(token_status_bytes, None),
            )),
            &mut drive_operations,
            &platform_version.drive,
        )?;

        Ok(drive_operations)
    }
}
