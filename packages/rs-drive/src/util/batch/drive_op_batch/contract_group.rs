use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;
use dpp::block::block_info::BlockInfo;
use dpp::contract_group::{ContractGroupInfo, ContractGroupMembership};
use dpp::identifier::Identifier;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

/// Operations on contract groups: identity-owned sets of contracts, contract document types and
/// contract tokens.
#[derive(Clone, Debug)]
pub enum ContractGroupOperationType {
    /// Registers a new contract group.
    RegisterContractGroup {
        /// The group id, derived from the registering identity and its nonce.
        contract_group_id: Identifier,
        /// The stored information: owner, name, description.
        info: ContractGroupInfo,
    },
    /// Records the contract group memberships of a newly created contract.
    AddContractGroupMemberships {
        /// The created contract.
        contract_id: Identifier,
        /// The memberships it declares.
        memberships: Vec<ContractGroupMembership>,
    },
}

impl DriveLowLevelOperationConverter for ContractGroupOperationType {
    fn into_low_level_drive_operations(
        self,
        drive: &Drive,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        _block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match self {
            ContractGroupOperationType::RegisterContractGroup {
                contract_group_id,
                info,
            } => drive.insert_contract_group_operations(
                contract_group_id,
                &info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            ContractGroupOperationType::AddContractGroupMemberships {
                contract_id,
                memberships,
            } => drive.insert_contract_group_memberships_operations(
                contract_id,
                &memberships,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
        }
    }
}
