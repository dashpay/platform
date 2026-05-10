//! Document put / create / replace state-transition builders.
//!
//! # Compatibility note (2026-05)
//!
//! Two intentionally different create-path entry points coexist:
//!
//! - The [`PutDocument::put_to_platform`] trait method is the **legacy
//!   native** entry point. It accepts
//!   `document_state_transition_entropy = None` on the create path and will
//!   auto-generate 32-byte entropy + rewrite `document.id` via
//!   [`Document::generate_document_id_v0`] before signing. In-tree callers
//!   such as `rs-platform-wallet` (DashPay profile creation) rely on this
//!   fallback.
//! - The strict [`build_signed_document_create_transition`] /
//!   [`build_signed_document_replace_transition`] helpers, used by the
//!   wasm-sdk `prepareDocumentCreate` / `prepareDocumentReplace` flows, do
//!   **not** auto-generate entropy. Callers must supply entropy whose
//!   derived `Document::generate_document_id_v0(...)` matches `document.id`;
//!   a mismatch is rejected before any identity-contract nonce is
//!   allocated.
//!
//! New prepare/sign-without-broadcast call sites should prefer the strict
//! builders so the supplied document id and entropy commit to the same
//! value.

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
    revision.is_some_and(|rev| rev > INITIAL_REVISION)
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

/// Strict create-path id check: documents handed to
/// [`build_signed_document_create_transition`] must already have their `id`
/// derived from the supplied entropy via [`Document::generate_document_id_v0`].
///
/// This guards against silently signing a transition whose committed
/// document id does not match the entropy bound into the create transition.
/// Callers that want id auto-generation should use the legacy
/// [`PutDocument::put_to_platform`] trait method, which still accepts
/// `entropy = None` and rewrites the document id before signing.
fn ensure_document_id_matches_entropy(
    document: &Document,
    document_type: &DocumentType,
    entropy: &[u8; 32],
) -> Result<(), Error> {
    let expected = Document::generate_document_id_v0(
        &document_type.data_contract_id(),
        &document.owner_id(),
        document_type.name(),
        entropy.as_slice(),
    );
    if document.id() != expected {
        return Err(Error::Generic(format!(
            "InvalidArgument: document.id does not match \
             generate_document_id_v0(contract_id, owner_id, document_type_name, entropy); \
             expected {expected}, got {got}. \
             Either set document.id to the derived value before calling \
             build_signed_document_create_transition, or use the legacy \
             PutDocument::put_to_platform trait method which auto-generates \
             entropy and rewrites the document id when entropy is None.",
            got = document.id()
        )));
    }
    Ok(())
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

/// Internal dispatch: build, sign, and structurally validate a document
/// create-or-replace [`StateTransition`] without broadcasting it.
///
/// This is intentionally **not** part of the public API. It is the
/// pre-broadcast core shared by [`build_signed_document_create_transition`],
/// [`build_signed_document_replace_transition`], and the legacy
/// [`PutDocument::put_to_platform`] trait method. It allocates a fresh
/// identity-contract nonce, picks the create-vs-replace branch based on the
/// document's revision, falls back to RNG-derived entropy + id auto-rewrite on
/// the create branch when `document_state_transition_entropy` is `None`,
/// applies `user_fee_increase` / `token_payment_info` /
/// `state_transition_creation_options` from `settings`, signs the transition,
/// and runs structure validation.
///
/// # Why this is not public
///
/// The auto-fallback create branch (entropy `None` → RNG entropy + rewritten
/// document id) is convenient for the legacy `PutDocument` trait but is a
/// footgun for prepare/sign-without-broadcast flows like the wasm-sdk's
/// `prepareDocumentCreate`, where the caller's already-derived document id
/// must commit to the entropy they pass in. Public callers must go through
/// [`build_signed_document_create_transition`] (which enforces the strict
/// id-matches-entropy check) or [`build_signed_document_replace_transition`].
///
/// # Revision validation
///
/// The dispatch is driven by the document revision and rejects only the
/// always-invalid `Some(0)` case. Strict per-intent validation lives in the
/// public create / replace helpers, which run before any nonce allocation.
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
async fn build_signed_document_create_or_replace_transition<S: Signer<IdentityPublicKey>>(
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
/// This is a fail-fast wrapper that enforces the create-path revision
/// boundary **and** the document-id-matches-entropy invariant before any
/// nonce allocation. The document revision must be unset or equal to
/// [`INITIAL_REVISION`]. Any other value (including `Some(0)` and revisions
/// greater than `INITIAL_REVISION`) is rejected here, mirroring the wasm-sdk's
/// `prepareDocumentCreate` guard so native callers get the same precise
/// behavior.
///
/// `document_state_transition_entropy` is required on the create path and
/// must match the entropy used to derive `document.id` via
/// [`Document::generate_document_id_v0`]. A mismatch is rejected here,
/// before the identity-contract nonce is allocated.
///
/// Callers that want id auto-generation (legacy native behavior) should use
/// the [`PutDocument::put_to_platform`] trait method, which accepts
/// `entropy = None` and rewrites the document id before signing.
///
/// On any pre-broadcast failure inside the dispatch (build, sign, or local
/// structure validation) the bumped identity-contract nonce is rolled back
/// so the local cache does not advance past a nonce the network never
/// observed.
#[allow(clippy::too_many_arguments)]
pub async fn build_signed_document_create_transition<S: Signer<IdentityPublicKey>>(
    sdk: &Sdk,
    document: &Document,
    document_type: &DocumentType,
    document_state_transition_entropy: [u8; 32],
    identity_public_key: &IdentityPublicKey,
    token_payment_info: Option<TokenPaymentInfo>,
    signer: &S,
    settings: Option<PutSettings>,
) -> Result<StateTransition, Error> {
    ensure_revision_for_create(document.revision())?;
    // Verify the caller's document id matches the entropy *before* we
    // allocate any identity-contract nonce, so a stale/wrong id never
    // bumps the local nonce cache.
    ensure_document_id_matches_entropy(
        document,
        document_type,
        &document_state_transition_entropy,
    )?;
    build_signed_document_create_or_replace_transition(
        sdk,
        document,
        document_type,
        Some(document_state_transition_entropy),
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
/// This is a fail-fast wrapper that enforces the replace-path revision
/// boundary before any nonce allocation: the document revision must be
/// greater than [`INITIAL_REVISION`]. `None`, `Some(0)`, and
/// `Some(INITIAL_REVISION)` are rejected here, mirroring the wasm-sdk's
/// `prepareDocumentReplace` guard so native callers get the same precise
/// behavior.
///
/// On any pre-broadcast failure inside the dispatch (build, sign, or local
/// structure validation) the bumped identity-contract nonce is rolled back
/// so the local cache does not advance past a nonce the network never
/// observed.
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
    /// Legacy native put-document entry point.
    ///
    /// **Backwards-compatibility note:** unlike the strict
    /// [`build_signed_document_create_transition`] / wasm
    /// `prepareDocumentCreate` builders, this trait method accepts
    /// `document_state_transition_entropy = None` on the create path and
    /// auto-generates 32-byte entropy + rewrites `document.id` via
    /// [`Document::generate_document_id_v0`] before signing. This preserves
    /// the original `PutDocument` behavior used by in-tree callers such as
    /// `rs-platform-wallet` profile creation. New prepare/sign-without-broadcast
    /// call sites should use the strict create/replace builders so
    /// the supplied document id and entropy commit to the same value.
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
        // Route through the strict create/replace helpers so callers get the
        // same fail-fast revision-vs-intent guarantees as the wasm-sdk
        // `prepareDocumentCreate` / `prepareDocumentReplace` paths. The
        // dispatch is driven by the document revision: unset or
        // `INITIAL_REVISION` selects create; revisions strictly greater than
        // `INITIAL_REVISION` select replace; `Some(0)` is rejected by the
        // strict replace helper before any nonce allocation.
        let transition = if self.revision().is_none() || self.revision() == Some(INITIAL_REVISION) {
            // Create path. Preserve legacy behavior: when the caller did not
            // supply entropy, generate it and rewrite `document.id` so the
            // pair stays consistent before we hand the (document, entropy)
            // to the strict create helper. The strict helper still verifies
            // that `document.id == generate_document_id_v0(entropy)` before
            // allocating any nonce, so the legacy fallback cannot mask an
            // id/entropy mismatch.
            let (resolved_document, resolved_entropy) = resolve_document_create_entropy(
                self,
                &document_type,
                document_state_transition_entropy,
            );
            build_signed_document_create_transition(
                sdk,
                &resolved_document,
                &document_type,
                resolved_entropy,
                &identity_public_key,
                token_payment_info,
                signer,
                settings,
            )
            .await?
        } else {
            // Replace path: entropy is unused; the strict helper enforces
            // `revision > INITIAL_REVISION`.
            build_signed_document_replace_transition(
                sdk,
                self,
                &document_type,
                &identity_public_key,
                token_payment_info,
                signer,
                settings,
            )
            .await?
        };

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
        assert!(!is_document_replace_revision(Some(0)));
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

    /// Failing-signer used by the rollback test below to deterministically
    /// fail signing **after** nonce allocation. Mirrors the nonce-cache test
    /// pattern in `internal_cache::mod` (`rollback_decrements_when_cache_matches_allocated_nonce`).
    #[derive(Debug)]
    struct AlwaysFailingSigner;

    #[async_trait::async_trait]
    impl dpp::identity::signer::Signer<IdentityPublicKey> for AlwaysFailingSigner {
        async fn sign(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<dpp::platform_value::BinaryData, dpp::ProtocolError> {
            Err(dpp::ProtocolError::Generic(
                "deliberate signing failure for rollback test".to_string(),
            ))
        }

        async fn sign_create_witness(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<dpp::address_funds::AddressWitness, dpp::ProtocolError> {
            unreachable!("not used by document create transition signing")
        }

        fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
            true
        }
    }

    /// Pre-broadcast signing failure inside the strict create helper must
    /// roll the identity-contract nonce back so the cache does not advance
    /// past a nonce the network never observed. Asserting via "next
    /// allocation reuses the rolled-back value" mirrors the rollback test
    /// pattern in `internal_cache::mod` so future readers can map the two.
    #[tokio::test]
    async fn build_signed_document_create_rolls_back_nonce_on_signing_failure() {
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::identity::identity_public_key::{KeyType, Purpose, SecurityLevel};
        use dpp::platform_value::BinaryData;
        use drive_proof_verifier::types::IdentityContractNonceFetcher;

        let document_type = test_document_type();
        let contract_id = document_type.data_contract_id();
        let entropy = [0u8; 32];
        // Derive the document id from the entropy so the strict create
        // helper's id-matches-entropy guard passes and the failure happens
        // *after* nonce allocation, where rollback is what we're testing.
        let owner_id = Identifier::from([7; 32]);
        let derived_id = Document::generate_document_id_v0(
            &contract_id,
            &owner_id,
            document_type.name(),
            entropy.as_slice(),
        );
        let document = test_document(None, derived_id);
        assert_eq!(document.owner_id(), owner_id);

        // Build a key whose purpose / security level / enabled flag pass the
        // BatchTransition pre-sign verification, so the failure happens
        // inside `signer.sign` (i.e. *after* nonce allocation), not earlier.
        let identity_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![0u8; 33]),
            disabled_at: None,
        });

        let mut sdk = crate::Sdk::new_mock();
        sdk.mock()
            .expect_fetch::<IdentityContractNonceFetcher, _>(
                (owner_id, contract_id),
                Some(IdentityContractNonceFetcher(10u64)),
            )
            .await
            .expect("set IdentityContractNonceFetcher mock expectation");

        let signer = AlwaysFailingSigner;

        let err = build_signed_document_create_transition(
            &sdk,
            &document,
            &document_type,
            entropy,
            &identity_key,
            None,
            &signer,
            None,
        )
        .await
        .expect_err("signer failure must surface so the helper can roll back the allocated nonce");

        assert!(
            err.to_string().contains("deliberate signing failure"),
            "expected the signer's failure to surface, got: {err}"
        );

        // Cache was bumped from platform=10 to 11 during the failed attempt
        // and then rolled back to 10. Re-allocating with bump_first=true
        // must yield 11 again — proving the rolled-back nonce is reusable.
        let next = sdk
            .get_identity_contract_nonce(owner_id, contract_id, true, None)
            .await
            .expect("nonce allocation must succeed after rollback");
        assert_eq!(
            next, 11,
            "rolled-back nonce should be reused by the next allocation"
        );
    }

    /// The strict create helper must reject a document whose id does not
    /// match the supplied entropy *before* it allocates an
    /// identity-contract nonce. The post-condition we assert is:
    /// the very next nonce allocation (with `bump_first=true`) returns the
    /// expected first-bump value (1 over the platform-fetched 10), which
    /// proves the failed call did not bump the cache.
    #[tokio::test]
    async fn build_signed_document_create_rejects_id_entropy_mismatch_before_nonce_alloc() {
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::identity::identity_public_key::{KeyType, Purpose, SecurityLevel};
        use dpp::platform_value::BinaryData;
        use drive_proof_verifier::types::IdentityContractNonceFetcher;

        let document_type = test_document_type();
        let contract_id = document_type.data_contract_id();
        let entropy = [0u8; 32];
        // Intentionally use a document id that does NOT match
        // generate_document_id_v0(.., entropy = [0; 32]).
        let bogus_id = Identifier::from([0xAB; 32]);
        let document = test_document(None, bogus_id);
        let owner_id = document.owner_id();

        // Sanity-check that the bogus id really does not match the
        // expected derived id, otherwise this test would silently pass for
        // the wrong reason.
        let expected_id = Document::generate_document_id_v0(
            &contract_id,
            &owner_id,
            document_type.name(),
            entropy.as_slice(),
        );
        assert_ne!(
            bogus_id, expected_id,
            "test fixture must use an id that does not match the entropy"
        );

        let identity_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![0u8; 33]),
            disabled_at: None,
        });

        let mut sdk = crate::Sdk::new_mock();
        // Pre-load the platform-side nonce so a later `bump_first=true`
        // allocation has a deterministic value to return. If the strict
        // helper had (incorrectly) allocated a nonce before failing, the
        // first post-failure allocation would jump to 12 instead of 11.
        sdk.mock()
            .expect_fetch::<IdentityContractNonceFetcher, _>(
                (owner_id, contract_id),
                Some(IdentityContractNonceFetcher(10u64)),
            )
            .await
            .expect("set IdentityContractNonceFetcher mock expectation");

        let signer = AlwaysFailingSigner;

        let err = build_signed_document_create_transition(
            &sdk,
            &document,
            &document_type,
            entropy,
            &identity_key,
            None,
            &signer,
            None,
        )
        .await
        .expect_err("id-mismatch must error before nonce allocation");

        let msg = err.to_string();
        assert!(msg.contains("InvalidArgument"), "msg: {msg}");
        assert!(
            msg.contains("does not match"),
            "expected id-mismatch error, got: {msg}"
        );

        // No nonce allocation happened during the failed call, so the
        // first allocation now should be the platform value + 1 = 11.
        let next = sdk
            .get_identity_contract_nonce(owner_id, contract_id, true, None)
            .await
            .expect("nonce allocation must succeed after rejected attempt");
        assert_eq!(
            next, 11,
            "id-mismatch must reject before nonce allocation; next allocation should be 11"
        );
    }
}
