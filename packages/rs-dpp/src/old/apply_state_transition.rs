use std::sync::Arc;
use anyhow::anyhow;
use crate::data_contract::state_transition::data_contract_create_transition::apply_data_contract_create_transition_factory::ApplyDataContractCreateTransition;
use crate::data_contract::state_transition::data_contract_update_transition::apply_data_contract_update_transition_factory::ApplyDataContractUpdateTransition;
use crate::document::state_transition::documents_batch_transition::apply_documents_batch_transition_factory::ApplyDocumentsBatchTransition;
use crate::identity::state_transition::asset_lock_proof::AssetLockTransactionOutputFetcher;
use crate::identity::state_transition::identity_create_transition::ApplyIdentityCreateTransition;
use crate::identity::state_transition::identity_topup_transition::ApplyIdentityTopUpTransition;
use crate::identity::state_transition::identity_update_transition::apply_identity_update_transition::ApplyIdentityUpdateTransition;
use crate::identity::state_transition::identity_credit_transfer_transition::apply_identity_credit_transfer::ApplyIdentityCreditTransferTransition;
use crate::ProtocolError;
use crate::state_repository::StateRepositoryLike;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::state_transition::StateTransition;

#[derive(Clone)]
pub struct ApplyStateTransition<SR: StateRepositoryLike> {
    apply_data_contract_create_transition: ApplyDataContractCreateTransition<SR>,
    apply_data_contract_update_transition: ApplyDataContractUpdateTransition<SR>,
    apply_documents_batch_transition: ApplyDocumentsBatchTransition<SR>,
    apply_identity_create_transition: ApplyIdentityCreateTransition<SR>,
    apply_identity_top_up_transition: ApplyIdentityTopUpTransition<SR>,
    apply_identity_update_transition: ApplyIdentityUpdateTransition<SR>,
    apply_identity_credit_transfer_transition: ApplyIdentityCreditTransferTransition<SR>,
}

impl<SR> ApplyStateTransition<SR>
where
    SR: StateRepositoryLike,
{
    pub fn new(
        state_repository: Arc<SR>,
        asset_lock_transaction_output_fetcher: Arc<AssetLockTransactionOutputFetcher<SR>>,
    ) -> Self {
        Self {
            apply_data_contract_create_transition: ApplyDataContractCreateTransition::new(
                state_repository.clone(),
            ),
            apply_data_contract_update_transition: ApplyDataContractUpdateTransition::new(
                state_repository.clone(),
            ),
            apply_documents_batch_transition: ApplyDocumentsBatchTransition::new(
                state_repository.clone(),
            ),
            apply_identity_create_transition: ApplyIdentityCreateTransition::new(
                state_repository.clone(),
            ),
            apply_identity_top_up_transition: ApplyIdentityTopUpTransition::new(
                state_repository.clone(),
                asset_lock_transaction_output_fetcher,
            ),
            apply_identity_credit_transfer_transition: ApplyIdentityCreditTransferTransition::new(
                state_repository.clone(),
            ),
            apply_identity_update_transition: ApplyIdentityUpdateTransition::new(state_repository),
        }
    }

    pub async fn apply(&self, state_transition: &StateTransition) -> Result<(), ProtocolError> {
        // TODO(v0.24-backport): is it fine using default context here?
        //   (Check if applier is actually used in the drive executor)
        let execution_context = StateTransitionExecutionContext::default();
        match state_transition {
            StateTransition::DataContractCreate(st) => self
                .apply_data_contract_create_transition
                .apply_data_contract_create_transition(st, Some(&execution_context))
                .await
                .map_err(ProtocolError::from),
            StateTransition::DataContractUpdate(st) => self
                .apply_data_contract_update_transition
                .apply_data_contract_update_transition(st, &execution_context)
                .await
                .map_err(ProtocolError::from),
            StateTransition::DocumentsBatch(st) => {
                self.apply_documents_batch_transition
                    .apply(st, execution_context)
                    .await
            }
            StateTransition::IdentityCreate(st) => self
                .apply_identity_create_transition
                .apply_identity_create_transition(st, &execution_context)
                .await
                .map_err(ProtocolError::from),
            StateTransition::IdentityTopUp(st) => self
                .apply_identity_top_up_transition
                .apply(st, &execution_context)
                .await
                .map_err(ProtocolError::from),
            StateTransition::IdentityCreditWithdrawal(_) => {
                Err(ProtocolError::Error(anyhow!("Not implemented yet")))
            }
            StateTransition::IdentityUpdate(st) => {
                self.apply_identity_update_transition
                    .apply(st, &execution_context)
                    .await
            }
            StateTransition::IdentityCreditTransfer(st) => self
                .apply_identity_credit_transfer_transition
                .apply(st, &execution_context)
                .await
                .map_err(ProtocolError::from),
        }
    }
}
