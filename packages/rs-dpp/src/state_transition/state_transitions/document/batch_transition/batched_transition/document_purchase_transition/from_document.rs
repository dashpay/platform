use crate::consensus::basic::document::InvalidDocumentTransitionActionError;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::{Document, DocumentV0Getters};
use crate::fee::Credits;
use crate::prelude::Identifier;
use crate::prelude::IdentityNonce;
use crate::ProtocolError;
use platform_version::version::{FeatureVersion, PlatformVersion};

use crate::state_transition::batch_transition::batched_transition::document_purchase_transition::validate_structure::DocumentPurchaseTransitionStructureValidation;
use crate::state_transition::batch_transition::batched_transition::document_purchase_transition::DocumentPurchaseTransitionV0;
use crate::state_transition::batch_transition::batched_transition::DocumentPurchaseTransition;
use crate::tokens::token_payment_info::TokenPaymentInfo;

impl DocumentPurchaseTransition {
    #[allow(clippy::too_many_arguments)]
    pub fn from_document(
        document: Document,
        document_type: DocumentTypeRef,
        new_owner_id: Identifier,
        price: Credits,
        token_payment_info: Option<TokenPaymentInfo>,
        identity_contract_nonce: IdentityNonce,
        platform_version: &PlatformVersion,
        feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<Self, ProtocolError> {
        // Self-purchase is intentionally version-independent: every current
        // and future purchase transition version must reject transferring a
        // document to its existing owner before constructing the transition.
        if document.owner_id() == new_owner_id {
            return Err(ProtocolError::ConsensusError(Box::new(
                InvalidDocumentTransitionActionError::new(format!(
                    "on document type: {} identity trying to purchase a document that is already owned by the purchaser",
                    document_type.name()
                ))
                .into(),
            )));
        }
        match feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .document_purchase_state_transition
                .bounds
                .default_current_version,
        ) {
            0 => {
                let transition: DocumentPurchaseTransition =
                    DocumentPurchaseTransitionV0::from_document(
                        document,
                        document_type,
                        price,
                        token_payment_info,
                        identity_contract_nonce,
                        platform_version,
                        base_feature_version,
                    )?
                    .into();
                if let Some(error) = transition
                    .validate_structure(document_type, platform_version)?
                    .errors_to_consensus_protocol_error()
                {
                    return Err(error);
                }
                Ok(transition)
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentPurchaseTransition::from_document".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
