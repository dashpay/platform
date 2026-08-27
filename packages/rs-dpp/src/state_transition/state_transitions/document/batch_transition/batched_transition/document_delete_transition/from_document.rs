use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::Document;
use crate::prelude::IdentityNonce;
use crate::ProtocolError;
use platform_version::version::{FeatureVersion, PlatformVersion};

use crate::state_transition::batch_transition::batched_transition::document_delete_transition::DocumentDeleteTransitionV0;
use crate::state_transition::batch_transition::batched_transition::DocumentDeleteTransition;
use crate::tokens::token_payment_info::TokenPaymentInfo;

impl DocumentDeleteTransition {
    #[allow(clippy::too_many_arguments)]
    pub fn from_document(
        document: Document,
        document_type: DocumentTypeRef,
        token_payment_info: Option<TokenPaymentInfo>,
        identity_contract_nonce: IdentityNonce,
        platform_version: &PlatformVersion,
        feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<Self, ProtocolError> {
        // An indexOnly document is deleted from its values (V1); everything
        // else keeps the table's default (V0, delete-by-id). Selecting here
        // means every construction path — SDKs included — produces the
        // variant the ABCI structure gates require for the doctype, with no
        // per-client knowledge of the storage mode.
        let default_version = {
            use crate::data_contract::document_type::accessors::DocumentTypeV2Getters;
            if document_type.index_only() {
                1
            } else {
                platform_version
                    .dpp
                    .state_transition_serialization_versions
                    .document_delete_state_transition
                    .bounds
                    .default_current_version
            }
        };
        match feature_version.unwrap_or(default_version) {
            0 => Ok(DocumentDeleteTransitionV0::from_document(
                document,
                document_type,
                token_payment_info,
                identity_contract_nonce,
                platform_version,
                base_feature_version,
            )?
            .into()),
            1 => Ok(
                crate::state_transition::batch_transition::batched_transition::document_delete_transition::DocumentDeleteTransitionV1::from_document(
                    document,
                    document_type,
                    token_payment_info,
                    identity_contract_nonce,
                    platform_version,
                    base_feature_version,
                )?
                .into(),
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentDeleteTransition::from_document".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
}
