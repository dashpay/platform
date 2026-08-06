//! Per-identity DashPay social state.
//!
//! [`DashPayState`] groups the DashPay-contract layer (contacts,
//! requests, profile, payments, deferred crypto) that a
//! [`ManagedIdentity`](super::ManagedIdentity) carries on top of its
//! identity-core fields. The struct itself is pure data; the mutation
//! methods that maintain its invariants stay on `ManagedIdentity`
//! (they need the identity id and the persistence snapshot).

use crate::wallet::identity::{
    ContactProfileEntry, ContactRequest, DashPayProfile, EstablishedContact, PaymentEntry,
};
use dpp::prelude::Identifier;
use std::collections::{BTreeMap, BTreeSet};

/// DashPay social state carried by one [`ManagedIdentity`](super::ManagedIdentity).
#[derive(Debug, Clone, Default)]
pub struct DashPayState {
    /// Map of established contacts (bidirectional relationships) keyed by contact identity ID
    pub(super) established_contacts: BTreeMap<Identifier, EstablishedContact>,

    /// Map of sent contact requests (outgoing, not yet reciprocated) keyed by recipient ID
    pub(super) sent_contact_requests: BTreeMap<Identifier, ContactRequest>,

    /// Map of incoming contact requests (not yet accepted) keyed by sender ID
    pub(super) incoming_contact_requests: BTreeMap<Identifier, ContactRequest>,

    /// Senders this identity has chosen to **ignore** (per-sender mute,
    /// reversible — the local-only equivalent of "block"). Keyed by the
    /// sender's identity id.
    ///
    /// `ignore_sender` records a sender here so the recurring sync ingest
    /// path won't resurrect *any* of that sender's still-on-platform
    /// immutable `contactRequest` documents — including rotated ones with
    /// a bumped `accountReference`. Suppression is per-sender by design: if
    /// you ignored the person you ignored them; `unignore_sender` is the
    /// "changed my mind" affordance, which also rewinds the receive cursor
    /// so the next sweep re-fetches their requests.
    ///
    /// Local-only: there is no on-chain artifact (syncing it would leak who
    /// you ignored via the public contact-request indices). Cross-device
    /// sync is deferred to a future encrypted `profile` field.
    pub(super) ignored_senders: BTreeSet<Identifier>,

    /// DIP-15 auto-accept proofs that failed cryptographic verification (or
    /// were expired / malformed) during a `drain_auto_accepts` pass, keyed by
    /// `SHA256(sender_id ‖ proof_bytes)`. Consulted by the sync sweep's
    /// auto-accept enqueue gate so a structurally-valid but cryptographically
    /// bogus `autoAcceptProof` on an attacker-published contact request is not
    /// re-enqueued every sweep (which would keep the "waiting to finish setup"
    /// banner permanently tripped).
    ///
    /// Keying on `(sender, proof)` — not on the sender alone — means a
    /// **different** proof from the same sender still enqueues (a genuine
    /// re-issued proof is not blocked by a prior bad one). Only PERMANENT
    /// failures (invalid signature / expired / malformed / bad index) are
    /// recorded here; transient failures (provider unavailable, reciprocal-send
    /// failure) stay retryable and are never marked.
    ///
    /// In-memory only (never persisted): a relaunch clears it, so a bad proof is
    /// retried at most once per launch. The request stays manually acceptable
    /// meanwhile, and the sender never establishes off a bogus proof — so
    /// retrying once per launch is harmless and avoids a persisted tombstone for
    /// attacker-controlled input. Capped at
    /// `ManagedIdentity::AUTO_ACCEPT_VERIFY_FAILED_CAP` entries (arbitrary
    /// eviction over cap) so a griefer paying credits for many distinct
    /// malformed proofs can't grow it unboundedly for the process lifetime.
    pub(super) auto_accept_verify_failed: BTreeSet<[u8; 32]>,

    /// Incremental-sync high-water marks (`$createdAt` ms of the newest
    /// `contactRequest` fetched) per direction. `None` ⇒ never synced; the
    /// next sweep does a full fetch. Held in memory: it survives across sweeps
    /// within a session but resets to `None` on cold restart, triggering one
    /// full re-fetch (safe — ingest is a fixpoint, so under-shoot is free).
    /// Durable cross-relaunch persistence is a follow-up; when added, restore
    /// must tolerate only under-shoot — never a value higher than the contact
    /// state justifies.
    pub(super) high_water_received_ms: Option<u64>,
    /// High-water mark for the sent direction (`$ownerId == me`).
    pub(super) high_water_sent_ms: Option<u64>,

    /// DashPay profile (display name, bio, avatar, public message)
    /// published via the DashPay data contract. `None` until the
    /// profile has been fetched or set.
    pub profile: Option<DashPayProfile>,

    /// DashPay payment history keyed by transaction id (hex string).
    /// Each entry records a single Dash payment to or from a contact
    /// identity, with direction, amount, memo, and status.
    pub payments: BTreeMap<String, PaymentEntry>,

    /// SPV scan height at which each contact's historical sent-payment
    /// reconstruction sweep last completed.
    ///
    /// `reconcile_sent_payments_from_tx_history` is a restore-time recovery
    /// path. Re-running its full persisted-tx scan every recurring sync pass
    /// is pure overhead once a contact has been reconstructed — but "already
    /// swept" is only a safe answer for the history that existed at the time.
    ///
    /// Hence a height, not a flag. A sweep certifies the transaction table *as
    /// of* `synced_height`; the contact becomes eligible again the moment that
    /// height advances, because newly scanned blocks are exactly when new rows
    /// can appear. Nothing has to know whether the host has "finished"
    /// delivering history — an answer no callback provides — and no ordering
    /// between this sweep and the rescan reconcile has to hold.
    ///
    /// In steady state the height stops moving and the sweep stops running.
    ///
    /// In-memory only (never persisted): a relaunch re-sweeps once per contact,
    /// which is safe and far cheaper than re-scanning every pass forever.
    pub sent_payment_reconcile_swept_at: BTreeMap<Identifier, u32>,

    /// Cached **contact** profiles keyed by the contact's identity id —
    /// established contacts, pending incoming-request senders, and (later)
    /// ignored senders, independent of relationship state. Populated by
    /// `sync_contact_profiles`; public-data only (never `contactInfo`-derived).
    pub contact_profiles: BTreeMap<Identifier, ContactProfileEntry>,

    /// Contacts for which a historical L1 rescan has already been triggered this
    /// process lifetime (DIP-15 §12.6 coreHeight backfill). When the rescan
    /// reconcile lowers the wallet's SPV `synced_height` to a contact's funding
    /// height so the filter manager re-scans for payments that landed before the
    /// receival address was watched, the contact is recorded here so the
    /// recurring sweep does not re-lower the height every pass — which would
    /// reset the in-flight backfill and prevent it from ever completing.
    ///
    /// In-memory only (never persisted): a relaunch clears it, and because
    /// `synced_height` is restored at its monotonic high-water, an interrupted
    /// backfill is re-triggered on the next launch — self-healing. The cost of
    /// that reset is one historical re-match per launch while any contact is
    /// funded below the tip; the compact filters are reused from disk (not
    /// re-downloaded), so it is cheap. A persisted breadcrumb could make the
    /// backfill durable across a crash if that ever becomes necessary.
    pub rescan_triggered: BTreeSet<Identifier>,

    /// DashPay contact-crypto ops the unattended background sweep enqueued for
    /// THIS identity but could not perform because key material was unavailable
    /// (watch-only / signer locked). Drained when a signer is available
    /// (Keychain unlock, or any signer-present action). Secret-free — only
    /// on-chain ciphertext + public-key indices; each entry still carries its
    /// `owner_identity_id` (== this identity) as the drain's routing key and the
    /// SQLite key column. In-memory only for the live session: the queue is
    /// persisted to the changeset (SQLite backend), but cold-load restore is
    /// blocked upstream, so a re-imported wallet re-syncs from scratch and the
    /// sweep re-enqueues what it needs. Deliberately NOT captured by
    /// `IdentityEntry::from_managed` — persistence rides the flat changeset
    /// delta, not a per-identity snapshot.
    /// See [`PendingContactCrypto`](crate::changeset::PendingContactCrypto).
    pub pending_contact_crypto: Vec<crate::changeset::PendingContactCrypto>,
}

// Read access to the guarded relationship/cursor fields. Mutation goes
// through the invariant-holding methods on `ManagedIdentity` (or their
// `apply_*` replay counterparts) — the fields themselves are sealed to
// this module tree so no caller can insert a relationship without the
// auto-establish / tombstone / compare-and-advance rules running.
impl DashPayState {
    /// Established contacts (bidirectional relationships) keyed by
    /// contact identity id.
    pub fn established_contacts(&self) -> &BTreeMap<Identifier, EstablishedContact> {
        &self.established_contacts
    }

    /// Sent contact requests (outgoing, not yet reciprocated) keyed by
    /// recipient id.
    pub fn sent_contact_requests(&self) -> &BTreeMap<Identifier, ContactRequest> {
        &self.sent_contact_requests
    }

    /// Incoming contact requests (not yet accepted) keyed by sender id.
    pub fn incoming_contact_requests(&self) -> &BTreeMap<Identifier, ContactRequest> {
        &self.incoming_contact_requests
    }

    /// Senders this identity has chosen to ignore (per-sender mute).
    pub fn ignored_senders(&self) -> &BTreeSet<Identifier> {
        &self.ignored_senders
    }

    /// Received-direction incremental-sync cursor (`$createdAt` ms of the
    /// newest fetched `contactRequest`). `None` ⇒ never synced this session.
    pub fn high_water_received_ms(&self) -> Option<u64> {
        self.high_water_received_ms
    }

    /// Sent-direction incremental-sync cursor. `None` ⇒ never synced this
    /// session.
    pub fn high_water_sent_ms(&self) -> Option<u64> {
        self.high_water_sent_ms
    }
}
