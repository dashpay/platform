mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::contract_group::ContractGroupMembership;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Records the contract group memberships of a newly created contract: the forward entries
    /// under each group and the backwards references under the contract's own index.
    ///
    /// The contract must be new to the state: every tree keyed by the contract id is created
    /// here, and the groups (registered earlier or in the same batch) must exist. Consensus
    /// validation guarantees both.
    pub fn insert_contract_group_memberships(
        &self,
        contract_id: Identifier,
        memberships: &[ContractGroupMembership],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .contract_group
            .insert
            .insert_contract_group_memberships
        {
            0 => self.insert_contract_group_memberships_v0(
                contract_id,
                memberships,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_contract_group_memberships".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// The low level operations recording a new contract's contract group memberships. With
    /// layer information the operations are built for estimation only.
    pub fn insert_contract_group_memberships_operations(
        &self,
        contract_id: Identifier,
        memberships: &[ContractGroupMembership],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .contract_group
            .insert
            .insert_contract_group_memberships
        {
            0 => self.insert_contract_group_memberships_operations_v0(
                contract_id,
                memberships,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_contract_group_memberships_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
