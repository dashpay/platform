//! Managed identity that combines a Platform Identity with wallet-specific metadata
//!
//! This module provides the `ManagedIdentity` struct which wraps a Platform Identity
//! with additional metadata for wallet management.

mod contact_requests;
mod contacts;
mod dashpay;
mod identity_ops;
mod sync;

pub use dashpay::DashPayState;

// `block_time` + `key_storage` moved to `crate::wallet::identity::types`.
// Re-export so every `impl ManagedIdentity` block below keeps working
// unchanged and external users can still reach them through the old
// `state::managed_identity::*` path.
pub use crate::wallet::identity::types::block_time::{self, BlockTime};
pub use crate::wallet::identity::types::key_storage::{
    self, DpnsNameInfo, IdentityStatus, KeyStorage, PrivateKeyData,
};

use dpp::identity::Identity;

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
    /// the inner BTreeMap key. `None` for out-of-wallet identities; they
    /// have no HD-derivation context.
    pub identity_index: Option<u32>,

    /// Last block time when balance was updated for this identity
    pub last_updated_balance_block_time: Option<BlockTime>,

    /// Last block time when keys were synced for this identity
    pub last_synced_keys_block_time: Option<BlockTime>,

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

    /// DashPay social state layered on this identity: contacts, contact
    /// requests, profile, payments, sync cursors, deferred crypto. See
    /// [`DashPayState`] for the per-field contracts.
    ///
    /// Module-private on purpose: reads go through [`Self::dashpay`],
    /// invariant-carrying mutations through the methods on this type,
    /// and open-tier cache writes through the `*_mut` accessors. Keeping
    /// the field sealed also rules out whole-value replacement
    /// (`mem::take` / reassignment), which would silently wipe the
    /// guarded relationship maps and sync cursors.
    dashpay: DashPayState,
}

impl ManagedIdentity {
    /// Read access to this identity's DashPay social state.
    pub fn dashpay(&self) -> &DashPayState {
        &self.dashpay
    }

    /// Mutable access to the DashPay profile cache.
    ///
    /// Replay/restore surface: bypasses persistence on purpose (the
    /// changeset being applied is already durable). Live mutations that
    /// must persist go through [`Self::set_dashpay_profile`].
    pub fn dashpay_profile_mut(&mut self) -> &mut Option<crate::wallet::identity::DashPayProfile> {
        &mut self.dashpay.profile
    }

    /// Mutable access to the DashPay payment-history cache.
    ///
    /// Replay/restore surface: bypasses persistence and the
    /// rollback-on-persist-failure contract of
    /// [`Self::record_dashpay_payment`], which live mutations must use.
    pub fn dashpay_payments_mut(
        &mut self,
    ) -> &mut std::collections::BTreeMap<String, crate::wallet::identity::PaymentEntry> {
        &mut self.dashpay.payments
    }

    /// Mutable access to the per-contact sent-payment sweep table digests.
    ///
    /// In-memory only — never persisted; see the field docs on
    /// [`DashPayState::sent_payment_reconcile_swept_table`] for why the guard
    /// is a digest of the scanned table rather than a flag or a height.
    pub fn dashpay_sent_payment_reconcile_swept_table_mut(
        &mut self,
    ) -> &mut std::collections::BTreeMap<dpp::prelude::Identifier, [u8; 32]> {
        &mut self.dashpay.sent_payment_reconcile_swept_table
    }

    /// Mutable access to the cached contact profiles.
    ///
    /// Replay/restore surface: bypasses persistence on purpose (the
    /// changeset being applied is already durable). The live writer is
    /// the profile sync sweep, which persists a snapshot after writing.
    pub fn dashpay_contact_profiles_mut(
        &mut self,
    ) -> &mut std::collections::BTreeMap<
        dpp::prelude::Identifier,
        crate::wallet::identity::ContactProfileEntry,
    > {
        &mut self.dashpay.contact_profiles
    }

    /// Mutable access to the per-session contact-rescan guard set.
    ///
    /// In-memory only — never persisted; see the field docs on
    /// [`DashPayState::rescan_triggered`] for the self-healing contract.
    pub fn dashpay_rescan_triggered_mut(
        &mut self,
    ) -> &mut std::collections::BTreeSet<dpp::prelude::Identifier> {
        &mut self.dashpay.rescan_triggered
    }

    /// Mutable access to the deferred contact-crypto queue.
    ///
    /// The queue's dedup invariant (≤ 1 entry per
    /// `(owner, contact, kind)`) lives in
    /// [`upsert_pending_contact_crypto`](crate::changeset::upsert_pending_contact_crypto) —
    /// callers inserting entries must go through it.
    pub fn dashpay_pending_contact_crypto_mut(
        &mut self,
    ) -> &mut Vec<crate::changeset::PendingContactCrypto> {
        &mut self.dashpay.pending_contact_crypto
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::identity::ContactRequest;
    use crate::wallet::persister::{NoPlatformPersistence, WalletPersister};
    use dpp::identity::v0::IdentityV0;
    use dpp::prelude::Identifier;
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
        managed
            .add_incoming_contact_request(incoming_request, &p)
            .expect("setup persists");

        // Verify it's in incoming requests
        assert_eq!(managed.dashpay.incoming_contact_requests.len(), 1);
        assert_eq!(managed.dashpay.established_contacts.len(), 0);

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
        managed
            .add_sent_contact_request(outgoing_request, &p)
            .expect("setup persists");

        // Verify contact was established
        assert_eq!(managed.dashpay.incoming_contact_requests.len(), 0);
        assert_eq!(managed.dashpay.sent_contact_requests.len(), 0);
        assert_eq!(managed.dashpay.established_contacts.len(), 1);
        assert!(managed
            .dashpay
            .established_contacts
            .contains_key(&contact_id));
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
        managed
            .add_sent_contact_request(outgoing_request, &p)
            .expect("setup persists");

        // Verify it's in sent requests
        assert_eq!(managed.dashpay.sent_contact_requests.len(), 1);
        assert_eq!(managed.dashpay.established_contacts.len(), 0);

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
        managed
            .add_incoming_contact_request(incoming_request, &p)
            .expect("setup persists");

        // Verify contact was established
        assert_eq!(managed.dashpay.incoming_contact_requests.len(), 0);
        assert_eq!(managed.dashpay.sent_contact_requests.len(), 0);
        assert_eq!(managed.dashpay.established_contacts.len(), 1);
        assert!(managed
            .dashpay
            .established_contacts
            .contains_key(&contact_id));
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
        managed
            .add_sent_contact_request(outgoing_request, &p)
            .expect("setup persists");

        // Verify it stays in sent requests
        assert_eq!(managed.dashpay.sent_contact_requests.len(), 1);
        assert_eq!(managed.dashpay.established_contacts.len(), 0);

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
        managed
            .add_incoming_contact_request(incoming_request, &p)
            .expect("setup persists");

        // Verify both requests stay separate
        assert_eq!(managed.dashpay.sent_contact_requests.len(), 1);
        assert_eq!(managed.dashpay.incoming_contact_requests.len(), 1);
        assert_eq!(managed.dashpay.established_contacts.len(), 0);
    }

    #[test]
    fn test_disable_keys_stamps_disabled_at() {
        use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
        use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
        use dpp::platform_value::BinaryData;

        let make_key = |id: u32| {
            IdentityPublicKey::V0(IdentityPublicKeyV0 {
                id,
                key_type: KeyType::ECDSA_SECP256K1,
                purpose: Purpose::AUTHENTICATION,
                security_level: SecurityLevel::HIGH,
                contract_bounds: None,
                read_only: false,
                data: BinaryData::new(vec![0x02; 33]),
                disabled_at: None,
            })
        };

        let mut identity = create_test_identity();
        let mut keys = BTreeMap::new();
        keys.insert(0, make_key(0));
        keys.insert(1, make_key(1));
        identity.set_public_keys(keys);

        let mut managed = ManagedIdentity::new(identity, 0);
        let p = noop_persister();

        // Disable key 1 plus a non-existent key 9 (must be skipped).
        managed.disable_keys(&[1, 9], 1_700_000_000, &p);

        let pk0 = managed.identity.get_public_key_by_id(0).unwrap();
        let pk1 = managed.identity.get_public_key_by_id(1).unwrap();
        assert_eq!(pk0.disabled_at(), None, "untouched key must stay enabled");
        assert_eq!(
            pk1.disabled_at(),
            Some(1_700_000_000),
            "targeted key must be stamped disabled"
        );
        // The skipped key id must not have materialized a phantom row.
        assert!(
            managed.identity.get_public_key_by_id(9).is_none(),
            "non-existent key id must not be created"
        );
    }
}
