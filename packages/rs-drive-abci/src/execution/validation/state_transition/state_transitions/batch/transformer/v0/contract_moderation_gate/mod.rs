mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use std::collections::{BTreeMap, BTreeSet};
use v0::BatchTransitionContractModerationGateV0;

/// The contract moderation gate of the batch transformer (protocol version 14).
pub(super) trait BatchTransitionContractModerationGate {
    /// Gates the document transitions of `owner_id` against one contract on the contract's
    /// moderation lists. Returns the refusal to hand back when the signer is banned or under a
    /// live suspension, `None` to carry on. A suspension found lapsed is recorded in
    /// `lapsed_suspensions` for the batch to sweep.
    ///
    /// Before protocol version 14 the gate does not exist (`None` in the version table) and
    /// nothing is read.
    #[allow(clippy::too_many_arguments)]
    fn contract_moderation_gate(
        drive: &Drive,
        block_info: &BlockInfo,
        contract: &DataContract,
        owner_id: Identifier,
        document_transitions: &BTreeMap<&String, Vec<&DocumentTransition>>,
        lapsed_suspensions: &mut BTreeSet<(Identifier, Identifier)>,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ConsensusValidationResult<Vec<BatchedTransitionAction>>>, Error>;
}

impl BatchTransitionContractModerationGate for BatchTransition {
    fn contract_moderation_gate(
        drive: &Drive,
        block_info: &BlockInfo,
        contract: &DataContract,
        owner_id: Identifier,
        document_transitions: &BTreeMap<&String, Vec<&DocumentTransition>>,
        lapsed_suspensions: &mut BTreeSet<(Identifier, Identifier)>,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ConsensusValidationResult<Vec<BatchedTransitionAction>>>, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .contract_moderation_gate
        {
            None => Ok(None),
            Some(0) => Self::contract_moderation_gate_v0(
                drive,
                block_info,
                contract,
                owner_id,
                document_transitions,
                lapsed_suspensions,
                execution_context,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "documents batch transition: contract_moderation_gate".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContextMethodsV0;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::DefaultForPlatformVersion;

    fn moderated_contract(protocol_version: u32) -> DataContract {
        let mut contract =
            get_data_contract_fixture(None, 0, protocol_version).data_contract_owned();
        contract.set_config(contract.config().clone().with_moderation(Some(
            ContractModerationConfig {
                banlist: true,
                suspensions: true,
                moderators: ContractModerators::ContractOwner,
            },
        )));
        contract
    }

    /// No contract can declare moderation before protocol version 14, so this contract only
    /// exists in memory; the point is that the shared transformer asks nothing of Drive there.
    #[test]
    fn should_not_gate_or_read_before_protocol_version_14() {
        let platform_version = PlatformVersion::get(13).expect("protocol version 13");
        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(13)
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let contract = moderated_contract(13);
        let mut lapsed_suspensions = BTreeSet::new();
        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("execution context");

        let refusal = BatchTransition::contract_moderation_gate(
            &platform.drive,
            &BlockInfo::default(),
            &contract,
            Identifier::from([1; 32]),
            &BTreeMap::new(),
            &mut lapsed_suspensions,
            &mut execution_context,
            None,
            platform_version,
        )
        .expect("expected the gate to run");

        assert!(refusal.is_none());
        assert!(lapsed_suspensions.is_empty());
        assert!(execution_context.operations_slice().is_empty());
    }

    #[test]
    fn should_not_read_for_a_contract_without_moderation() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("execution context");

        let refusal = BatchTransition::contract_moderation_gate(
            &platform.drive,
            &BlockInfo::default(),
            &contract,
            Identifier::from([1; 32]),
            &BTreeMap::new(),
            &mut BTreeSet::new(),
            &mut execution_context,
            None,
            platform_version,
        )
        .expect("expected the gate to run");

        assert!(refusal.is_none());
        assert!(execution_context.operations_slice().is_empty());
    }
}
