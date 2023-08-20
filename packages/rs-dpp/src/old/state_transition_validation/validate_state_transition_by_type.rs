use async_trait::async_trait;

#[cfg(test)]
use mockall::{automock, predicate::*};
use platform_value::Value;

use crate::{state_transition::{
    state_transition_execution_context::StateTransitionExecutionContext, StateTransitionType,
}, validation::{AsyncDataValidatorWithContext, DataValidatorWithContext, SimpleConsensusValidationResult}, ProtocolError, BlsModule, state_repository::StateRepositoryLike, data_contract::state_transition::{data_contract_update_transition::validation::basic::DataContractUpdateTransitionBasicValidator, data_contract_create_transition::validation::state::validate_data_contract_create_transition_basic::DataContractCreateTransitionBasicValidator}, identity::{state_transition::{identity_create_transition::validation::basic::IdentityCreateTransitionBasicValidator, validate_public_key_signatures::PublicKeysSignaturesValidator, identity_update_transition::validate_identity_update_transition_basic::ValidateIdentityUpdateTransitionBasic, identity_topup_transition::validation::basic::IdentityTopUpTransitionBasicValidator, identity_credit_withdrawal_transition::validation::basic::validate_identity_credit_withdrawal_transition_basic::IdentityCreditWithdrawalTransitionBasicValidator}, validation::PublicKeysValidator}, document::validation::basic::validate_documents_batch_transition_basic::DocumentBatchTransitionBasicValidator};
use crate::identity::state_transition::identity_credit_transfer_transition::validation::basic::identity_credit_transfer_basic::IdentityCreditTransferTransitionBasicValidator;
use crate::validation::ConsensusValidationResult;

#[cfg_attr(test, automock)]
#[async_trait(?Send)]
pub trait ValidatorByStateTransitionType {
    async fn validate(
        &self,
        raw_state_transition: &Value,
        state_transition_type: StateTransitionType,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

pub struct StateTransitionByTypeValidator<SR, BLS>
where
    SR: StateRepositoryLike,
    BLS: BlsModule,
{
    data_contract_create_validator: DataContractCreateTransitionBasicValidator,
    data_contract_update_validator: DataContractUpdateTransitionBasicValidator<SR>,
    identity_create_validator: IdentityCreateTransitionBasicValidator<
        PublicKeysValidator<BLS>,
        PublicKeysValidator<BLS>,
        SR,
        PublicKeysSignaturesValidator<BLS>,
        BLS,
    >,
    identity_update_validator: ValidateIdentityUpdateTransitionBasic<
        PublicKeysValidator<BLS>,
        PublicKeysSignaturesValidator<BLS>,
    >,
    identity_top_up_validator: IdentityTopUpTransitionBasicValidator<SR>,
    identity_credit_withdrawal_validator: IdentityCreditWithdrawalTransitionBasicValidator,
    document_batch_validator: DocumentBatchTransitionBasicValidator<SR>,
    identity_credit_transfer_validator: IdentityCreditTransferTransitionBasicValidator,
}

impl<SR, BLS> StateTransitionByTypeValidator<SR, BLS>
where
    SR: StateRepositoryLike,
    BLS: BlsModule,
{
    pub fn new(
        data_contract_create_validator: DataContractCreateTransitionBasicValidator,
        data_contract_update_validator: DataContractUpdateTransitionBasicValidator<SR>,
        identity_create_validator: IdentityCreateTransitionBasicValidator<
            PublicKeysValidator<BLS>,
            PublicKeysValidator<BLS>,
            SR,
            PublicKeysSignaturesValidator<BLS>,
            BLS,
        >,
        identity_update_validator: ValidateIdentityUpdateTransitionBasic<
            PublicKeysValidator<BLS>,
            PublicKeysSignaturesValidator<BLS>,
        >,
        identity_top_up_validator: IdentityTopUpTransitionBasicValidator<SR>,
        identity_credit_withdrawal_validator: IdentityCreditWithdrawalTransitionBasicValidator,
        document_batch_validator: DocumentBatchTransitionBasicValidator<SR>,
        identity_credit_transfer_validator: IdentityCreditTransferTransitionBasicValidator,
    ) -> Self {
        StateTransitionByTypeValidator {
            data_contract_create_validator,
            data_contract_update_validator,
            identity_create_validator,
            identity_update_validator,
            identity_top_up_validator,
            identity_credit_withdrawal_validator,
            document_batch_validator,
            identity_credit_transfer_validator,
        }
    }
}

#[async_trait(?Send)]
impl<SR, BLS> ValidatorByStateTransitionType for StateTransitionByTypeValidator<SR, BLS>
where
    SR: StateRepositoryLike,
    BLS: BlsModule,
{
    async fn validate(
        &self,
        raw_state_transition: &Value,
        state_transition_type: StateTransitionType,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let mut result = ConsensusValidationResult::default();

        let validation_result = match state_transition_type {
            StateTransitionType::DataContractCreate => self
                .data_contract_create_validator
                .validate(raw_state_transition, execution_context)?,
            StateTransitionType::DataContractUpdate => {
                self.data_contract_update_validator
                    .validate(raw_state_transition, execution_context)
                    .await?
            }
            StateTransitionType::IdentityCreate => {
                self.identity_create_validator
                    .validate(raw_state_transition, execution_context)
                    .await?
            }
            StateTransitionType::IdentityUpdate => self
                .identity_update_validator
                .validate(raw_state_transition)?,
            StateTransitionType::IdentityTopUp => {
                self.identity_top_up_validator
                    .validate(raw_state_transition, execution_context)
                    .await?
            }
            StateTransitionType::IdentityCreditWithdrawal => {
                self.identity_credit_withdrawal_validator
                    .validate(raw_state_transition)
                    .await?
            }
            StateTransitionType::DocumentsBatch => {
                self.document_batch_validator
                    .validate(raw_state_transition, execution_context)
                    .await?
            }
            StateTransitionType::IdentityCreditTransfer => {
                self.identity_credit_transfer_validator
                    .validate(raw_state_transition)
                    .await?
            }
        };

        result.merge(validation_result);

        Ok(result)
    }
}
