//! Core identity operations for ManagedIdentity

use super::key_storage::{DpnsFetch, DpnsNameInfo, IdentityStatus};
use super::ManagedIdentity;
use crate::changeset::{
    IdentityChangeSet, IdentityEntry, IdentityKeyEntry, IdentityKeysChangeSet, KeyWithBreadcrumb,
    PersistenceError,
};
use crate::wallet::persister::WalletPersister;
use crate::BlockTime;
use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dpp::identity::identity_public_key::accessors::v0::{
    IdentityPublicKeyGettersV0, IdentityPublicKeySettersV0,
};
use dpp::identity::identity_public_key::accessors::v1::IdentityPublicKeyGettersV1;
use dpp::identity::{Identity, IdentityPublicKey};
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
    /// Private-key bytes / derivation breadcrumbs do not ride along
    /// here — `ManagedIdentity` carries no `key_storage`,
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
            pending_balance_snapshot: None,
            last_synced_keys_block_time: None,
            status: Default::default(),
            dpns_names: Vec::new(),
            contested_dpns_names: Vec::new(),
            wallet_id: None,
            dashpay: Default::default(),
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
            pending_balance_snapshot: None,
            last_synced_keys_block_time: None,
            status: Default::default(),
            dpns_names: Vec::new(),
            contested_dpns_names: Vec::new(),
            wallet_id: None,
            dashpay: Default::default(),
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
        self.dashpay.profile = profile;
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
        // Insert, snapshot, persist — but roll the change back if the store
        // fails so in-memory state stays equal to the persisted state.
        // Otherwise a caller's `contains_key` retry guard sees the
        // un-persisted entry and skips the next sweep's re-attempt, permanently
        // dropping a Sent entry + memo that has no on-chain recovery (the
        // self-healing sweep callers re-derive Received from UTXOs next pass).
        // A failed overwrite restores the previous entry rather than deleting
        // it. Either way the persist result is returned, not swallowed: the
        // user-initiated send path (`send_payment`) surfaces it in the UI.
        let previous = self.dashpay.payments.insert(tx_id.clone(), entry.clone());
        let mut cs: crate::changeset::PlatformWalletChangeSet = self.snapshot_changeset().into();
        // Ride the changed row on `dashpay_payments_overlay` as well: the
        // identity snapshot above carries the FULL payments map (blob-style
        // persisters overwrite the whole entry), so a delta-style persister
        // projecting the snapshot would replay the identity's complete
        // history on every recorded payment — unbounded per-call work as
        // history grows, against the `PlatformWalletPersistence` bounded-
        // work guidance. The single-row overlay is the bounded carrier
        // those persisters (the FFI vtable) project instead; every payment
        // mutation funnels through this method, so the overlay sees every
        // write.
        cs.dashpay_payments_overlay = Some(BTreeMap::from([(
            self.id(),
            BTreeMap::from([(tx_id.clone(), entry)]),
        )]));
        if let Err(e) = persister.store(cs) {
            match previous {
                Some(prev) => {
                    self.dashpay.payments.insert(tx_id, prev);
                }
                None => {
                    self.dashpay.payments.remove(&tx_id);
                }
            }
            return Err(e);
        }
        Ok(())
    }

    /// All DashPay payments to or from `contact_id` (keyed by txid), newest
    /// first. Both `send_payment` and the receival recorder stamp
    /// `counterparty_id`, so this is the per-contact tx history without a
    /// separate tx→contact reverse-lookup table.
    pub fn payments_for_contact(
        &self,
        contact_id: &Identifier,
    ) -> Vec<(String, crate::wallet::identity::PaymentEntry)> {
        self.dashpay
            .payments
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

    /// Merge fetched DPNS names into the list, skipping labels already
    /// present, and persist the result as one snapshot. Returns how many
    /// names were added; nothing is persisted when that is zero.
    ///
    /// Use this for a batch instead of calling [`Self::add_dpns_name`] per
    /// name: every persisted snapshot is read downstream as the identity's
    /// complete owned set, so a snapshot per name makes the first one look
    /// like the others were lost. Add-only — DPNS queries are capped by a
    /// limit, so a label missing from a fetch is not proof it left.
    pub fn merge_dpns_names(
        &mut self,
        names: impl IntoIterator<Item = DpnsNameInfo>,
        persister: &WalletPersister,
    ) -> u32 {
        let mut merged = self.dpns_names.clone();
        let mut added = 0u32;
        for name in names {
            if merged.iter().any(|existing| existing.label == name.label) {
                continue;
            }
            merged.push(name);
            added += 1;
        }
        if added > 0 {
            self.set_dpns_names(merged, persister);
        }
        added
    }

    /// Apply one DPNS username fetch for this identity; returns how many
    /// labels are new.
    ///
    /// On a [`DpnsFetch::Complete`] fetch (the identity's whole owned set)
    /// of an identity the wallet only watches (no `wallet_id`) the list is
    /// replaced with it — departed names drop out, an empty result clears
    /// the list — while labels already known keep their `acquired_at`, and
    /// nothing is persisted when the list is unchanged.
    ///
    /// A wallet-owned identity only ever merges new labels
    /// ([`Self::merge_dpns_names`]): the DPNS marketplace sweep discovers its
    /// departures from this list and classifies them (sale, transfer,
    /// deletion), so a username fetch must not remove a label first. A
    /// [`DpnsFetch::Partial`] fetch cannot prove that a missing label left,
    /// so it only merges too. Either way one snapshot at most.
    pub fn apply_fetched_dpns_names(
        &mut self,
        names: Vec<DpnsNameInfo>,
        fetch: DpnsFetch,
        persister: &WalletPersister,
    ) -> u32 {
        if fetch == DpnsFetch::Partial || self.wallet_id.is_some() {
            return self.merge_dpns_names(names, persister);
        }
        // Known labels still owned, in their existing order, then new ones.
        let mut next: Vec<DpnsNameInfo> = self
            .dpns_names
            .iter()
            .filter(|known| names.iter().any(|name| name.label == known.label))
            .cloned()
            .collect();
        let mut added = 0u32;
        for name in names {
            if next.iter().any(|existing| existing.label == name.label) {
                continue;
            }
            next.push(name);
            added += 1;
        }
        if next != self.dpns_names {
            self.set_dpns_names(next, persister);
        }
        added
    }

    /// Replace the DPNS-name list wholesale.
    ///
    /// Use this when a sync round (or a confirmed sale/transfer) has the
    /// canonical set of names owned by this identity. `IdentityChangeSet::merge`
    /// and replay both treat this field as a complete last-write-wins
    /// snapshot, so names that left the identity (sold / transferred
    /// away) are removed, including by an empty snapshot — the same
    /// policy as [`Self::set_contested_dpns_names`].
    pub fn set_dpns_names(&mut self, names: Vec<DpnsNameInfo>, persister: &WalletPersister) {
        self.dpns_names = names;
        let cs = self.snapshot_changeset();
        if let Err(e) = persister.store(cs.into()) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
    }

    /// Remove one DPNS name by label (the sold / transferred-away case).
    ///
    /// No-op (no changeset emitted) when the label isn't present.
    pub fn remove_dpns_name(&mut self, label: &str, persister: &WalletPersister) {
        let before = self.dpns_names.len();
        self.dpns_names.retain(|n| n.label != label);
        if self.dpns_names.len() == before {
            return;
        }
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
    /// contested names from Platform. `IdentityChangeSet::merge` and replay
    /// both treat this field as a complete last-write-wins snapshot so
    /// resolved contests are removed, including by an empty snapshot.
    pub fn set_contested_dpns_names(&mut self, labels: Vec<String>, persister: &WalletPersister) {
        self.contested_dpns_names = labels;
        let cs = self.snapshot_changeset();
        if let Err(e) = persister.store(cs.into()) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
    }

    /// Fold an identity fetched from Platform, at `block_time` (its proof's
    /// block), into the cached one.
    ///
    /// The balance is ordered by proof height, not by revision (it changes
    /// without a revision bump), as every verified balance is: the fetch's
    /// balance replaces the cache's only when its block is above the cache's
    /// balance watermark, and is then queued for the next store (see
    /// `store_keys_and_balance`).
    ///
    /// A key's limits are not ordered by revision either (a limits update
    /// claims none), but they only ever rise, so each shared key keeps the
    /// higher of the two (see `with_raised_limits`) at any revision.
    ///
    /// A fetch at the cache's revision or above is otherwise taken whole,
    /// except that a key the cache holds disabled stays disabled (a disable
    /// is permanent on Platform). Revisions only grow on Platform, so a fetch
    /// at a lower revision was answered before an update the cache already
    /// holds (one applied locally after it landed, or a newer refresh, from a
    /// node that lags behind): the cache keeps its revision and key set; it
    /// takes only a disable the fetch shows (the fetch's timestamp is
    /// Platform's own) and a raised limit. Returns whether it was taken
    /// whole.
    ///
    /// A fetch at a lower revision whose keys are the cache's is taken whole
    /// too (see `misses_no_update`): it lacks no update the cache holds, so
    /// the cache's revision is the wrong one. A base build's post-broadcast
    /// bump (`cached + 1`) left it one past the chain when a fetch had
    /// already taken the update.
    ///
    /// Only the in-memory identity changes here; the caller persists.
    pub fn take_fetched_identity(&mut self, fetched: &Identity, block_time: BlockTime) -> bool {
        let balance = self.identity.balance();
        let cached = self.identity.public_keys().clone();
        let taken = takes_fetched_whole(&self.identity, fetched);
        let mut keys = if taken {
            self.identity = fetched.clone();
            self.identity.set_balance(balance);
            fetched.public_keys().clone()
        } else {
            cached.clone()
        };
        for (key_id, key) in keys.iter_mut() {
            if let Some(fetched_key) = fetched.public_keys().get(key_id) {
                if let Some(folded) = fold_fetched_key(cached.get(key_id), fetched_key, taken) {
                    *key = folded;
                }
            }
        }
        self.identity.set_public_keys(keys);
        self.take_fetched_balance(fetched.balance(), block_time);
        taken
    }

    /// Fold `fetched` (see [`Self::take_fetched_identity`]) and store the row
    /// of every key the fold adds or changes (a disable, a raised limit, a
    /// key added at a newer revision, on this device or another) with the
    /// identity's scalar snapshot as one changeset, so the stored key set
    /// matches the stored revision after a restart.
    ///
    /// Used where a fetch comes without derived breadcrumbs (refresh), so the
    /// rows carry none. Every persister keeps the breadcrumb a row already
    /// holds when an upsert carries none, and a load or discovery, which
    /// verify a key against the seed, give a new key its breadcrumb.
    ///
    /// A failed store is returned; memory holds the fold.
    pub fn store_fetched_identity(
        &mut self,
        fetched: &Identity,
        block_time: BlockTime,
        persister: &WalletPersister,
    ) -> Result<(), PersistenceError> {
        let before = self.identity.public_keys().clone();
        self.take_fetched_identity(fetched, block_time);
        let identity_id = self.id();
        let mut keys_cs = IdentityKeysChangeSet::default();
        for (key_id, key) in self.identity.public_keys() {
            if before.get(key_id) == Some(key) {
                continue;
            }
            keys_cs.upserts.insert(
                (identity_id, *key_id),
                IdentityKeyEntry {
                    identity_id,
                    key_id: *key_id,
                    public_key: key.clone(),
                    public_key_hash: pubkey_hash_of(key),
                    wallet_id: None,
                    derivation_indices: None,
                },
            );
        }
        self.store_keys_and_balance(keys_cs, persister)
    }

    /// Fold `fetched` (see [`Self::take_fetched_identity`]) with the key
    /// `decisions` derived for it, and store the folded keys with their
    /// breadcrumbs and the identity's scalar snapshot as one changeset (see
    /// `store_keys_and_balance`). Used where a fetch comes with derived
    /// breadcrumbs (loading and discovery); an identity added from this
    /// fetch just before (see `IdentityManager::add_identity_unstored`)
    /// reaches the store in the same changeset as its keys. A failed store
    /// is returned; memory holds the fold.
    pub fn store_fetched_identity_keys(
        &mut self,
        fetched: &Identity,
        decisions: Vec<KeyWithBreadcrumb>,
        block_time: BlockTime,
        persister: &WalletPersister,
    ) -> Result<(), PersistenceError> {
        let decisions = self.key_decisions_for_fetch(fetched, decisions);
        self.take_fetched_identity(fetched, block_time);
        let keys_cs = self.layer_keys(decisions);
        self.store_keys_and_balance(keys_cs, persister)
    }

    /// The key decisions to store for `decisions` built from `fetched`, each
    /// key folded as [`Self::take_fetched_identity`] folds it, so storing its
    /// breadcrumb (a key's derivation never changes) never takes back a
    /// disable or a raised limit the cache holds. For a fetch at the cache's
    /// revision or above that is the fetched key, disabled when the cache's
    /// copy is. For an older fetch it is the cache's copy, disabled when the
    /// fetch shows a disable; either way each limit is the higher of the two.
    /// A key the cache lacks is left out of an older fetch's decisions, since
    /// an older answer cannot show whether a later update disabled it, and a
    /// fetch at least as new will bring it. A lower-revision fetch with the
    /// cache's keys, which the fold takes whole, gets the same decisions
    /// either way.
    fn key_decisions_for_fetch(
        &self,
        fetched: &Identity,
        decisions: Vec<KeyWithBreadcrumb>,
    ) -> Vec<KeyWithBreadcrumb> {
        let taken = fetched.revision() >= self.identity.revision();
        decisions
            .into_iter()
            .filter_map(|mut decision| {
                let cached = self.identity.public_keys().get(&decision.key.id());
                decision.key = fold_fetched_key(cached, &decision.key, taken)?;
                Some(decision)
            })
            .collect()
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
    ) -> Result<(), crate::changeset::PersistenceError> {
        // Single-key form of [`Self::add_keys`] — one canonical
        // key-layering + changeset path so the two can't drift.
        self.add_keys(
            vec![crate::changeset::KeyWithBreadcrumb {
                key: public_key,
                breadcrumb: derivation_breadcrumb,
            }],
            persister,
        )
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
        keys: Vec<KeyWithBreadcrumb>,
        persister: &WalletPersister,
    ) -> Result<(), PersistenceError> {
        if keys.is_empty() {
            return Ok(());
        }
        self.store_keys_with_snapshot(keys, persister)
    }

    /// Layer `keys` in memory and store them with the identity's scalar
    /// snapshot as one changeset, even when `keys` is empty (the snapshot
    /// alone, e.g. a landed revision). The store's failure is returned.
    pub fn store_keys_with_snapshot(
        &mut self,
        keys: Vec<KeyWithBreadcrumb>,
        persister: &WalletPersister,
    ) -> Result<(), PersistenceError> {
        let keys_cs = self.layer_keys(keys);
        let cs = crate::changeset::PlatformWalletChangeSet {
            identities: Some(self.snapshot_changeset()),
            identity_keys: Some(keys_cs),
            ..Default::default()
        };
        // Surface the persist failure — these rows carry the per-key
        // derivation breadcrumb + verified scalar that make an imported /
        // restored identity signable. Swallowing a failed store here would
        // leave the keys in memory but absent from the client store, so the
        // identity comes back watch-only after restart with no signal.
        persister.store(cs)?;
        Ok(())
    }

    /// Layer `keys` onto this identity in memory, returning their key rows.
    fn layer_keys(&mut self, keys: Vec<KeyWithBreadcrumb>) -> IdentityKeysChangeSet {
        let identity_id = self.id();
        let mut current = self.identity.public_keys().clone();
        let mut keys_cs = IdentityKeysChangeSet::default();
        for KeyWithBreadcrumb {
            key: public_key,
            breadcrumb,
        } in keys
        {
            let key_id = public_key.id();
            let public_key_hash = pubkey_hash_of(&public_key);
            current.insert(key_id, public_key.clone());
            let (wallet_id, derivation_indices) = match breadcrumb {
                Some((wallet_id, identity_index, key_index)) => (
                    Some(wallet_id),
                    Some(crate::changeset::IdentityKeyDerivationIndices {
                        identity_index,
                        key_index,
                    }),
                ),
                None => (None, None),
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
                },
            );
        }
        self.identity.set_public_keys(current);
        keys_cs
    }

    /// Replace one public key of the identity with `public_key`, the key as it is stored
    /// after a key limits update, and emit a single-key [`IdentityKeysChangeSet`] upsert for
    /// it: the limits-side counterpart to [`Self::add_key`] and [`Self::disable_keys`].
    ///
    /// The key is layered by id, so a key the identity did not hold is added. The entry carries
    /// no derivation breadcrumb: a limits update changes nothing about where the key came from,
    /// and the managed identity does not know per key whether it was derived from the wallet
    /// seed or supplied from outside. Every persister keeps the breadcrumb it already holds for
    /// the key when an upsert arrives without one, so the private-key linkage survives the
    /// upsert; inventing coordinates here would overwrite it for an external key.
    ///
    /// Does **not** touch the identity revision: a key limits update claims none.
    pub fn replace_key(
        &mut self,
        public_key: dpp::identity::IdentityPublicKey,
        persister: &WalletPersister,
    ) -> Result<(), crate::changeset::PersistenceError> {
        let identity_id = self.id();
        let key_id = public_key.id();
        let public_key_hash = pubkey_hash_of(&public_key);

        let mut keys = self.identity.public_keys().clone();
        keys.insert(key_id, public_key.clone());
        self.identity.set_public_keys(keys);

        let mut keys_cs = IdentityKeysChangeSet::default();
        keys_cs.upserts.insert(
            (identity_id, key_id),
            IdentityKeyEntry {
                identity_id,
                key_id,
                public_key,
                public_key_hash,
                wallet_id: None,
                derivation_indices: None,
            },
        );
        let cs = crate::changeset::PlatformWalletChangeSet {
            identities: Some(self.snapshot_changeset()),
            identity_keys: Some(keys_cs),
            ..Default::default()
        };
        persister.store(cs)?;
        Ok(())
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

/// Whether `fetched` is folded into `cached` as an answer at least as new
/// (see [`ManagedIdentity::take_fetched_identity`]): at the cache's revision
/// or above, or below it while missing no update the cache holds.
fn takes_fetched_whole(cached: &Identity, fetched: &Identity) -> bool {
    fetched.revision() >= cached.revision()
        || misses_no_update(cached.public_keys(), fetched.public_keys())
}

/// Whether a fetch with `fetched` keys, at a lower revision than a cache
/// with `cached` keys, lacks no update the cache holds: the two key sets are
/// the same, key for key and field for field, `disabled_at` included, the
/// limits aside.
///
/// Only an identity update moves an identity's revision, and Drive takes
/// one only when it adds or disables at least one key. An added key has a
/// new id (keys are never removed); a disable sets `disabled_at` to the
/// block's time, later than any earlier disable of that key. A limits
/// update claims no revision, and a masternode identity's keys change with
/// Core, at revision 0 (it has no MASTER key to sign an update with). So
/// two key sets at different revisions always differ, and a lower revision
/// with the cache's keys means the cache's revision is wrong.
///
/// A disable the cache stamped with the local clock (a landed update's
/// placeholder) differs from Platform's time, so such a cache is not taken
/// back on the first fetch: that fetch replaces the stamp with Platform's
/// time (see `with_disable_from`), and the next one takes its revision.
fn misses_no_update(
    cached: &BTreeMap<KeyID, IdentityPublicKey>,
    fetched: &BTreeMap<KeyID, IdentityPublicKey>,
) -> bool {
    let without_limits = |key: &IdentityPublicKey| key.clone().with_limits(None, None);
    cached.len() == fetched.len()
        && cached
            .iter()
            .zip(fetched)
            .all(|((cached_id, cached), (fetched_id, fetched))| {
                cached_id == fetched_id && without_limits(cached) == without_limits(fetched)
            })
}

/// How a fetched key lands in the cache, the one rule the identity fold and
/// its key decisions share. From a fetch taken whole (`taken`) it is the
/// fetched key, disabled when the cache's copy is (a disable is permanent
/// on Platform; Platform's timestamp first). From an older fetch it is the
/// cache's copy with the fetch's disable, or nothing for a key the cache
/// lacks. Either way each limit is the higher of the two copies'.
fn fold_fetched_key(
    cached: Option<&IdentityPublicKey>,
    fetched: &IdentityPublicKey,
    taken: bool,
) -> Option<IdentityPublicKey> {
    if !taken {
        return cached
            .map(|cached| with_raised_limits(with_disable_from(cached, fetched), fetched));
    }
    let mut key = fetched.clone();
    let Some(cached) = cached else {
        return Some(key);
    };
    if key.disabled_at().is_none() {
        if let Some(disabled_at) = cached.disabled_at() {
            key.set_disabled_at(disabled_at);
        }
    }
    Some(with_raised_limits(key, cached))
}

/// `key` with each of its limits raised to `other`'s where that is higher.
/// Consensus only ever raises a key's limits (a budget top-up, a later
/// expiry; anything else is refused as not raising it) and never adds or
/// removes one, so the higher value is the newer one whichever answer
/// carries it, and a delayed answer cannot take a raise back. A limit only
/// one copy has is therefore one the other copy lost, so it is taken from
/// whichever copy has it.
fn with_raised_limits(key: IdentityPublicKey, other: &IdentityPublicKey) -> IdentityPublicKey {
    fn higher(own: Option<u64>, other: Option<u64>) -> Option<u64> {
        match (own, other) {
            (Some(own), Some(other)) => Some(own.max(other)),
            (own, other) => own.or(other),
        }
    }
    let total_budget = higher(key.total_budget(), other.total_budget());
    let expires_at = higher(key.expires_at(), other.expires_at());
    if (total_budget, expires_at) == (key.total_budget(), key.expires_at()) {
        return key;
    }
    key.with_limits(total_budget, expires_at)
}

/// The cache's copy of a key, disabled when an older fetch's copy is: a
/// disable is permanent on Platform. The fetch's timestamp is Platform's
/// own, so it wins over the cache's, which may be a local placeholder.
fn with_disable_from(cached: &IdentityPublicKey, fetched: &IdentityPublicKey) -> IdentityPublicKey {
    let mut key = cached.clone();
    if let Some(disabled_at) = fetched.disabled_at().or(cached.disabled_at()) {
        key.set_disabled_at(disabled_at);
    }
    key
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
    use dpp::identity::identity_public_key::v1::IdentityPublicKeyV1;
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::{Identity, IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;
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

    /// Persister whose every `store` fails — pins that a key-persist failure
    /// is surfaced, not swallowed.
    struct FailingPersister;
    impl PlatformWalletPersistence for FailingPersister {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            Err(PersistenceError::backend("add_keys store armed to fail"))
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

    /// A fetch at a lower revision than the cache's was answered before an
    /// update the cache holds: the cache keeps its revision and key set, and
    /// takes only a disable the fetch shows (Platform's timestamp first) and
    /// a raised limit. A fetch at the same revision or higher replaces the
    /// cache but never takes back a disable or a raise. Either way the
    /// balance follows the proof's block, not the revision.
    #[test]
    fn should_take_a_fetched_identity_whole_only_when_it_is_not_older() {
        fn limited(id: KeyID, budget: u64, disabled_at: Option<u64>) -> IdentityPublicKey {
            IdentityPublicKey::V1(IdentityPublicKeyV1 {
                id,
                purpose: Purpose::AUTHENTICATION,
                security_level: SecurityLevel::HIGH,
                contract_bounds: None,
                key_type: KeyType::ECDSA_SECP256K1,
                read_only: false,
                data: BinaryData::new(vec![id as u8; 33]),
                disabled_at,
                total_budget: Some(budget),
                expires_at: None,
            })
        }
        fn block(height: u64) -> BlockTime {
            BlockTime::new(height, 1, height)
        }
        fn at(revision: u64, balance: u64, keys: Vec<IdentityPublicKey>) -> Identity {
            Identity::V0(IdentityV0 {
                id: Identifier::from([1u8; 32]),
                public_keys: keys.into_iter().map(|key| (key.id(), key)).collect(),
                balance,
                revision,
            })
        }
        // The cache took an update at 6 that disabled key 3, and holds a
        // balance proved at block 9; the fetch at 5 still shows key 3 live,
        // a lower budget and key 4, which the cache never saw. Budgets only
        // rise, so the lower one is older, and the fetch's balance is no
        // newer than the cache's proof.
        let cache = at(6, 10, vec![limited(3, 100, Some(9))]);
        let older = at(5, 20, vec![limited(3, 50, None), limited(4, 1, None)]);
        let mut managed = ManagedIdentity::new(cache.clone(), 0);
        managed.last_updated_balance_block_time = Some(block(9));
        for height in [8, 9] {
            assert!(!managed.take_fetched_identity(&older, block(height)));
            assert_eq!(managed.identity, cache);
        }
        // A balance proved at a later block is taken whatever the revision.
        assert!(!managed.take_fetched_identity(&older, block(10)));
        assert_eq!(managed.identity, at(6, 20, vec![limited(3, 100, Some(9))]));
        assert_eq!(managed.last_updated_balance_block_time, Some(block(10)));

        // A disable is permanent: one the older fetch shows sticks even
        // when the cache's copy is live. A higher budget it shows is a raise
        // the cache missed (a limits update claims no revision).
        let mut live = ManagedIdentity::new(at(6, 10, vec![limited(3, 100, None)]), 0);
        live.last_updated_balance_block_time = Some(block(9));
        let disabling = at(5, 20, vec![limited(3, 500, Some(4))]);
        assert!(!live.take_fetched_identity(&disabling, block(9)));
        assert_eq!(live.identity, at(6, 10, vec![limited(3, 500, Some(4))]));
        // Both disabled: the fetch's timestamp is Platform's, the cache's
        // may be a local placeholder.
        let mut stamped = ManagedIdentity::new(at(6, 10, vec![limited(3, 100, Some(99))]), 0);
        assert!(
            !stamped.take_fetched_identity(&at(5, 10, vec![limited(3, 100, Some(4))]), block(9))
        );
        assert_eq!(stamped.identity.public_keys()[&3].disabled_at(), Some(4));

        // An answer at the cache's revision or above is taken whole...
        let same = at(6, 30, vec![limited(3, 600, Some(7)), limited(4, 1, None)]);
        assert!(managed.take_fetched_identity(&same, block(11)));
        assert_eq!(managed.identity, same);
        assert_eq!(managed.last_updated_balance_block_time, Some(block(11)));
        let newer = at(7, 40, vec![limited(3, 600, Some(7))]);
        assert!(managed.take_fetched_identity(&newer, block(12)));
        assert_eq!(managed.identity, newer);

        // ...except for a balance its proof does not order after the cache's,
        // a disable the cache holds and the answer lacks, and a budget the
        // cache holds higher: a delayed answer cannot take a raise back.
        let delayed = at(7, 35, vec![limited(3, 550, None)]);
        for height in [11, 12] {
            assert!(managed.take_fetched_identity(&delayed, block(height)));
            assert_eq!(managed.identity, at(7, 40, vec![limited(3, 600, Some(7))]));
            assert_eq!(managed.last_updated_balance_block_time, Some(block(12)));
        }
    }

    /// Each limit is the higher of the two copies', since consensus only
    /// raises a key's limits; a limit only one copy has is taken from it,
    /// since none is ever added or removed, so the other copy lost it.
    #[test]
    fn should_keep_the_higher_of_each_key_limit() {
        let limited = |total_budget, expires_at| key(3).with_limits(total_budget, expires_at);
        assert_eq!(
            with_raised_limits(limited(Some(100), Some(10)), &limited(Some(50), Some(20))),
            limited(Some(100), Some(20))
        );
        assert_eq!(
            with_raised_limits(limited(Some(100), None), &limited(Some(50), Some(20))),
            limited(Some(100), Some(20))
        );
        assert_eq!(
            with_raised_limits(key(3), &limited(Some(50), Some(20))),
            limited(Some(50), Some(20))
        );
        assert_eq!(with_raised_limits(key(3), &key(3)), key(3));
    }

    /// An older fetch's key decisions carry the cache's copy of each key
    /// (its disabled state, each limit the higher of the two), and a key
    /// only the fetch has is left out.
    #[test]
    fn should_fold_an_older_fetch_s_key_decisions_into_the_cache_s_keys() {
        let mut disabled = key(3);
        disabled.set_disabled_at(9);
        let managed = ManagedIdentity::new(
            Identity::V0(IdentityV0 {
                id: Identifier::from([1u8; 32]),
                public_keys: BTreeMap::from([(3, disabled.clone())]),
                balance: 0,
                revision: 6,
            }),
            0,
        );
        let fetched = |revision| {
            Identity::V0(IdentityV0 {
                id: Identifier::from([1u8; 32]),
                public_keys: BTreeMap::from([(3, key(3)), (4, key(4))]),
                balance: 0,
                revision,
            })
        };
        let decisions = || {
            vec![
                KeyWithBreadcrumb {
                    key: key(3),
                    breadcrumb: None,
                },
                KeyWithBreadcrumb {
                    key: key(4),
                    breadcrumb: None,
                },
            ]
        };
        let folded = managed.key_decisions_for_fetch(&fetched(5), decisions());
        assert_eq!(folded.len(), 1);
        assert_eq!(folded[0].key, disabled);
        assert_eq!(
            managed
                .key_decisions_for_fetch(&fetched(6), decisions())
                .len(),
            2
        );
    }

    /// A cache one past the chain (a base build's post-broadcast bump after a
    /// fetch had taken the update) holds the chain's keys at a higher
    /// revision. A fetch at the chain's revision with exactly those keys is
    /// taken whole, so the next update carries the revision Drive takes; each
    /// limit stays the higher of the two. A fetch that lacks a key the cache
    /// holds, shows a live key the cache holds disabled, or shows an earlier
    /// disable time (a later update disabled the key again) is an answer from
    /// before an update the cache holds and is folded as older.
    #[test]
    fn should_take_a_lower_revision_back_only_when_it_misses_no_update() {
        fn at(revision: u64, keys: Vec<IdentityPublicKey>) -> Identity {
            Identity::V0(IdentityV0 {
                id: Identifier::from([1u8; 32]),
                public_keys: keys.into_iter().map(|key| (key.id(), key)).collect(),
                balance: 0,
                revision,
            })
        }
        let disabled = |id, disabled_at| {
            let mut key = key(id);
            key.set_disabled_at(disabled_at);
            key
        };
        let block = BlockTime::new(1, 1, 1);
        let chain = vec![key(0), disabled(1, 7), key(3)];

        // Inflated: the chain's keys at revision 3; the cache also holds a
        // limit raise the fetch's node has not seen.
        let mut raised = chain.clone();
        raised[2] = key(3).with_limits(Some(500), None);
        let mut inflated = ManagedIdentity::new(at(3, raised.clone()), 0);
        assert!(inflated.take_fetched_identity(&at(2, chain.clone()), block));
        assert_eq!(inflated.identity, at(2, raised));
        // Two past the chain is the same state and heals the same way.
        let mut far = ManagedIdentity::new(at(4, chain.clone()), 0);
        assert!(far.take_fetched_identity(&at(2, chain.clone()), block));
        assert_eq!(far.identity.revision(), 2);

        // Delayed answers (the chain's keys at 2), from before an update at 3
        // the cache holds.
        let delayed = [
            // The update added key 4.
            vec![key(0), disabled(1, 7), key(3), key(4)],
            // The update disabled key 0.
            vec![disabled(0, 9), disabled(1, 7), key(3)],
            // The update disabled key 1 again, at its own block's time.
            vec![key(0), disabled(1, 9), key(3)],
        ];
        for cache in delayed {
            let mut managed = ManagedIdentity::new(at(3, cache.clone()), 0);
            assert!(!managed.take_fetched_identity(&at(2, chain.clone()), block));
            assert_eq!(managed.identity.revision(), 3, "{cache:?}");
            assert_eq!(managed.identity.public_keys().len(), cache.len());
        }
        // A key only the fetch has: folded as older, the key left out.
        let mut lacking = ManagedIdentity::new(at(3, vec![key(0), disabled(1, 7)]), 0);
        assert!(!lacking.take_fetched_identity(&at(2, chain.clone()), block));
        assert_eq!(lacking.identity, at(3, vec![key(0), disabled(1, 7)]));

        // The bump stamped key 1's disable with the local clock: the first
        // fetch replaces the stamp with Platform's time, the next one takes
        // the revision.
        let mut stamped = ManagedIdentity::new(at(3, vec![key(0), disabled(1, 99), key(3)]), 0);
        assert!(!stamped.take_fetched_identity(&at(2, chain.clone()), block));
        assert_eq!(stamped.identity, at(3, chain.clone()));
        assert!(stamped.take_fetched_identity(&at(2, chain.clone()), block));
        assert_eq!(stamped.identity, at(2, chain.clone()));
    }

    /// A key decision keeps the higher of each limit at any revision, so a
    /// load answered before a limits update the cache holds does not store
    /// the lower one.
    #[test]
    fn should_keep_a_raised_limit_in_a_fetch_s_key_decisions() {
        let identity = |revision, budget| {
            Identity::V0(IdentityV0 {
                id: Identifier::from([1u8; 32]),
                public_keys: BTreeMap::from([(3, key(3).with_limits(Some(budget), None))]),
                balance: 0,
                revision,
            })
        };
        let managed = ManagedIdentity::new(identity(6, 500), 0);
        for revision in [5, 6, 7] {
            let decisions = vec![KeyWithBreadcrumb {
                key: key(3).with_limits(Some(100), None),
                breadcrumb: None,
            }];
            let folded = managed.key_decisions_for_fetch(&identity(revision, 100), decisions);
            assert_eq!(folded[0].key.total_budget(), Some(500), "at {revision}");
        }
    }

    /// A refresh stores, with the scalar snapshot and in one changeset, the
    /// row of every key its fold adds or changes (here a key another device
    /// added and a disable), without a breadcrumb, and no row for a key it
    /// leaves as it was. An older answer stores no key row.
    #[test]
    fn should_store_the_keys_a_refresh_adds_or_changes() {
        let id = Identifier::from([1u8; 32]);
        let wallet_id: WalletId = [0xAB; 32];
        let identity = |revision, keys: Vec<IdentityPublicKey>| {
            Identity::V0(IdentityV0 {
                id,
                public_keys: keys.into_iter().map(|key| (key.id(), key)).collect(),
                balance: 0,
                revision,
            })
        };
        let block = |height| BlockTime::new(height, 1, height);
        let persister = std::sync::Arc::new(CapturingPersister::default());
        let p = WalletPersister::new(wallet_id, std::sync::Arc::clone(&persister) as _);
        let mut managed = ManagedIdentity::new(identity(6, vec![key(1), key(2)]), 0);
        managed.wallet_id = Some(wallet_id);

        let mut disabled = key(2);
        disabled.set_disabled_at(5);
        managed
            .store_fetched_identity(
                &identity(7, vec![key(1), disabled.clone(), key(4)]),
                block(1),
                &p,
            )
            .unwrap();
        {
            let stores = persister.stores.lock().unwrap();
            assert_eq!(stores.len(), 1, "keys and snapshot in one changeset");
            let snapshot = stores[0].identities.as_ref().unwrap();
            assert_eq!(snapshot.identities[&id].revision, 7);
            let rows = &stores[0].identity_keys.as_ref().unwrap().upserts;
            assert_eq!(
                rows.keys().map(|(_, key_id)| *key_id).collect::<Vec<_>>(),
                vec![2, 4]
            );
            assert_eq!(rows[&(id, 2)].public_key, disabled);
            assert!(rows
                .values()
                .all(|row| row.derivation_indices.is_none() && row.wallet_id.is_none()));
        }

        // An answer from before the cache's revision changes no key.
        persister.stores.lock().unwrap().clear();
        managed
            .store_fetched_identity(&identity(6, vec![key(1), key(2)]), block(2), &p)
            .unwrap();
        let stores = persister.stores.lock().unwrap();
        assert_eq!(stores.len(), 1);
        assert!(stores[0].identity_keys.as_ref().unwrap().upserts.is_empty());
        assert_eq!(managed.identity.revision(), 7);
        assert_eq!(managed.identity.public_keys()[&2], disabled);
    }

    /// A batch of fetched names lands as one snapshot carrying every name,
    /// skips labels already held, and persists nothing when all are known.
    /// One snapshot per name made the first read as the complete owned set.
    #[test]
    fn merge_dpns_names_persists_one_complete_snapshot() {
        let identity = Identity::V0(IdentityV0 {
            id: Identifier::from([1u8; 32]),
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        });
        let mut managed = ManagedIdentity::new(identity, 0);
        let persister = std::sync::Arc::new(CapturingPersister::default());
        let p = WalletPersister::new([0xAB; 32], std::sync::Arc::clone(&persister) as _);
        let name = |label: &str| DpnsNameInfo {
            label: label.to_string(),
            acquired_at: None,
        };

        managed.add_dpns_name(name("alice"), &p);
        let added = managed.merge_dpns_names([name("alice"), name("bob"), name("carol")], &p);

        assert_eq!(added, 2);
        let stores = persister.stores.lock().unwrap();
        assert_eq!(
            stores.len(),
            2,
            "one store for add_dpns_name, one for the merge"
        );
        let labels: Vec<_> = stores[1]
            .identities
            .as_ref()
            .expect("identity snapshot")
            .identities
            .values()
            .next()
            .expect("one identity")
            .dpns_names
            .iter()
            .map(|n| n.label.as_str())
            .collect();
        assert_eq!(labels, ["alice", "bob", "carol"]);
        drop(stores);

        assert_eq!(managed.merge_dpns_names([name("bob")], &p), 0);
        assert_eq!(
            persister.stores.lock().unwrap().len(),
            2,
            "nothing new, nothing stored"
        );
    }

    fn dpns_test_identity() -> (
        ManagedIdentity,
        std::sync::Arc<CapturingPersister>,
        WalletPersister,
    ) {
        let identity = Identity::V0(IdentityV0 {
            id: Identifier::from([1u8; 32]),
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        });
        let persister = std::sync::Arc::new(CapturingPersister::default());
        let p = WalletPersister::new([0xAB; 32], std::sync::Arc::clone(&persister) as _);
        (ManagedIdentity::new(identity, 0), persister, p)
    }

    fn dpns_name(label: &str, acquired_at: Option<u64>) -> DpnsNameInfo {
        DpnsNameInfo {
            label: label.to_string(),
            acquired_at,
        }
    }

    fn dpns_store_count(persister: &CapturingPersister) -> usize {
        persister.stores.lock().unwrap().len()
    }

    /// A complete fetch is the owned set: a departed label drops out, a known
    /// label keeps its timestamp, a new one is added — in one store.
    #[test]
    fn complete_dpns_fetch_replaces_the_list_in_one_store() {
        let (mut managed, persister, p) = dpns_test_identity();
        managed.set_dpns_names(
            vec![dpns_name("alice", Some(10)), dpns_name("bob", Some(20))],
            &p,
        );
        let before = dpns_store_count(&persister);

        let added = managed.apply_fetched_dpns_names(
            vec![dpns_name("bob", Some(99)), dpns_name("carol", Some(99))],
            DpnsFetch::Complete,
            &p,
        );

        assert_eq!(added, 1);
        assert_eq!(
            managed.dpns_names,
            vec![dpns_name("bob", Some(20)), dpns_name("carol", Some(99))]
        );
        assert_eq!(dpns_store_count(&persister), before + 1);
    }

    /// An empty complete fetch means the identity owns no names any more.
    #[test]
    fn empty_complete_dpns_fetch_clears_the_list() {
        let (mut managed, persister, p) = dpns_test_identity();
        managed.set_dpns_names(vec![dpns_name("alice", Some(10))], &p);
        let before = dpns_store_count(&persister);

        assert_eq!(
            managed.apply_fetched_dpns_names(Vec::new(), DpnsFetch::Complete, &p),
            0
        );

        assert!(managed.dpns_names.is_empty());
        assert_eq!(dpns_store_count(&persister), before + 1);
    }

    /// A full page may be truncated: a label missing from it stays.
    #[test]
    fn partial_dpns_fetch_only_adds() {
        let (mut managed, _persister, p) = dpns_test_identity();
        managed.set_dpns_names(vec![dpns_name("alice", Some(10))], &p);

        let added =
            managed.apply_fetched_dpns_names(vec![dpns_name("bob", None)], DpnsFetch::Partial, &p);

        assert_eq!(added, 1);
        assert_eq!(
            managed.dpns_names,
            vec![dpns_name("alice", Some(10)), dpns_name("bob", None)]
        );
    }

    /// A wallet-owned identity never loses a label to a username fetch, even
    /// a complete one (including an empty complete fetch): it only gains the
    /// new labels. The marketplace sweep discovers its departures from this
    /// list, so pruning here would erase the only trigger.
    #[test]
    fn complete_dpns_fetch_on_a_wallet_identity_only_adds() {
        let (mut managed, _persister, p) = dpns_test_identity();
        managed.wallet_id = Some([0xAB; 32]);
        managed.set_dpns_names(
            vec![dpns_name("alice", Some(10)), dpns_name("bob", Some(20))],
            &p,
        );

        let added = managed.apply_fetched_dpns_names(
            vec![dpns_name("carol", None)],
            DpnsFetch::Complete,
            &p,
        );
        assert_eq!(added, 1);
        let labels: Vec<&str> = managed
            .dpns_names
            .iter()
            .map(|n| n.label.as_str())
            .collect();
        assert_eq!(labels, ["alice", "bob", "carol"]);

        assert_eq!(
            managed.apply_fetched_dpns_names(Vec::new(), DpnsFetch::Complete, &p),
            0
        );
        assert_eq!(
            managed.dpns_names.len(),
            3,
            "an empty complete fetch prunes nothing"
        );
    }

    /// Re-fetching the same complete set (even in another order, with fresh
    /// timestamps) changes nothing and stores nothing.
    #[test]
    fn unchanged_complete_dpns_fetch_stores_nothing() {
        let (mut managed, persister, p) = dpns_test_identity();
        managed.set_dpns_names(
            vec![dpns_name("alice", Some(10)), dpns_name("bob", Some(20))],
            &p,
        );
        let before = dpns_store_count(&persister);

        let added = managed.apply_fetched_dpns_names(
            vec![dpns_name("bob", Some(99)), dpns_name("alice", Some(99))],
            DpnsFetch::Complete,
            &p,
        );

        assert_eq!(added, 0);
        assert_eq!(dpns_store_count(&persister), before);
        assert_eq!(managed.dpns_names[0], dpns_name("alice", Some(10)));
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

        // Key 0 is re-derivable (breadcrumb), key 1 is watch-only (None).
        managed
            .add_keys(
                vec![
                    crate::changeset::KeyWithBreadcrumb {
                        key: key(0),
                        breadcrumb: Some((wallet_id, 7, 0)),
                    },
                    crate::changeset::KeyWithBreadcrumb {
                        key: key(1),
                        breadcrumb: None,
                    },
                ],
                &p,
            )
            .expect("add_keys persists in test");

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
        assert_eq!(
            upserts[&(id, 1)].derivation_indices,
            None,
            "watch-only key carries no breadcrumb"
        );
        assert_eq!(upserts[&(id, 1)].wallet_id, None);
    }

    /// `replace_key` emits the updated key without a derivation breadcrumb. The managed
    /// identity does not track per key whether it was wallet-derived, so coordinates built
    /// from `(wallet_id, identity_index, key_id)` would be invented for an external key and
    /// overwrite the linkage the client persisted; every persister keeps its existing
    /// breadcrumb when an upsert carries none.
    #[test]
    fn replace_key_carries_no_breadcrumb() {
        use dpp::identity::identity_public_key::accessors::v1::IdentityPublicKeyGettersV1;

        let identity = Identity::V0(IdentityV0 {
            id: Identifier::from([1u8; 32]),
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        });
        let mut managed = ManagedIdentity::new(identity, 7);
        let wallet_id: WalletId = [0xAB; 32];
        managed.wallet_id = Some(wallet_id);
        let persister = std::sync::Arc::new(CapturingPersister::default());
        let p = WalletPersister::new(wallet_id, std::sync::Arc::clone(&persister) as _);

        // Key 1 came from outside the wallet: no breadcrumb.
        managed
            .add_keys(
                vec![crate::changeset::KeyWithBreadcrumb {
                    key: key(1),
                    breadcrumb: None,
                }],
                &p,
            )
            .expect("add_keys persists in test");

        managed
            .replace_key(key(1).with_limits(Some(1_000), None), &p)
            .expect("replace_key persists in test");

        let stores = persister.stores.lock().unwrap();
        let cs = stores.last().expect("a changeset was stored");
        let id = managed.id();
        let entry = &cs
            .identity_keys
            .as_ref()
            .expect("identity_keys present")
            .upserts[&(id, 1)];
        assert_eq!(entry.public_key.total_budget(), Some(1_000));
        assert_eq!(
            entry.derivation_indices, None,
            "a limits-only upsert must not invent derivation coordinates"
        );
        assert_eq!(entry.wallet_id, None);
        assert!(cs.identities.is_some(), "the identity snapshot rides along");
        assert_eq!(
            managed.identity.public_keys()[&1].total_budget(),
            Some(1_000),
            "the in-memory key is replaced"
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
        managed
            .add_keys(Vec::new(), &p)
            .expect("empty add_keys is a no-op Ok");
        assert!(
            persister.stores.lock().unwrap().is_empty(),
            "empty add_keys stores nothing"
        );
    }

    /// A failed key-persist must SURFACE as `Err`, not be swallowed — else an
    /// imported / restored identity comes back watch-only after restart with no
    /// signal. The pre-fix `add_keys` logged the error and returned `()`.
    #[test]
    fn add_keys_surfaces_persist_failure() {
        let identity = Identity::V0(IdentityV0 {
            id: Identifier::from([1u8; 32]),
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        });
        let mut managed = ManagedIdentity::new(identity, 0);
        let wallet_id: WalletId = [0xAB; 32];
        let p = WalletPersister::new(wallet_id, std::sync::Arc::new(FailingPersister) as _);

        let result = managed.add_keys(
            vec![crate::changeset::KeyWithBreadcrumb {
                key: key(0),
                breadcrumb: Some((wallet_id, 7, 0)),
            }],
            &p,
        );
        assert!(
            result.is_err(),
            "a failed key-persist must surface, not be swallowed"
        );
    }

    /// Recording a payment emits BOTH carriers with the right granularity:
    /// the identity snapshot keeps the full payments map (blob-style
    /// persisters overwrite the whole entry) while
    /// `dashpay_payments_overlay` carries EXACTLY the one changed
    /// `(owner, txid)` row — never the accumulated history. Delta-style
    /// persisters (the FFI vtable) project only the overlay, so this
    /// single-row shape is what keeps per-store work bounded as history
    /// grows: N recorded payments must emit N overlay rows total, not
    /// 1 + 2 + … + N.
    #[test]
    fn record_dashpay_payment_emits_single_row_overlay() {
        let owner_id = Identifier::from([1u8; 32]);
        let identity = Identity::V0(IdentityV0 {
            id: owner_id,
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        });
        let mut managed = ManagedIdentity::new(identity, 0);
        let capturing = std::sync::Arc::new(CapturingPersister::default());
        let p = WalletPersister::new([0xAB; 32], capturing.clone() as _);
        let alice = Identifier::from([0xAA; 32]);

        managed
            .record_dashpay_payment(
                "tx1".into(),
                PaymentEntry::new_sent(alice, 100, Some("rent".into())),
                &p,
            )
            .expect("record tx1");
        managed
            .record_dashpay_payment(
                "tx2".into(),
                PaymentEntry::new_received(alice, 250, None),
                &p,
            )
            .expect("record tx2");

        let stores = capturing.stores.lock().unwrap();
        assert_eq!(stores.len(), 2);

        // Second store: the snapshot map has both rows, the overlay only
        // the newly recorded one.
        let cs = &stores[1];
        let snapshot = cs
            .identities
            .as_ref()
            .expect("identity snapshot rides along")
            .identities
            .get(&owner_id)
            .expect("owner entry");
        assert_eq!(
            snapshot.dashpay_payments.len(),
            2,
            "full map on the blob carrier"
        );
        let overlay = cs
            .dashpay_payments_overlay
            .as_ref()
            .expect("overlay must carry the delta")
            .get(&owner_id)
            .expect("owner overlay");
        assert_eq!(
            overlay.len(),
            1,
            "the overlay must carry only the changed row, not the history"
        );
        let row = overlay.get("tx2").expect("the just-recorded txid");
        assert_eq!(row.amount_duffs, 250);

        // A status flip re-emits the same single (owner, txid) row.
        let mut confirmed = PaymentEntry::new_sent(alice, 100, Some("rent".into()));
        confirmed.status = crate::wallet::identity::PaymentStatus::Confirmed;
        drop(stores);
        managed
            .record_dashpay_payment("tx1".into(), confirmed.clone(), &p)
            .expect("flip tx1");
        let stores = capturing.stores.lock().unwrap();
        let overlay = stores[2]
            .dashpay_payments_overlay
            .as_ref()
            .expect("overlay on the flip")
            .get(&owner_id)
            .expect("owner overlay");
        assert_eq!(overlay.len(), 1);
        assert_eq!(overlay.get("tx1"), Some(&confirmed));
    }

    /// A failed payment persist must NOT strand the entry in memory — else a
    /// caller's `contains_key` retry guard skips the next sweep's re-attempt
    /// and the Sent entry + memo (no on-chain recovery) is permanently lost.
    /// Pre-fix, `record_dashpay_payment` inserted before persisting, so a
    /// failed store left the entry in memory.
    #[test]
    fn record_dashpay_payment_rolls_back_fresh_on_persist_failure() {
        let identity = Identity::V0(IdentityV0 {
            id: Identifier::from([1u8; 32]),
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        });
        let mut managed = ManagedIdentity::new(identity, 0);
        let p = WalletPersister::new([0xAB; 32], std::sync::Arc::new(FailingPersister) as _);
        let alice = Identifier::from([0xAA; 32]);

        let result = managed.record_dashpay_payment(
            "tx1".into(),
            PaymentEntry::new_sent(alice, 100, Some("rent".into())),
            &p,
        );
        assert!(result.is_err(), "a failed payment persist must surface");
        assert!(
            !managed.dashpay.payments.contains_key("tx1"),
            "a failed persist must not strand the entry in memory, else the \
             contains_key retry guard skips the re-attempt"
        );
    }

    /// A failed *overwrite* (e.g. Pending→Confirmed) must restore the previous
    /// entry, not leave the un-persisted new value (or delete the row).
    #[test]
    fn record_dashpay_payment_restores_previous_on_failed_overwrite() {
        let identity = Identity::V0(IdentityV0 {
            id: Identifier::from([1u8; 32]),
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        });
        let mut managed = ManagedIdentity::new(identity, 0);
        let p = WalletPersister::new([0xAB; 32], std::sync::Arc::new(FailingPersister) as _);
        let alice = Identifier::from([0xAA; 32]);

        // Pre-existing (persisted) entry.
        managed
            .dashpay
            .payments
            .insert("tx1".into(), PaymentEntry::new_sent(alice, 100, None));

        // Overwrite attempt fails to persist → previous must survive intact.
        let result = managed.record_dashpay_payment(
            "tx1".into(),
            PaymentEntry::new_sent(alice, 999, Some("changed".into())),
            &p,
        );
        assert!(result.is_err());
        let kept = managed
            .dashpay
            .payments
            .get("tx1")
            .expect("previous survives");
        assert_eq!(
            kept.amount_duffs, 100,
            "a failed overwrite must restore the previous entry, not the un-persisted new value"
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
            .dashpay
            .payments
            .insert("t1".into(), PaymentEntry::new_sent(alice, 100, None));
        managed
            .dashpay
            .payments
            .insert("t2".into(), PaymentEntry::new_received(bob, 200, None));
        managed
            .dashpay
            .payments
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
