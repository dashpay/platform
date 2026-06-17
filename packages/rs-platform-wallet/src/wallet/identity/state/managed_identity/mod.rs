//! Managed identity that combines a Platform Identity with wallet-specific metadata
//!
//! This module provides the `ManagedIdentity` struct which wraps a Platform Identity
//! with additional metadata for wallet management.

mod contact_requests;
mod contacts;
mod identity_ops;
mod sync;

// `block_time` + `key_storage` moved to `crate::wallet::identity::types`.
// Re-export so every `impl ManagedIdentity` block below keeps working
// unchanged and external users can still reach them through the old
// `state::managed_identity::*` path.
pub use crate::wallet::identity::types::block_time::{self, BlockTime};
pub use crate::wallet::identity::types::key_storage::{
    self, DpnsNameInfo, IdentityStatus, KeyStorage, PrivateKeyData,
};

use crate::wallet::identity::{
    ContactProfileEntry, ContactRequest, DashPayProfile, EstablishedContact, PaymentEntry,
};
use dpp::identity::Identity;
use dpp::prelude::Identifier;
use std::collections::BTreeMap;

/// A managed identity that combines an Identity with wallet-specific metadata.
///
/// Two buckets the manager keeps these in:
///
/// - `IdentityManager.wallet_identities[wallet_id][identity_index]` —
///   wallet-owned, signing-capable. `wallet_id == Some(_)`,
///   `identity_index == Some(_)`.
/// - `IdentityManager.out_of_wallet_identities[identity_id]` —
///   observed read-only. `wallet_id == None`, `identity_index == None`.
///   Cannot sign — not because of an explicit "watched" flag, but because
///   there's no wallet to derive private keys from.
#[derive(Debug, Clone)]
pub struct ManagedIdentity {
    /// The Platform identity
    pub identity: Identity,

    /// The BIP-9 HD identity index used during registration or discovery.
    ///
    /// This is the index in the derivation path
    /// `m/9'/coin'/5'/0'/key_type'/identity_index'/key_id'`.
    /// Recorded during identity registration or gap-limit discovery so that
    /// subsequent operations (signing, ECDH) can derive the correct keys.
    ///
    /// `Some(idx)` when this identity lives in a wallet's bucket — `idx` is
    /// the inner BTreeMap key. `None` for out-of-wallet identities (formerly
    /// "watched"); they have no HD-derivation context.
    pub identity_index: Option<u32>,

    /// Last block time when balance was updated for this identity
    pub last_updated_balance_block_time: Option<BlockTime>,

    /// Last block time when keys were synced for this identity
    pub last_synced_keys_block_time: Option<BlockTime>,

    /// Map of established contacts (bidirectional relationships) keyed by contact identity ID
    pub established_contacts: BTreeMap<Identifier, EstablishedContact>,

    /// Map of sent contact requests (outgoing, not yet reciprocated) keyed by recipient ID
    pub sent_contact_requests: BTreeMap<Identifier, ContactRequest>,

    /// Map of incoming contact requests (not yet accepted) keyed by sender ID
    pub incoming_contact_requests: BTreeMap<Identifier, ContactRequest>,

    /// Rejected-request tombstones (G5 stage 1) keyed by
    /// `(sender_id, account_reference)`.
    ///
    /// A `reject_contact_request` records the `(sender, accountReference)`
    /// of the dropped incoming request here so the recurring sync ingest
    /// path won't resurrect the still-on-platform immutable document. The
    /// key deliberately includes `account_reference`: a once-rejected
    /// sender CAN re-request via a bumped `accountReference` (DIP-15
    /// rotation), and that rotated request is NOT suppressed.
    pub rejected_contact_requests:
        BTreeMap<(Identifier, u32), crate::changeset::RejectedContactRequest>,

    /// Identity lifecycle status on Platform.
    pub status: IdentityStatus,

    /// DPNS usernames associated with this identity.
    pub dpns_names: Vec<DpnsNameInfo>,

    /// DPNS labels this identity is currently contending for —
    /// names that are still in the contested-voting period and
    /// haven't been resolved (won or locked) yet.
    ///
    /// Caches only the label list. Contest metadata (contenders,
    /// votes, end time) changes throughout the voting period, so
    /// caching it would go stale quickly — the UI queries it fresh
    /// via `Sdk::get_non_resolved_dpns_contests_for_identity` when
    /// it needs the details. Resolved contests (won or locked)
    /// should migrate off this list and onto `dpns_names` on
    /// success.
    pub contested_dpns_names: Vec<String>,

    /// Wallet identifier (`SHA256(root_pub_key || chain_code)`) of
    /// the wallet that owns this identity, if known. Set during
    /// gap-limit scan and identity recovery.
    ///
    /// Denormalized — when the identity lives in the wallet bucket
    /// this is also the outer `BTreeMap` key. `None` means the identity
    /// lives in the out-of-wallet bucket (observed only).
    pub wallet_id: Option<[u8; 32]>,

    /// DashPay profile (display name, bio, avatar, public message)
    /// published via the DashPay data contract. `None` until the
    /// profile has been fetched or set.
    pub dashpay_profile: Option<DashPayProfile>,

    /// DashPay payment history keyed by transaction id (hex string).
    /// Each entry records a single Dash payment to or from a contact
    /// identity, with direction, amount, memo, and status.
    pub dashpay_payments: BTreeMap<String, PaymentEntry>,

    /// Incremental-sync high-water marks (`$createdAt` ms of the newest
    /// `contactRequest` fetched) per direction. `None` ⇒ never synced; the
    /// next sweep does a full fetch. Restored from the persister; a lost or
    /// too-low value just triggers one extra full re-fetch (ingest is a
    /// fixpoint), so restore must tolerate only under-shoot — never restore a
    /// value higher than the contact state justifies. See
    /// `docs/dashpay/SYNC_CORRECTNESS_SPEC.md` §4.1.
    pub high_water_received_ms: Option<u64>,
    /// High-water mark for the sent direction (`$ownerId == me`).
    pub high_water_sent_ms: Option<u64>,

    /// Cached **contact** profiles keyed by the contact's identity id —
    /// established contacts, pending incoming-request senders, and (later)
    /// ignored senders, independent of relationship state. Populated by
    /// `sync_contact_profiles`; public-data only (never `contactInfo`-derived).
    /// See `docs/dashpay/SYNC_CORRECTNESS_SPEC.md` §4.5.
    pub contact_profiles: BTreeMap<Identifier, ContactProfileEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::persister::{NoPlatformPersistence, WalletPersister};
    use dpp::identity::v0::IdentityV0;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    fn noop_persister() -> WalletPersister {
        WalletPersister::new([0u8; 32], Arc::new(NoPlatformPersistence))
    }

    fn create_test_identity() -> Identity {
        let identity_v0 = IdentityV0 {
            id: Identifier::from([1u8; 32]),
            public_keys: BTreeMap::new(),
            balance: 1000,
            revision: 1,
        };
        Identity::V0(identity_v0)
    }

    #[test]
    fn test_managed_identity_creation() {
        let identity = create_test_identity();
        let managed = ManagedIdentity::new(identity, 0);

        assert_eq!(managed.id(), Identifier::from([1u8; 32]));
        assert_eq!(managed.balance(), 1000);
        assert_eq!(managed.revision(), 1);
        assert_eq!(managed.last_updated_balance_block_time, None);
        assert_eq!(managed.last_synced_keys_block_time, None);
    }

    #[test]
    fn test_balance_block_time() {
        let identity = create_test_identity();
        let mut managed = ManagedIdentity::new(identity, 0);
        let p = noop_persister();

        let block_time = BlockTime::new(100000, 900000, 1234567890);
        managed.update_balance_block_time(block_time, &p);

        assert_eq!(managed.last_updated_balance_block_time, Some(block_time));
        assert_eq!(
            managed.last_updated_balance_block_time.unwrap().height,
            100000
        );
        assert_eq!(
            managed.last_updated_balance_block_time.unwrap().core_height,
            900000
        );
        assert_eq!(
            managed.last_updated_balance_block_time.unwrap().timestamp,
            1234567890
        );
    }

    #[test]
    fn test_keys_sync_block_time() {
        let identity = create_test_identity();
        let mut managed = ManagedIdentity::new(identity, 0);
        let p = noop_persister();

        let block_time = BlockTime::new(50000, 450000, 9876543210);
        managed.update_keys_sync_block_time(block_time, &p);

        assert_eq!(managed.last_synced_keys_block_time, Some(block_time));
        assert_eq!(managed.last_synced_keys_block_time.unwrap().height, 50000);
        assert_eq!(
            managed.last_synced_keys_block_time.unwrap().core_height,
            450000
        );
        assert_eq!(
            managed.last_synced_keys_block_time.unwrap().timestamp,
            9876543210
        );
    }

    #[test]
    fn test_needs_balance_update() {
        let identity = create_test_identity();
        let mut managed = ManagedIdentity::new(identity, 0);
        let p = noop_persister();

        // Never updated - needs update
        assert_eq!(managed.needs_balance_update(1000, 100), true);

        // Just updated
        let block_time = BlockTime::new(100, 900, 1000);
        managed.update_balance_block_time(block_time, &p);
        assert_eq!(managed.needs_balance_update(1050, 100), false);

        // Old update - needs update
        assert_eq!(managed.needs_balance_update(1200, 100), true);
    }

    #[test]
    fn test_needs_keys_sync() {
        let identity = create_test_identity();
        let mut managed = ManagedIdentity::new(identity, 0);
        let p = noop_persister();

        // Never synced - needs sync
        assert_eq!(managed.needs_keys_sync(1000, 100), true);

        // Just synced
        let block_time = BlockTime::new(100, 900, 1000);
        managed.update_keys_sync_block_time(block_time, &p);
        assert_eq!(managed.needs_keys_sync(1050, 100), false);

        // Old sync - needs sync
        assert_eq!(managed.needs_keys_sync(1200, 100), true);
    }

    #[test]
    fn test_auto_establish_on_sent_request() {
        let identity = create_test_identity();
        let mut managed = ManagedIdentity::new(identity, 0);
        let p = noop_persister();

        let contact_id = Identifier::from([2u8; 32]);
        let our_id = Identifier::from([1u8; 32]);

        // First, add an incoming request from the contact
        let incoming_request = ContactRequest::new(
            contact_id,
            our_id,
            0,
            0,
            0,
            vec![0u8; 96],
            100000,
            1234567890,
        );
        managed.add_incoming_contact_request(incoming_request, &p);

        // Verify it's in incoming requests
        assert_eq!(managed.incoming_contact_requests.len(), 1);
        assert_eq!(managed.established_contacts.len(), 0);

        // Now add a sent request to the same contact - should auto-establish
        let outgoing_request = ContactRequest::new(
            our_id,
            contact_id,
            0,
            0,
            0,
            vec![0u8; 96],
            100000,
            1234567891,
        );
        managed.add_sent_contact_request(outgoing_request, &p);

        // Verify contact was established
        assert_eq!(managed.incoming_contact_requests.len(), 0);
        assert_eq!(managed.sent_contact_requests.len(), 0);
        assert_eq!(managed.established_contacts.len(), 1);
        assert!(managed.established_contacts.contains_key(&contact_id));
    }

    #[test]
    fn test_auto_establish_on_incoming_request() {
        let identity = create_test_identity();
        let mut managed = ManagedIdentity::new(identity, 0);
        let p = noop_persister();

        let contact_id = Identifier::from([2u8; 32]);
        let our_id = Identifier::from([1u8; 32]);

        // First, add a sent request to the contact
        let outgoing_request = ContactRequest::new(
            our_id,
            contact_id,
            0,
            0,
            0,
            vec![0u8; 96],
            100000,
            1234567890,
        );
        managed.add_sent_contact_request(outgoing_request, &p);

        // Verify it's in sent requests
        assert_eq!(managed.sent_contact_requests.len(), 1);
        assert_eq!(managed.established_contacts.len(), 0);

        // Now add an incoming request from the same contact - should auto-establish
        let incoming_request = ContactRequest::new(
            contact_id,
            our_id,
            0,
            0,
            0,
            vec![0u8; 96],
            100000,
            1234567891,
        );
        managed.add_incoming_contact_request(incoming_request, &p);

        // Verify contact was established
        assert_eq!(managed.incoming_contact_requests.len(), 0);
        assert_eq!(managed.sent_contact_requests.len(), 0);
        assert_eq!(managed.established_contacts.len(), 1);
        assert!(managed.established_contacts.contains_key(&contact_id));
    }

    #[test]
    fn test_no_auto_establish_without_reciprocal() {
        let identity = create_test_identity();
        let mut managed = ManagedIdentity::new(identity, 0);
        let p = noop_persister();

        let contact_id = Identifier::from([2u8; 32]);
        let our_id = Identifier::from([1u8; 32]);

        // Add a sent request without a reciprocal incoming request
        let outgoing_request = ContactRequest::new(
            our_id,
            contact_id,
            0,
            0,
            0,
            vec![0u8; 96],
            100000,
            1234567890,
        );
        managed.add_sent_contact_request(outgoing_request, &p);

        // Verify it stays in sent requests
        assert_eq!(managed.sent_contact_requests.len(), 1);
        assert_eq!(managed.established_contacts.len(), 0);

        // Add an incoming request from a different contact
        let other_contact_id = Identifier::from([3u8; 32]);
        let incoming_request = ContactRequest::new(
            other_contact_id,
            our_id,
            0,
            0,
            0,
            vec![0u8; 96],
            100000,
            1234567891,
        );
        managed.add_incoming_contact_request(incoming_request, &p);

        // Verify both requests stay separate
        assert_eq!(managed.sent_contact_requests.len(), 1);
        assert_eq!(managed.incoming_contact_requests.len(), 1);
        assert_eq!(managed.established_contacts.len(), 0);
    }
}
