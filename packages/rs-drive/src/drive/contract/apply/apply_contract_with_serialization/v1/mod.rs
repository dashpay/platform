use crate::drive::contract::estimated_stored_contract_size;
use crate::drive::contract::paths::{contract_keeping_history_root_path, contract_root_path};
use crate::util::estimated_layers::EstimatedLayers;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::CalculatedCostOperation;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use crate::util::grove_operations::{DirectQueryType, QueryType};
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::string_encoding::Encoding;
use dpp::prelude::DataContract;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::serialization::{
    PlatformDeserializableWithPotentialValidationFromVersionedStructureTrusted,
    PlatformSerializableWithPlatformVersion,
};

use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg, TreeType};
use std::borrow::Cow;

impl Drive {
    /// Applies a contract and returns the fee for applying: generation 0's, with the operations
    /// of generation 1 ([`Self::apply_contract_with_serialization_operations_v1`]).
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn apply_contract_with_serialization_v1(
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
            None::<EstimatedLayers>
        } else {
            Some(EstimatedLayers::new())
        };
        let batch_operations = self.apply_contract_with_serialization_operations_v1(
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
            None,
        )?;
        Ok(fees)
    }

    /// Gets the operations for applying a contract with its serialization, generation 1:
    /// generation 0's, but a stored contract whose bytes differ from the new serialization is
    /// the contract applied, and nothing is written, when it has the applied contract's version
    /// and its format version 1 bytes are the applied contract's. From protocol version 15 a
    /// contract is stored compiled (format version 2), so an unchanged contract stored before
    /// in format version 1 is in another format than its new serialization; applying it writes
    /// nothing, and it stays in the format it was stored in.
    ///
    /// The stored contract is decoded once, as generation 0 decodes it for an update, and its
    /// version decides first: a contract update and a token configuration update always bump
    /// it, so they do no work generation 0 does not. Only a contract applied again at its own
    /// version, as a protocol upgrade re-applies a system contract, is serialized again to
    /// compare.
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub(super) fn apply_contract_with_serialization_operations_v1(
        &self,
        contract: &DataContract,
        contract_serialization: Vec<u8>,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<EstimatedLayers>,
        storage_flags: Option<Cow<StorageFlags>>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        // overlying structure
        let mut already_exists = false;
        // The stored contract the applied one changes, when it changes it
        let mut original_contract = None;

        // no estimated_costs_only_with_layer_info, means we want to apply to state
        let direct_query_type = if estimated_costs_only_with_layer_info.is_none() {
            DirectQueryType::StatefulDirectQuery
        } else {
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::NormalTree,
                // we can ignore flags as this is just an approximation; a contract stored
                // compiled is read at the most it can take
                query_target: QueryTargetValue(estimated_stored_contract_size(platform_version)),
            }
        };

        // We can do a get direct because there are no references involved
        match self.grove_get_raw_optional(
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
                        original_contract = Self::original_contract_changed_by_v1(
                            &stored_contract_bytes,
                            contract,
                            &contract_serialization,
                            platform_version,
                        )?;
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
                                original_contract = Self::original_contract_changed_by_v1(
                                    &stored_contract_bytes,
                                    contract,
                                    &contract_serialization,
                                    platform_version,
                                )?;
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
            Err(Error::GroveDB(e))
                if matches!(
                    e.as_ref(),
                    grovedb::Error::PathKeyNotFound(_)
                        | grovedb::Error::PathNotFound(_)
                        | grovedb::Error::PathParentLayerNotFound(_)
                ) =>
            {
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
            if let Some(original_contract) = original_contract {
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
                transaction,
                platform_version,
            )?;
        }
        Ok(drive_operations)
    }

    /// The stored contract, whose bytes are `stored`, when applying `contract`, serialized as
    /// `serialization`, changes it, or `None` when the stored contract is the one applied: its
    /// bytes are the new serialization, or it has the applied contract's version and its
    /// format version 1 bytes are the applied contract's.
    fn original_contract_changed_by_v1(
        stored: &[u8],
        contract: &DataContract,
        serialization: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<Option<DataContract>, Error> {
        if stored == serialization {
            return Ok(None);
        }
        let stored_contract =
            DataContract::versioned_deserialize_trusted(stored, false, platform_version)?;
        // Format version 1 holds the version: another version is another contract
        if stored_contract.version() != contract.version() {
            return Ok(Some(stored_contract));
        }
        if stored_contract.serialize_to_bytes_with_platform_version(platform_version)?
            == contract.serialize_to_bytes_with_platform_version(platform_version)?
        {
            Ok(None)
        } else {
            Ok(Some(stored_contract))
        }
    }
}
