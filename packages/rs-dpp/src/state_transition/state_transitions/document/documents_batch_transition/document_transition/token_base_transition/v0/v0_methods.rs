use crate::prelude::IdentityNonce;
use crate::state_transition::documents_batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
use platform_value::Identifier;

/// A trait that contains getter and setter methods for `TokenBaseTransitionV0`
pub trait TokenBaseTransitionV0Methods {
    /// Returns the document ID.
    fn id(&self) -> Identifier;

    /// Sets the document ID.
    fn set_id(&mut self, id: Identifier);

    /// Returns the document type name.
    fn token_id(&self) -> u16;

    /// Sets the token id.
    fn set_token_id(&mut self, token_id: u16);

    /// Returns the data contract ID.
    fn data_contract_id(&self) -> Identifier;
    fn data_contract_id_ref(&self) -> &Identifier;

    /// Sets the data contract ID.
    fn set_data_contract_id(&mut self, data_contract_id: Identifier);
    fn identity_contract_nonce(&self) -> IdentityNonce;
    fn set_identity_contract_nonce(&mut self, identity_contract_nonce: IdentityNonce);
}

impl TokenBaseTransitionV0Methods for TokenBaseTransitionV0 {
    fn id(&self) -> Identifier {
        self.id
    }

    fn set_id(&mut self, id: Identifier) {
        self.id = id;
    }

    fn token_id(&self) -> u16 {
        self.token_id
    }

    fn set_token_id(&mut self, token_id: u16) {
        self.token_id = token_id;
    }

    fn data_contract_id(&self) -> Identifier {
        self.data_contract_id
    }

    fn data_contract_id_ref(&self) -> &Identifier {
        &self.data_contract_id
    }

    fn set_data_contract_id(&mut self, data_contract_id: Identifier) {
        self.data_contract_id = data_contract_id;
    }

    fn identity_contract_nonce(&self) -> IdentityNonce {
        self.identity_contract_nonce
    }

    fn set_identity_contract_nonce(&mut self, identity_contract_nonce: IdentityNonce) {
        self.identity_contract_nonce = identity_contract_nonce;
    }
}
