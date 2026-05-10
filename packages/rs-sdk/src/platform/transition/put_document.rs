use super::broadcast::BroadcastStateTransition;
use super::validation::ensure_valid_state_transition_structure;
use super::waitable::Waitable;
use crate::platform::transition::put_settings::PutSettings;
use crate::{Error, Sdk};
use dpp::dashcore::secp256k1::rand::rngs::StdRng;
use dpp::dashcore::secp256k1::rand::{Rng, SeedableRng};
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentType;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters, INITIAL_REVISION};
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::StateTransition;
use dpp::tokens::token_payment_info::TokenPaymentInfo;

fn is_document_replace_revision(revision: Option<u64>) -> bool {
    revision.is_some_and(|rev| rev != INITIAL_REVISION)
}

fn resolve_document_create_entropy(
    document: &Document,
    document_type: &DocumentType,
    document_state_transition_entropy: Option<[u8; 32]>,
) -> (Document, [u8; 32]) {
    document_state_transition_entropy
        .map(|entropy| (document.clone(), entropy))
        .unwrap_or_else(|| {
            let mut rng = StdRng::from_entropy();
            let mut doc = document.clone();
            let entropy = rng.gen::<[u8; 32]>();
            doc.set_id(Document::generate_document_id_v0(
                &document_type.data_contract_id(),
                &doc.owner_id(),
                document_type.name(),
                entropy.as_slice(),
            ));
            (doc, entropy)
        })
}

#[async_trait::async_trait]
/// A trait for putting a document to platform
pub trait PutDocument<S: Signer<IdentityPublicKey>>: Waitable {
    /// Puts a document on platform
    /// setting settings to `None` sets default connection behavior
    #[allow(clippy::too_many_arguments)]
    async fn put_to_platform(
        &self,
        sdk: &Sdk,
        document_type: DocumentType,
        document_state_transition_entropy: Option<[u8; 32]>,
        identity_public_key: IdentityPublicKey,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error>;

    /// Puts a document on platform and waits for the confirmation proof
    #[allow(clippy::too_many_arguments)]
    async fn put_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        document_type: DocumentType,
        document_state_transition_entropy: Option<[u8; 32]>,
        identity_public_key: IdentityPublicKey,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Document, Error>;
}

/// Build, sign, and structurally validate a document create-or-replace
/// [`StateTransition`] without broadcasting it.
///
/// This is the pre-broadcast half of [`PutDocument::put_to_platform`]: it
/// allocates a fresh identity-contract nonce, picks the create-vs-replace
/// branch based on the document's revision, fills in entropy when missing,
/// applies `user_fee_increase` / `token_payment_info` /
/// `state_transition_creation_options` from `settings`, signs the transition,
/// and runs structure validation. The caller decides whether (and how) to
/// broadcast the returned, signed transition.
///
/// Errors from this function may have already advanced the local nonce cache
/// without a corresponding remote nonce consumption; if the caller cannot
/// safely retry the transition itself, it should call
/// [`Sdk::refresh_identity_nonce`] to resync.
#[allow(clippy::too_many_arguments)]
pub async fn build_signed_document_create_or_replace_transition<S: Signer<IdentityPublicKey>>(
    sdk: &Sdk,
    document: &Document,
    document_type: &DocumentType,
    document_state_transition_entropy: Option<[u8; 32]>,
    identity_public_key: &IdentityPublicKey,
    token_payment_info: Option<TokenPaymentInfo>,
    signer: &S,
    settings: Option<PutSettings>,
) -> Result<StateTransition, Error> {
    let new_identity_contract_nonce = sdk
        .get_identity_contract_nonce(
            document.owner_id(),
            document_type.data_contract_id(),
            true,
            settings,
        )
        .await?;

    let put_settings = settings.unwrap_or_default();
    let transition = if is_document_replace_revision(document.revision()) {
        BatchTransition::new_document_replacement_transition_from_document(
            document.clone(),
            document_type.as_ref(),
            identity_public_key,
            new_identity_contract_nonce,
            put_settings.user_fee_increase.unwrap_or_default(),
            token_payment_info,
            signer,
            sdk.version(),
            put_settings.state_transition_creation_options,
        )
        .await?
    } else {
        let (doc, entropy) = resolve_document_create_entropy(
            document,
            document_type,
            document_state_transition_entropy,
        );
        BatchTransition::new_document_creation_transition_from_document(
            doc,
            document_type.as_ref(),
            entropy,
            identity_public_key,
            new_identity_contract_nonce,
            put_settings.user_fee_increase.unwrap_or_default(),
            token_payment_info,
            signer,
            sdk.version(),
            put_settings.state_transition_creation_options,
        )
        .await?
    };
    ensure_valid_state_transition_structure(&transition, sdk.version())?;
    Ok(transition)
}

#[async_trait::async_trait]
impl<S: Signer<IdentityPublicKey>> PutDocument<S> for Document {
    async fn put_to_platform(
        &self,
        sdk: &Sdk,
        document_type: DocumentType,
        document_state_transition_entropy: Option<[u8; 32]>,
        identity_public_key: IdentityPublicKey,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error> {
        let transition = build_signed_document_create_or_replace_transition(
            sdk,
            self,
            &document_type,
            document_state_transition_entropy,
            &identity_public_key,
            token_payment_info,
            signer,
            settings,
        )
        .await?;

        // response is empty for a broadcast, result comes from the stream wait for state transition result
        transition
            .broadcast(sdk, Some(settings.unwrap_or_default()))
            .await?;
        Ok(transition)
    }

    async fn put_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        document_type: DocumentType,
        document_state_transition_entropy: Option<[u8; 32]>,
        identity_public_key: IdentityPublicKey,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Document, Error> {
        let state_transition = self
            .put_to_platform(
                sdk,
                document_type,
                document_state_transition_entropy,
                identity_public_key,
                token_payment_info,
                signer,
                settings,
            )
            .await?;

        Self::wait_for_response(sdk, state_transition, settings).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::document::DocumentV0;
    use dpp::platform_value::Value;
    use dpp::prelude::Identifier;
    use dpp::version::PlatformVersion;
    use serde_json::json;
    use std::collections::BTreeMap;

    fn test_document_type() -> DocumentType {
        let platform_version = PlatformVersion::latest();
        let schema = serde_json::from_value::<Value>(json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "position": 0
                }
            },
            "additionalProperties": false,
        }))
        .expect("document schema");

        let config = DataContractConfig::default_for_version(platform_version)
            .expect("default data contract config");

        DocumentType::try_from_schema(
            Identifier::random(),
            1,
            config.version(),
            "note",
            schema,
            None,
            &BTreeMap::new(),
            &config,
            true,
            &mut vec![],
            platform_version,
        )
        .expect("document type")
    }

    fn test_document(revision: Option<u64>, id: Identifier) -> Document {
        Document::V0(DocumentV0 {
            id,
            owner_id: Identifier::from([7; 32]),
            properties: Default::default(),
            revision,
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        })
    }

    #[test]
    fn branch_selection_uses_revision_rules() {
        assert!(!is_document_replace_revision(None));
        assert!(!is_document_replace_revision(Some(INITIAL_REVISION)));
        assert!(is_document_replace_revision(Some(INITIAL_REVISION + 1)));
    }

    #[test]
    fn creation_entropy_fallback_regenerates_document_id() {
        let document_type = test_document_type();
        let original_id = Identifier::from([3; 32]);
        let document = test_document(None, original_id);

        let (resolved_document, entropy) =
            resolve_document_create_entropy(&document, &document_type, None);

        let expected_id = Document::generate_document_id_v0(
            &document_type.data_contract_id(),
            &document.owner_id(),
            document_type.name(),
            entropy.as_slice(),
        );

        assert_eq!(resolved_document.id(), expected_id);
        assert_ne!(resolved_document.id(), original_id);
    }

    #[test]
    fn provided_entropy_preserves_existing_document_id() {
        let document_type = test_document_type();
        let original_id = Identifier::from([9; 32]);
        let document = test_document(Some(INITIAL_REVISION), original_id);
        let provided_entropy = [11; 32];

        let (resolved_document, resolved_entropy) =
            resolve_document_create_entropy(&document, &document_type, Some(provided_entropy));

        assert_eq!(resolved_entropy, provided_entropy);
        assert_eq!(resolved_document.id(), original_id);
    }
}
