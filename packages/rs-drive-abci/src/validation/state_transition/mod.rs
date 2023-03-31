mod data_contract_create;
mod data_contract_update;
mod documents_batch;
mod identity_create;
mod identity_credit_withdrawal;
mod identity_update;

use crate::platform::Platform;
use dpp::consensus::ConsensusError;
use dpp::state_transition::StateTransitionAction::{
    DataContractCreateAction, DataContractUpdateAction, DocumentsBatchAction, IdentityCreateAction,
    IdentityCreditWithdrawalAction, IdentityTopUpAction, IdentityUpdateAction,
};
use dpp::state_transition::{StateTransition, StateTransitionAction};
use dpp::validation::{SimpleValidationResult, ValidationResult};
use dpp::ProtocolError;
use drive::drive::Drive;

pub trait StateTransitionValidation<C> {
    fn validate_all(
        &self,
        platform: &Platform<C>,
    ) -> Result<ValidationResult<StateTransitionAction>, ProtocolError> {
        let result = self.validate_type()?;
        if !result.is_valid() {
            return Ok(ValidationResult::<StateTransitionAction>::new_with_errors(
                result.errors,
            ));
        }
        let result = self.validate_signature()?;
        if !result.is_valid() {
            return Ok(ValidationResult::<StateTransitionAction>::new_with_errors(
                result.errors,
            ));
        }
        let result = self.validate_key_signature()?;
        if !result.is_valid() {
            return Ok(ValidationResult::<StateTransitionAction>::new_with_errors(
                result.errors,
            ));
        }
        let result = self.validate_state()?;
        if !result.is_valid() {
            return Ok(result);
        } else {
            let action = result.into_data()?;
            action.validate_fee()
        }
    }
    fn validate_type(&self) -> Result<SimpleValidationResult, ProtocolError>;
    fn validate_signature(&self) -> Result<SimpleValidationResult, ProtocolError>;
    fn validate_key_signature(&self) -> Result<SimpleValidationResult, ProtocolError>;
    fn validate_state(
        &self,
        drive: &Drive,
    ) -> Result<ValidationResult<StateTransitionAction>, ProtocolError>;
}

impl<C> StateTransitionValidation<C> for StateTransition {
    fn validate_type(&self) -> Result<SimpleValidationResult, ProtocolError> {
        match state_transition {
            StateTransition::DataContractCreate(st) => st.validate_type(drive),
            StateTransition::DataContractUpdate(st) => st.validate_type(drive),
            StateTransition::IdentityCreate(st) => st.validate_type(drive),
            StateTransition::IdentityUpdate(st) => st.validate_type(drive),
            StateTransition::IdentityTopUp(st) => st.validate_type(drive),
            StateTransition::IdentityCreditWithdrawal(st) => st.validate_type(drive),
            StateTransition::DocumentsBatch(st) => st.validate_type(drive),
        }
    }

    fn validate_signature(&self) -> Result<SimpleValidationResult, ProtocolError> {
        match state_transition {
            StateTransition::DataContractCreate(st) => st.validate_signature(drive),
            StateTransition::DataContractUpdate(st) => st.validate_signature(drive),
            StateTransition::IdentityCreate(st) => st.validate_signature(drive),
            StateTransition::IdentityUpdate(st) => st.validate_signature(drive),
            StateTransition::IdentityTopUp(st) => st.validate_signature(drive),
            StateTransition::IdentityCreditWithdrawal(st) => st.validate_signature(drive),
            StateTransition::DocumentsBatch(st) => st.validate_signature(drive),
        }
    }

    fn validate_key_signature(&self) -> Result<SimpleValidationResult, ProtocolError> {
        match state_transition {
            StateTransition::DataContractCreate(st) => st.validate_key_signature(drive),
            StateTransition::DataContractUpdate(st) => st.validate_key_signature(drive),
            StateTransition::IdentityCreate(st) => st.validate_key_signature(drive),
            StateTransition::IdentityUpdate(st) => st.validate_key_signature(drive),
            StateTransition::IdentityTopUp(st) => st.validate_key_signature(drive),
            StateTransition::IdentityCreditWithdrawal(st) => st.validate_key_signature(drive),
            StateTransition::DocumentsBatch(st) => st.validate_key_signature(drive),
        }
    }

    fn validate_state(
        &self,
        drive: &Drive,
    ) -> Result<ValidationResult<StateTransitionAction>, ProtocolError> {
        match state_transition {
            StateTransition::DataContractCreate(st) => st.validate_state(drive),
            StateTransition::DataContractUpdate(st) => st.validate_state(drive),
            StateTransition::IdentityCreate(st) => st.validate_state(drive),
            StateTransition::IdentityUpdate(st) => st.validate_state(drive),
            StateTransition::IdentityTopUp(st) => st.validate_state(drive),
            StateTransition::IdentityCreditWithdrawal(st) => st.validate_state(drive),
            StateTransition::DocumentsBatch(st) => st.validate_state(drive),
        }
    }
}
