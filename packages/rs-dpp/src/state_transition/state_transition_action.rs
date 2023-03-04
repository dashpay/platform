use crate::data_contract::state_transition::{DataContractCreateTransitionAction, DataContractUpdateTransitionAction};
use crate::document::state_transition::documents_batch_transition::DocumentsBatchTransitionAction;
use crate::identity::state_transition::identity_create_transition::IdentityCreateTransitionAction;
use crate::identity::state_transition::identity_topup_transition::IdentityTopUpTransitionAction;
use crate::identity::state_transition::identity_update_transition::IdentityUpdateTransitionAction;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateTransitionAction {
    DataContractCreateAction(DataContractCreateTransitionAction),
    DataContractUpdateAction(DataContractUpdateTransitionAction),
    DocumentsBatchAction(DocumentsBatchTransitionAction),
    IdentityCreateAction(IdentityCreateTransitionAction),
    IdentityTopUpAction(IdentityTopUpTransitionAction),
    IdentityCreditWithdrawalAction(IdentityCreditWithdrawalTransitionAction),
    IdentityUpdateAction(IdentityUpdateTransitionAction),
}