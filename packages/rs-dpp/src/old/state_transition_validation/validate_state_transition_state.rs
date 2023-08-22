use crate::validation::{AsyncDataValidator, ConsensusValidationResult};

pub struct StateTransitionStateValidator<SR>
where
    SR: StateRepositoryLike + Clone,
{
    state_repository: SR,
    data_contract_create_validator: DataContractCreateTransitionStateValidator<SR>,
    data_contract_update_validator: DataContractUpdateTransitionStateValidator<SR>,
    identity_create_validator: IdentityCreateTransitionStateValidator<SR>,
    identity_update_validator:
        IdentityUpdateTransitionStateValidator<IdentityUpdatePublicKeysValidator, SR>,
    identity_top_up_validator: IdentityTopUpTransitionStateValidator<SR>,
    identity_credit_withdrawal_validator: IdentityCreditWithdrawalTransitionValidator<SR>,
    document_batch_validator: DocumentsBatchTransitionStateValidator<SR>,
    identity_credit_transfer_validator: IdentityCreditTransferTransitionStateValidator<SR>,
}

impl<SR> StateTransitionStateValidator<SR>
where
    SR: StateRepositoryLike + Clone,
{
    pub fn new(state_repository: SR) -> Self {
        let wrapped_state_repository = Arc::new(state_repository.clone());

        let data_contract_create_validator =
            DataContractCreateTransitionStateValidator::new(state_repository.clone());
        let data_contract_update_validator =
            DataContractUpdateTransitionStateValidator::new(state_repository.clone());
        let identity_create_validator =
            IdentityCreateTransitionStateValidator::new(state_repository.clone());
        let identity_update_validator = IdentityUpdateTransitionStateValidator::new(
            wrapped_state_repository.clone(),
            Arc::new(IdentityUpdatePublicKeysValidator {}),
        );
        let identity_top_up_validator =
            IdentityTopUpTransitionStateValidator::new(state_repository.clone());
        let identity_credit_withdrawal_validator =
            IdentityCreditWithdrawalTransitionValidator::new(wrapped_state_repository);
        let document_batch_validator =
            DocumentsBatchTransitionStateValidator::new(state_repository.clone());
        let identity_credit_transfer_validator =
            IdentityCreditTransferTransitionStateValidator::new(state_repository.clone());

        StateTransitionStateValidator {
            state_repository,
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

    pub async fn validate(
        &self,
        state_transition: &StateTransition,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, ProtocolError> {
        match state_transition {
            StateTransition::DataContractCreate(st) => Ok(self
                .data_contract_create_validator
                .validate(st, execution_context)
                .await?
                .map(DataContractCreateAction)),

            StateTransition::DataContractUpdate(st) => Ok(self
                .data_contract_update_validator
                .validate(st, execution_context)
                .await?
                .map(DataContractUpdateAction)),
            StateTransition::IdentityCreate(st) => Ok(self
                .identity_create_validator
                .validate(st, execution_context)
                .await?
                .map(IdentityCreateAction)),
            StateTransition::IdentityUpdate(st) => Ok(self
                .identity_update_validator
                .validate(st, execution_context)
                .await?
                .map(IdentityUpdateAction)),
            StateTransition::IdentityTopUp(st) => Ok(self
                .identity_top_up_validator
                .validate(st, execution_context)
                .await?
                .map(IdentityTopUpAction)),
            StateTransition::IdentityCreditWithdrawal(st) => Ok(self
                .identity_credit_withdrawal_validator
                .validate_identity_credit_withdrawal_transition_state(st, execution_context)
                .await?
                .map(IdentityCreditWithdrawalAction)),
            StateTransition::DocumentsBatch(st) => Ok(self
                .document_batch_validator
                .validate(st, execution_context)
                .await?
                .map(DocumentsBatchAction)),
            StateTransition::IdentityCreditTransfer(st) => Ok(self
                .identity_credit_transfer_validator
                .validate(st, execution_context)
                .await?
                .map(IdentityCreditTransferAction)),
        }
    }
}
