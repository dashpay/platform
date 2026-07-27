use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use dpp::fee::Credits;
use dpp::prefunded_specialized_balance::PrefundedSpecializedBalanceIdentifier;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use std::collections::BTreeMap;

pub(crate) trait StateTransitionPrefundedSpecializedBalanceValidationV0 {
    /// Validates the state transition by analyzing the changes in the platform state after applying the transaction.
    ///
    /// # Arguments
    ///
    /// * `platform` - A reference to the platform containing the state data.
    /// * `tx` - The transaction argument to be applied.
    ///
    /// # Type Parameters
    ///
    /// * `C: CoreRPCLike` - A type constraint indicating that C should implement `CoreRPCLike`.
    ///
    /// # Returns
    ///
    /// * `Result<ConsensusValidationResult<StateTransitionAction>, Error>` - A result with either a ConsensusValidationResult containing a StateTransitionAction or an Error.
    fn validate_minimum_prefunded_specialized_balance_pre_check(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<
        ConsensusValidationResult<BTreeMap<PrefundedSpecializedBalanceIdentifier, Credits>>,
        Error,
    >;

    /// Do we use a prefunded specialized balance for payment
    fn uses_prefunded_specialized_balance_for_payment(&self) -> bool {
        false
    }
}

impl StateTransitionPrefundedSpecializedBalanceValidationV0 for StateTransition {
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
        match self {
            StateTransition::MasternodeVote(masternode_vote_transition) => {
                masternode_vote_transition.validate_minimum_prefunded_specialized_balance_pre_check(
                    drive,
                    tx,
                    execution_context,
                    platform_version,
                )
            }
            _ => Ok(ConsensusValidationResult::new()),
        }
    }

    fn uses_prefunded_specialized_balance_for_payment(&self) -> bool {
        matches!(self, StateTransition::MasternodeVote(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make_data_contract_create_st() -> StateTransition {
        use dpp::tests::fixtures::get_data_contract_fixture;
        use platform_version::TryIntoPlatformVersioned;
        let platform_version = platform_version::version::PlatformVersion::latest();
        let created_data_contract =
            get_data_contract_fixture(None, 1, platform_version.protocol_version);
        let transition: dpp::state_transition::data_contract_create_transition::DataContractCreateTransition =
            created_data_contract.try_into_platform_versioned(platform_version).unwrap();
        transition.into()
    }

    use dpp::state_transition::batch_transition::BatchTransition;
    use dpp::state_transition::batch_transition::BatchTransitionV0;
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
    use dpp::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
    use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
    use dpp::state_transition::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
    use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;

    mod uses_prefunded_specialized_balance_for_payment {
        use super::*;

        #[test]
        fn should_return_true_only_for_masternode_vote() {
            let st = StateTransition::MasternodeVote(MasternodeVoteTransition::V0(
                MasternodeVoteTransitionV0::default(),
            ));
            assert!(st.uses_prefunded_specialized_balance_for_payment());
        }

        #[test]
        fn should_return_false_for_non_masternode_vote_transitions() {
            let transitions: Vec<(&str, StateTransition)> = vec![
                ("DataContractCreate", make_data_contract_create_st()),
                (
                    "Batch",
                    StateTransition::Batch(BatchTransition::V0(BatchTransitionV0::default())),
                ),
                (
                    "IdentityCreate",
                    StateTransition::IdentityCreate(IdentityCreateTransition::V0(
                        IdentityCreateTransitionV0::default(),
                    )),
                ),
            ];
            for (name, st) in transitions {
                assert!(
                    !st.uses_prefunded_specialized_balance_for_payment(),
                    "expected false for {}",
                    name
                );
            }
        }
    }
}
