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
//!
//! # Behavior changes in this release (semver-significant)
//!
//! The legacy public [`PutDocument::put_to_platform`] trait method now
//! performs two additional **local** validations on the create path, both
//! of which run **before any identity-contract nonce is allocated** so a
//! caller mistake cannot advance the local nonce cache past a nonce the
//! network never observed:
//!
//! 1. **`Some(0)` revisions are rejected.** Revision `0` was never valid
//!    on either the create or replace path (create requires unset /
//!    [`INITIAL_REVISION`]; replace requires strictly greater than
//!    [`INITIAL_REVISION`]), but the previous implementation silently
//!    fell through to the create path. It now surfaces as
//!    [`Error::InvalidArgument`].
//! 2. **`Some(entropy)` is checked against `document.id`.** When the
//!    caller supplies entropy on the create path the trait now locally
//!    rejects (via the strict [`build_signed_document_create_transition`]
//!    helper that backs it) if the supplied entropy does not derive
//!    `document.id` via [`Document::generate_document_id_v0`].
//!
//! `document_state_transition_entropy = None` still preserves the legacy
//! auto-generate-entropy / rewrite-id behavior for in-tree callers
//! (e.g. `rs-platform-wallet` profile creation) that opt into it.
//!
//! [`build_signed_document_create_or_replace_transition`] remains public
//! for source compatibility with downstream native callers that depended
//! on it before the strict helpers were introduced. New callers should
//! prefer the strict create/replace helpers above.

use super::broadcast::BroadcastStateTransition;
use super::validation::ensure_valid_state_transition_structure;
use super::waitable::Waitable;
use crate::platform::transition::put_settings::PutSettings;
use crate::{Error, Sdk};
use dpp::dashcore::secp256k1::rand::rngs::StdRng;
use dpp::dashcore::secp256k1::rand::{Rng, SeedableRng};
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::{DocumentType, DocumentTypeRef};
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters, INITIAL_REVISION};
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::Identifier;
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
        return Err(Error::InvalidArgument(
            "document revision 0 is invalid; \
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
///
/// Exposed publicly so out-of-tree callers (notably the wasm-sdk
/// `prepareDocumentCreate` / `documentCreate` revision guards) can
/// delegate acceptance to this single source of truth instead of
/// re-implementing the matching rules. Wasm callers still own their own
/// error messaging (API-name guidance, dedicated revision-0 wording) and
/// only consult this function for the accept/reject decision.
pub fn ensure_revision_for_create(revision: Option<u64>) -> Result<(), Error> {
    match revision {
        None => Ok(()),
        Some(rev) if rev == INITIAL_REVISION => Ok(()),
        Some(rev) => Err(Error::InvalidArgument(format!(
            "document revision is {rev} but create requires revision \
             to be unset or {INITIAL_REVISION}; use the replace path for revisions > {INITIAL_REVISION}"
        ))),
    }
}

/// Strict revision guard for the document **replace** path.
///
/// Accepts only `Some(rev)` with `rev > INITIAL_REVISION`. Rejects `None`,
/// `Some(0)`, and `Some(INITIAL_REVISION)`. This is the rs-sdk-side fail-fast
/// equivalent of the wasm-sdk `ensureDocumentReplaceRevision` guard.
///
/// Exposed publicly so out-of-tree callers (notably the wasm-sdk
/// `prepareDocumentReplace` / `documentReplace` revision guards) can
/// delegate acceptance to this single source of truth instead of
/// re-implementing the matching rules. Wasm callers still own their own
/// error messaging (API-name guidance, dedicated revision-0 wording) and
/// only consult this function for the accept/reject decision.
pub fn ensure_revision_for_replace(revision: Option<u64>) -> Result<(), Error> {
    match revision {
        Some(rev) if rev > INITIAL_REVISION => Ok(()),
        Some(rev) => Err(Error::InvalidArgument(format!(
            "document revision is {rev} but replace requires revision > \
             {INITIAL_REVISION}; use the create path for new documents"
        ))),
        None => Err(Error::InvalidArgument(
            "document must have a revision set for replace; \
             use the create path for new documents"
                .to_string(),
        )),
    }
}

/// Platform-version-dispatched document-id derivation.
///
/// Both the strict create-path id check and the legacy create-path entropy
/// fallback go through this helper so the document-id formula has exactly
/// **one** canonical dispatch site in this module.
///
/// The active version is read from
/// `platform_version.dpp.document_versions.document_method_versions.derive_document_id`.
/// `0` selects [`Document::generate_document_id_v0`]; an unknown version
/// surfaces as [`dpp::ProtocolError::UnknownVersionMismatch`] so a new
/// derivation introduced in a future platform version is rejected fast at
/// every call site instead of silently using the v0 formula.
fn derive_document_id(
    document_type: DocumentTypeRef<'_>,
    owner_id: &Identifier,
    entropy: &[u8; 32],
    platform_version: &dpp::version::PlatformVersion,
) -> Result<Identifier, dpp::ProtocolError> {
    derive_document_id_from_parts(
        &document_type.data_contract_id(),
        owner_id,
        document_type.name(),
        entropy,
        platform_version,
    )
}

/// Platform-version-dispatched document-id derivation from raw parts.
///
/// Identical to [`derive_document_id`] but accepts the bare
/// `(contract_id, owner_id, document_type_name)` tuple instead of a
/// [`DocumentTypeRef`]. Exposed publicly so out-of-tree callers
/// (e.g. the `wasm-sdk` fast id-vs-entropy check on
/// `prepareDocumentCreate`) can dispatch through the same single
/// `DocumentMethodVersions::derive_document_id` match without
/// duplicating the version table or carrying a `DocumentType` value.
///
/// `0` selects [`Document::generate_document_id_v0`]; an unknown
/// version surfaces as [`dpp::ProtocolError::UnknownVersionMismatch`]
/// so a new derivation introduced in a future platform version is
/// rejected fast at every call site instead of silently using the v0
/// formula.
pub fn derive_document_id_from_parts(
    contract_id: &Identifier,
    owner_id: &Identifier,
    document_type_name: &str,
    entropy: &[u8; 32],
    platform_version: &dpp::version::PlatformVersion,
) -> Result<Identifier, dpp::ProtocolError> {
    match platform_version
        .dpp
        .document_versions
        .document_method_versions
        .derive_document_id
    {
        0 => Ok(Document::generate_document_id_v0(
            contract_id,
            owner_id,
            document_type_name,
            entropy.as_slice(),
        )),
        version => Err(dpp::ProtocolError::UnknownVersionMismatch {
            method: "derive_document_id".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}

/// Strict create-path id check: documents handed to
/// [`build_signed_document_create_transition`] must already have their `id`
/// derived from the supplied entropy via [`derive_document_id`].
///
/// This guards against silently signing a transition whose committed
/// document id does not match the entropy bound into the create transition.
/// Callers that want id auto-generation should use the legacy
/// [`PutDocument::put_to_platform`] trait method, which still accepts
/// `entropy = None` and rewrites the document id before signing.
pub(crate) fn ensure_document_id_matches_entropy(
    document: &Document,
    document_type: DocumentTypeRef<'_>,
    entropy: &[u8; 32],
    platform_version: &dpp::version::PlatformVersion,
) -> Result<(), Error> {
    let expected = derive_document_id(
        document_type,
        &document.owner_id(),
        entropy,
        platform_version,
    )
    .map_err(Error::Protocol)?;
    if document.id() != expected {
        return Err(Error::InvalidArgument(format!(
            "document.id does not match the platform-version-dispatched \
             document-id derivation \
             (contract_id, owner_id, document_type_name, entropy); \
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
    platform_version: &dpp::version::PlatformVersion,
) -> Result<(Document, [u8; 32]), Error> {
    match document_state_transition_entropy {
        Some(entropy) => Ok((document.clone(), entropy)),
        None => {
            let mut rng = StdRng::from_entropy();
            let mut doc = document.clone();
            let entropy = rng.gen::<[u8; 32]>();
            // Use the centralized dispatched derivation so the legacy
            // auto-generate fallback always agrees with the strict
            // id-matches-entropy check in `ensure_document_id_matches_entropy`.
            let id = derive_document_id(
                document_type.as_ref(),
                &doc.owner_id(),
                &entropy,
                platform_version,
            )
            .map_err(Error::Protocol)?;
            doc.set_id(id);
            Ok((doc, entropy))
        }
    }
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

/// Legacy create-or-replace dispatch: build, sign, and structurally validate
/// a document [`StateTransition`] without broadcasting it.
///
/// **Legacy / source-compatible only.** This helper is retained as a
/// public, source-compatible entry point for native callers that already
/// depend on it. It dispatches between create and replace based on the
/// document's revision and supports the legacy
/// `document_state_transition_entropy = None` fallback (RNG-derived
/// entropy + id auto-rewrite) on the create branch.
///
/// **New callers should prefer the strict helpers**
/// [`build_signed_document_create_transition`] /
/// [`build_signed_document_replace_transition`] for fail-fast intent and
/// document-id-matches-entropy checks — this dispatch helper only rejects
/// the always-invalid `Some(0)` revision and does not enforce the strict
/// id-matches-entropy invariant by itself. The strict helpers run their
/// validation **before** any nonce allocation.
///
/// # Behavior
///
/// Allocates a fresh identity-contract nonce, picks the create-vs-replace
/// branch based on the document's revision, falls back to RNG-derived
/// entropy + id auto-rewrite on the create branch when
/// `document_state_transition_entropy` is `None`, applies
/// `user_fee_increase` / `token_payment_info` /
/// `state_transition_creation_options` from `settings`, signs the
/// transition, and runs structure validation.
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
#[deprecated(
    note = "use build_signed_document_create_transition or build_signed_document_replace_transition for strict intent validation"
)]
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
    build_signed_document_create_or_replace_transition_legacy(
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

/// Private implementation backing the deprecated public legacy dispatcher.
///
/// Internal strict helpers route through this private entry point so
/// in-tree call sites do not trigger the public deprecation warning.
#[allow(clippy::too_many_arguments)]
async fn build_signed_document_create_or_replace_transition_legacy<S: Signer<IdentityPublicKey>>(
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
    // Clone-once owned dispatch: the strict create path never re-resolves
    // entropy (caller already supplied it), so route past the legacy
    // create-or-replace dispatcher to avoid an extra Document clone in the
    // `Some(entropy)` branch of `resolve_document_create_entropy`.
    build_signed_document_create_transition_owned(
        sdk,
        document.clone(),
        document_type,
        document_state_transition_entropy,
        identity_public_key,
        token_payment_info,
        signer,
        settings,
    )
    .await
}

/// Internal owned-document variant of
/// [`build_signed_document_create_transition`].
///
/// Validates revision and id-matches-entropy **before** any nonce
/// allocation, allocates an identity-contract nonce, and dispatches with
/// the document moved by value so `BatchTransition::new_document_creation_transition_from_document`
/// gets ownership without a second clone.
///
/// This is the single-clone entry point used by the legacy
/// [`PutDocument::put_to_platform`] None-entropy fallback (which resolves
/// entropy + rewrites the document id once, then hands the owned document
/// to this helper).
#[allow(clippy::too_many_arguments)]
async fn build_signed_document_create_transition_owned<S: Signer<IdentityPublicKey>>(
    sdk: &Sdk,
    document: Document,
    document_type: &DocumentType,
    entropy: [u8; 32],
    identity_public_key: &IdentityPublicKey,
    token_payment_info: Option<TokenPaymentInfo>,
    signer: &S,
    settings: Option<PutSettings>,
) -> Result<StateTransition, Error> {
    ensure_revision_for_create(document.revision())?;
    // Verify the caller's document id matches the entropy *before* we
    // allocate any identity-contract nonce, so a stale/wrong id never
    // bumps the local nonce cache.
    ensure_document_id_matches_entropy(&document, document_type.as_ref(), &entropy, sdk.version())?;

    let owner_id = document.owner_id();
    let contract_id = document_type.data_contract_id();
    let new_identity_contract_nonce = sdk
        .get_identity_contract_nonce(owner_id, contract_id, true, settings)
        .await?;

    let result = build_and_sign_create_after_nonce(
        sdk,
        document,
        document_type,
        entropy,
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

/// Inner build/sign/validation step for the strict create path.
///
/// Runs after the identity-contract nonce has been allocated; the caller is
/// responsible for rolling that nonce back if this returns an error. Moves
/// `document` into `BatchTransition::new_document_creation_transition_from_document`
/// so the strict create path performs a single Document clone end-to-end.
#[allow(clippy::too_many_arguments)]
async fn build_and_sign_create_after_nonce<S: Signer<IdentityPublicKey>>(
    sdk: &Sdk,
    document: Document,
    document_type: &DocumentType,
    entropy: [u8; 32],
    identity_public_key: &IdentityPublicKey,
    token_payment_info: Option<TokenPaymentInfo>,
    signer: &S,
    settings: Option<PutSettings>,
    new_identity_contract_nonce: u64,
) -> Result<StateTransition, Error> {
    let put_settings = settings.unwrap_or_default();
    let transition = BatchTransition::new_document_creation_transition_from_document(
        document,
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
    .await?;
    ensure_valid_state_transition_structure(&transition, sdk.version())?;
    Ok(transition)
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
    // Validate revision *before* allocating a nonce so caller mistakes
    // never bump the local nonce cache.
    ensure_revision_for_replace(document.revision())?;

    let owner_id = document.owner_id();
    let contract_id = document_type.data_contract_id();
    let new_identity_contract_nonce = sdk
        .get_identity_contract_nonce(owner_id, contract_id, true, settings)
        .await?;

    let result = build_and_sign_replace_after_nonce(
        sdk,
        document,
        document_type,
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

/// Inner build/sign/validation step for the strict replace path.
///
/// Runs after the identity-contract nonce has been allocated; the caller is
/// responsible for rolling that nonce back if this returns an error. The
/// replace path goes straight to
/// [`BatchTransition::new_document_replacement_transition_from_document`]
/// — entropy is intentionally not threaded through here because replacement
/// transitions do not derive a new document id.
#[allow(clippy::too_many_arguments)]
async fn build_and_sign_replace_after_nonce<S: Signer<IdentityPublicKey>>(
    sdk: &Sdk,
    document: &Document,
    document_type: &DocumentType,
    identity_public_key: &IdentityPublicKey,
    token_payment_info: Option<TokenPaymentInfo>,
    signer: &S,
    settings: Option<PutSettings>,
    new_identity_contract_nonce: u64,
) -> Result<StateTransition, Error> {
    let put_settings = settings.unwrap_or_default();
    let transition = BatchTransition::new_document_replacement_transition_from_document(
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
    .await?;
    ensure_valid_state_transition_structure(&transition, sdk.version())?;
    Ok(transition)
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
            sdk.version(),
        )?;
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
    /// `rs-platform-wallet` profile creation.
    ///
    /// When `document_state_transition_entropy = Some(entropy)` on the
    /// create path the call now locally rejects (before any nonce
    /// allocation) if the entropy does not derive `document.id` via
    /// [`Document::generate_document_id_v0`] — the strict create helper
    /// that backs this routing enforces the id-matches-entropy invariant.
    /// `None` still auto-generates entropy and rewrites the document id
    /// for legacy callers that opt into that behavior.
    ///
    /// New prepare/sign-without-broadcast call sites should use the strict
    /// create/replace builders so the supplied document id and entropy
    /// commit to the same value.
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
        // `INITIAL_REVISION` select replace.
        //
        // Reject `Some(0)` up front with the dispatch-aware
        // `ensure_revision_nonzero` message rather than letting it fall into
        // the replace branch — the replace-helper message says "use the
        // create path", which would be misleading for `put_to_platform`
        // callers (they aren't picking a branch themselves).
        ensure_revision_nonzero(self.revision())?;
        let transition = if self.revision().is_none() || self.revision() == Some(INITIAL_REVISION) {
            // Create path. Avoid the outer pre-resolve clone when the
            // caller already supplied entropy: pass `self` straight to the
            // strict create helper, which clones once internally for
            // `BatchTransition::new_document_creation_transition_from_document`.
            //
            // For the legacy `None` entropy fallback we resolve once here
            // (generate entropy + rewrite document id) and hand the owned
            // document to `build_signed_document_create_transition_owned`,
            // so the create path performs a single Document clone end-to-end.
            // The strict id-matches-entropy check runs before any nonce
            // allocation in both branches.
            match document_state_transition_entropy {
                Some(entropy) => {
                    build_signed_document_create_transition(
                        sdk,
                        self,
                        &document_type,
                        entropy,
                        &identity_public_key,
                        token_payment_info,
                        signer,
                        settings,
                    )
                    .await?
                }
                None => {
                    let (resolved_document, resolved_entropy) =
                        resolve_document_create_entropy(self, &document_type, None, sdk.version())?;
                    build_signed_document_create_transition_owned(
                        sdk,
                        resolved_document,
                        &document_type,
                        resolved_entropy,
                        &identity_public_key,
                        token_payment_info,
                        signer,
                        settings,
                    )
                    .await?
                }
            }
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
        assert!(matches!(err, Error::InvalidArgument(_)), "err: {err:?}");
        let msg = err.to_string();
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
        assert!(matches!(zero, Error::InvalidArgument(_)), "err: {zero:?}");
        assert!(zero.to_string().contains("create requires revision"));

        let above = ensure_revision_for_create(Some(INITIAL_REVISION + 1))
            .expect_err("revision > INITIAL_REVISION must error on create path");
        assert!(matches!(above, Error::InvalidArgument(_)), "err: {above:?}");
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
        assert!(
            matches!(missing, Error::InvalidArgument(_)),
            "err: {missing:?}"
        );
        assert!(missing.to_string().contains("must have a revision set"));

        let zero =
            ensure_revision_for_replace(Some(0)).expect_err("revision 0 must error on replace");
        assert!(matches!(zero, Error::InvalidArgument(_)), "err: {zero:?}");
        assert!(zero.to_string().contains("replace requires revision"));

        let initial = ensure_revision_for_replace(Some(INITIAL_REVISION))
            .expect_err("INITIAL_REVISION must error on replace path");
        assert!(
            matches!(initial, Error::InvalidArgument(_)),
            "err: {initial:?}"
        );
        assert!(initial.to_string().contains("replace requires revision"));
    }

    /// `derive_document_id_from_parts` must produce the same id bytes
    /// as the underlying `Document::generate_document_id_v0` for any
    /// `(contract_id, owner_id, document_type_name, entropy)` tuple on
    /// the v0 arm — otherwise the strict id-matches-entropy guard, the
    /// legacy auto-generate fallback, and the wasm-sdk fast pre-check
    /// could disagree silently.
    #[test]
    fn derive_document_id_from_parts_matches_generate_document_id_v0() {
        let document_type = test_document_type();
        let owner_id = Identifier::from([0x42; 32]);
        let entropy = [0xCCu8; 32];

        let derived = derive_document_id_from_parts(
            &document_type.data_contract_id(),
            &owner_id,
            document_type.name(),
            &entropy,
            PlatformVersion::latest(),
        )
        .expect("v0 arm must succeed on latest platform version");
        let direct = Document::generate_document_id_v0(
            &document_type.data_contract_id(),
            &owner_id,
            document_type.name(),
            entropy.as_slice(),
        );

        assert_eq!(derived, direct);
    }

    /// `derive_document_id` must dispatch on
    /// `platform_version.dpp.document_versions.document_method_versions.derive_document_id`,
    /// matching the v0 formula on the v0 arm and surfacing
    /// `UnknownVersionMismatch` for any other version constant. The
    /// dispatch is checked by mutating the platform-version field
    /// directly so the test exercises the match arm without depending on
    /// a future platform version landing.
    #[test]
    fn derive_document_id_dispatches_on_platform_version() {
        let document_type = test_document_type();
        let owner_id = Identifier::from([0x55; 32]);
        let entropy = [0xAAu8; 32];

        let v0_id = Document::generate_document_id_v0(
            &document_type.data_contract_id(),
            &owner_id,
            document_type.name(),
            entropy.as_slice(),
        );

        let latest = PlatformVersion::latest();
        let derived = derive_document_id(document_type.as_ref(), &owner_id, &entropy, latest)
            .expect("v0 arm must succeed on latest platform version");
        assert_eq!(derived, v0_id);

        // Synthesize an unknown future version of the derivation to prove
        // the dispatcher rejects it instead of silently using the v0
        // formula.
        let mut bumped = latest.clone();
        bumped
            .dpp
            .document_versions
            .document_method_versions
            .derive_document_id = 99;
        let err = derive_document_id(document_type.as_ref(), &owner_id, &entropy, &bumped)
            .expect_err("unknown derive_document_id version must error");
        match err {
            dpp::ProtocolError::UnknownVersionMismatch {
                method,
                known_versions,
                received,
            } => {
                assert_eq!(method, "derive_document_id");
                assert_eq!(known_versions, vec![0]);
                assert_eq!(received, 99);
            }
            other => panic!("expected UnknownVersionMismatch, got {other:?}"),
        }
    }

    #[test]
    fn creation_entropy_fallback_regenerates_document_id() {
        let document_type = test_document_type();
        let original_id = Identifier::from([3; 32]);
        let document = test_document(None, original_id);

        let (resolved_document, entropy) = resolve_document_create_entropy(
            &document,
            &document_type,
            None,
            PlatformVersion::latest(),
        )
        .expect("resolve_document_create_entropy must accept latest platform version");

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

        let (resolved_document, resolved_entropy) = resolve_document_create_entropy(
            &document,
            &document_type,
            Some(provided_entropy),
            PlatformVersion::latest(),
        )
        .expect("resolve_document_create_entropy must accept latest platform version");

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

    /// Pre-broadcast signing failure inside the strict replace helper must
    /// roll the identity-contract nonce back so the cache does not advance
    /// past a nonce the network never observed. Mirrors the create-side
    /// rollback test above; asserting via "next allocation reuses the
    /// rolled-back value" matches the rollback pattern in
    /// `internal_cache::mod`.
    #[tokio::test]
    async fn build_signed_document_replace_rolls_back_nonce_on_signing_failure() {
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::identity::identity_public_key::{KeyType, Purpose, SecurityLevel};
        use dpp::platform_value::BinaryData;
        use drive_proof_verifier::types::IdentityContractNonceFetcher;

        let document_type = test_document_type();
        let contract_id = document_type.data_contract_id();
        // Replace requires revision > INITIAL_REVISION; the document id is
        // not entropy-derived for the replace path, so any id works.
        let owner_id = Identifier::from([7; 32]);
        let document = test_document(Some(INITIAL_REVISION + 1), Identifier::from([3; 32]));
        assert_eq!(document.owner_id(), owner_id);

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

        let err = build_signed_document_replace_transition(
            &sdk,
            &document,
            &document_type,
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

        assert!(matches!(err, Error::InvalidArgument(_)), "err: {err:?}");
        let msg = err.to_string();
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
