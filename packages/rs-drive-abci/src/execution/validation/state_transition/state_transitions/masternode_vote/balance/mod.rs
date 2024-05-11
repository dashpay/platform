use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::masternode_vote::balance::v0::MasternodeVoteTransitionBalanceValidationV0;
use crate::execution::validation::state_transition::processor::v0::StateTransitionPrefundedSpecializedBalanceValidationV0;
use dpp::fee::Credits;
use dpp::prefunded_specialized_balance::PrefundedSpecializedBalanceIdentifier;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use std::collections::BTreeMap;

pub(crate) mod v0;

impl StateTransitionPrefundedSpecializedBalanceValidationV0 for MasternodeVoteTransition {
    fn validate_minimum_prefunded_specialized_balance_pre_check(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<
        ConsensusValidationResult<BTreeMap<PrefundedSpecializedBalanceIdentifier, Credits>>,
        Error,
    > {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .masternode_vote_state_transition
            .advanced_minimum_balance_pre_check
        {
            Some(0) => self.validate_advanced_minimum_balance_pre_check_v0(
                drive,
                tx,
                execution_context,
                platform_version,
            ),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "masternode vote transition: validate_balance".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "masternode vote transition: validate_balance".to_string(),
                known_versions: vec![0],
            })),
        }
    }

    fn uses_prefunded_specialized_balance_for_payment(&self) -> bool {
        true
    }
}
