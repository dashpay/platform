use std::sync::Arc;

use dpp::identifier::Identifier;
use dpp::identity::PartialIdentity;
use dpp::ProtocolError;
use dpp::state_transition::StateTransition;
use dpp::validation::ConsensusValidationResult;
use dpp::version::{PlatformVersion};
use drive::drive::contract::DataContractFetchInfo;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;
use crate::error::Error;
use crate::error::execution::ExecutionError;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::common::validate_state_transition_identity_signed::v0::ValidateStateTransitionIdentitySignatureV0;

pub mod v0;

pub type GetDataContractFn = fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>;

pub trait ValidateStateTransitionIdentitySignature {
    fn validate_state_transition_identity_signed(
        &self,
        drive: &Drive,
        action: Option<&StateTransitionAction>,
        request_revision: bool,
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<PartialIdentity>, Error>;
}

impl ValidateStateTransitionIdentitySignature for StateTransition {
    fn validate_state_transition_identity_signed(
        &self,
        drive: &Drive,
        action: Option<&StateTransitionAction>,
        request_revision: bool,
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<PartialIdentity>, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .validate_state_transition_identity_signed
        {
            0 => self.validate_state_transition_identity_signed_v0(
                drive,
                action,
                request_revision,
                transaction,
                execution_context,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "StateTransition::validate_state_transition_identity_signature".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
