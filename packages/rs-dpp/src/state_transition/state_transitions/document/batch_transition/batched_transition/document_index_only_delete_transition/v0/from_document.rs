use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::property_names::CREATED_AT;
use crate::document::{Document, DocumentV0Getters};
use crate::prelude::IdentityNonce;
use crate::state_transition::batch_transition::batched_transition::document_index_only_delete_transition::DocumentIndexOnlyDeleteTransitionV0;
use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use crate::tokens::token_payment_info::TokenPaymentInfo;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::{FeatureVersion, PlatformVersion};

impl DocumentIndexOnlyDeleteTransitionV0 {
    pub(crate) fn from_document(
        document: Document,
        document_type: DocumentTypeRef,
        token_payment_info: Option<TokenPaymentInfo>,
        identity_contract_nonce: IdentityNonce,
        platform_version: &PlatformVersion,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<Self, ProtocolError> {
        Ok(DocumentIndexOnlyDeleteTransitionV0 {
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
            // under its system key exactly when the doctype requires it
            // (an indexed `$createdAt` forces the requirement, and it
            // feeds the row commitment) — keyed on the TYPE, not on
            // whatever the local `Document` object happens to carry, so
            // construction always emits the payload shape the structure
            // validation accepts.
            data: {
                let mut data = document.properties().clone();
                if document_type.required_fields().contains(CREATED_AT) {
                    let created_at = document.created_at().ok_or_else(|| {
                        ProtocolError::Generic(format!(
                            "an indexOnly document of type {} requires $createdAt, but the \
                             document being deleted does not carry one",
                            document_type.name()
                        ))
                    })?;
                    data.insert(CREATED_AT.to_string(), Value::U64(created_at));
                }
                data
            },
        })
    }
}
