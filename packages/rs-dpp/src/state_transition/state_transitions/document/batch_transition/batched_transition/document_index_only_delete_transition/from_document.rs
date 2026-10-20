use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::Document;
use crate::prelude::IdentityNonce;
use crate::ProtocolError;
use platform_version::version::{FeatureVersion, PlatformVersion};

use crate::state_transition::batch_transition::batched_transition::document_index_only_delete_transition::DocumentIndexOnlyDeleteTransitionV0;
use crate::state_transition::batch_transition::batched_transition::DocumentIndexOnlyDeleteTransition;
use crate::tokens::token_payment_info::TokenPaymentInfo;

impl DocumentIndexOnlyDeleteTransition {
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
        // `None` bounds mean the kind does not exist at this platform
        // version (it joined the wire at PV14) — constructing one there
        // could only ever produce a transition the network refuses.
        let bounds = platform_version
            .dpp
            .state_transition_serialization_versions
            .document_index_only_delete_state_transition
            .as_ref()
            .ok_or_else(|| {
                ProtocolError::Generic(
                    "indexOnly delete transitions do not exist at this platform version"
                        .to_string(),
                )
            })?;
        match feature_version.unwrap_or(bounds.bounds.default_current_version) {
            0 => Ok(DocumentIndexOnlyDeleteTransitionV0::from_document(
                document,
                document_type,
                token_payment_info,
                identity_contract_nonce,
                platform_version,
                base_feature_version,
            )?
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentIndexOnlyDeleteTransition::from_document".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
