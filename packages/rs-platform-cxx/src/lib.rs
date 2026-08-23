// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! Transport-free Dash Platform client internals exposed to C++ embedders.
//!
//! - `verify`: Drive/GroveDB proof verification over DAPI wire bytes;
//! - `decode`: DPP decoders for identities and DPNS/DashPay documents;
//! - `st`: state-transition construction with callback-based signing.
//!
//! The `#[cxx::bridge]` below exposes thin adapters over those modules.
//! Signing crosses the FFI as a digest callback (`WalletSigner`,
//! `dash/platform/signer.h`) so private keys never leave the embedder.

pub mod decode;
pub mod provider;
pub mod st;
pub mod types;
pub mod verify;

use types::{BuiltTransition, KeyInfo};

#[allow(clippy::too_many_arguments)]
#[cxx::bridge(namespace = "platform_ffi")]
mod ffi {
    /// A byte vector, wrapped because cxx shared structs cannot hold
    /// Vec<Vec<u8>>.
    #[derive(Clone)]
    struct FfiBytes {
        data: Vec<u8>,
    }

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

    /// Verified document query result (serialized documents; empty means
    /// proven no matches).
    struct FfiVerifiedDocs {
        documents: Vec<FfiBytes>,
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
        // --- Node-local verification context ----------------------------
        /// Sets the network ("main"/"test"/"regtest"/"devnet") and the
        /// Platform activation core height (0 = unknown; only consulted by
        /// query paths that require it).
        fn set_context(network_id: &str, platform_activation_height: u32) -> Result<()>;
        /// Replaces the stored quorum keys of `quorum_type` with `keys`.
        fn update_quorum_keys(quorum_type: u8, keys: Vec<FfiQuorumKey>) -> Result<()>;

        // --- FromProof verification over (request, response) bytes ------
        fn verify_get_identity_nonce(request: &[u8], response: &[u8]) -> Result<FfiVerifiedU64>;
        fn verify_get_identity_contract_nonce(
            request: &[u8],
            response: &[u8],
        ) -> Result<FfiVerifiedU64>;
        fn verify_get_identity(request: &[u8], response: &[u8]) -> Result<FfiVerifiedIdentity>;
        fn verify_get_identity_by_pubkey_hash(
            request: &[u8],
            response: &[u8],
        ) -> Result<FfiVerifiedIdentity>;
        fn verify_get_documents(request: &[u8], response: &[u8]) -> Result<FfiVerifiedDocs>;
        fn verify_get_contested_vote_state(
            request: &[u8],
            response: &[u8],
        ) -> Result<FfiVerifiedContested>;

        // --- DPP decoders -----------------------------------------------
        fn decode_identity(bytes: &[u8]) -> Result<FfiIdentity>;
        fn decode_identity_public_key(bytes: &[u8]) -> Result<FfiIdentityKey>;
        fn decode_dpns_domain(doc_bytes: &[u8]) -> Result<FfiDpnsName>;
        fn decode_dashpay_profile(doc_bytes: &[u8]) -> Result<FfiProfile>;
        fn decode_contact_request(doc_bytes: &[u8]) -> Result<FfiContactRequest>;

        // --- State transitions ------------------------------------------
        fn st_build_dpns_preorder(
            identity_id: &[u8],
            identity_contract_nonce: u64,
            label: &str,
            preorder_salt: &[u8],
            signature_public_key_id: u32,
            key: FfiIdentityKey,
            signer: &WalletSigner,
        ) -> Result<FfiBuiltTransition>;
        fn st_build_dpns_domain(
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

fn key_info(key: &ffi::FfiIdentityKey) -> KeyInfo {
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
        identity: identity
            .as_ref()
            .map(ffi_identity)
            .unwrap_or_else(|| ffi::FfiIdentity {
                id: Vec::new(),
                balance: 0,
                revision: 0,
                keys: Vec::new(),
            }),
        meta: ffi_meta(meta),
    }
}

// ---------------------------------------------------------------------------
// Bridge implementations: verification context.
// ---------------------------------------------------------------------------

fn set_context(network_id: &str, platform_activation_height: u32) -> Result<(), String> {
    provider::set_context(network_id, platform_activation_height)
}

fn update_quorum_keys(quorum_type: u8, keys: Vec<ffi::FfiQuorumKey>) -> Result<(), String> {
    let keys = keys
        .into_iter()
        .map(|key| {
            Ok(provider::QuorumKey {
                quorum_hash: types::id32(&key.quorum_hash, "quorum hash")?,
                public_key: key.pubkey.as_slice().try_into().map_err(|_| {
                    format!(
                        "quorum public key must be 48 bytes, got {}",
                        key.pubkey.len()
                    )
                })?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    provider::update_quorum_keys(quorum_type, keys);
    Ok(())
}

// ---------------------------------------------------------------------------
// Bridge implementations: FromProof verification.
// ---------------------------------------------------------------------------

fn verify_get_identity_nonce(
    request: &[u8],
    response: &[u8],
) -> Result<ffi::FfiVerifiedU64, String> {
    verify::verify_get_identity_nonce(request, response).map(ffi_verified_u64)
}

fn verify_get_identity_contract_nonce(
    request: &[u8],
    response: &[u8],
) -> Result<ffi::FfiVerifiedU64, String> {
    verify::verify_get_identity_contract_nonce(request, response).map(ffi_verified_u64)
}

fn verify_get_identity(
    request: &[u8],
    response: &[u8],
) -> Result<ffi::FfiVerifiedIdentity, String> {
    verify::verify_get_identity(request, response).map(ffi_verified_identity)
}

fn verify_get_identity_by_pubkey_hash(
    request: &[u8],
    response: &[u8],
) -> Result<ffi::FfiVerifiedIdentity, String> {
    verify::verify_get_identity_by_pubkey_hash(request, response).map(ffi_verified_identity)
}

fn verify_get_documents(request: &[u8], response: &[u8]) -> Result<ffi::FfiVerifiedDocs, String> {
    let (documents, meta) = verify::verify_get_documents(request, response)?;
    Ok(ffi::FfiVerifiedDocs {
        documents: documents
            .into_iter()
            .map(|data| ffi::FfiBytes { data })
            .collect(),
        meta: ffi_meta(meta),
    })
}

fn verify_get_contested_vote_state(
    request: &[u8],
    response: &[u8],
) -> Result<ffi::FfiVerifiedContested, String> {
    let (state, meta) = verify::verify_get_contested_vote_state(request, response)?;
    Ok(ffi::FfiVerifiedContested {
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
    })
}

fn decode_identity(bytes: &[u8]) -> Result<ffi::FfiIdentity, String> {
    decode::decode_identity(bytes).map(|identity| ffi_identity(&identity))
}

fn decode_identity_public_key(bytes: &[u8]) -> Result<ffi::FfiIdentityKey, String> {
    decode::decode_identity_public_key(bytes).map(|key| ffi_key(&key))
}

fn decode_dpns_domain(doc_bytes: &[u8]) -> Result<ffi::FfiDpnsName, String> {
    let name = decode::decode_dpns_domain(doc_bytes)?;
    Ok(ffi::FfiDpnsName {
        label: name.label,
        normalized_label: name.normalized_label,
        parent_domain: name.parent_domain,
        identity: name.identity.to_vec(),
        document_id: name.document_id.to_vec(),
        owner_id: name.owner_id.to_vec(),
    })
}

fn decode_dashpay_profile(doc_bytes: &[u8]) -> Result<ffi::FfiProfile, String> {
    let profile = decode::decode_dashpay_profile(doc_bytes)?;
    Ok(ffi::FfiProfile {
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
    })
}

fn decode_contact_request(doc_bytes: &[u8]) -> Result<ffi::FfiContactRequest, String> {
    let request = decode::decode_contact_request(doc_bytes)?;
    Ok(ffi::FfiContactRequest {
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
    })
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

fn check_key_id(signature_public_key_id: u32, key: &ffi::FfiIdentityKey) -> Result<(), String> {
    if signature_public_key_id != key.id {
        return Err(format!(
            "signature public key id {signature_public_key_id} does not match key id {}",
            key.id
        ));
    }
    Ok(())
}

fn st_build_dpns_preorder(
    identity_id: &[u8],
    identity_contract_nonce: u64,
    label: &str,
    preorder_salt: &[u8],
    signature_public_key_id: u32,
    key: ffi::FfiIdentityKey,
    signer: &ffi::WalletSigner,
) -> Result<ffi::FfiBuiltTransition, String> {
    check_key_id(signature_public_key_id, &key)?;
    let handle = SignerHandle(signer);
    let sign_fn = |key_id: u32, digest: [u8; 32]| handle.sign(key_id, digest);
    st::build_dpns_preorder(
        identity_id,
        identity_contract_nonce,
        label,
        preorder_salt,
        &key_info(&key),
        &sign_fn,
    )
    .map(ffi_built)
}

#[allow(clippy::too_many_arguments)]
fn st_build_dpns_domain(
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
    check_key_id(signature_public_key_id, &key)?;
    let handle = SignerHandle(signer);
    let sign_fn = |key_id: u32, digest: [u8; 32]| handle.sign(key_id, digest);
    st::build_dpns_domain(
        identity_id,
        identity_contract_nonce,
        label,
        normalized_label,
        parent_domain,
        preorder_salt,
        &key_info(&key),
        &sign_fn,
    )
    .map(ffi_built)
}

#[allow(clippy::too_many_arguments)]
fn st_build_profile(
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
    check_key_id(signature_public_key_id, &key)?;
    let handle = SignerHandle(signer);
    let sign_fn = |key_id: u32, digest: [u8; 32]| handle.sign(key_id, digest);
    st::build_profile(
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
        &key_info(&key),
        &sign_fn,
    )
    .map(ffi_built)
}

#[allow(clippy::too_many_arguments)]
fn st_build_contact_request(
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
    check_key_id(signature_public_key_id, &key)?;
    let handle = SignerHandle(signer);
    let sign_fn = |key_id: u32, digest: [u8; 32]| handle.sign(key_id, digest);
    st::build_contact_request(
        identity_id,
        identity_contract_nonce,
        to_user_id,
        encrypted_public_key,
        sender_key_index,
        recipient_key_index,
        account_reference,
        encrypted_account_label,
        entropy,
        &key_info(&key),
        &sign_fn,
    )
    .map(ffi_built)
}

#[allow(clippy::too_many_arguments)]
fn st_build_identity_create(
    is_instant: bool,
    transaction: &[u8],
    instant_lock: &[u8],
    output_index: u32,
    core_chain_locked_height: u32,
    out_point: &[u8],
    keys: Vec<ffi::FfiNewIdentityKey>,
    signer: &ffi::WalletSigner,
) -> Result<ffi::FfiBuiltTransition, String> {
    let proof = if is_instant {
        st::AssetLockProofInput::Instant {
            transaction: transaction.to_vec(),
            instant_lock: instant_lock.to_vec(),
            output_index,
        }
    } else {
        st::AssetLockProofInput::Chain {
            core_chain_locked_height,
            out_point: out_point
                .try_into()
                .map_err(|_| format!("outpoint must be 36 bytes, got {}", out_point.len()))?,
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
    st::build_identity_create(proof, &keys, &sign_fn).map(ffi_built)
}
