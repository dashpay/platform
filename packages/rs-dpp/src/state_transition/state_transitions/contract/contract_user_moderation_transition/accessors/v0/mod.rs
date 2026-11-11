use crate::prelude::{Identifier, IdentityNonce};
use crate::state_transition::contract_user_moderation_transition::v0::ContractUserModerationAction;

pub trait ContractUserModerationTransitionAccessorsV0 {
    fn set_owner_id(&mut self, id: Identifier);
    fn set_data_contract_id(&mut self, id: Identifier);
    /// The moderated contract
    fn data_contract_id(&self) -> Identifier;
    fn set_identity_contract_nonce(&mut self, nonce: IdentityNonce);
    /// The signer's nonce for the contract
    fn identity_contract_nonce(&self) -> IdentityNonce;
    fn set_action(&mut self, action: ContractUserModerationAction);
    /// What is done, to whom
    fn action(&self) -> &ContractUserModerationAction;
    /// The identity the action targets, `None` for a document deletion
    fn target_identity_id(&self) -> Option<Identifier> {
        self.action().identity_id()
    }
}
