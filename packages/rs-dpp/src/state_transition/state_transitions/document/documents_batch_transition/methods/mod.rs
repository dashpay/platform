#[cfg(feature = "state-transition-signing")]
use crate::data_contract::document_type::DocumentTypeRef;
#[cfg(feature = "state-transition-signing")]
use crate::document::Document;
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::IdentityPublicKey;
use crate::prelude::IdentityNonce;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::UserFeeIncrease;
use crate::state_transition::documents_batch_transition::document_transition::DocumentTransition;
use crate::state_transition::documents_batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use crate::state_transition::documents_batch_transition::DocumentsBatchTransition;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::documents_batch_transition::DocumentsBatchTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
#[cfg(feature = "state-transition-signing")]
use crate::ProtocolError;
#[cfg(feature = "state-transition-signing")]
use platform_value::Identifier;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::{FeatureVersion, PlatformVersion};

pub mod v0;

impl DocumentsBatchTransitionMethodsV0 for DocumentsBatchTransition {
    fn all_purchases_amount(&self) -> Option<Credits> {
        match self {
            DocumentsBatchTransition::V0(v0) => v0.all_purchases_amount(),
        }
    }

    fn set_transitions(&mut self, transitions: Vec<DocumentTransition>) {
        match self {
            DocumentsBatchTransition::V0(v0) => v0.set_transitions(transitions),
        }
    }

    fn set_identity_contract_nonce(&mut self, identity_contract_nonce: IdentityNonce) {
        match self {
            DocumentsBatchTransition::V0(v0) => {
                v0.set_identity_contract_nonce(identity_contract_nonce)
            }
        }
    }

    #[cfg(feature = "state-transition-signing")]
    fn new_document_creation_transition_from_document<S: Signer>(
        document: Document,
        document_type: DocumentTypeRef,
        entropy: [u8; 32],
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        batch_feature_version: Option<FeatureVersion>,
        create_feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        match batch_feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .documents_batch_state_transition
                .default_current_version,
        ) {
            0 => Ok(
                DocumentsBatchTransitionV0::new_document_creation_transition_from_document(
                    document,
                    document_type,
                    entropy,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    signer,
                    platform_version,
                    batch_feature_version,
                    create_feature_version,
                    base_feature_version,
                )?,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentsBatchTransition::new_created_from_document".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    #[cfg(feature = "state-transition-signing")]
    fn new_document_replacement_transition_from_document<S: Signer>(
        document: Document,
        document_type: DocumentTypeRef,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        batch_feature_version: Option<FeatureVersion>,
        replace_feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        match batch_feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .documents_batch_state_transition
                .default_current_version,
        ) {
            0 => Ok(
                DocumentsBatchTransitionV0::new_document_replacement_transition_from_document(
                    document,
                    document_type,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    signer,
                    platform_version,
                    batch_feature_version,
                    replace_feature_version,
                    base_feature_version,
                )?,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method:
                    "DocumentsBatchTransition::new_document_replacement_transition_from_document"
                        .to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    #[cfg(feature = "state-transition-signing")]
    fn new_document_transfer_transition_from_document<S: Signer>(
        document: Document,
        document_type: DocumentTypeRef,
        recipient_owner_id: Identifier,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        batch_feature_version: Option<FeatureVersion>,
        transfer_feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        match batch_feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .documents_batch_state_transition
                .default_current_version,
        ) {
            0 => Ok(
                DocumentsBatchTransitionV0::new_document_transfer_transition_from_document(
                    document,
                    document_type,
                    recipient_owner_id,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    signer,
                    platform_version,
                    batch_feature_version,
                    transfer_feature_version,
                    base_feature_version,
                )?,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method:
                    "DocumentsBatchTransition::new_document_replacement_transition_from_document"
                        .to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    #[cfg(feature = "state-transition-signing")]
    fn new_document_deletion_transition_from_document<S: Signer>(
        document: Document,
        document_type: DocumentTypeRef,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        batch_feature_version: Option<FeatureVersion>,
        delete_feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        match batch_feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .documents_batch_state_transition
                .default_current_version,
        ) {
            0 => Ok(
                DocumentsBatchTransitionV0::new_document_deletion_transition_from_document(
                    document,
                    document_type,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    signer,
                    platform_version,
                    batch_feature_version,
                    delete_feature_version,
                    base_feature_version,
                )?,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentsBatchTransition::new_document_deletion_transition_from_document"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    #[cfg(feature = "state-transition-signing")]
    fn new_document_update_price_transition_from_document<S: Signer>(
        document: Document,
        document_type: DocumentTypeRef,
        price: Credits,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        batch_feature_version: Option<FeatureVersion>,
        update_price_feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        match batch_feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .documents_batch_state_transition
                .default_current_version,
        ) {
            0 => Ok(
                DocumentsBatchTransitionV0::new_document_update_price_transition_from_document(
                    document,
                    document_type,
                    price,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    signer,
                    platform_version,
                    batch_feature_version,
                    update_price_feature_version,
                    base_feature_version,
                )?,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method:
                    "DocumentsBatchTransition::new_document_update_price_transition_from_document"
                        .to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    #[cfg(feature = "state-transition-signing")]
    fn new_document_purchase_transition_from_document<S: Signer>(
        document: Document,
        document_type: DocumentTypeRef,
        new_owner_id: Identifier,
        price: Credits,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        batch_feature_version: Option<FeatureVersion>,
        purchase_feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        match batch_feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .documents_batch_state_transition
                .default_current_version,
        ) {
            0 => Ok(
                DocumentsBatchTransitionV0::new_document_purchase_transition_from_document(
                    document,
                    document_type,
                    new_owner_id,
                    price,
                    identity_public_key,
                    identity_contract_nonce,
                    user_fee_increase,
                    signer,
                    platform_version,
                    batch_feature_version,
                    purchase_feature_version,
                    base_feature_version,
                )?,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentsBatchTransition::new_document_purchase_transition_from_document"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
