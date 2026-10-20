mod v0;

use crate::drive::contract::moderation::types::ContractTeamActionWrite;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// The operations writing one member's proposal or approval of one of a contract's team
    /// actions, as a token group action's signature is written: each approval is its own sum
    /// item under the action, flagged with the member, and nothing is ever rewritten.
    ///
    /// * A proposal ([`ContractTeamActionWrite::Propose`]) creates the action under the active
    ///   actions: its info, flagged with the proposer, and the sum tree of its approvals holding
    ///   the proposer's. One that meets the rule alone is written straight to the closed
    ///   actions, without storage flags.
    /// * An approval ([`ContractTeamActionWrite::Approve`]) adds its own sum item to the active
    ///   action's approvals, and deletes those of members no longer on the team that it found.
    /// * The approval that meets the action's rule ([`ContractTeamActionWrite::Close`]) closes
    ///   it instead: the action is written under the closed actions without storage flags, the
    ///   approvals given before that counted and its info moved there with it, those of members
    ///   no longer on the team deleted, which refunds each to whoever its flags name, and the
    ///   active action is deleted.
    ///
    /// Validation read the action and its approvals, so nothing is read here: a closing write
    /// lists every approval the active action holds, and an approval is never one of them.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract whose seated team votes on the action.
    /// * `action_id`: The action.
    /// * `signer_id`: The member that proposes or approves, who pays for what it adds.
    /// * `write`: What the signature writes: a proposal, an approval, or the closing approval.
    /// * `block_info`: The block the write runs in, whose epoch the storage flags name.
    /// * `estimated_costs_only_with_layer_info`: The estimation map, when only estimating costs.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<LowLevelDriveOperation>)` with the operations of the write.
    /// * `Err(Error)` when the method version is unknown, building an operation fails, or a
    ///   proposal's action id is already taken.
    #[allow(clippy::too_many_arguments)]
    pub fn add_contract_team_action_signature_operations(
        &self,
        contract_id: Identifier,
        action_id: Identifier,
        signer_id: Identifier,
        write: &ContractTeamActionWrite,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .moderation
            .add_contract_team_action_signature
        {
            0 => self.add_contract_team_action_signature_operations_v0(
                contract_id,
                action_id,
                signer_id,
                write,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_contract_team_action_signature_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
