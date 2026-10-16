// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! A C++ binding of `dash-sdk` for an embedder that supplies its own
//! endpoints, quorum keys ([`context`]) and wallet ([`signer`]). Each entry
//! point ([`client`]) forwards to `dash-sdk` and returns a [`ffi::Status`].
//! No panic reaches C++: every entry point runs under [`guarded`].

#[cfg(panic = "abort")]
compile_error!("dash-sdk-cxx requires panic = \"unwind\"; its FFI guards rely on catch_unwind");

mod client;
mod context;
mod marshal;
mod runtime;
mod signer;

use std::panic::{catch_unwind, AssertUnwindSafe};

use client::PlatformClient;

#[cxx::bridge(namespace = "dash_sdk")]
pub mod ffi {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u8)]
    enum Network {
        Mainnet = 0,
        Testnet = 1,
        Devnet = 2,
        Regtest = 3,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u8)]
    enum ProxyKind {
        /// Connect directly; `address` must be empty.
        None = 0,
        /// SOCKS5 at a numeric `ip:port` (IPv6 in brackets).
        Tcp = 1,
        /// SOCKS5 on the Unix socket at path `address`.
        Unix = 2,
    }

    /// The SOCKS5 proxy every Platform connection goes through, with no
    /// direct fallback. `isolate`: fresh random credentials per connection,
    /// so Tor gives each its own circuit.
    #[derive(Debug, Clone)]
    struct Proxy {
        kind: ProxyKind,
        address: String,
        isolate: bool,
    }

    /// `platform_llmq_type`: the only LLMQ type proofs may be signed by.
    /// `endpoints`: `https://host:port` evonode addresses, the host a
    /// literal IP, or an onion name when a proxy is set. Nothing is resolved
    /// locally. Fixed for the client's lifetime.
    #[derive(Debug, Clone)]
    struct Config {
        network: Network,
        platform_llmq_type: u8,
        proxy: Proxy,
        endpoints: Vec<String>,
    }

    /// The outcome of a call. A result's value is meaningful only under
    /// `Ok`; `message` is for logs.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u8)]
    enum StatusKind {
        Ok = 0,
        /// A proof shows the object does not exist.
        NotFound = 1,
        /// Nothing was proved: transport failure, timeout, proxy failure,
        /// no usable endpoint, or the client is shut down.
        Unavailable = 2,
        /// A response failed verification (proof, quorum, staleness) or a
        /// node refused the request.
        Rejected = 3,
        /// Platform rejected a state transition; see `consensus_code`.
        Consensus = 4,
        /// Bad input from the embedder, a refused callback, or a bug.
        Internal = 5,
    }

    #[derive(Debug, Clone)]
    struct Status {
        kind: StatusKind,
        consensus_code: u32,
        message: String,
    }

    /// An identity public key; `disabled_at == 0` means enabled.
    #[derive(Debug, Clone, Default)]
    struct IdentityKey {
        id: u32,
        purpose: u8,
        security_level: u8,
        key_type: u8,
        read_only: bool,
        data: Vec<u8>,
        disabled_at: u64,
    }

    #[derive(Debug, Clone, Default)]
    struct Identity {
        id: [u8; 32],
        balance: u64,
        revision: u64,
        keys: Vec<IdentityKey>,
    }

    #[derive(Debug, Clone)]
    struct IdentityResult {
        status: Status,
        identity: Identity,
    }

    #[derive(Debug, Clone)]
    struct IdResult {
        status: Status,
        id: [u8; 32],
    }

    #[derive(Debug, Clone)]
    struct BalanceResult {
        status: Status,
        balance: u64,
    }

    /// A DPNS name. `identity` is the identity it resolves to, all zero
    /// when its record names none.
    #[derive(Debug, Clone, Default)]
    struct DpnsName {
        label: String,
        normalized_label: String,
        owner: [u8; 32],
        identity: [u8; 32],
    }

    /// `complete`: true only when the SDK proved the end of the list.
    #[derive(Debug, Clone)]
    struct NamesResult {
        status: Status,
        names: Vec<DpnsName>,
        complete: bool,
    }

    #[derive(Debug, Clone, Default)]
    struct Contender {
        identity: [u8; 32],
        votes: u32,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u8)]
    enum WinnerKind {
        /// Still open, or not finished.
        None = 0,
        Identity = 1,
        Locked = 2,
    }

    #[derive(Debug, Clone)]
    struct VoteState {
        contenders: Vec<Contender>,
        abstain: u32,
        lock: u32,
        winner_kind: WinnerKind,
        winner: [u8; 32],
    }

    #[derive(Debug, Clone)]
    struct VoteStateResult {
        status: Status,
        state: VoteState,
    }

    /// A contested DPNS name: its normalized label, end time and state.
    /// `state` is left empty by `current_contests`.
    #[derive(Debug, Clone)]
    struct Contest {
        name: String,
        ends_at_ms: u64,
        state: VoteState,
    }

    #[derive(Debug, Clone)]
    struct ContestsResult {
        status: Status,
        contests: Vec<Contest>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u8)]
    enum VoteChoice {
        TowardsIdentity = 0,
        Abstain = 1,
        Lock = 2,
    }

    /// A masternode's vote on a DPNS name; `towards` is all zero unless
    /// `choice` is `TowardsIdentity`.
    #[derive(Debug, Clone)]
    struct NameVote {
        normalized_label: String,
        choice: VoteChoice,
        towards: [u8; 32],
    }

    #[derive(Debug, Clone)]
    struct NameVotesResult {
        status: Status,
        votes: Vec<NameVote>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u8)]
    enum ContactDirection {
        Sent = 0,
        Received = 1,
    }

    /// A DashPay `contactRequest` document. Empty vectors mean the field is
    /// absent.
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
        created_at_ms: u64,
        created_at_core_height: u32,
    }

    #[derive(Debug, Clone)]
    struct ContactRequestsResult {
        status: Status,
        requests: Vec<ContactRequest>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u8)]
    enum AssetLockProofKind {
        /// `transaction` and `instant_lock` consensus-encoded; the credit
        /// output at `output_index`.
        Instant = 0,
        /// `out_point` (txid || vout, consensus-encoded) ChainLocked at or
        /// below `core_chain_locked_height`.
        Chain = 1,
    }

    #[derive(Debug, Clone)]
    struct AssetLockProof {
        kind: AssetLockProofKind,
        transaction: Vec<u8>,
        instant_lock: Vec<u8>,
        output_index: u32,
        core_chain_locked_height: u32,
        out_point: [u8; 36],
    }

    /// `signing_key_id`: the sender's key that signs the document.
    /// `account_label` empty = none.
    #[derive(Debug, Clone)]
    struct ContactRequestInput {
        sender: [u8; 32],
        recipient: [u8; 32],
        sender_key_index: u32,
        recipient_key_index: u32,
        signing_key_id: u32,
        account_reference: u32,
        account_label: String,
    }

    /// The DIP-15 compact extended public key, unvalidated beyond its
    /// length: the embedder checks `public_key` is a valid point.
    #[derive(Debug, Clone)]
    struct CompactXpub {
        parent_fingerprint: [u8; 4],
        chain_code: [u8; 32],
        public_key: [u8; 33],
    }

    unsafe extern "C++" {
        include!("dash/sdk/callbacks.h");

        type WalletSigner;
        type ContactKeys;
        type CoreContext;

        fn SignIdentityKey(
            self: &WalletSigner,
            key_id: u32,
            key_type: u8,
            key_data: &[u8],
            signable: &[u8],
            sig_out: &mut [u8; 65],
        ) -> bool;
        fn SignAssetLock(
            self: &WalletSigner,
            key_ref: u32,
            sighash: &[u8; 32],
            sig_out: &mut [u8; 65],
        ) -> bool;

        fn SharedSecret(
            self: &ContactKeys,
            sender_key_id: u32,
            recipient_key: &[u8; 33],
            secret_out: &mut [u8; 32],
        ) -> bool;
        fn CompactXpub(
            self: &ContactKeys,
            recipient: &[u8; 32],
            account_reference: u32,
            xpub_out: &mut [u8; 69],
        ) -> bool;

        fn QuorumPublicKey(
            self: &CoreContext,
            llmq_type: u8,
            quorum_hash: &[u8; 32],
            core_chain_locked_height: u32,
            key_out: &mut [u8; 48],
        ) -> bool;
        fn PlatformActivationHeight(self: &CoreContext, height_out: &mut u32) -> bool;
    }

    extern "Rust" {
        /// One SDK instance, its runtime and its `CoreContext`. Thread-safe:
        /// calls may run concurrently, each blocking its calling thread.
        type PlatformClient;

        /// A rust::Error for a config it cannot honour: never a direct fallback.
        fn new_platform_client(
            config: &Config,
            context: UniquePtr<CoreContext>,
        ) -> Result<Box<PlatformClient>>;
        /// Refuses new calls, cancels the running ones and waits for every
        /// one to return, so no callback runs once it returns; the
        /// CoreContext is destroyed before it returns. Later calls are
        /// `Unavailable`. Idempotent; dropping the client calls it. Must not
        /// be called from a callback, or while holding a lock a callback
        /// takes.
        fn shutdown(self: &PlatformClient);

        // Reads: every value is verified against the CoreContext.
        fn get_identity(self: &PlatformClient, id: &[u8; 32]) -> IdentityResult;
        fn get_identity_by_pubkey_hash(
            self: &PlatformClient,
            pubkey_hash: &[u8; 20],
        ) -> IdentityResult;
        /// The identity a DPNS name (label or `label.dash`) resolves to.
        /// `NotFound` means "does not resolve", not "available": a contested
        /// name has no domain until its contest ends.
        fn resolve_name(self: &PlatformClient, name: &str) -> IdResult;
        /// Reads at most `max_pages` pages of `page_limit` names (both >= 1).
        fn names_of_identity(
            self: &PlatformClient,
            identity: &[u8; 32],
            page_limit: u32,
            max_pages: u32,
        ) -> NamesResult;
        /// Up to `limit` (>= 1) names whose normalized label starts with
        /// `prefix`. `complete` is always false: the SDK does not report
        /// whether it reached the end.
        fn search_names(self: &PlatformClient, prefix: &str, limit: u32) -> NamesResult;
        /// Unresolved contests `identity` takes part in; best effort, the
        /// SDK examines at most 100 contests.
        fn pending_contests_for_identity(
            self: &PlatformClient,
            identity: &[u8; 32],
        ) -> ContestsResult;
        /// The contest for a DPNS label (normalized here).
        fn contested_vote_state(self: &PlatformClient, label: &str) -> VoteStateResult;
        /// DPNS names whose contest is open, with their end times. Best
        /// effort: the SDK may stop after its first page of 100 polls.
        fn current_contests(self: &PlatformClient) -> ContestsResult;
        /// One proved page (up to 100) of the DPNS votes the masternode
        /// `pro_tx_hash` has cast, possibly not all: the SDK cannot ask for
        /// the next page. Platform records votes by proTxHash, not by voter
        /// identity. Hash in the embedder's internal byte order.
        fn identity_votes(self: &PlatformClient, pro_tx_hash: &[u8; 32]) -> NameVotesResult;
        /// Every request sent by or to `identity`, created after `since_ms`
        /// (0 = all).
        fn contact_requests(
            self: &PlatformClient,
            identity: &[u8; 32],
            direction: ContactDirection,
            since_ms: u64,
        ) -> ContactRequestsResult;

        // Writes: one end-to-end SDK call each, waiting for the proved
        // result. Callback objects are moved in.
        /// Creates the identity holding `keys` (each must be enabled).
        fn register_identity(
            self: &PlatformClient,
            keys: &[IdentityKey],
            proof: &AssetLockProof,
            asset_lock_key_ref: u32,
            signer: UniquePtr<WalletSigner>,
        ) -> IdentityResult;
        /// The identity's balance after the top-up.
        fn top_up_identity(
            self: &PlatformClient,
            identity: &[u8; 32],
            proof: &AssetLockProof,
            asset_lock_key_ref: u32,
            signer: UniquePtr<WalletSigner>,
        ) -> BalanceResult;
        /// Preorder then domain, signed with `key_id`; the domain document
        /// id. `contest_fund`: the most a contested name may pay into its
        /// contest; 0 lets the SDK price it just before the domain is sent.
        fn register_dpns_name(
            self: &PlatformClient,
            label: &str,
            identity: &[u8; 32],
            key_id: u32,
            contest_fund: u64,
            signer: UniquePtr<WalletSigner>,
        ) -> IdResult;
        /// The contact request document id.
        fn send_contact_request(
            self: &PlatformClient,
            input: &ContactRequestInput,
            signer: UniquePtr<WalletSigner>,
            keys: UniquePtr<ContactKeys>,
        ) -> IdResult;
        /// Votes on the contest for `label` (normalized here) as the
        /// masternode `pro_tx_hash` (internal byte order), signing with its
        /// voting key, whose HASH160 (CKeyID bytes, not reversed) is
        /// `voting_key_hash`. `towards` is read only for `TowardsIdentity`.
        /// A refused voting key gets the SDK's diagnosis appended to the
        /// message. `Ok` when the SDK reports the vote stored; on a duplicate
        /// broadcast it reports whatever vote Platform holds for the poll.
        fn cast_masternode_vote(
            self: &PlatformClient,
            pro_tx_hash: &[u8; 32],
            voting_key_hash: &[u8; 20],
            label: &str,
            choice: VoteChoice,
            towards: &[u8; 32],
            signer: UniquePtr<WalletSigner>,
        ) -> Status;

        // No network. A panic answers the safe value: not valid, contested.
        fn is_valid_username(label: &str) -> bool;
        fn is_contested_username(label: &str) -> bool;
        /// Decrypts `encryptedPublicKey` with the wallet's ECDH secret.
        fn decrypt_contact_xpub(
            shared_secret: &[u8; 32],
            encrypted_public_key: &[u8],
        ) -> Result<CompactXpub>;
    }
}

/// Runs a bridge body; a panic becomes the value `on_panic` builds from its
/// message instead of the process abort cxx would perform.
pub(crate) fn guarded<T>(
    what: &str,
    body: impl FnOnce() -> T,
    on_panic: impl FnOnce(String) -> T,
) -> T {
    catch_unwind(AssertUnwindSafe(body)).unwrap_or_else(|payload| {
        let message = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or("non-string panic payload");
        on_panic(format!("{what} panicked: {message}"))
    })
}

fn new_platform_client(
    config: &ffi::Config,
    context: cxx::UniquePtr<ffi::CoreContext>,
) -> Result<Box<PlatformClient>, String> {
    guarded(
        "new_platform_client",
        || PlatformClient::new(config, context).map(Box::new),
        Err,
    )
}

fn is_valid_username(label: &str) -> bool {
    guarded(
        "is_valid_username",
        || dash_sdk::platform::dpns_usernames::is_valid_username(label),
        |_| false,
    )
}

fn is_contested_username(label: &str) -> bool {
    contested(
        dash_sdk::platform::dpns_usernames::is_contested_username,
        label,
    )
}

/// Fails closed: a contested answer keeps Core off the ordinary-name path.
fn contested(check: fn(&str) -> bool, label: &str) -> bool {
    guarded("is_contested_username", || check(label), |_| true)
}

fn decrypt_contact_xpub(
    shared_secret: &[u8; 32],
    encrypted_public_key: &[u8],
) -> Result<ffi::CompactXpub, String> {
    guarded(
        "decrypt_contact_xpub",
        || {
            let secret = zeroize::Zeroizing::new(*shared_secret);
            let plain = zeroize::Zeroizing::new(
                platform_encryption::decrypt_extended_public_key(&secret, encrypted_public_key)
                    .map_err(|e| format!("cannot decrypt the contact xpub: {e}"))?,
            );
            let xpub = platform_encryption::parse_compact_xpub(&plain)
                .map_err(|e| format!("not a DIP-15 compact xpub: {e}"))?;
            Ok(ffi::CompactXpub {
                parent_fingerprint: xpub.parent_fingerprint,
                chain_code: xpub.chain_code,
                public_key: xpub.public_key,
            })
        },
        Err,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panic_becomes_the_fallback_value() {
        let value = guarded(
            "probe",
            || -> u8 { panic!("hostile bytes") },
            |m| {
                assert!(m.contains("probe panicked: hostile bytes"), "{m}");
                7
            },
        );
        assert_eq!(value, 7);
        assert!(contested(|_| panic!("hostile label"), "alice"));
    }

    /// The xpub is returned as decrypted: the public key is not normalized
    /// or validated, so the embedder checks the point itself.
    #[test]
    fn a_contact_xpub_round_trips_unmodified() {
        let secret = [5u8; 32];
        let mut plain = [0u8; platform_encryption::COMPACT_XPUB_LEN];
        for (i, byte) in plain.iter_mut().enumerate() {
            *byte = i as u8;
        }
        let blob = platform_encryption::encrypt_extended_public_key(&secret, &[3; 16], &plain);
        let xpub = decrypt_contact_xpub(&secret, &blob).expect("decrypts");
        assert_eq!(xpub.parent_fingerprint, plain[..4]);
        assert_eq!(xpub.chain_code, plain[4..36]);
        assert_eq!(xpub.public_key, plain[36..]);
        assert!(decrypt_contact_xpub(&[6; 32], &blob).is_err());
        assert!(decrypt_contact_xpub(&secret, &blob[..20]).is_err());
    }
}
