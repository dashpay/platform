/// contract
pub mod contract;
/// documents
pub mod document;
/// identity
pub mod identity;

/// system
pub mod system;

use crate::state_transition_action::contract::data_contract_create::DataContractCreateTransitionAction;
use crate::state_transition_action::contract::data_contract_update::DataContractUpdateTransitionAction;
use crate::state_transition_action::document::documents_batch::DocumentsBatchTransitionAction;
use crate::state_transition_action::identity::identity_create::IdentityCreateTransitionAction;
use crate::state_transition_action::identity::identity_credit_transfer::IdentityCreditTransferTransitionAction;
use crate::state_transition_action::identity::identity_credit_withdrawal::IdentityCreditWithdrawalTransitionAction;
use crate::state_transition_action::identity::identity_topup::IdentityTopUpTransitionAction;
use crate::state_transition_action::identity::identity_update::IdentityUpdateTransitionAction;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;
use crate::state_transition_action::system::bump_identity_nonce_action::BumpIdentityNonceAction;
use derive_more::From;

/// ST action
#[derive(Debug, Clone, From)]
pub enum StateTransitionAction {
    /// data contract create
    DataContractCreateAction(DataContractCreateTransitionAction),
    /// data contract update
    DataContractUpdateAction(DataContractUpdateTransitionAction),
    /// documents batch
    DocumentsBatchAction(DocumentsBatchTransitionAction),
    /// identity create
    IdentityCreateAction(IdentityCreateTransitionAction),
    /// identity topup
    IdentityTopUpAction(IdentityTopUpTransitionAction),
    /// identity credit withdrawal
    IdentityCreditWithdrawalAction(IdentityCreditWithdrawalTransitionAction),
    /// identity update
    IdentityUpdateAction(IdentityUpdateTransitionAction),
    /// identity credit transfer
    IdentityCreditTransferAction(IdentityCreditTransferTransitionAction),
    /// bump identity nonce action
    /// this can only come in this form from identity state transitions that do not use asset locks
    /// it will also only happen if the state validation fails
    BumpIdentityNonceAction(BumpIdentityNonceAction),
    /// bump identity contract nonce action
    /// this can only come in this form from the document contract update state transition
    /// it will also only happen if the state validation fails
    BumpIdentityDataContractNonceAction(BumpIdentityDataContractNonceAction),
}
