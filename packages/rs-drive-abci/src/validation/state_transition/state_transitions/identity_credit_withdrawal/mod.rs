mod identity_and_signatures;
mod state;
mod structure;

use dpp::identity::PartialIdentity;
use dpp::{identity::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition, state_transition::StateTransitionAction, validation::{ConsensusValidationResult, SimpleConsensusValidationResult}};
use dpp::consensus::basic::identity::{InvalidIdentityCreditWithdrawalTransitionCoreFeeError, InvalidIdentityCreditWithdrawalTransitionOutputScriptError, NotImplementedIdentityCreditWithdrawalTransitionPoolingError};
use dpp::consensus::signature::IdentityNotFoundError;
use dpp::consensus::state::identity::IdentityInsufficientBalanceError;
use dpp::consensus::state::identity::invalid_identity_revision_error::InvalidIdentityRevisionError;
use dpp::consensus::state::state_error::StateError;
use dpp::identity::state_transition::identity_credit_withdrawal_transition::{IdentityCreditWithdrawalTransitionAction, Pooling};
use dpp::identity::state_transition::identity_credit_withdrawal_transition::validation::basic::validate_identity_credit_withdrawal_transition_basic::IDENTITY_CREDIT_WITHDRAWAL_TRANSITION_SCHEMA_VALIDATOR;
use dpp::util::is_fibonacci_number::is_fibonacci_number;
use drive::grovedb::TransactionArg;
use drive::drive::Drive;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use crate::validation::state_transition::common::validate_schema;
use crate::validation::state_transition::identity_credit_withdrawal::identity_and_signatures::v0::StateTransitionIdentityAndSignaturesValidationV0;
use crate::validation::state_transition::identity_credit_withdrawal::state::v0::StateTransitionStateValidationV0;
use crate::validation::state_transition::identity_credit_withdrawal::structure::v0::StateTransitionStructureValidationV0;
use crate::validation::state_transition::key_validation::validate_state_transition_identity_signature_v0;
use crate::validation::state_transition::processor::v0::StateTransitionValidationV0;
use crate::validation::state_transition::transformer::StateTransitionActionTransformerV0;

impl StateTransitionActionTransformerV0 for IdentityCreditWithdrawalTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        _tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        //todo: use protocol version to determine validation
        self.transform_into_action_v0(platform)
    }
}

impl StateTransitionValidationV0 for IdentityCreditWithdrawalTransition {
    fn validate_structure(
        &self,
        _drive: &Drive,
        protocol_version: u32,
        _tx: TransactionArg,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        //todo: use protocol version to determine validation
        self.validate_structure_v0()
    }

    fn validate_identity_and_signatures(
        &self,
        drive: &Drive,
        protocol_version: u32,
        transaction: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        //todo: use protocol version to determine validation
        self.validate_identity_and_signatures_v0(drive, transaction)
    }

    fn validate_state<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        //todo: use protocol version to determine validation
        self.validate_state_v0(platform, tx)
    }
}
