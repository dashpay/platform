use crate::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;

use crate::prelude::IdentityNonce;
use platform_value::Identifier;

/// A trait that contains getter and setter methods for `DocumentBaseTransitionV0`
pub trait DocumentBaseTransitionV0Methods {
    /// Returns the document ID.
    fn id(&self) -> Identifier;

    /// Sets the document ID.
    fn set_id(&mut self, id: Identifier);

    /// Returns the document type name.
    fn document_type_name(&self) -> &String;

    /// Returns the document type name as owned.
    fn document_type_name_owned(self) -> String;

    /// Sets the document type name.
    fn set_document_type_name(&mut self, document_type_name: String);

    /// Returns the data contract ID.
    fn data_contract_id(&self) -> Identifier;
    fn data_contract_id_ref(&self) -> &Identifier;

    /// Sets the data contract ID.
    fn set_data_contract_id(&mut self, data_contract_id: Identifier);
    fn identity_contract_nonce(&self) -> IdentityNonce;
    fn set_identity_contract_nonce(&mut self, identity_contract_nonce: IdentityNonce);
}

impl DocumentBaseTransitionV0Methods for DocumentBaseTransitionV0 {
    fn id(&self) -> Identifier {
        self.id
    }

    fn set_id(&mut self, id: Identifier) {
        self.id = id;
    }

    fn document_type_name(&self) -> &String {
        &self.document_type_name
    }

    fn document_type_name_owned(self) -> String {
        self.document_type_name
    }

    fn set_document_type_name(&mut self, document_type_name: String) {
        self.document_type_name = document_type_name;
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
