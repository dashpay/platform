pub mod contract;
pub mod document;
pub mod identity;

use crate::state_transition_action::contract::data_contract_create::DataContractCreateTransitionAction;
use crate::state_transition_action::contract::data_contract_update::DataContractUpdateTransitionAction;
use crate::state_transition_action::document::documents_batch::DocumentsBatchTransitionAction;
use crate::state_transition_action::identity::identity_create::IdentityCreateTransitionAction;
use crate::state_transition_action::identity::identity_credit_transfer::IdentityCreditTransferTransitionAction;
use crate::state_transition_action::identity::identity_credit_withdrawal::IdentityCreditWithdrawalTransitionAction;
use crate::state_transition_action::identity::identity_topup::IdentityTopUpTransitionAction;
use crate::state_transition_action::identity::identity_update::IdentityUpdateTransitionAction;
use derive_more::From;

#[derive(Debug, Clone, From)]
pub enum StateTransitionAction<'a> {
    DataContractCreateAction(DataContractCreateTransitionAction),
    DataContractUpdateAction(DataContractUpdateTransitionAction),
    DocumentsBatchAction(DocumentsBatchTransitionAction<'a>),
    IdentityCreateAction(IdentityCreateTransitionAction),
    IdentityTopUpAction(IdentityTopUpTransitionAction),
    IdentityCreditWithdrawalAction(IdentityCreditWithdrawalTransitionAction),
    IdentityUpdateAction(IdentityUpdateTransitionAction),
    IdentityCreditTransferAction(IdentityCreditTransferTransitionAction),
}
