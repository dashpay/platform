// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! Dash Platform for C++ embedders, as a thin `cxx` shell over `dash-sdk`.
//!
//! `dash-sdk` owns transport, retries, proof verification, the signed-time
//! window and the protocol-version ratchet. This crate adds what only the
//! embedder can supply or decide: a push-model trust context (`provider`:
//! endpoints, Platform quorum keys, the local ChainLock height), one tokio
//! runtime (`runtime`), the SDK instance and its endpoint set (`client`), the
//! proved reads with the shell's own freshness checks (`ops`), the two signer
//! adapters over the embedder's wallet (`signer`), state-transition assembly
//! over dpp (`builders`) and a few pure helpers (`helpers`).
//!
//! Every bridge entry point runs under [`guarded`]: cxx turns an `Err` into a
//! C++ `rust::Error`, but a panic reaching its shim aborts the embedding
//! process, and every response byte comes from an untrusted node. The crate
//! therefore refuses to build with `panic = "abort"`.

#[cfg(panic = "abort")]
compile_error!(
    "dash-platform-cxx requires panic = \"unwind\"; its FFI guards rely on catch_unwind"
);

pub mod builders;
pub mod client;
pub mod helpers;
pub mod ops;
pub mod provider;
pub mod runtime;
pub mod signer;
mod sync;

use std::panic::{catch_unwind, AssertUnwindSafe};

use client::Client;
use zeroize::Zeroizing;

#[cxx::bridge(namespace = "platform_ffi")]
pub mod ffi {
    /// Outcome class of a bridge call. The value of a `Verified*` result is
    /// only meaningful under `Ok` and `UnsupportedProtocolVersion`; the
    /// fine-grained reason is in `Status::message`, for logs.
    ///
    /// Read results carry no `UnsupportedProtocolVersion` for a proven
    /// absence: it is `ProvenAbsent` whatever the version, the version
    /// itself is in `meta.protocol_version`, and the builders refuse until
    /// the embedder is updated.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u8)]
    enum StatusKind {
        Ok = 0,
        /// The proof shows the queried object (or every match) does not exist.
        ProvenAbsent = 1,
        /// Broadcast: a node already holds this state transition.
        AlreadyExists = 2,
        /// Broadcast: a node rejected the state transition with a consensus
        /// error; `consensus_code` carries its code.
        Consensus = 3,
        /// No endpoints, no local ChainLock anchor, the client is shut down,
        /// a transport failure or timeout, every address is banned, or a
        /// read the network's protocol version cannot answer correctly.
        Unavailable = 4,
        /// A proof, signature, quorum, freshness or shell-watermark check
        /// refused the response, or a node definitively refused the request
        /// (a broadcast without a consensus code, a read it would not serve,
        /// a response above the size bound).
        Rejected = 5,
        /// A verified response signed for another Tenderdash chain.
        ChainIdMismatch = 6,
        /// A verified response from a protocol version this build does not
        /// know; the value is returned, writes must stop until an update.
        UnsupportedProtocolVersion = 7,
        /// A bug: bad input from the embedder, a panic, or an SDK error that
        /// no other kind describes.
        Internal = 8,
    }

    #[derive(Debug, Clone, Default)]
    struct Status {
        kind: StatusKind,
        consensus_code: u32,
        message: String,
    }

    /// Network the client serves. `network`: 0 mainnet, 1 testnet,
    /// 2 devnet, 3 regtest. `platform_llmq_type`: the LLMQ type Platform
    /// quorums use on this network; a proof signed by any other type is
    /// refused. `tenderdash_chain_id`: a verified response carrying another
    /// id is a signed response from another chain.
    #[derive(Debug, Clone)]
    struct Config {
        network: u8,
        tenderdash_chain_id: String,
        platform_llmq_type: u8,
    }

    /// A Platform quorum's BLS public key. `hash` is the quorum hash in
    /// the embedder's internal `uint256` byte order; the shell normalizes.
    #[derive(Debug, Clone)]
    struct QuorumKey {
        hash: [u8; 32],
        pubkey: [u8; 48],
    }

    /// The authenticated `ResponseMetadata` of a verified response; the
    /// quorum signature covers every field through the signed `StateId`.
    #[derive(Debug, Clone, Default)]
    struct Meta {
        height: u64,
        core_chain_locked_height: u32,
        time_ms: u64,
        protocol_version: u32,
        chain_id: String,
    }

    /// Where an identity key may be used.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u8)]
    enum BoundsKind {
        NoBounds = 0,
        SingleContract = 1,
        SingleContractDocumentType = 2,
        /// The members of the contract group `contract_id`.
        ContractGroup = 3,
    }

    #[derive(Debug, Clone, Default)]
    struct ContractBounds {
        kind: BoundsKind,
        contract_id: [u8; 32],
        document_type: String,
    }

    /// One identity public key; `disabled_at == 0` means enabled.
    #[derive(Debug, Clone, Default)]
    struct IdentityKey {
        id: u32,
        purpose: u8,
        security_level: u8,
        key_type: u8,
        read_only: bool,
        data: Vec<u8>,
        disabled_at: u64,
        bounds: ContractBounds,
    }

    #[derive(Debug, Clone, Default)]
    struct Identity {
        id: [u8; 32],
        balance: u64,
        revision: u64,
        keys: Vec<IdentityKey>,
    }

    /// A DPNS `domain` document.
    #[derive(Debug, Clone, Default)]
    struct DpnsName {
        label: String,
        normalized_label: String,
        parent: String,
        identity: [u8; 32],
        document_id: [u8; 32],
        owner: [u8; 32],
    }

    /// A DashPay `profile` document. Empty strings and vectors mean the
    /// field is absent. The avatar and payment-address fields are carried
    /// for `build_profile`, which keeps them on a replace; the embedder
    /// renders none of them.
    #[derive(Debug, Clone, Default)]
    struct Profile {
        document_id: [u8; 32],
        owner: [u8; 32],
        revision: u64,
        display_name: String,
        public_message: String,
        avatar_url: String,
        avatar_hash: Vec<u8>,
        avatar_fingerprint: Vec<u8>,
        core_payment_address: Vec<u8>,
        platform_payment_address: Vec<u8>,
        shielded_address: Vec<u8>,
        created_at: u64,
        updated_at: u64,
    }

    /// A DashPay `contactRequest` document.
    #[derive(Debug, Clone, Default)]
    struct ContactRequest {
        document_id: [u8; 32],
        owner: [u8; 32],
        to_user_id: [u8; 32],
        encrypted_public_key: Vec<u8>,
        sender_key_index: u32,
        recipient_key_index: u32,
        account_reference: u32,
        encrypted_account_label: Vec<u8>,
        auto_accept_proof: Vec<u8>,
        created_at: u64,
        core_height_created_at: u32,
    }

    #[derive(Debug, Clone, Default)]
    struct Contender {
        identity: [u8; 32],
        votes: u32,
        has_votes: bool,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u8)]
    enum WinnerKind {
        /// The contest is still open.
        NoWinner = 0,
        WonByIdentity = 1,
        Locked = 2,
    }

    /// The vote state of a contested DPNS name. `ends_at` is the time of
    /// the block that finalized the contest, 0 while it is open.
    #[derive(Debug, Clone, Default)]
    struct ContestedState {
        contenders: Vec<Contender>,
        abstain: u32,
        lock: u32,
        winner_kind: WinnerKind,
        winner: [u8; 32],
        ends_at: u64,
    }

    /// Cursor of a paged read: pass `next_start_after` to the next call
    /// while `has_more`. One bridge call is one page.
    #[derive(Debug, Clone, Default)]
    struct Page {
        next_start_after: [u8; 32],
        has_more: bool,
    }

    #[derive(Debug, Clone, Default)]
    struct VerifiedIdentity {
        status: Status,
        meta: Meta,
        value: Identity,
    }

    #[derive(Debug, Clone, Default)]
    struct VerifiedU64 {
        status: Status,
        meta: Meta,
        value: u64,
    }

    #[derive(Debug, Clone, Default)]
    struct VerifiedDpnsName {
        status: Status,
        meta: Meta,
        value: DpnsName,
    }

    #[derive(Debug, Clone, Default)]
    struct VerifiedDpnsNames {
        status: Status,
        meta: Meta,
        items: Vec<DpnsName>,
        page: Page,
    }

    #[derive(Debug, Clone, Default)]
    struct VerifiedProfile {
        status: Status,
        meta: Meta,
        value: Profile,
    }

    #[derive(Debug, Clone, Default)]
    struct VerifiedContactRequests {
        status: Status,
        meta: Meta,
        items: Vec<ContactRequest>,
        page: Page,
    }

    #[derive(Debug, Clone, Default)]
    struct VerifiedContested {
        status: Status,
        meta: Meta,
        value: ContestedState,
    }

    /// The node's answer to a broadcast: advisory, never proof-backed.
    /// Success is `Ok` or `AlreadyExists`; the embedder confirms every
    /// write with a proved re-query.
    #[derive(Debug, Clone, Default)]
    struct BroadcastResult {
        status: Status,
    }

    /// A signed state transition. `hash` is its transaction id (single
    /// SHA256 of `bytes`); `object_id` the identity id or document id it
    /// creates or replaces.
    #[derive(Debug, Clone, Default)]
    struct Built {
        bytes: Vec<u8>,
        hash: [u8; 32],
        object_id: [u8; 32],
    }

    /// A key to register with a new identity; always ECDSA secp256k1.
    #[derive(Debug, Clone)]
    struct NewIdentityKey {
        id: u32,
        purpose: u8,
        security_level: u8,
        pubkey: [u8; 33],
        bounds: ContractBounds,
    }

    /// The funding of an identity registration: an InstantSend-locked
    /// asset-lock transaction (`is_instant`, consensus-encoded transaction
    /// and islock, `output_index` of the credit output) or a ChainLocked
    /// outpoint (`core_chain_locked_height`, `out_point` as txid ‖ vout).
    #[derive(Debug, Clone)]
    struct AssetLockProofInput {
        is_instant: bool,
        transaction: Vec<u8>,
        instant_lock: Vec<u8>,
        output_index: u32,
        core_chain_locked_height: u32,
        out_point: [u8; 36],
    }

    /// The fields of a profile the embedder edits; an empty string omits
    /// the field.
    #[derive(Debug, Clone, Default)]
    struct ProfileInput {
        display_name: String,
        public_message: String,
    }

    /// DIP-15 compact extended public key: parent fingerprint ‖ chain code
    /// ‖ compressed public key, the plaintext of `encryptedPublicKey`.
    #[derive(Debug, Clone)]
    struct CompactXpub {
        parent_fingerprint: [u8; 4],
        chain_code: [u8; 32],
        public_key: [u8; 33],
    }

    /// A contact request to mint. `shared_secret` is the ECDH secret the
    /// embedder derived between its `sender_key_index` key and the
    /// recipient's `recipient_pubkey` (the key at `recipient_key_index`);
    /// `account_reference` is already masked; `account_label` empty = none.
    #[derive(Debug, Clone)]
    struct ContactRequestInput {
        to_user_id: [u8; 32],
        sender_key_index: u32,
        recipient_key_index: u32,
        recipient_pubkey: [u8; 33],
        account_reference: u32,
        compact_xpub: CompactXpub,
        shared_secret: [u8; 32],
        account_label: String,
    }

    /// An unmasked DIP-15 `accountReference`.
    #[derive(Debug, Clone, Default)]
    struct AccountRef {
        version: u32,
        account_index: u32,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u8)]
    enum SystemContract {
        Dpns = 0,
        Dashpay = 1,
    }

    unsafe extern "C++" {
        include!("dash/platform/signer.h");

        /// The embedder's wallet. `SignForKey` receives the identity key id
        /// and the full signable preimage and answers with a 65-byte compact
        /// recoverable ECDSA signature over its double SHA256.
        /// `SignAssetLockSighash` signs the 32-byte digest with the asset
        /// lock's outpoint key. Both are called synchronously on the thread
        /// that called the builder; `false` refuses.
        type WalletSigner;
        fn SignForKey(
            self: &WalletSigner,
            key_id: u32,
            signable: &[u8],
            sig_out: &mut Vec<u8>,
        ) -> bool;
        fn SignAssetLockSighash(
            self: &WalletSigner,
            sighash: &[u8; 32],
            sig_out: &mut Vec<u8>,
        ) -> bool;
    }

    extern "Rust" {
        /// One Platform SDK instance with its runtime, endpoint set, trust
        /// context and freshness state. Thread-safe; reads block the
        /// calling thread until the request completes or `shutdown`.
        type PlatformClient;

        fn new_platform_client(cfg: &Config) -> Result<Box<PlatformClient>>;

        // --- Trust inputs, pushed by the embedder ---------------------------
        /// Replaces the evonode endpoint set (`https://host:port`). An
        /// empty set drops the SDK and closes every Platform connection; a
        /// set that removes endpoints closes the connections to them (the
        /// retained entries keep their ban state). Trust inputs, freshness
        /// state and the verified protocol version are kept throughout.
        fn set_endpoints(self: &PlatformClient, https_uris: &[String]) -> Result<()>;
        /// Replaces the Platform quorum keys (the full active set).
        fn set_quorum_keys(self: &PlatformClient, keys: &[QuorumKey]);
        /// Pushes the embedder's best ChainLock height, the anchor of the
        /// proof-staleness floor. Monotonic; until the first push proved
        /// reads are not dispatched and return `Unavailable`.
        fn set_chainlock_height(self: &PlatformClient, height: u32);
        /// Aborts the in-flight request and releases the runtime. Reads
        /// afterwards return `Unavailable`. Idempotent.
        fn shutdown(self: &PlatformClient);

        // --- Proved reads, one SDK request each -----------------------------
        fn get_identity(self: &PlatformClient, id: &[u8; 32]) -> VerifiedIdentity;
        fn get_identity_by_pubkey_hash(
            self: &PlatformClient,
            pubkey_hash: &[u8; 20],
        ) -> VerifiedIdentity;
        /// `ProvenAbsent` = the identity has not used the contract yet;
        /// the next nonce is then 1, as after a value of 0.
        fn get_identity_contract_nonce(
            self: &PlatformClient,
            id: &[u8; 32],
            contract_id: &[u8; 32],
        ) -> VerifiedU64;
        fn resolve_name(self: &PlatformClient, normalized_label: &str) -> VerifiedDpnsName;
        /// One page of up to `limit` (clamped to 1..=100) names starting
        /// with `prefix` (1 to 63 characters once normalized), ascending;
        /// `start_after` all zero starts over.
        fn search_names(
            self: &PlatformClient,
            prefix: &str,
            limit: u32,
            start_after: &[u8; 32],
        ) -> VerifiedDpnsNames;
        /// One page of up to 100 names; `start_after` all zero starts over.
        /// Below protocol version 14 a continuation is `Unavailable`: Drive
        /// answers it with an empty page, whatever names remain.
        fn names_of_identity(
            self: &PlatformClient,
            identity: &[u8; 32],
            start_after: &[u8; 32],
        ) -> VerifiedDpnsNames;
        fn get_profile(self: &PlatformClient, owner: &[u8; 32]) -> VerifiedProfile;
        /// One page of up to 100 requests sent to (`to_me`) or by
        /// `identity`, created after `since_ms` (0 = all), oldest first.
        fn get_contact_requests(
            self: &PlatformClient,
            identity: &[u8; 32],
            to_me: bool,
            since_ms: u64,
            start_after: &[u8; 32],
        ) -> VerifiedContactRequests;
        fn get_contested_vote_state(
            self: &PlatformClient,
            normalized_label: &str,
        ) -> VerifiedContested;

        // --- Broadcast --------------------------------------------------------
        fn broadcast(self: &PlatformClient, state_transition: &[u8]) -> BroadcastResult;

        // --- Builders: no network, signer called on the calling thread ------
        // Every builder needs the protocol version a verified read has shown
        // the network to run (an error before the first successful read),
        // since transition rules change across versions.
        fn build_identity_create(
            self: &PlatformClient,
            proof: &AssetLockProofInput,
            keys: &[NewIdentityKey],
            signer: &WalletSigner,
        ) -> Result<Built>;
        fn build_dpns_preorder(
            self: &PlatformClient,
            owner: &[u8; 32],
            nonce: u64,
            label: &str,
            salt: &[u8; 32],
            key: &IdentityKey,
            signer: &WalletSigner,
        ) -> Result<Built>;
        fn build_dpns_domain(
            self: &PlatformClient,
            owner: &[u8; 32],
            nonce: u64,
            label: &str,
            salt: &[u8; 32],
            key: &IdentityKey,
            signer: &WalletSigner,
        ) -> Result<Built>;
        /// Creates the profile when `existing.document_id` is all zero,
        /// otherwise replaces `existing` (as `get_profile` returned it) at
        /// its revision + 1, keeping every field `input` does not edit.
        fn build_profile(
            self: &PlatformClient,
            owner: &[u8; 32],
            nonce: u64,
            existing: &Profile,
            input: &ProfileInput,
            key: &IdentityKey,
            signer: &WalletSigner,
        ) -> Result<Built>;
        fn build_contact_request(
            self: &PlatformClient,
            sender: &Identity,
            recipient: &Identity,
            nonce: u64,
            input: &ContactRequestInput,
            key: &IdentityKey,
            signer: &WalletSigner,
        ) -> Result<Built>;

        // --- Pure helpers -----------------------------------------------------
        fn normalize_label(label: &str) -> String;
        fn is_valid_username(label: &str) -> bool;
        fn is_contested_username(label: &str) -> bool;
        /// Credits a contested name registration prefunds, at the protocol
        /// version a verified read has shown the network to run; an error
        /// before that (the amount changed across versions).
        fn contested_vote_fund_credits(self: &PlatformClient) -> Result<u64>;
        fn credits_per_duff() -> u64;
        fn system_contract_id(which: SystemContract) -> Result<[u8; 32]>;
        fn dip15_decrypt_xpub(shared_secret: &[u8; 32], ciphertext: &[u8]) -> Result<CompactXpub>;
        fn dip15_account_reference_from_mac(
            mac: &[u8; 32],
            account_index: u32,
            version: u32,
        ) -> u32;
        fn dip15_unmask_account_reference_from_mac(mac: &[u8; 32], reference: u32) -> AccountRef;
        fn dip15_select_recipient_key(identity: &Identity) -> Result<u32>;
        fn dip15_receive_keys_acceptable(
            sender_purpose: u8,
            recipient_purpose: u8,
            recipient_key_id: u32,
        ) -> bool;
    }
}

impl Default for ffi::StatusKind {
    /// `Internal`, so a result that was never filled in cannot read as
    /// success.
    fn default() -> Self {
        ffi::StatusKind::Internal
    }
}

impl Default for ffi::BoundsKind {
    fn default() -> Self {
        ffi::BoundsKind::NoBounds
    }
}

impl Default for ffi::WinnerKind {
    fn default() -> Self {
        ffi::WinnerKind::NoWinner
    }
}

impl ffi::Status {
    pub fn ok() -> Self {
        ffi::Status {
            kind: ffi::StatusKind::Ok,
            consensus_code: 0,
            message: String::new(),
        }
    }

    pub fn new(kind: ffi::StatusKind, message: impl Into<String>) -> Self {
        ffi::Status {
            kind,
            consensus_code: 0,
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ffi::StatusKind::Internal, message)
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(ffi::StatusKind::Unavailable, message)
    }
}

/// The bridge's opaque client type. Dropping it shuts the client down.
pub struct PlatformClient(Client);

impl PlatformClient {
    /// The wrapped client, for Rust callers (tests) that configure it
    /// directly.
    pub fn inner(&self) -> &Client {
        &self.0
    }
}

impl Drop for PlatformClient {
    fn drop(&mut self) {
        // Third-party drop paths run here (tonic channels, the runtime); a
        // panic on the way out must not abort the embedder.
        let _ = catch_unwind(AssertUnwindSafe(|| self.0.shutdown()));
    }
}

/// Runs a bridge body, turning a panic into the value `on_panic` builds
/// from the panic message (an `Err` the C++ side receives as `rust::Error`,
/// or an `Internal` status) instead of the process abort cxx would perform.
fn guarded<T>(what: &str, body: impl FnOnce() -> T, on_panic: impl FnOnce(String) -> T) -> T {
    catch_unwind(AssertUnwindSafe(body)).unwrap_or_else(|payload| {
        let message = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or("<non-string panic payload>");
        on_panic(format!("{what} panicked: {message}"))
    })
}

fn fallible<T>(what: &str, body: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    guarded(what, body, Err)
}

/// [`guarded`] for the infallible entry points: a panic yields the type's
/// default (`()`, `false`, `0`, an empty string or struct), which the
/// embedder reads as a refusal.
fn guarded_default<T: Default>(what: &str, body: impl FnOnce() -> T) -> T {
    guarded(what, body, |_| T::default())
}

fn new_platform_client(cfg: &ffi::Config) -> Result<Box<PlatformClient>, String> {
    fallible("new_platform_client", || {
        Client::new(cfg).map(|client| Box::new(PlatformClient(client)))
    })
}

impl PlatformClient {
    fn set_endpoints(&self, https_uris: &[String]) -> Result<(), String> {
        fallible("set_endpoints", || self.0.set_endpoints(https_uris))
    }

    fn set_quorum_keys(&self, keys: &[ffi::QuorumKey]) {
        guarded_default("set_quorum_keys", || {
            self.0.provider().set_quorum_keys(keys)
        })
    }

    fn set_chainlock_height(&self, height: u32) {
        guarded_default("set_chainlock_height", || {
            self.0.provider().set_local_core_chain_locked_height(height)
        })
    }

    fn shutdown(&self) {
        guarded_default("shutdown", || self.0.shutdown())
    }

    fn get_identity(&self, id: &[u8; 32]) -> ffi::VerifiedIdentity {
        guarded(
            "get_identity",
            || ops::get_identity(&self.0, *id),
            ops::failed,
        )
    }

    fn get_identity_by_pubkey_hash(&self, pubkey_hash: &[u8; 20]) -> ffi::VerifiedIdentity {
        guarded(
            "get_identity_by_pubkey_hash",
            || ops::get_identity_by_pubkey_hash(&self.0, *pubkey_hash),
            ops::failed,
        )
    }

    fn get_identity_contract_nonce(
        &self,
        id: &[u8; 32],
        contract_id: &[u8; 32],
    ) -> ffi::VerifiedU64 {
        guarded(
            "get_identity_contract_nonce",
            || ops::get_identity_contract_nonce(&self.0, *id, *contract_id),
            ops::failed,
        )
    }

    fn resolve_name(&self, normalized_label: &str) -> ffi::VerifiedDpnsName {
        guarded(
            "resolve_name",
            || ops::resolve_name(&self.0, normalized_label),
            ops::failed,
        )
    }

    fn search_names(
        &self,
        prefix: &str,
        limit: u32,
        start_after: &[u8; 32],
    ) -> ffi::VerifiedDpnsNames {
        guarded(
            "search_names",
            || ops::search_names(&self.0, prefix, limit, cursor(start_after)),
            ops::failed,
        )
    }

    fn names_of_identity(
        &self,
        identity: &[u8; 32],
        start_after: &[u8; 32],
    ) -> ffi::VerifiedDpnsNames {
        guarded(
            "names_of_identity",
            || ops::names_of_identity(&self.0, *identity, cursor(start_after)),
            ops::failed,
        )
    }

    fn get_profile(&self, owner: &[u8; 32]) -> ffi::VerifiedProfile {
        guarded(
            "get_profile",
            || ops::get_profile(&self.0, *owner),
            ops::failed,
        )
    }

    fn get_contact_requests(
        &self,
        identity: &[u8; 32],
        to_me: bool,
        since_ms: u64,
        start_after: &[u8; 32],
    ) -> ffi::VerifiedContactRequests {
        guarded(
            "get_contact_requests",
            || ops::get_contact_requests(&self.0, *identity, to_me, since_ms, cursor(start_after)),
            ops::failed,
        )
    }

    fn get_contested_vote_state(&self, normalized_label: &str) -> ffi::VerifiedContested {
        guarded(
            "get_contested_vote_state",
            || ops::get_contested_vote_state(&self.0, normalized_label),
            ops::failed,
        )
    }

    fn broadcast(&self, state_transition: &[u8]) -> ffi::BroadcastResult {
        guarded(
            "broadcast",
            || ops::broadcast(&self.0, state_transition),
            |message| ffi::BroadcastResult {
                status: ffi::Status::internal(message),
            },
        )
    }

    fn build_identity_create(
        &self,
        proof: &ffi::AssetLockProofInput,
        keys: &[ffi::NewIdentityKey],
        signer: &ffi::WalletSigner,
    ) -> Result<ffi::Built, String> {
        fallible("build_identity_create", || {
            builders::build_identity_create(
                self.0.verified_platform_version()?,
                proof,
                keys,
                signer,
            )
        })
    }

    fn build_dpns_preorder(
        &self,
        owner: &[u8; 32],
        nonce: u64,
        label: &str,
        salt: &[u8; 32],
        key: &ffi::IdentityKey,
        signer: &ffi::WalletSigner,
    ) -> Result<ffi::Built, String> {
        fallible("build_dpns_preorder", || {
            builders::build_dpns_preorder(
                self.0.verified_platform_version()?,
                *owner,
                nonce,
                label,
                salt,
                builders::fresh_entropy(),
                key,
                signer,
            )
        })
    }

    fn build_dpns_domain(
        &self,
        owner: &[u8; 32],
        nonce: u64,
        label: &str,
        salt: &[u8; 32],
        key: &ffi::IdentityKey,
        signer: &ffi::WalletSigner,
    ) -> Result<ffi::Built, String> {
        fallible("build_dpns_domain", || {
            builders::build_dpns_domain(
                self.0.verified_platform_version()?,
                *owner,
                nonce,
                label,
                salt,
                builders::fresh_entropy(),
                key,
                signer,
            )
        })
    }

    fn build_profile(
        &self,
        owner: &[u8; 32],
        nonce: u64,
        existing: &ffi::Profile,
        input: &ffi::ProfileInput,
        key: &ffi::IdentityKey,
        signer: &ffi::WalletSigner,
    ) -> Result<ffi::Built, String> {
        fallible("build_profile", || {
            builders::build_profile(
                self.0.verified_platform_version()?,
                *owner,
                nonce,
                existing,
                input,
                builders::fresh_entropy(),
                key,
                signer,
            )
        })
    }

    fn build_contact_request(
        &self,
        sender: &ffi::Identity,
        recipient: &ffi::Identity,
        nonce: u64,
        input: &ffi::ContactRequestInput,
        key: &ffi::IdentityKey,
        signer: &ffi::WalletSigner,
    ) -> Result<ffi::Built, String> {
        fallible("build_contact_request", || {
            let version = self.0.verified_platform_version()?;
            let sdk = self.0.sdk()?;
            builders::build_contact_request(
                &sdk, version, sender, recipient, nonce, input, key, signer,
            )
        })
    }

    fn contested_vote_fund_credits(&self) -> Result<u64, String> {
        fallible("contested_vote_fund_credits", || {
            Ok(helpers::contested_vote_fund_credits(
                self.0.verified_platform_version()?,
            ))
        })
    }
}

/// A paging cursor: all zero means "from the start".
fn cursor(start_after: &[u8; 32]) -> Option<[u8; 32]> {
    (*start_after != [0u8; 32]).then_some(*start_after)
}

fn normalize_label(label: &str) -> String {
    guarded_default("normalize_label", || helpers::normalize_label(label))
}

fn is_valid_username(label: &str) -> bool {
    guarded_default("is_valid_username", || helpers::is_valid_username(label))
}

fn is_contested_username(label: &str) -> bool {
    guarded_default("is_contested_username", || {
        helpers::is_contested_username(label)
    })
}

fn credits_per_duff() -> u64 {
    helpers::credits_per_duff()
}

fn system_contract_id(which: ffi::SystemContract) -> Result<[u8; 32], String> {
    fallible("system_contract_id", || helpers::system_contract_id(which))
}

fn dip15_decrypt_xpub(
    shared_secret: &[u8; 32],
    ciphertext: &[u8],
) -> Result<ffi::CompactXpub, String> {
    fallible("dip15_decrypt_xpub", || {
        let secret = Zeroizing::new(*shared_secret);
        helpers::dip15_decrypt_xpub(&secret, ciphertext)
    })
}

fn dip15_account_reference_from_mac(mac: &[u8; 32], account_index: u32, version: u32) -> u32 {
    guarded_default("dip15_account_reference_from_mac", || {
        helpers::dip15_account_reference_from_mac(mac, account_index, version)
    })
}

fn dip15_unmask_account_reference_from_mac(mac: &[u8; 32], reference: u32) -> ffi::AccountRef {
    guarded_default("dip15_unmask_account_reference_from_mac", || {
        helpers::dip15_unmask_account_reference_from_mac(mac, reference)
    })
}

fn dip15_select_recipient_key(identity: &ffi::Identity) -> Result<u32, String> {
    fallible("dip15_select_recipient_key", || {
        helpers::dip15_select_recipient_key(identity)
    })
}

fn dip15_receive_keys_acceptable(
    sender_purpose: u8,
    recipient_purpose: u8,
    recipient_key_id: u32,
) -> bool {
    guarded_default("dip15_receive_keys_acceptable", || {
        helpers::dip15_receive_keys_acceptable(sender_purpose, recipient_purpose, recipient_key_id)
    })
}
