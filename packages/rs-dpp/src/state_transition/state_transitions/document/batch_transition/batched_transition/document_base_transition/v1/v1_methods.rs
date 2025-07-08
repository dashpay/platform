use crate::prelude::IdentityNonce;
use crate::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use crate::state_transition::batch_transition::document_base_transition::v1::DocumentBaseTransitionV1;
use crate::tokens::token_payment_info::TokenPaymentInfo;
use platform_value::Identifier;

/// A trait that contains getter and setter methods for `DocumentBaseTransitionV0`
pub trait DocumentBaseTransitionV1Methods: DocumentBaseTransitionV0Methods {
    /// Returns the token payment info.
    fn token_payment_info(&self) -> Option<TokenPaymentInfo>;

    /// Returns the token payment info.
    fn token_payment_info_ref(&self) -> &Option<TokenPaymentInfo>;

    /// Sets the token payment info.
    fn set_token_payment_info(&mut self, token_payment_info: TokenPaymentInfo);

    /// Clears the token payment info.
    fn clear_token_payment_info(&mut self);
}

impl DocumentBaseTransitionV1Methods for DocumentBaseTransitionV1 {
    fn token_payment_info(&self) -> Option<TokenPaymentInfo> {
        self.token_payment_info
    }

    fn token_payment_info_ref(&self) -> &Option<TokenPaymentInfo> {
        &self.token_payment_info
    }

    fn set_token_payment_info(&mut self, token_payment_info: TokenPaymentInfo) {
        self.token_payment_info = Some(token_payment_info);
    }

    fn clear_token_payment_info(&mut self) {
        self.token_payment_info = None;
    }
}

impl DocumentBaseTransitionV0Methods for DocumentBaseTransitionV1 {
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
