use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::Document;
use crate::fee::Credits;
use crate::prelude::IdentityNonce;
use crate::ProtocolError;
use platform_version::version::{FeatureVersion, PlatformVersion};

use crate::state_transition::batch_transition::batched_transition::document_purchase_transition::DocumentPurchaseTransitionV0;
use crate::state_transition::batch_transition::batched_transition::DocumentPurchaseTransition;
use crate::tokens::token_payment_info::TokenPaymentInfo;

impl DocumentPurchaseTransition {
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
                .document_purchase_state_transition
                .bounds
                .default_current_version,
        ) {
            0 => Ok(DocumentPurchaseTransitionV0::from_document(
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
                method: "DocumentPurchaseTransition::from_document".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
