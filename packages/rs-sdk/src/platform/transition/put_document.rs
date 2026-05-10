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

/// Reject documents whose revision is `Some(0)` for the create-or-replace
/// dispatch helper. Both create and replace require non-zero revisions, so
/// `0` is always invalid regardless of caller intent.
fn ensure_revision_nonzero(revision: Option<u64>) -> Result<(), Error> {
    if matches!(revision, Some(0)) {
        return Err(Error::Generic(
            "InvalidArgument: document revision 0 is invalid; \
             use unset or 1 (INITIAL_REVISION) for create, or > 1 for replace"
                .to_string(),
        ));
    }
    Ok(())
}

/// Strict revision guard for the document **create** path.
///
/// Accepts `None` and `Some(INITIAL_REVISION)`. Rejects `Some(0)` and any
/// revision strictly greater than `INITIAL_REVISION`. This is the rs-sdk-side
/// fail-fast equivalent of the wasm-sdk `ensureDocumentCreateRevision` guard.
fn ensure_revision_for_create(revision: Option<u64>) -> Result<(), Error> {
    match revision {
        None => Ok(()),
        Some(rev) if rev == INITIAL_REVISION => Ok(()),
        Some(rev) => Err(Error::Generic(format!(
            "InvalidArgument: document revision is {rev} but create requires revision \
             to be unset or {INITIAL_REVISION}; use the replace path for revisions > {INITIAL_REVISION}"
        ))),
    }
}

/// Strict revision guard for the document **replace** path.
///
/// Accepts only `Some(rev)` with `rev > INITIAL_REVISION`. Rejects `None`,
/// `Some(0)`, and `Some(INITIAL_REVISION)`. This is the rs-sdk-side fail-fast
/// equivalent of the wasm-sdk `ensureDocumentReplaceRevision` guard.
fn ensure_revision_for_replace(revision: Option<u64>) -> Result<(), Error> {
    match revision {
        Some(rev) if rev > INITIAL_REVISION => Ok(()),
        Some(rev) => Err(Error::Generic(format!(
            "InvalidArgument: document revision is {rev} but replace requires revision > \
             {INITIAL_REVISION}; use the create path for new documents"
        ))),
        None => Err(Error::Generic(
            "InvalidArgument: document must have a revision set for replace; \
             use the create path for new documents"
                .to_string(),
        )),
    }
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
/// # Revision validation
///
/// The dispatch is driven by the document revision and rejects only the
/// always-invalid `Some(0)` case. Use
/// [`build_signed_document_create_transition`] or
/// [`build_signed_document_replace_transition`] for fail-fast validation that
/// also enforces caller intent (mismatched create/replace revisions error
/// before any nonce allocation).
///
/// # Nonce handling on local errors
///
/// On any **pre-broadcast** failure (build, sign, or local structure
/// validation) this helper conditionally rolls back the bumped
/// identity-contract nonce via
/// [`Sdk::rollback_identity_contract_nonce`], so the local cache does not
/// advance past a nonce the network never observed. The rollback only adjusts
/// the cache entry if it still equals the nonce allocated by this attempt, so
/// concurrent allocations are not clobbered.
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
    // Reject the always-invalid `Some(0)` revision before allocating any
    // nonce. Strict create/replace intent validation is the job of the
    // dedicated helpers below.
    ensure_revision_nonzero(document.revision())?;

    let owner_id = document.owner_id();
    let contract_id = document_type.data_contract_id();
    let new_identity_contract_nonce = sdk
        .get_identity_contract_nonce(owner_id, contract_id, true, settings)
        .await?;

    let result = build_and_sign_create_or_replace_after_nonce(
        sdk,
        document,
        document_type,
        document_state_transition_entropy,
        identity_public_key,
        token_payment_info,
        signer,
        settings,
        new_identity_contract_nonce,
    )
    .await;

    match result {
        Ok(transition) => Ok(transition),
        Err(err) => {
            sdk.rollback_identity_contract_nonce(
                owner_id,
                contract_id,
                new_identity_contract_nonce,
            )
            .await;
            Err(err)
        }
    }
}

/// Build, sign, and structurally validate a document **create** transition
/// without broadcasting it.
///
/// This is a fail-fast wrapper around
/// [`build_signed_document_create_or_replace_transition`] that enforces the
/// create-path revision boundary before any nonce allocation: the document
/// revision must be unset or equal to [`INITIAL_REVISION`]. Any other value
/// (including `Some(0)` and revisions greater than `INITIAL_REVISION`) is
/// rejected here, mirroring the wasm-sdk's `prepareDocumentCreate` guard so
/// native callers get the same precise behavior.
///
/// See [`build_signed_document_create_or_replace_transition`] for the local
/// nonce-rollback semantics on build/sign/validation failures.
#[allow(clippy::too_many_arguments)]
pub async fn build_signed_document_create_transition<S: Signer<IdentityPublicKey>>(
    sdk: &Sdk,
    document: &Document,
    document_type: &DocumentType,
    document_state_transition_entropy: Option<[u8; 32]>,
    identity_public_key: &IdentityPublicKey,
    token_payment_info: Option<TokenPaymentInfo>,
    signer: &S,
    settings: Option<PutSettings>,
) -> Result<StateTransition, Error> {
    ensure_revision_for_create(document.revision())?;
    build_signed_document_create_or_replace_transition(
        sdk,
        document,
        document_type,
        document_state_transition_entropy,
        identity_public_key,
        token_payment_info,
        signer,
        settings,
    )
    .await
}

/// Build, sign, and structurally validate a document **replace** transition
/// without broadcasting it.
///
/// This is a fail-fast wrapper around
/// [`build_signed_document_create_or_replace_transition`] that enforces the
/// replace-path revision boundary before any nonce allocation: the document
/// revision must be greater than [`INITIAL_REVISION`]. `None`, `Some(0)`,
/// and `Some(INITIAL_REVISION)` are rejected here, mirroring the wasm-sdk's
/// `prepareDocumentReplace` guard so native callers get the same precise
/// behavior.
///
/// See [`build_signed_document_create_or_replace_transition`] for the local
/// nonce-rollback semantics on build/sign/validation failures.
#[allow(clippy::too_many_arguments)]
pub async fn build_signed_document_replace_transition<S: Signer<IdentityPublicKey>>(
    sdk: &Sdk,
    document: &Document,
    document_type: &DocumentType,
    identity_public_key: &IdentityPublicKey,
    token_payment_info: Option<TokenPaymentInfo>,
    signer: &S,
    settings: Option<PutSettings>,
) -> Result<StateTransition, Error> {
    ensure_revision_for_replace(document.revision())?;
    build_signed_document_create_or_replace_transition(
        sdk,
        document,
        document_type,
        None, // entropy is unused on the replace path
        identity_public_key,
        token_payment_info,
        signer,
        settings,
    )
    .await
}

/// Inner build/sign/validation step shared by the create-or-replace dispatch.
/// Runs after the identity-contract nonce has been allocated; the caller is
/// responsible for rolling that nonce back if this returns an error.
#[allow(clippy::too_many_arguments)]
async fn build_and_sign_create_or_replace_after_nonce<S: Signer<IdentityPublicKey>>(
    sdk: &Sdk,
    document: &Document,
    document_type: &DocumentType,
    document_state_transition_entropy: Option<[u8; 32]>,
    identity_public_key: &IdentityPublicKey,
    token_payment_info: Option<TokenPaymentInfo>,
    signer: &S,
    settings: Option<PutSettings>,
    new_identity_contract_nonce: u64,
) -> Result<StateTransition, Error> {
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
    fn ensure_revision_nonzero_rejects_only_zero() {
        assert!(ensure_revision_nonzero(None).is_ok());
        assert!(ensure_revision_nonzero(Some(INITIAL_REVISION)).is_ok());
        assert!(ensure_revision_nonzero(Some(INITIAL_REVISION + 1)).is_ok());
        assert!(ensure_revision_nonzero(Some(u64::MAX)).is_ok());

        let err = ensure_revision_nonzero(Some(0)).expect_err("revision 0 must error");
        let msg = err.to_string();
        assert!(msg.contains("InvalidArgument"), "msg: {msg}");
        assert!(msg.contains("revision 0"), "msg: {msg}");
    }

    #[test]
    fn ensure_revision_for_create_accepts_none_and_initial_revision() {
        assert!(ensure_revision_for_create(None).is_ok());
        assert!(ensure_revision_for_create(Some(INITIAL_REVISION)).is_ok());
    }

    #[test]
    fn ensure_revision_for_create_rejects_zero_and_above_initial() {
        let zero = ensure_revision_for_create(Some(0)).expect_err("revision 0 must error");
        assert!(zero.to_string().contains("InvalidArgument"));
        assert!(zero.to_string().contains("create requires revision"));

        let above = ensure_revision_for_create(Some(INITIAL_REVISION + 1))
            .expect_err("revision > INITIAL_REVISION must error on create path");
        assert!(above.to_string().contains("InvalidArgument"));
        assert!(above.to_string().contains("replace path"));
    }

    #[test]
    fn ensure_revision_for_replace_accepts_only_above_initial_revision() {
        assert!(ensure_revision_for_replace(Some(INITIAL_REVISION + 1)).is_ok());
        assert!(ensure_revision_for_replace(Some(INITIAL_REVISION + 100)).is_ok());
    }

    #[test]
    fn ensure_revision_for_replace_rejects_missing_zero_and_initial_revision() {
        let missing = ensure_revision_for_replace(None).expect_err("missing revision must error");
        assert!(missing.to_string().contains("InvalidArgument"));
        assert!(missing.to_string().contains("must have a revision set"));

        let zero =
            ensure_revision_for_replace(Some(0)).expect_err("revision 0 must error on replace");
        assert!(zero.to_string().contains("InvalidArgument"));
        assert!(zero.to_string().contains("replace requires revision"));

        let initial = ensure_revision_for_replace(Some(INITIAL_REVISION))
            .expect_err("INITIAL_REVISION must error on replace path");
        assert!(initial.to_string().contains("InvalidArgument"));
        assert!(initial.to_string().contains("replace requires revision"));
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
