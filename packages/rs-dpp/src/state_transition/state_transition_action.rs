use derive_more::From;

use crate::document::state_transition::documents_batch_transition::DocumentsBatchTransitionAction;
use crate::identity::state_transition::identity_create_transition::IdentityCreateTransitionAction;
use crate::identity::state_transition::identity_topup_transition::IdentityTopUpTransitionAction;
use crate::identity::state_transition::identity_update_transition::IdentityUpdateTransitionAction;

use crate::data_contract::state_transition::data_contract_create_transition::DataContractCreateTransitionAction;
use crate::data_contract::state_transition::data_contract_update_transition::DataContractUpdateTransitionAction;
use crate::identity::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransitionAction;
use crate::identity::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransitionAction;

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
