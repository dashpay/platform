// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! Dash Platform for C++ embedders, as a thin `cxx` bridge over `dash-sdk`.
//!
//! - `client`: the SDK instance, its runtime and the freshness policy;
//! - `queries`: the proved queries, through the SDK's `Fetch`/`FetchMany`;
//! - `decode`: flattening of identities and DPNS / DashPay documents;
//! - `st`: state-transition construction with callback-based signing.
//!
//! The embedder supplies what only it knows: the evonode endpoints from its
//! masternode list, the Platform quorum keys from its LLMQ store, its best
//! ChainLock height, and signatures from its wallet. Signing crosses the FFI
//! as a digest callback (`WalletSigner`, `dash/platform/signer.h`) so private
//! keys never leave the embedder.
//!
//! Every fallible `extern "Rust"` body runs under
//! [`std::panic::catch_unwind`]: cxx turns a `Result::Err` into a C++
//! `rust::Error`, but a panic that reaches its shim is a deterministic abort
//! of the embedding process. All responses are untrusted DAPI bytes, so a
//! panic anywhere in the SDK is reported as an error instead. The three
//! infallible entry points (`new_platform_client`, `set_core_chain_locked_height`,
//! `shutdown`) touch no untrusted input and swallow a panic themselves. (This
//! only helps under `panic = "unwind"`; the crate must not be built with
//! `panic = "abort"`.)

pub mod client;
pub mod decode;
pub mod provider;
pub mod queries;
pub mod st;
pub mod types;

use client::Client;
use types::{BuiltTransition, KeyInfo};

#[allow(clippy::too_many_arguments)]
#[cxx::bridge(namespace = "platform_ffi")]
mod ffi {
    /// One identity public key. `has_disabled_at == false` means the key is
    /// not disabled.
    #[derive(Clone)]
    struct FfiIdentityKey {
        id: u32,
        purpose: u8,
        security_level: u8,
        key_type: u8,
        read_only: bool,
        data: Vec<u8>,
        has_disabled_at: bool,
        disabled_at: u64,
    }

    /// One contender of a contested resource: identity id and its vote
    /// tally (when requested/available).
    struct FfiContender {
        identity: Vec<u8>,
        has_votes: bool,
        votes: u32,
    }

    /// Decoded identity.
    struct FfiIdentity {
        id: Vec<u8>,
        balance: u64,
        revision: u64,
        keys: Vec<FfiIdentityKey>,
    }

    /// Decoded DPNS domain document.
    #[derive(Clone)]
    struct FfiDpnsName {
        label: String,
        normalized_label: String,
        parent_domain: String,
        identity: Vec<u8>,
        document_id: Vec<u8>,
        owner_id: Vec<u8>,
    }

    /// Decoded DashPay profile document. Empty vectors/strings and zero
    /// timestamps mean the field is absent.
    struct FfiProfile {
        document_id: Vec<u8>,
        owner_id: Vec<u8>,
        display_name: String,
        public_message: String,
        avatar_url: String,
        avatar_hash: Vec<u8>,
        avatar_fingerprint: Vec<u8>,
        created_at: u64,
        updated_at: u64,
        revision: u64,
    }

    /// Decoded DashPay contactRequest document.
    #[derive(Clone)]
    struct FfiContactRequest {
        owner_id: Vec<u8>,
        to_user_id: Vec<u8>,
        encrypted_public_key: Vec<u8>,
        sender_key_index: u32,
        recipient_key_index: u32,
        account_reference: u32,
        encrypted_account_label: Vec<u8>,
        core_height_created_at: u32,
        created_at: u64,
        document_id: Vec<u8>,
    }

    /// A quorum BLS public key pushed from the node's LLMQ store.
    /// `quorum_hash` (32 bytes) is in the byte order DAPI proofs carry it
    /// (display order); `pubkey` is the 48-byte basic-scheme public key.
    #[derive(Clone)]
    struct FfiQuorumKey {
        quorum_hash: Vec<u8>,
        pubkey: Vec<u8>,
    }

    /// Authenticated ResponseMetadata fields of a verified response (the
    /// quorum signature covers them via the StateId sign bytes).
    #[derive(Clone)]
    struct FfiMeta {
        height: u64,
        core_chain_locked_height: u32,
        time_ms: u64,
        protocol_version: u32,
        chain_id: String,
    }

    /// Verified optional u64 (nonce); `present == false` means proven
    /// absent.
    struct FfiVerifiedU64 {
        present: bool,
        value: u64,
        meta: FfiMeta,
    }

    /// Verified optional identity; `present == false` means proven absent.
    struct FfiVerifiedIdentity {
        present: bool,
        identity: FfiIdentity,
        meta: FfiMeta,
    }

    /// Verified optional DPNS name; `present == false` means proven absent.
    struct FfiVerifiedDpnsName {
        present: bool,
        name: FfiDpnsName,
        meta: FfiMeta,
    }

    /// Verified DPNS name list (empty means proven no matches).
    struct FfiVerifiedDpnsNames {
        names: Vec<FfiDpnsName>,
        meta: FfiMeta,
    }

    /// Verified optional DashPay profile; `present == false` means proven
    /// absent.
    struct FfiVerifiedProfile {
        present: bool,
        profile: FfiProfile,
        meta: FfiMeta,
    }

    /// Verified contact request list (empty means proven no matches).
    struct FfiVerifiedContactRequests {
        requests: Vec<FfiContactRequest>,
        meta: FfiMeta,
    }

    /// Verified contested-resource vote state.
    struct FfiVerifiedContested {
        /// False when the contest was cryptographically proven absent.
        contest_found: bool,
        contenders: Vec<FfiContender>,
        has_abstain: bool,
        abstain_votes: u32,
        has_lock: bool,
        lock_votes: u32,
        /// True once the poll finished (awarded or locked).
        finished: bool,
        locked: bool,
        has_winner: bool,
        winner: Vec<u8>,
        finished_at_time_ms: u64,
        meta: FfiMeta,
    }

    /// Outcome of broadcasting a state transition. `accepted == false`
    /// carries the node's rejection; that channel is informational (not
    /// proof-backed), success is confirmed by a proved re-query.
    struct FfiBroadcastResult {
        accepted: bool,
        error: String,
        error_code: u32,
    }

    /// A public key to register with a new identity.
    struct FfiNewIdentityKey {
        id: u32,
        purpose: u8,
        security_level: u8,
        /// Compressed secp256k1 public key (33 bytes).
        pubkey: Vec<u8>,
    }

    /// A built, signed state transition. `hash` is sha256(bytes), the wait
    /// handle for waitForStateTransitionResult.
    struct FfiBuiltTransition {
        bytes: Vec<u8>,
        hash: Vec<u8>,
    }

    unsafe extern "C++" {
        include!("dash/platform/signer.h");

        /// Wallet-backed signer. `SignDigestForKey` signs a 32-byte digest
        /// with the wallet key identified by `key_id` (`u32::MAX` selects
        /// the one-time asset-lock key of an identity registration) and
        /// returns a 65-byte compact recoverable ECDSA signature, or false
        /// on failure.
        type WalletSigner;
        fn SignDigestForKey(
            self: &WalletSigner,
            key_id: u32,
            digest: &[u8],
            sig_out: &mut Vec<u8>,
        ) -> bool;
    }

    extern "Rust" {
        /// One Platform SDK instance: its runtime, endpoint set, trust
        /// context and freshness state. Thread-safe; every method may be
        /// called from any thread and blocks until its request completes or
        /// `shutdown` interrupts it.
        type PlatformClient;

        fn new_platform_client() -> Box<PlatformClient>;

        // --- Node-local context ------------------------------------------
        /// Sets the network ("main"/"test"/"regtest"/"devnet"), the LLMQ
        /// type its Platform quorums use (proofs signed by any other type are
        /// refused), its Tenderdash chain id (a verified response signed for
        /// another chain is refused), the lowest protocol version the network
        /// can be running (the SDK ratchets up from it) and the Platform
        /// activation core height (0 = unknown). Clears any previously
        /// pushed quorum keys and endpoint set.
        fn set_context(
            self: &PlatformClient,
            network_id: &str,
            platform_quorum_type: u32,
            tenderdash_chain_id: &str,
            protocol_version: u32,
            platform_activation_height: u32,
        ) -> Result<()>;
        /// Replaces the evonode endpoint set (`https://host:port` URIs from
        /// the deterministic masternode list). Rebuilds the SDK when the set
        /// changed.
        fn set_endpoints(self: &PlatformClient, endpoints: Vec<String>) -> Result<()>;
        /// Replaces the stored Platform quorum keys with `keys`.
        fn update_quorum_keys(self: &PlatformClient, keys: Vec<FfiQuorumKey>) -> Result<()>;
        /// Updates the node's best ChainLock height, the anchor of the
        /// staleness floor applied to every verified response.
        fn set_core_chain_locked_height(self: &PlatformClient, height: u32);
        /// Interrupts in-flight requests and releases the runtime.
        fn shutdown(self: &PlatformClient);

        // --- Proved queries -----------------------------------------------
        fn get_identity(self: &PlatformClient, id: &[u8]) -> Result<FfiVerifiedIdentity>;
        fn get_identity_by_pubkey_hash(
            self: &PlatformClient,
            pubkey_hash: &[u8],
        ) -> Result<FfiVerifiedIdentity>;
        fn get_identity_nonce(self: &PlatformClient, id: &[u8]) -> Result<FfiVerifiedU64>;
        fn get_identity_contract_nonce(
            self: &PlatformClient,
            id: &[u8],
            contract_id: &[u8],
        ) -> Result<FfiVerifiedU64>;
        fn resolve_name(
            self: &PlatformClient,
            normalized_label: &str,
        ) -> Result<FfiVerifiedDpnsName>;
        fn search_names(
            self: &PlatformClient,
            prefix: &str,
            limit: u32,
        ) -> Result<FfiVerifiedDpnsNames>;
        fn names_of_identity(
            self: &PlatformClient,
            identity: &[u8],
        ) -> Result<FfiVerifiedDpnsNames>;
        fn get_profile(self: &PlatformClient, owner_id: &[u8]) -> Result<FfiVerifiedProfile>;
        fn get_contact_requests(
            self: &PlatformClient,
            identity: &[u8],
            to_me: bool,
        ) -> Result<FfiVerifiedContactRequests>;
        fn get_contested_vote_state(
            self: &PlatformClient,
            normalized_label: &str,
        ) -> Result<FfiVerifiedContested>;

        // --- Broadcast ---------------------------------------------------
        fn broadcast_state_transition(
            self: &PlatformClient,
            state_transition: &[u8],
        ) -> Result<FfiBroadcastResult>;

        // --- DPP decoders (stored bytes, under the network's version) ----
        fn decode_identity(self: &PlatformClient, bytes: &[u8]) -> Result<FfiIdentity>;
        fn decode_identity_public_key(
            self: &PlatformClient,
            bytes: &[u8],
        ) -> Result<FfiIdentityKey>;
        fn decode_dpns_domain(self: &PlatformClient, doc_bytes: &[u8]) -> Result<FfiDpnsName>;
        fn decode_dashpay_profile(self: &PlatformClient, doc_bytes: &[u8]) -> Result<FfiProfile>;
        fn decode_contact_request(
            self: &PlatformClient,
            doc_bytes: &[u8],
        ) -> Result<FfiContactRequest>;

        // --- State transitions ------------------------------------------
        fn st_build_dpns_preorder(
            self: &PlatformClient,
            identity_id: &[u8],
            identity_contract_nonce: u64,
            label: &str,
            preorder_salt: &[u8],
            signature_public_key_id: u32,
            key: FfiIdentityKey,
            signer: &WalletSigner,
        ) -> Result<FfiBuiltTransition>;
        fn st_build_dpns_domain(
            self: &PlatformClient,
            identity_id: &[u8],
            identity_contract_nonce: u64,
            label: &str,
            normalized_label: &str,
            parent_domain: &str,
            preorder_salt: &[u8],
            signature_public_key_id: u32,
            key: FfiIdentityKey,
            signer: &WalletSigner,
        ) -> Result<FfiBuiltTransition>;
        fn st_build_profile(
            self: &PlatformClient,
            identity_id: &[u8],
            identity_contract_nonce: u64,
            display_name: &str,
            public_message: &str,
            avatar_url: &str,
            avatar_hash: &[u8],
            avatar_fingerprint: &[u8],
            revision: u64,
            has_existing_doc_id: bool,
            existing_document_id: &[u8],
            entropy: &[u8],
            signature_public_key_id: u32,
            key: FfiIdentityKey,
            signer: &WalletSigner,
        ) -> Result<FfiBuiltTransition>;
        fn st_build_contact_request(
            self: &PlatformClient,
            identity_id: &[u8],
            identity_contract_nonce: u64,
            to_user_id: &[u8],
            encrypted_public_key: &[u8],
            sender_key_index: u32,
            recipient_key_index: u32,
            account_reference: u32,
            encrypted_account_label: &[u8],
            entropy: &[u8],
            signature_public_key_id: u32,
            key: FfiIdentityKey,
            signer: &WalletSigner,
        ) -> Result<FfiBuiltTransition>;
        fn st_build_identity_create(
            self: &PlatformClient,
            is_instant: bool,
            transaction: &[u8],
            instant_lock: &[u8],
            output_index: u32,
            core_chain_locked_height: u32,
            out_point: &[u8],
            keys: Vec<FfiNewIdentityKey>,
            signer: &WalletSigner,
        ) -> Result<FfiBuiltTransition>;
    }
}

/// The bridge's opaque client type: [`Client`] behind a `Box` the embedder
/// owns.
pub struct PlatformClient(Client);

impl PlatformClient {
    /// The wrapped client, for Rust callers (tests) that configure it
    /// directly.
    pub fn inner(&self) -> &Client {
        &self.0
    }
}

fn new_platform_client() -> Box<PlatformClient> {
    Box::new(PlatformClient(Client::new()))
}

/// Runs a bridge body, turning a panic into an `Err` the C++ side receives as
/// a `rust::Error` instead of the process abort cxx would otherwise perform.
fn guarded<T>(what: &str, body: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)).unwrap_or_else(|payload| {
        let message = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or("<non-string panic payload>");
        Err(format!("{what} panicked: {message}"))
    })
}

// ---------------------------------------------------------------------------
// Conversions between the plain-Rust core types and the flat FFI structs.
// ---------------------------------------------------------------------------

fn ffi_key(key: &KeyInfo) -> ffi::FfiIdentityKey {
    ffi::FfiIdentityKey {
        id: key.id,
        purpose: key.purpose,
        security_level: key.security_level,
        key_type: key.key_type,
        read_only: key.read_only,
        data: key.data.clone(),
        has_disabled_at: key.disabled_at.is_some(),
        disabled_at: key.disabled_at.unwrap_or(0),
    }
}

fn key_info_from_ffi(key: &ffi::FfiIdentityKey) -> KeyInfo {
    KeyInfo {
        id: key.id,
        purpose: key.purpose,
        security_level: key.security_level,
        key_type: key.key_type,
        read_only: key.read_only,
        data: key.data.clone(),
        disabled_at: key.has_disabled_at.then_some(key.disabled_at),
    }
}

fn ffi_built(built: BuiltTransition) -> ffi::FfiBuiltTransition {
    ffi::FfiBuiltTransition {
        bytes: built.bytes,
        hash: built.hash.to_vec(),
    }
}

fn ffi_meta(meta: types::Meta) -> ffi::FfiMeta {
    ffi::FfiMeta {
        height: meta.height,
        core_chain_locked_height: meta.core_chain_locked_height,
        time_ms: meta.time_ms,
        protocol_version: meta.protocol_version,
        chain_id: meta.chain_id,
    }
}

fn ffi_verified_u64((value, meta): (Option<u64>, types::Meta)) -> ffi::FfiVerifiedU64 {
    ffi::FfiVerifiedU64 {
        present: value.is_some(),
        value: value.unwrap_or(0),
        meta: ffi_meta(meta),
    }
}

fn ffi_identity(identity: &types::IdentityInfo) -> ffi::FfiIdentity {
    ffi::FfiIdentity {
        id: identity.id.to_vec(),
        balance: identity.balance,
        revision: identity.revision,
        keys: identity.keys.iter().map(ffi_key).collect(),
    }
}

fn ffi_verified_identity(
    (identity, meta): (Option<types::IdentityInfo>, types::Meta),
) -> ffi::FfiVerifiedIdentity {
    ffi::FfiVerifiedIdentity {
        present: identity.is_some(),
        identity: ffi_identity(&identity.unwrap_or_default()),
        meta: ffi_meta(meta),
    }
}

fn ffi_verified_name(
    (name, meta): (Option<types::DpnsName>, types::Meta),
) -> ffi::FfiVerifiedDpnsName {
    ffi::FfiVerifiedDpnsName {
        present: name.is_some(),
        name: ffi_dpns_name(name.unwrap_or_default()),
        meta: ffi_meta(meta),
    }
}

fn ffi_verified_profile(
    (profile, meta): (Option<types::Profile>, types::Meta),
) -> ffi::FfiVerifiedProfile {
    ffi::FfiVerifiedProfile {
        present: profile.is_some(),
        profile: ffi_profile(profile.unwrap_or_default()),
        meta: ffi_meta(meta),
    }
}

fn ffi_verified_requests(
    (requests, meta): (Vec<types::ContactRequest>, types::Meta),
) -> ffi::FfiVerifiedContactRequests {
    ffi::FfiVerifiedContactRequests {
        requests: requests.into_iter().map(ffi_contact_request).collect(),
        meta: ffi_meta(meta),
    }
}

fn ffi_dpns_name(name: types::DpnsName) -> ffi::FfiDpnsName {
    ffi::FfiDpnsName {
        label: name.label,
        normalized_label: name.normalized_label,
        parent_domain: name.parent_domain,
        identity: name.identity.to_vec(),
        document_id: name.document_id.to_vec(),
        owner_id: name.owner_id.to_vec(),
    }
}

fn ffi_verified_names(
    (names, meta): (Vec<types::DpnsName>, types::Meta),
) -> ffi::FfiVerifiedDpnsNames {
    ffi::FfiVerifiedDpnsNames {
        names: names.into_iter().map(ffi_dpns_name).collect(),
        meta: ffi_meta(meta),
    }
}

fn ffi_profile(profile: types::Profile) -> ffi::FfiProfile {
    ffi::FfiProfile {
        document_id: profile.document_id.to_vec(),
        owner_id: profile.owner_id.to_vec(),
        display_name: profile.display_name,
        public_message: profile.public_message,
        avatar_url: profile.avatar_url,
        avatar_hash: profile.avatar_hash,
        avatar_fingerprint: profile.avatar_fingerprint,
        created_at: profile.created_at,
        updated_at: profile.updated_at,
        revision: profile.revision,
    }
}

fn ffi_contact_request(request: types::ContactRequest) -> ffi::FfiContactRequest {
    ffi::FfiContactRequest {
        owner_id: request.owner_id.to_vec(),
        to_user_id: request.to_user_id.to_vec(),
        encrypted_public_key: request.encrypted_public_key,
        sender_key_index: request.sender_key_index,
        recipient_key_index: request.recipient_key_index,
        account_reference: request.account_reference,
        encrypted_account_label: request.encrypted_account_label,
        core_height_created_at: request.core_height_created_at,
        created_at: request.created_at,
        document_id: request.document_id.to_vec(),
    }
}

fn ffi_contested(
    (state, meta): (types::ContestedVoteState, types::Meta),
) -> ffi::FfiVerifiedContested {
    ffi::FfiVerifiedContested {
        contest_found: state.contest_found,
        contenders: state
            .contenders
            .iter()
            .map(|(identity, votes)| ffi::FfiContender {
                identity: identity.to_vec(),
                has_votes: votes.is_some(),
                votes: votes.unwrap_or(0),
            })
            .collect(),
        has_abstain: state.abstain_votes.is_some(),
        abstain_votes: state.abstain_votes.unwrap_or(0),
        has_lock: state.lock_votes.is_some(),
        lock_votes: state.lock_votes.unwrap_or(0),
        finished: state.finished,
        locked: state.locked,
        has_winner: state.winner.is_some(),
        winner: state.winner.map(|id| id.to_vec()).unwrap_or_default(),
        finished_at_time_ms: state.finished_at_time_ms,
        meta: ffi_meta(meta),
    }
}

/// Shareable handle to the C++ signer. The bridge functions run the async
/// dpp builders to completion on the calling thread with a local executor,
/// so the signer is never actually accessed from another thread; the
/// `Send + Sync` assertion only satisfies dpp's `Signer: Send + Sync`
/// bound.
struct SignerHandle<'a>(&'a ffi::WalletSigner);
unsafe impl Send for SignerHandle<'_> {}
unsafe impl Sync for SignerHandle<'_> {}

impl SignerHandle<'_> {
    fn sign(&self, key_id: u32, digest: [u8; 32]) -> Option<Vec<u8>> {
        let mut signature = Vec::new();
        self.0
            .SignDigestForKey(key_id, &digest, &mut signature)
            .then_some(signature)
    }
}

/// The transition's `signature_public_key_id` is set by dpp from the key
/// itself; a caller passing a different id has a bookkeeping bug worth
/// reporting rather than silently signing with the key it did pass.
fn check_key_id(signature_public_key_id: u32, key: &ffi::FfiIdentityKey) -> Result<(), String> {
    if signature_public_key_id != key.id {
        return Err(format!(
            "signature public key id {signature_public_key_id} does not match key id {}",
            key.id
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Bridge implementations.
// ---------------------------------------------------------------------------

impl PlatformClient {
    /// Shared preamble of the keyed state-transition builders: the key-id
    /// consistency check, the signer handle and the network's platform
    /// version, all under the panic guard.
    fn with_signer(
        &self,
        what: &str,
        signature_public_key_id: u32,
        key: ffi::FfiIdentityKey,
        signer: &ffi::WalletSigner,
        build: impl FnOnce(
            &'static platform_version::version::PlatformVersion,
            &KeyInfo,
            st::SignFn<'_>,
        ) -> Result<BuiltTransition, String>,
    ) -> Result<ffi::FfiBuiltTransition, String> {
        guarded(what, || {
            check_key_id(signature_public_key_id, &key)?;
            let handle = SignerHandle(signer);
            let sign_fn = |key_id: u32, digest: [u8; 32]| handle.sign(key_id, digest);
            build(
                self.0.platform_version()?,
                &key_info_from_ffi(&key),
                &sign_fn,
            )
            .map(ffi_built)
        })
    }
}

// The builders mirror the bridge signatures, whose argument lists are the
// flattened document fields C++ passes.
#[allow(clippy::too_many_arguments)]
impl PlatformClient {
    fn set_context(
        &self,
        network_id: &str,
        platform_quorum_type: u32,
        tenderdash_chain_id: &str,
        protocol_version: u32,
        platform_activation_height: u32,
    ) -> Result<(), String> {
        guarded("set_context", || {
            self.0.set_context(provider::Context {
                network: provider::parse_network(network_id)?,
                platform_quorum_type,
                tenderdash_chain_id: tenderdash_chain_id.to_string(),
                protocol_version,
                platform_activation_height,
            })
        })
    }

    fn set_endpoints(&self, endpoints: Vec<String>) -> Result<(), String> {
        guarded("set_endpoints", || self.0.set_endpoints(endpoints))
    }

    fn update_quorum_keys(&self, keys: Vec<ffi::FfiQuorumKey>) -> Result<(), String> {
        guarded("update_quorum_keys", || {
            let keys = keys
                .into_iter()
                .map(|key| {
                    Ok(provider::QuorumKey {
                        quorum_hash: types::id32(&key.quorum_hash, "quorum hash")?,
                        public_key: types::fixed(&key.pubkey, "quorum public key")?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            self.0.provider().update_quorum_keys(keys)
        })
    }

    fn set_core_chain_locked_height(&self, height: u32) {
        self.0.set_core_chain_locked_height(height);
    }

    fn shutdown(&self) {
        // Third-party drop paths run here (tonic channels, the runtime); a
        // panic on the way out must not abort the embedder.
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.0.shutdown()));
    }

    fn get_identity(&self, id: &[u8]) -> Result<ffi::FfiVerifiedIdentity, String> {
        guarded("get_identity", || {
            queries::get_identity(&self.0, id).map(ffi_verified_identity)
        })
    }

    fn get_identity_by_pubkey_hash(
        &self,
        pubkey_hash: &[u8],
    ) -> Result<ffi::FfiVerifiedIdentity, String> {
        guarded("get_identity_by_pubkey_hash", || {
            queries::get_identity_by_pubkey_hash(&self.0, pubkey_hash).map(ffi_verified_identity)
        })
    }

    fn get_identity_nonce(&self, id: &[u8]) -> Result<ffi::FfiVerifiedU64, String> {
        guarded("get_identity_nonce", || {
            queries::get_identity_nonce(&self.0, id).map(ffi_verified_u64)
        })
    }

    fn get_identity_contract_nonce(
        &self,
        id: &[u8],
        contract_id: &[u8],
    ) -> Result<ffi::FfiVerifiedU64, String> {
        guarded("get_identity_contract_nonce", || {
            queries::get_identity_contract_nonce(&self.0, id, contract_id).map(ffi_verified_u64)
        })
    }

    fn resolve_name(&self, normalized_label: &str) -> Result<ffi::FfiVerifiedDpnsName, String> {
        guarded("resolve_name", || {
            queries::resolve_name(&self.0, normalized_label).map(ffi_verified_name)
        })
    }

    fn search_names(&self, prefix: &str, limit: u32) -> Result<ffi::FfiVerifiedDpnsNames, String> {
        guarded("search_names", || {
            queries::search_names(&self.0, prefix, limit).map(ffi_verified_names)
        })
    }

    fn names_of_identity(&self, identity: &[u8]) -> Result<ffi::FfiVerifiedDpnsNames, String> {
        guarded("names_of_identity", || {
            queries::names_of_identity(&self.0, identity).map(ffi_verified_names)
        })
    }

    fn get_profile(&self, owner_id: &[u8]) -> Result<ffi::FfiVerifiedProfile, String> {
        guarded("get_profile", || {
            queries::get_profile(&self.0, owner_id).map(ffi_verified_profile)
        })
    }

    fn get_contact_requests(
        &self,
        identity: &[u8],
        to_me: bool,
    ) -> Result<ffi::FfiVerifiedContactRequests, String> {
        guarded("get_contact_requests", || {
            queries::get_contact_requests(&self.0, identity, to_me).map(ffi_verified_requests)
        })
    }

    fn get_contested_vote_state(
        &self,
        normalized_label: &str,
    ) -> Result<ffi::FfiVerifiedContested, String> {
        guarded("get_contested_vote_state", || {
            queries::get_contested_vote_state(&self.0, normalized_label).map(ffi_contested)
        })
    }

    fn broadcast_state_transition(
        &self,
        state_transition: &[u8],
    ) -> Result<ffi::FfiBroadcastResult, String> {
        guarded("broadcast_state_transition", || {
            let outcome = queries::broadcast_state_transition(&self.0, state_transition)?;
            Ok(match outcome {
                Ok(()) => ffi::FfiBroadcastResult {
                    accepted: true,
                    error: String::new(),
                    error_code: 0,
                },
                Err(rejection) => ffi::FfiBroadcastResult {
                    accepted: false,
                    error: rejection.message,
                    error_code: rejection.code,
                },
            })
        })
    }

    fn decode_identity(&self, bytes: &[u8]) -> Result<ffi::FfiIdentity, String> {
        guarded("decode_identity", || {
            decode::decode_identity(bytes).map(|identity| ffi_identity(&identity))
        })
    }

    fn decode_identity_public_key(&self, bytes: &[u8]) -> Result<ffi::FfiIdentityKey, String> {
        guarded("decode_identity_public_key", || {
            decode::decode_identity_public_key(bytes).map(|key| ffi_key(&key))
        })
    }

    fn decode_dpns_domain(&self, doc_bytes: &[u8]) -> Result<ffi::FfiDpnsName, String> {
        guarded("decode_dpns_domain", || {
            decode::decode_dpns_domain(doc_bytes, self.0.platform_version()?).map(ffi_dpns_name)
        })
    }

    fn decode_dashpay_profile(&self, doc_bytes: &[u8]) -> Result<ffi::FfiProfile, String> {
        guarded("decode_dashpay_profile", || {
            decode::decode_dashpay_profile(doc_bytes, self.0.platform_version()?).map(ffi_profile)
        })
    }

    fn decode_contact_request(&self, doc_bytes: &[u8]) -> Result<ffi::FfiContactRequest, String> {
        guarded("decode_contact_request", || {
            decode::decode_contact_request(doc_bytes, self.0.platform_version()?)
                .map(ffi_contact_request)
        })
    }

    fn st_build_dpns_preorder(
        &self,
        identity_id: &[u8],
        identity_contract_nonce: u64,
        label: &str,
        preorder_salt: &[u8],
        signature_public_key_id: u32,
        key: ffi::FfiIdentityKey,
        signer: &ffi::WalletSigner,
    ) -> Result<ffi::FfiBuiltTransition, String> {
        self.with_signer(
            "st_build_dpns_preorder",
            signature_public_key_id,
            key,
            signer,
            |version, key, sign| {
                st::build_dpns_preorder(
                    version,
                    identity_id,
                    identity_contract_nonce,
                    label,
                    preorder_salt,
                    key,
                    sign,
                )
            },
        )
    }

    fn st_build_dpns_domain(
        &self,
        identity_id: &[u8],
        identity_contract_nonce: u64,
        label: &str,
        normalized_label: &str,
        parent_domain: &str,
        preorder_salt: &[u8],
        signature_public_key_id: u32,
        key: ffi::FfiIdentityKey,
        signer: &ffi::WalletSigner,
    ) -> Result<ffi::FfiBuiltTransition, String> {
        self.with_signer(
            "st_build_dpns_domain",
            signature_public_key_id,
            key,
            signer,
            |version, key, sign| {
                st::build_dpns_domain(
                    version,
                    identity_id,
                    identity_contract_nonce,
                    label,
                    normalized_label,
                    parent_domain,
                    preorder_salt,
                    key,
                    sign,
                )
            },
        )
    }

    fn st_build_profile(
        &self,
        identity_id: &[u8],
        identity_contract_nonce: u64,
        display_name: &str,
        public_message: &str,
        avatar_url: &str,
        avatar_hash: &[u8],
        avatar_fingerprint: &[u8],
        revision: u64,
        has_existing_doc_id: bool,
        existing_document_id: &[u8],
        entropy: &[u8],
        signature_public_key_id: u32,
        key: ffi::FfiIdentityKey,
        signer: &ffi::WalletSigner,
    ) -> Result<ffi::FfiBuiltTransition, String> {
        self.with_signer(
            "st_build_profile",
            signature_public_key_id,
            key,
            signer,
            |version, key, sign| {
                st::build_profile(
                    version,
                    identity_id,
                    identity_contract_nonce,
                    display_name,
                    public_message,
                    avatar_url,
                    avatar_hash,
                    avatar_fingerprint,
                    revision,
                    has_existing_doc_id.then_some(existing_document_id),
                    entropy,
                    key,
                    sign,
                )
            },
        )
    }

    fn st_build_contact_request(
        &self,
        identity_id: &[u8],
        identity_contract_nonce: u64,
        to_user_id: &[u8],
        encrypted_public_key: &[u8],
        sender_key_index: u32,
        recipient_key_index: u32,
        account_reference: u32,
        encrypted_account_label: &[u8],
        entropy: &[u8],
        signature_public_key_id: u32,
        key: ffi::FfiIdentityKey,
        signer: &ffi::WalletSigner,
    ) -> Result<ffi::FfiBuiltTransition, String> {
        self.with_signer(
            "st_build_contact_request",
            signature_public_key_id,
            key,
            signer,
            |version, key, sign| {
                st::build_contact_request(
                    version,
                    identity_id,
                    identity_contract_nonce,
                    to_user_id,
                    encrypted_public_key,
                    sender_key_index,
                    recipient_key_index,
                    account_reference,
                    encrypted_account_label,
                    entropy,
                    key,
                    sign,
                )
            },
        )
    }

    fn st_build_identity_create(
        &self,
        is_instant: bool,
        transaction: &[u8],
        instant_lock: &[u8],
        output_index: u32,
        core_chain_locked_height: u32,
        out_point: &[u8],
        keys: Vec<ffi::FfiNewIdentityKey>,
        signer: &ffi::WalletSigner,
    ) -> Result<ffi::FfiBuiltTransition, String> {
        guarded("st_build_identity_create", || {
            let proof = if is_instant {
                st::AssetLockProofInput::Instant {
                    transaction: transaction.to_vec(),
                    instant_lock: instant_lock.to_vec(),
                    output_index,
                }
            } else {
                st::AssetLockProofInput::Chain {
                    core_chain_locked_height,
                    out_point: types::fixed(out_point, "outpoint")?,
                }
            };
            let keys: Vec<st::NewIdentityKey> = keys
                .into_iter()
                .map(|key| st::NewIdentityKey {
                    id: key.id,
                    purpose: key.purpose,
                    security_level: key.security_level,
                    pubkey: key.pubkey,
                })
                .collect();
            let handle = SignerHandle(signer);
            let sign_fn = |key_id: u32, digest: [u8; 32]| handle.sign(key_id, digest);
            st::build_identity_create(self.0.platform_version()?, proof, &keys, &sign_fn)
                .map(ffi_built)
        })
    }
}
