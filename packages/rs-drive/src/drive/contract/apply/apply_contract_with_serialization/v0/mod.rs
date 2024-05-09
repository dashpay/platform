use crate::drive::contract::paths::{contract_keeping_history_root_path, contract_root_path};
use crate::drive::defaults::CONTRACT_MAX_SERIALIZED_SIZE;

use crate::drive::flags::StorageFlags;
use crate::drive::grove_operations::QueryTarget::QueryTargetValue;
use crate::drive::grove_operations::{DirectQueryType, QueryType};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use crate::fee::op::LowLevelDriveOperation::CalculatedCostOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::string_encoding::Encoding;
use dpp::prelude::DataContract;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::borrow::Cow;
use std::collections::HashMap;

impl Drive {
    /// Applies a contract and returns the fee for applying.
    /// If the contract already exists, an update is applied, otherwise an insert.
    #[inline(always)]
    pub(super) fn apply_contract_with_serialization_v0(
        &self,
        contract: &DataContract,
        contract_serialization: Vec<u8>,
        block_info: BlockInfo,
        apply: bool,
        storage_flags: Option<Cow<StorageFlags>>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut cost_operations = vec![];
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };
        let batch_operations = self.apply_contract_with_serialization_operations_v0(
            contract,
            contract_serialization,
            &block_info,
            &mut estimated_costs_only_with_layer_info,
            storage_flags,
            transaction,
            platform_version,
        )?;
        let fetch_cost = LowLevelDriveOperation::combine_cost_operations(&batch_operations);
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            &mut cost_operations,
            &platform_version.drive,
        )?;
        cost_operations.push(CalculatedCostOperation(fetch_cost));
        let fees = Drive::calculate_fee(
            None,
            Some(cost_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
        )?;
        Ok(fees)
    }

    /// Gets the operations for applying a contract with it's serialization
    /// If the contract already exists, we get operations for an update
    /// Otherwise we get operations for an insert
    #[inline(always)]
    pub(super) fn apply_contract_with_serialization_operations_v0(
        &self,
        contract: &DataContract,
        contract_serialization: Vec<u8>,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        storage_flags: Option<Cow<StorageFlags>>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        // overlying structure
        let mut already_exists = false;
        let mut original_contract_stored_data = vec![];

        // no estimated_costs_only_with_layer_info, means we want to apply to state
        let direct_query_type = if estimated_costs_only_with_layer_info.is_none() {
            DirectQueryType::StatefulDirectQuery
        } else {
            DirectQueryType::StatelessDirectQuery {
                in_tree_using_sums: false,
                // we can ignore flags as this is just an approximation
                // and it's doubtful that contracts will always be inserted at max size
                query_target: QueryTargetValue(CONTRACT_MAX_SERIALIZED_SIZE as u32),
            }
        };

        // We can do a get direct because there are no references involved
        match self.grove_get_raw(
            (&contract_root_path(contract.id_ref().as_bytes())).into(),
            &[0],
            direct_query_type,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        ) {
            Ok(Some(stored_element)) => {
                match stored_element {
                    Element::Item(stored_contract_bytes, _) => {
                        already_exists = true;
                        if contract_serialization != stored_contract_bytes {
                            original_contract_stored_data = stored_contract_bytes;
                        }
                    }
                    Element::Tree(..) => {
                        // we are in a tree, this means that the contract keeps history, we need to fetch the actual latest contract
                        already_exists = true;
                        // we need to get the latest of a contract that keeps history, can't be raw since there is a reference
                        let stored_element = self
                            .grove_get(
                                (&contract_keeping_history_root_path(contract.id_ref().as_bytes()))
                                    .into(),
                                &[0],
                                QueryType::StatefulQuery,
                                transaction,
                                &mut drive_operations,
                                &platform_version.drive,
                            )?
                            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                                "we should have an element for the contract",
                            )))?;
                        match stored_element {
                            Element::Item(stored_contract_bytes, _) => {
                                if contract_serialization != stored_contract_bytes {
                                    original_contract_stored_data = stored_contract_bytes;
                                }
                            }
                            _ => {
                                return Err(Error::Drive(DriveError::CorruptedDriveState(format!("expecting an item for the last reference of a contract that keeps history {}", contract.id().to_string(Encoding::Base58)))));
                            }
                        }
                    }
                    _ => {
                        return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                            "expecting an item or a tree at contract root {}",
                            contract.id().to_string(Encoding::Base58)
                        ))));
                    }
                }
            }
            Ok(None) => {
                // we are in estimated costs
                // keep already_exists at false
            }
            Err(Error::GroveDB(grovedb::Error::PathKeyNotFound(_)))
            | Err(Error::GroveDB(grovedb::Error::PathNotFound(_)))
            | Err(Error::GroveDB(grovedb::Error::PathParentLayerNotFound(_))) => {
                // keep already_exists at false
            }
            Err(e) => {
                return Err(e);
            }
        };

        let contract_element = Element::Item(
            contract_serialization,
            StorageFlags::map_cow_to_some_element_flags(storage_flags),
        );

        if already_exists {
            if !original_contract_stored_data.is_empty() {
                let original_contract = DataContract::versioned_deserialize(
                    &original_contract_stored_data,
                    false,
                    platform_version,
                )?;
                // if the contract is not mutable update_contract will return an error
                self.update_contract_add_operations(
                    contract_element,
                    contract,
                    &original_contract,
                    block_info,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    &mut drive_operations,
                    platform_version,
                )?;
            }
        } else {
            self.insert_contract_add_operations(
                contract_element,
                contract,
                block_info,
                estimated_costs_only_with_layer_info,
                &mut drive_operations,
                platform_version,
            )?;
        }
        Ok(drive_operations)
    }
}
