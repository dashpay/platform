use dpp::identity::PartialIdentity;
use dpp::{
    identity::state_transition::identity_topup_transition::IdentityTopUpTransition,
    state_transition::StateTransitionAction,
    validation::{ConsensusValidationResult, SimpleConsensusValidationResult},
};
use drive::grovedb::TransactionArg;
use drive::{drive::Drive, grovedb::Transaction};

use crate::error::Error;
use crate::rpc::core::CoreRPCLike;

use super::StateTransitionValidation;

impl StateTransitionValidation for IdentityTopUpTransition {
    fn validate_structure(
        &self,
        drive: &Drive,
        tx: TransactionArg,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        todo!()
    }

    fn validate_signatures(
        &self,
        drive: &Drive,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        todo!()
    }

    fn validate_state<C: CoreRPCLike>(
        &self,
        drive: &Drive,
        core: &C,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        todo!()
    }
}
