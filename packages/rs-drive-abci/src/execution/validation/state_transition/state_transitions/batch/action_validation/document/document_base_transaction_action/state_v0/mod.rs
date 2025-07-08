use dpp::block::block_info::BlockInfo;
use dpp::consensus::ConsensusError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::{IdentityDoesNotHaveEnoughTokenBalanceError, IdentityTokenAccountFrozenError};
use dpp::identifier::Identifier;
use dpp::tokens::info::v0::IdentityTokenInfoV0Accessors;
use dpp::validation::SimpleConsensusValidationResult;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0};
use crate::platform_types::platform::PlatformStateRef;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait DocumentBaseTransitionActionStateValidationV0 {
#[allow(clippy::too_many_arguments)]
    fn validate_state_v0(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        transition_type: &str,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}
impl DocumentBaseTransitionActionStateValidationV0 for DocumentBaseTransitionAction {
    fn validate_state_v0(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        transition_type: &str,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        // The following was introduced with tokens, since there are no token costs before v9 there was no reason to
        // create a new version for state verification

        if let Some((token_id, _, cost_in_tokens)) = self.token_cost() {
            let (maybe_identity_token_info, fee_result) =
                platform.drive.fetch_identity_token_info_with_costs(
                    token_id.to_buffer(),
                    owner_id.to_buffer(),
                    block_info,
                    true,
                    transaction,
                    platform_version,
                )?;

            execution_context
                .add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

            if let Some(identity_token_info) = maybe_identity_token_info {
                // if we have an info we need to make sure we are not frozen for this identity
                if identity_token_info.frozen() {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(StateError::IdentityTokenAccountFrozenError(
                            IdentityTokenAccountFrozenError::new(
                                token_id,
                                owner_id,
                                format!("Document {} token payment", transition_type),
                            ),
                        )),
                    ));
                }
            }

            let (maybe_identity_token_balance, fee_result) =
                platform.drive.fetch_identity_token_balance_with_costs(
                    token_id.to_buffer(),
                    owner_id.to_buffer(),
                    block_info,
                    true,
                    transaction,
                    platform_version,
                )?;
            let identity_token_balance = maybe_identity_token_balance.unwrap_or_default();
            execution_context
                .add_operation(ValidationOperation::PrecalculatedOperation(fee_result));
            if identity_token_balance < cost_in_tokens {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(
                        StateError::IdentityDoesNotHaveEnoughTokenBalanceError(
                            IdentityDoesNotHaveEnoughTokenBalanceError::new(
                                token_id,
                                owner_id,
                                cost_in_tokens,
                                identity_token_balance,
                                format!("Document {} token payment", transition_type),
                            ),
                        ),
                    ),
                ));
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
