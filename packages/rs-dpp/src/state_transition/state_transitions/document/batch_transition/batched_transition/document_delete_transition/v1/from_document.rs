use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::{Document, DocumentV0Getters};
use crate::prelude::IdentityNonce;
use crate::state_transition::batch_transition::batched_transition::document_delete_transition::DocumentDeleteTransitionV1;
use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use crate::tokens::token_payment_info::TokenPaymentInfo;
use crate::ProtocolError;
use platform_version::version::{FeatureVersion, PlatformVersion};

impl DocumentDeleteTransitionV1 {
    pub(crate) fn from_document(
        document: Document,
        document_type: DocumentTypeRef,
        token_payment_info: Option<TokenPaymentInfo>,
        identity_contract_nonce: IdentityNonce,
        platform_version: &PlatformVersion,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<Self, ProtocolError> {
        Ok(DocumentDeleteTransitionV1 {
            base: DocumentBaseTransition::from_document(
                &document,
                document_type,
                token_payment_info,
                identity_contract_nonce,
                platform_version,
                base_feature_version,
            )?,
            // The values ARE the document on an indexOnly type — the
            // delete carries them so every index entry can be recomputed
            // without a primary-storage fetch. `$createdAt` rides along
            // under its system key when set: an indexOnly type may index
            // it, and then it is part of the entry paths being removed.
            data: {
                let mut data = document.properties().clone();
                if let Some(created_at) = document.created_at() {
                    data.insert(
                        crate::document::property_names::CREATED_AT.to_string(),
                        platform_value::Value::U64(created_at),
                    );
                }
                data
            },
        })
    }
}
