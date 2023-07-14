pub mod identity;
pub mod document;
pub mod contract;

use derive_more::From;
use crate::state_transition::data_contract_create_transition::DataContractCreateTransitionAction;

#[derive(Debug, Clone, From)]
pub enum StateTransitionAction {
    DataContractCreateAction(DataContractCreateTransitionAction),
    DataContractUpdateAction(DataContractUpdateTransitionAction),
    DocumentsBatchAction(DocumentsBatchTransitionAction),
    IdentityCreateAction(IdentityCreateTransitionAction),
    IdentityTopUpAction(IdentityTopUpTransitionAction),
    IdentityCreditWithdrawalAction(IdentityCreditWithdrawalTransitionAction),
    IdentityUpdateAction(IdentityUpdateTransitionAction),
    IdentityCreditTransferAction(IdentityCreditTransferTransitionAction),
}
