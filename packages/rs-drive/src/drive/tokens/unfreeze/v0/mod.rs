use crate::drive::tokens::paths::token_identity_infos_path;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::tokens::info::v0::IdentityTokenInfoV0Accessors;
use dpp::tokens::info::IdentityTokenInfo;
use dpp::version::PlatformVersion;
use grovedb::{batch::KeyInfoPath, Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    pub(super) fn token_unfreeze_v0(
        &self,
        token_id: Identifier,
        frozen_identity_id: Identifier,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations = vec![];

        self.token_unfreeze_add_to_operations_v0(
            token_id,
            frozen_identity_id,
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

    pub(super) fn token_unfreeze_add_to_operations_v0(
        &self,
        token_id: Identifier,
        frozen_identity_id: Identifier,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info =
            if apply { None } else { Some(HashMap::new()) };

        let batch_operations = self.token_unfreeze_operations_v0(
            token_id,
            frozen_identity_id,
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

    pub(super) fn token_unfreeze_operations_v0(
        &self,
        token_id: Identifier,
        frozen_identity_id: Identifier,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_token_identity_infos(
                token_id.to_buffer(),
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        // no estimated_costs_only_with_layer_info, means we want to apply to state
        let direct_query_type = if estimated_costs_only_with_layer_info.is_none() {
            DirectQueryType::StatefulDirectQuery
        } else {
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::NormalTree,
                query_target: QueryTargetValue(8),
            }
        };

        let token_info_path = token_identity_infos_path(token_id.as_bytes());
        match self
            .grove_get_raw_optional_item(
                (&token_info_path).into(),
                frozen_identity_id.as_slice(),
                direct_query_type,
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?
            .map(|bytes| IdentityTokenInfo::deserialize_from_bytes(&bytes))
            .transpose()?
        {
            None => {
                let identity_token_info_bytes = IdentityTokenInfo::new(false, platform_version)?
                    .serialize_consume_to_bytes()?;
                drive_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
                    token_info_path.iter().map(|a| a.to_vec()).collect(),
                    frozen_identity_id.to_vec(),
                    Element::new_item(identity_token_info_bytes),
                ));
            }
            Some(mut token_info) => {
                if token_info.frozen() {
                    token_info.set_frozen(false);
                    let identity_token_info_bytes = token_info.serialize_consume_to_bytes()?;
                    drive_operations.push(
                        LowLevelDriveOperation::replace_for_known_path_key_element(
                            token_info_path.iter().map(|a| a.to_vec()).collect(),
                            frozen_identity_id.to_vec(),
                            Element::new_item(identity_token_info_bytes),
                        ),
                    );
                }
            }
        };

        Ok(drive_operations)
    }
}
