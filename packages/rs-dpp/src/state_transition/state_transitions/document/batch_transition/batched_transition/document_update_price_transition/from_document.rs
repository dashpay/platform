use platform_version::version::protocol_version::PlatformVersion;
use versioned_feature_core::FeatureVersion;
use crate::data_contract::document_type::{DocumentTypeRef};
use crate::document::{Document};
use crate::balances::credits::Credits;
use crate::prelude::IdentityNonce;
use crate::ProtocolError;
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::DocumentUpdatePriceTransition;
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::document_update_price_transition::DocumentUpdatePriceTransitionV0;
use crate::tokens::token_payment_info::TokenPaymentInfo;

impl DocumentUpdatePriceTransition {
    #[allow(clippy::too_many_arguments)]
    pub fn from_document(
        document: Document,
        document_type: DocumentTypeRef,
        price: Credits,
        token_payment_info: Option<TokenPaymentInfo>,
        identity_contract_nonce: IdentityNonce,
        platform_version: &PlatformVersion,
        feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<Self, ProtocolError> {
        match feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .document_update_price_state_transition
                .bounds
                .default_current_version,
        ) {
            0 => Ok(DocumentUpdatePriceTransitionV0::from_document(
                document,
                document_type,
                price,
                token_payment_info,
                identity_contract_nonce,
                platform_version,
                base_feature_version,
            )?
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentUpdatePriceTransition::from_document".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
