use dpp::{
    identity::state_transition::identity_topup_transition::IdentityTopUpTransition,
    state_transition::StateTransitionAction,
    validation::{ConsensusValidationResult, SimpleConsensusValidationResult},
};
use drive::grovedb::TransactionArg;
use drive::{drive::Drive, grovedb::Transaction};

use crate::error::Error;

use super::StateTransitionValidation;

impl StateTransitionValidation for IdentityTopUpTransition {
    fn validate_type(
        &self,
        drive: &Drive,
        tx: TransactionArg,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        todo!()
    }

    fn validate_signature(&self, drive: &Drive) -> Result<SimpleConsensusValidationResult, Error> {
        todo!()
    }

    fn validate_key_signature(&self) -> Result<SimpleConsensusValidationResult, Error> {
        todo!()
    }

    fn validate_state(
        &self,
        drive: &Drive,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        todo!()
    }
}
