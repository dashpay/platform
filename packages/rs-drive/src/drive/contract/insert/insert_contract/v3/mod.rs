use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v1::TokenConfigurationV1Getters;
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;
use dpp::tokens::calculate_token_id;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Protocol 15 contract insertion, including shielded pools for opted-in tokens.
    #[inline(always)]
    pub(super) fn insert_contract_v3(
        &self,
        contract: &DataContract,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        let contract_element =
            Self::contract_element_for_insert_v1(contract, &block_info, platform_version)?;

        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };
        let batch_operations = self.insert_contract_operations_v3(
            contract_element,
            contract,
            &block_info,
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

    /// The operations for adding a contract, appended to `drive_operations`.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn insert_contract_add_operations_v3(
        &self,
        contract_element: Element,
        contract: &DataContract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let batch_operations = self.insert_contract_operations_v3(
            contract_element,
            contract,
            block_info,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;
        drive_operations.extend(batch_operations);
        Ok(())
    }

    /// Protocol 14 operations with token pool trees for opted-in tokens.
    fn insert_contract_operations_v3(
        &self,
        contract_element: Element,
        contract: &DataContract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];
        self.insert_contract_add_operations_v2(
            contract_element,
            contract,
            block_info,
            estimated_costs_only_with_layer_info,
            &mut batch_operations,
            transaction,
            platform_version,
        )?;

        for (position, configuration) in contract.tokens() {
            if !configuration.has_shielded_pool() {
                continue;
            }
            let token_id = calculate_token_id(contract.id().as_bytes(), *position);
            // An update is estimated through the insert path (the stateless existence read
            // reports nothing), so a pool the token already owns must not be priced as created
            // again; a real insert never finds one.
            if estimated_costs_only_with_layer_info.is_some()
                && self.has_token_shielded_pool(
                    token_id,
                    transaction,
                    &mut vec![],
                    platform_version,
                )?
            {
                continue;
            }
            batch_operations.extend(self.create_token_shielded_pool_trees_operations(
                token_id,
                false,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?);
        }
        Ok(batch_operations)
    }
}
