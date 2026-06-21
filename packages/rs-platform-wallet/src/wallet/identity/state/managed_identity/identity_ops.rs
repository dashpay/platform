//! Core identity operations for ManagedIdentity

use super::key_storage::{DpnsNameInfo, IdentityStatus};
use super::ManagedIdentity;
use crate::changeset::{IdentityChangeSet, IdentityEntry, IdentityKeyEntry, IdentityKeysChangeSet};
use crate::wallet::persister::WalletPersister;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::Identity;
use dpp::identity::{KeyID, TimestampMillis};
use dpp::prelude::Identifier;
use dpp::util::hash::ripemd160_sha256;
use std::collections::BTreeMap;

/// Compute the 20-byte RIPEMD160(SHA256) hash of the DPP public-key
/// bytes. Shared between `keys_snapshot_changeset` so the FFI surface
/// always carries a consistent pre-hashed form.
fn pubkey_hash_of(pub_key: &dpp::identity::IdentityPublicKey) -> [u8; 20] {
    let mut out = [0u8; 20];
    out.copy_from_slice(ripemd160_sha256(pub_key.data().as_slice()).as_slice());
    out
}

impl ManagedIdentity {
    /// Helper: produce an [`IdentityChangeSet`] containing a scalar-only
    /// [`IdentityEntry`] snapshot of `self`.
    ///
    /// Every scalar mutation method on `ManagedIdentity` calls this
    /// after mutating state so that the returned changeset faithfully
    /// describes the resulting identity. Public keys + private-key
    /// storage are snapshotted separately via
    /// [`Self::keys_snapshot_changeset`] so scalar mutations don't drag
    /// the full key payload through every persist.
    pub(crate) fn snapshot_changeset(&self) -> IdentityChangeSet {
        let mut cs = IdentityChangeSet::default();
        cs.identities
            .insert(self.id(), IdentityEntry::from_managed(self));
        cs
    }

    /// Helper: produce an [`IdentityKeysChangeSet`] containing one
    /// [`IdentityKeyEntry`] upsert per registered public key on this
    /// identity.
    ///
    /// Private-key bytes / derivation breadcrumbs no longer ride along
    /// here — `ManagedIdentity` doesn't carry `key_storage` anymore,
    /// so every emitted entry has `wallet_id == None` and
    /// `derivation_indices == None`. Callers that need the
    /// breadcrumb (e.g. registration / discovery) emit a dedicated
    /// `IdentityKeysChangeSet` themselves with the right
    /// `(wallet_id, identity_index, key_index)` pair.
    pub(crate) fn keys_snapshot_changeset(&self) -> IdentityKeysChangeSet {
        let identity_id = self.id();
        let mut upserts = BTreeMap::new();
        for (key_id, pub_key) in self.identity.public_keys() {
            upserts.insert(
                (identity_id, *key_id),
                IdentityKeyEntry {
                    identity_id,
                    key_id: *key_id,
                    public_key: pub_key.clone(),
                    public_key_hash: pubkey_hash_of(pub_key),
                    wallet_id: None,
                    derivation_indices: None,
                    // Watch-only snapshot — no secret rides this path.
                    private_key: None,
                },
            );
        }
        IdentityKeysChangeSet {
            upserts,
            removed: Default::default(),
        }
    }

    /// Create a new wallet-owned managed identity with its BIP-9 HD
    /// identity index. The resulting `identity_index` field is
    /// `Some(identity_index)` — use [`Self::new_out_of_wallet`] for
    /// observed identities that have no derivation context.
    pub fn new(identity: Identity, identity_index: u32) -> Self {
        Self {
            identity,
            identity_index: Some(identity_index),
            last_updated_balance_block_time: None,
            last_synced_keys_block_time: None,
            established_contacts: Default::default(),
            sent_contact_requests: Default::default(),
            incoming_contact_requests: Default::default(),
            ignored_senders: Default::default(),
            status: Default::default(),
            dpns_names: Vec::new(),
            contested_dpns_names: Vec::new(),
            wallet_id: None,
            dashpay_profile: None,
            dashpay_payments: BTreeMap::new(),
            high_water_received_ms: None,
            high_water_sent_ms: None,
            contact_profiles: BTreeMap::new(),
        }
    }

    /// Create a new out-of-wallet managed identity (observed read-only).
    ///
    /// Out-of-wallet identities don't belong to any local wallet, so
    /// `identity_index` is `None` and `wallet_id` is left as `None` —
    /// signing / derivation paths must guard on these and reject the
    /// out-of-wallet case explicitly.
    pub fn new_out_of_wallet(identity: Identity) -> Self {
        Self {
            identity,
            identity_index: None,
            last_updated_balance_block_time: None,
            last_synced_keys_block_time: None,
            established_contacts: Default::default(),
            sent_contact_requests: Default::default(),
            incoming_contact_requests: Default::default(),
            ignored_senders: Default::default(),
            status: Default::default(),
            dpns_names: Vec::new(),
            contested_dpns_names: Vec::new(),
            wallet_id: None,
            dashpay_profile: None,
            dashpay_payments: BTreeMap::new(),
            high_water_received_ms: None,
            high_water_sent_ms: None,
            contact_profiles: BTreeMap::new(),
        }
    }

    /// Set the DashPay profile for this identity.
    ///
    /// Persists the resulting changeset via `persister` and returns `()`.
    /// Pass `None` to clear the profile.
    ///
    /// TODO: This is `pub` transitionally — evo-tool's backend tasks
    /// still fetch/create DashPay profiles themselves and push results
    /// back here. `IdentityWallet::sync_profiles` now covers the
    /// happy path; once evo-tool's `platform_wallet_cache.rs` migrates
    /// off the direct setter, this should drop to `pub(crate)`.
    pub fn set_dashpay_profile(
        &mut self,
        profile: Option<crate::wallet::identity::DashPayProfile>,
        persister: &WalletPersister,
    ) {
        self.dashpay_profile = profile;
        let cs = self.snapshot_changeset();
        if let Err(e) = persister.store(cs.into()) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
    }

    /// Record a DashPay payment under its transaction id.
    ///
    /// If an entry with the same `tx_id` already exists, it is
    /// overwritten (last-write-wins) — the expected use case is
    /// updating the status field from `Pending` to `Confirmed` or
    /// `Failed` after broadcast.
    ///
    /// Persists the resulting changeset via `persister` and returns `()`.
    ///
    /// TODO: Same transitional `pub` as `set_dashpay_profile` — see
    /// that method's doc comment for rationale.
    pub fn record_dashpay_payment(
        &mut self,
        tx_id: String,
        entry: crate::wallet::identity::PaymentEntry,
        persister: &WalletPersister,
    ) -> Result<(), crate::changeset::PersistenceError> {
        self.dashpay_payments.insert(tx_id, entry);
        let cs = self.snapshot_changeset();
        // Returns the persist result instead of swallowing it. The
        // user-initiated send path (`send_payment`) propagates it so a
        // failed write surfaces in the UI rather than silently dropping a
        // Sent entry + memo that has no on-chain recovery. The self-healing
        // sweep callers (live recorder / reconcile of Received) log and
        // continue — the next sweep re-derives those from UTXOs.
        persister.store(cs.into())
    }

    /// All DashPay payments to or from `contact_id` (keyed by txid), newest
    /// first. Both `send_payment` and the receival recorder stamp
    /// `counterparty_id`, so this is the per-contact tx history without a
    /// separate tx→contact reverse-lookup table.
    pub fn payments_for_contact(
        &self,
        contact_id: &Identifier,
    ) -> Vec<(String, crate::wallet::identity::PaymentEntry)> {
        self.dashpay_payments
            .iter()
            .filter(|(_, p)| p.counterparty_id == contact_id)
            .map(|(tx_id, p)| (tx_id.clone(), p.clone()))
            .collect()
    }

    /// Get the identity ID
    pub fn id(&self) -> Identifier {
        self.identity.id()
    }

    /// Get the identity's balance
    pub fn balance(&self) -> u64 {
        self.identity.balance()
    }

    /// Get the identity's revision
    pub fn revision(&self) -> u64 {
        self.identity.revision()
    }

    /// Set the identity lifecycle status.
    ///
    /// Persists the resulting changeset via `persister` and returns `()`.
    pub fn set_status(&mut self, status: IdentityStatus, persister: &WalletPersister) {
        self.status = status;
        let cs = self.snapshot_changeset();
        if let Err(e) = persister.store(cs.into()) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
    }

    /// Add a DPNS name associated with this identity.
    ///
    /// Persists the resulting changeset via `persister` and returns `()`.
    pub fn add_dpns_name(&mut self, name: DpnsNameInfo, persister: &WalletPersister) {
        self.dpns_names.push(name);
        let cs = self.snapshot_changeset();
        if let Err(e) = persister.store(cs.into()) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
    }

    /// Append a contested DPNS label this identity is contending for.
    ///
    /// Dedup is enforced — the same label isn't added twice. When a
    /// contest resolves (won), the caller should move the label from
    /// `contested_dpns_names` onto `dpns_names` via a separate
    /// mutation; on lock / loss, the label just gets dropped. Both
    /// cases are out of scope for this method.
    ///
    /// Persists the resulting snapshot via `persister`.
    pub fn add_contested_dpns_name(&mut self, label: String, persister: &WalletPersister) {
        if self.contested_dpns_names.contains(&label) {
            return;
        }
        self.contested_dpns_names.push(label);
        let cs = self.snapshot_changeset();
        if let Err(e) = persister.store(cs.into()) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
    }

    /// Replace the contested-name list wholesale.
    ///
    /// Use this when a sync round pulls the canonical set of
    /// contested names from Platform — the merge-time dedup-append
    /// policy on `IdentityChangeSet` would otherwise accumulate
    /// stale labels (contests that resolved but still appear in
    /// the local cache). Emitting a full snapshot here + running
    /// the sync path on identity reapply bakes the authoritative
    /// set into state.
    pub fn set_contested_dpns_names(&mut self, labels: Vec<String>, persister: &WalletPersister) {
        self.contested_dpns_names = labels;
        let cs = self.snapshot_changeset();
        if let Err(e) = persister.store(cs.into()) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
    }

    /// Layer an `IdentityPublicKey` onto this identity's
    /// `public_keys` map and emit a single-key
    /// [`IdentityKeysChangeSet`] upsert carrying the full derivation
    /// breadcrumb.
    ///
    /// `wallet_id` and `(identity_index, key_index)` together let the
    /// client (iOS Keychain) re-derive the matching private key from
    /// the wallet seed at the DIP-9 identity authentication path.
    /// Pass `None` for a watch-only key the wallet didn't derive.
    pub fn add_key(
        &mut self,
        public_key: dpp::identity::IdentityPublicKey,
        derivation_breadcrumb: Option<crate::changeset::KeyDerivationBreadcrumb>,
        persister: &WalletPersister,
    ) {
        // Single-key form of [`Self::add_keys`] — one canonical
        // key-layering + changeset path so the two can't drift. This
        // entry point carries no secret (callers that have a verified
        // scalar go through `add_keys` directly), so `verified_scalar`
        // is `None`.
        self.add_keys(
            vec![crate::changeset::KeyWithBreadcrumb {
                key: public_key,
                breadcrumb: derivation_breadcrumb,
                verified_scalar: None,
            }],
            persister,
        );
    }

    /// Layer several `IdentityPublicKey`s onto this identity and emit ONE
    /// batched [`IdentityKeysChangeSet`] carrying each key's derivation
    /// breadcrumb (`Some((wallet_id, identity_index, key_index))`) or
    /// `None` for a watch-only key the wallet can't re-derive.
    ///
    /// The single-write batch form of [`Self::add_key`], used by discovery
    /// to materialize every re-derivable key of a freshly found identity in
    /// one persist round (rather than one round per key) and to carry the
    /// authoritative per-key breadcrumb set in a single changeset (no
    /// order-dependent watch-only-then-override). No-op on an empty list.
    pub fn add_keys(
        &mut self,
        keys: Vec<crate::changeset::KeyWithBreadcrumb>,
        persister: &WalletPersister,
    ) {
        use dpp::identity::accessors::IdentitySettersV0;

        if keys.is_empty() {
            return;
        }
        let identity_id = self.id();
        let mut current = self.identity.public_keys().clone();
        let mut keys_cs = IdentityKeysChangeSet::default();
        for crate::changeset::KeyWithBreadcrumb {
            key: public_key,
            breadcrumb,
            verified_scalar,
        } in keys
        {
            let key_id = public_key.id();
            let public_key_hash = pubkey_hash_of(&public_key);
            current.insert(key_id, public_key.clone());
            // Couple the carried scalar to the breadcrumb: a scalar is only
            // useful with its `(identity_index, key_index)`, and a client
            // stores the bytes only when both are present, so dropping a
            // stray scalar that arrived without a breadcrumb keeps the two
            // from ever diverging (and stops a verified secret from being
            // silently discarded downstream).
            let (wallet_id, derivation_indices, private_key) = match breadcrumb {
                Some((wallet_id, identity_index, key_index)) => (
                    Some(wallet_id),
                    Some(crate::changeset::IdentityKeyDerivationIndices {
                        identity_index,
                        key_index,
                    }),
                    verified_scalar,
                ),
                None => (None, None, None),
            };
            keys_cs.upserts.insert(
                (identity_id, key_id),
                IdentityKeyEntry {
                    identity_id,
                    key_id,
                    public_key,
                    public_key_hash,
                    wallet_id,
                    derivation_indices,
                    private_key,
                },
            );
        }
        self.identity.set_public_keys(current);
        let cs = crate::changeset::PlatformWalletChangeSet {
            identities: Some(self.snapshot_changeset()),
            identity_keys: Some(keys_cs),
            ..Default::default()
        };
        if let Err(e) = persister.store(cs) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
    }

    /// Stamp `disabled_at` on the public keys named by `key_ids` and
    /// emit a single-key [`IdentityKeysChangeSet`] upsert per affected
    /// key — the disable-side counterpart to [`Self::add_key`].
    ///
    /// Mirrors `add_key`'s persistence shape: the matching
    /// `IdentityPublicKey` records are mutated in place on the DPP
    /// `Identity` (so every signing / introspection path sees the
    /// disabled flag immediately) and a combined changeset is persisted
    /// so the client's `PersistentPublicKey.disabledAt` rows flip
    /// without a network re-fetch. Each emitted entry reuses the same
    /// `(wallet_id, identity_index, key_index)` derivation breadcrumb
    /// `add_key` carries, so the client re-derives idempotently and
    /// keeps the key's existing private-key linkage rather than
    /// dropping it on the upsert. Out-of-wallet identities (no
    /// derivation context) emit a breadcrumb-less entry, matching
    /// their watch-only state.
    ///
    /// `key_ids` not present on the identity are skipped (logged) and
    /// no changeset is emitted when nothing matched. `disabled_at` is
    /// the timestamp stamped on every matching key — callers pass the
    /// local broadcast time; the next Platform refresh reconciles it to
    /// the authoritative on-chain block time.
    ///
    /// Does **not** touch the identity revision — the caller bumps it
    /// alongside the update transition.
    pub fn disable_keys(
        &mut self,
        key_ids: &[KeyID],
        disabled_at: TimestampMillis,
        persister: &WalletPersister,
    ) {
        use dpp::identity::accessors::IdentitySettersV0;
        use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeySettersV0;

        let identity_id = self.id();

        // Reconstruct the derivation breadcrumb from the cached
        // wallet/identity slot. Wallet-owned identities carry both
        // halves; out-of-wallet ones have neither (and no private key
        // to preserve), so they fall through to a watch-only entry.
        let breadcrumb = match (self.wallet_id, self.identity_index) {
            (Some(wallet_id), Some(identity_index)) => Some((wallet_id, identity_index)),
            _ => None,
        };

        // Operate on a clone of the key map, mirroring `add_key`, then
        // write it back once all matching keys have been stamped.
        let mut keys = self.identity.public_keys().clone();
        let mut keys_cs = IdentityKeysChangeSet::default();

        for &key_id in key_ids {
            let Some(public_key) = keys.get_mut(&key_id) else {
                tracing::warn!(
                    identity = %identity_id,
                    key_id,
                    "disable_keys: key id not present on identity; skipping",
                );
                continue;
            };

            public_key.set_disabled_at(disabled_at);
            let public_key_hash = pubkey_hash_of(public_key);

            let (wallet_id, derivation_indices) = match breadcrumb {
                Some((wallet_id, identity_index)) => (
                    Some(wallet_id),
                    Some(crate::changeset::IdentityKeyDerivationIndices {
                        identity_index,
                        key_index: key_id,
                    }),
                ),
                None => (None, None),
            };

            keys_cs.upserts.insert(
                (identity_id, key_id),
                IdentityKeyEntry {
                    identity_id,
                    key_id,
                    public_key: public_key.clone(),
                    public_key_hash,
                    wallet_id,
                    derivation_indices,
                    // Disabling an already-materialized key carries no
                    // scalar — the client keeps its existing Keychain item.
                    private_key: None,
                },
            );
        }

        // Nothing matched — leave state untouched rather than churning a
        // no-op changeset.
        if keys_cs.upserts.is_empty() {
            return;
        }

        self.identity.set_public_keys(keys);

        let cs = crate::changeset::PlatformWalletChangeSet {
            identities: Some(self.snapshot_changeset()),
            identity_keys: Some(keys_cs),
            ..Default::default()
        };
        if let Err(e) = persister.store(cs) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changeset::{
        ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
    };
    use crate::wallet::identity::PaymentEntry;
    use crate::wallet::platform_wallet::WalletId;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::{Identity, IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    /// Persister that records every store so a test can inspect the exact
    /// changeset `add_keys` emits.
    #[derive(Default)]
    struct CapturingPersister {
        stores: Mutex<Vec<PlatformWalletChangeSet>>,
    }
    impl PlatformWalletPersistence for CapturingPersister {
        fn store(
            &self,
            _wallet_id: WalletId,
            changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            self.stores.lock().unwrap().push(changeset);
            Ok(())
        }
        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }
        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    fn key(id: KeyID) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: dpp::platform_value::BinaryData::new(vec![0x02; 33]),
            disabled_at: None,
        })
    }

    /// `add_keys` records each key's breadcrumb (or `None` for watch-only)
    /// in one batched changeset and lands every key in the DPP identity.
    /// Pins the materialization side of the imported-identity-signing fix.
    #[test]
    fn add_keys_emits_breadcrumbs_per_key() {
        let identity = Identity::V0(IdentityV0 {
            id: Identifier::from([1u8; 32]),
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        });
        let mut managed = ManagedIdentity::new(identity, 0);
        let wallet_id: WalletId = [0xAB; 32];
        let persister = std::sync::Arc::new(CapturingPersister::default());
        let p = WalletPersister::new(wallet_id, std::sync::Arc::clone(&persister) as _);

        // Key 0 is re-derivable (breadcrumb + verified scalar), key 1 is
        // watch-only (None).
        let scalar = zeroize::Zeroizing::new([0x11u8; 32]);
        managed.add_keys(
            vec![
                crate::changeset::KeyWithBreadcrumb {
                    key: key(0),
                    breadcrumb: Some((wallet_id, 7, 0)),
                    verified_scalar: Some(scalar.clone()),
                },
                crate::changeset::KeyWithBreadcrumb {
                    key: key(1),
                    breadcrumb: None,
                    verified_scalar: None,
                },
            ],
            &p,
        );

        // Both keys landed in the DPP identity.
        assert_eq!(managed.identity.public_keys().len(), 2);

        let stores = persister.stores.lock().unwrap();
        let upserts = &stores
            .last()
            .expect("a changeset was stored")
            .identity_keys
            .as_ref()
            .expect("identity_keys present")
            .upserts;
        let id = managed.id();
        assert_eq!(
            upserts[&(id, 0)].derivation_indices,
            Some(crate::changeset::IdentityKeyDerivationIndices {
                identity_index: 7,
                key_index: 0,
            }),
            "reproducible key carries its breadcrumb"
        );
        assert_eq!(upserts[&(id, 0)].wallet_id, Some(wallet_id));
        // The verified scalar is moved into the changeset entry so the
        // client persister stores it without re-deriving from a mnemonic.
        assert_eq!(
            upserts[&(id, 0)].private_key.as_deref(),
            Some(&*scalar),
            "reproducible key carries its verified scalar"
        );
        assert_eq!(
            upserts[&(id, 1)].derivation_indices,
            None,
            "watch-only key carries no breadcrumb"
        );
        assert_eq!(upserts[&(id, 1)].wallet_id, None);
        assert!(
            upserts[&(id, 1)].private_key.is_none(),
            "watch-only key carries no secret"
        );
    }

    /// An empty `add_keys` is a no-op — no changeset stored.
    #[test]
    fn add_keys_empty_is_noop() {
        let identity = Identity::V0(IdentityV0 {
            id: Identifier::from([1u8; 32]),
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        });
        let mut managed = ManagedIdentity::new(identity, 0);
        let persister = std::sync::Arc::new(CapturingPersister::default());
        let p = WalletPersister::new([0xAB; 32], std::sync::Arc::clone(&persister) as _);
        managed.add_keys(Vec::new(), &p);
        assert!(
            persister.stores.lock().unwrap().is_empty(),
            "empty add_keys stores nothing"
        );
    }

    /// A scalar that arrives WITHOUT a breadcrumb is dropped, never carried:
    /// the entry stays watch-only (no indices, no secret). Pins the
    /// scalar/breadcrumb coupling so a verified secret can't reach the client
    /// without the `(identity_index, key_index)` it needs to be stored.
    #[test]
    fn add_keys_drops_scalar_without_breadcrumb() {
        let identity = Identity::V0(IdentityV0 {
            id: Identifier::from([1u8; 32]),
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        });
        let mut managed = ManagedIdentity::new(identity, 0);
        let persister = std::sync::Arc::new(CapturingPersister::default());
        let p = WalletPersister::new([0xAB; 32], std::sync::Arc::clone(&persister) as _);

        managed.add_keys(
            vec![crate::changeset::KeyWithBreadcrumb {
                key: key(0),
                breadcrumb: None,
                verified_scalar: Some(zeroize::Zeroizing::new([0x22u8; 32])),
            }],
            &p,
        );

        let stores = persister.stores.lock().unwrap();
        let upserts = &stores
            .last()
            .expect("a changeset was stored")
            .identity_keys
            .as_ref()
            .expect("identity_keys present")
            .upserts;
        let entry = &upserts[&(managed.id(), 0)];
        assert_eq!(entry.derivation_indices, None);
        assert_eq!(entry.wallet_id, None);
        assert!(
            entry.private_key.is_none(),
            "a scalar without a breadcrumb must be dropped, not carried"
        );
    }

    #[test]
    fn payments_for_contact_filters_by_counterparty() {
        let identity = Identity::V0(IdentityV0 {
            id: Identifier::from([1u8; 32]),
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        });
        let mut managed = ManagedIdentity::new(identity, 0);
        let alice = Identifier::from([0xAA; 32]);
        let bob = Identifier::from([0xBB; 32]);

        managed
            .dashpay_payments
            .insert("t1".into(), PaymentEntry::new_sent(alice, 100, None));
        managed
            .dashpay_payments
            .insert("t2".into(), PaymentEntry::new_received(bob, 200, None));
        managed
            .dashpay_payments
            .insert("t3".into(), PaymentEntry::new_sent(alice, 300, None));

        let for_alice = managed.payments_for_contact(&alice);
        assert_eq!(for_alice.len(), 2, "both sent payments to alice");
        assert!(for_alice.iter().all(|(_, p)| p.counterparty_id == alice));
        assert_eq!(managed.payments_for_contact(&bob).len(), 1);
        assert_eq!(
            managed
                .payments_for_contact(&Identifier::from([0xCC; 32]))
                .len(),
            0,
            "unknown contact has no payments"
        );
    }
}
