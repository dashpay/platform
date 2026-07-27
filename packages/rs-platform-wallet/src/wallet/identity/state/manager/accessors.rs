//! Read accessors for [`IdentityManager`].
//!
//! Typed getters that scan both buckets, plus aggregated balance and
//! a derived gap-limit watermark helper. Mutating methods live in
//! [`super::lifecycle`].

use super::{IdentityLocation, IdentityManager, RegistrationIndex};
use crate::wallet::identity::state::managed_identity::ManagedIdentity;
use crate::wallet::platform_wallet::WalletId;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::prelude::Identifier;

impl IdentityManager {
    /// Look up a managed identity by id across both buckets.
    ///
    /// O(log n): hits the side-index for the bucket discriminant +
    /// inner key, then a single `BTreeMap::get` hop into the right
    /// bucket. The index is maintained as an invariant by every add /
    /// remove path in this module (see field doc on `location_index`).
    pub fn identity(&self, identity_id: &Identifier) -> Option<&ManagedIdentity> {
        match self.location_index().get(identity_id).copied()? {
            IdentityLocation::InWallet {
                wallet_id,
                registration_index,
            } => self
                .wallet_identities
                .get(&wallet_id)?
                .get(&registration_index),
            IdentityLocation::OutOfWallet => self.out_of_wallet_identities.get(identity_id),
        }
    }

    /// Mutable counterpart to [`Self::identity`]. Same O(log n) shape.
    pub fn identity_mut(&mut self, identity_id: &Identifier) -> Option<&mut ManagedIdentity> {
        // Pull the location out by value first so the immutable borrow
        // on `location_index` ends before we take the mutable borrow on
        // the bucket.
        match self.location_index().get(identity_id).copied()? {
            IdentityLocation::InWallet {
                wallet_id,
                registration_index,
            } => self
                .wallet_identities
                .get_mut(&wallet_id)?
                .get_mut(&registration_index),
            IdentityLocation::OutOfWallet => self.out_of_wallet_identities.get_mut(identity_id),
        }
    }

    /// Snapshot every managed identity (both buckets) into an owned
    /// `Vec<&Identity>`. Used by callers that want a flat list without
    /// caring about which bucket each identity lives in.
    pub fn all_identities(&self) -> Vec<&Identity> {
        let mut out: Vec<&Identity> = self
            .out_of_wallet_identities
            .values()
            .map(|m| &m.identity)
            .collect();
        for inner in self.wallet_identities.values() {
            for managed in inner.values() {
                out.push(&managed.identity);
            }
        }
        out
    }

    /// Iterate every managed identity (both buckets) as `&ManagedIdentity`.
    ///
    /// Unlike [`Self::all_identities`] (`&Identity` only) and
    /// [`Self::identity_ids`] (ids only), this exposes the full per-identity
    /// state — including the in-memory-only `pending_contact_crypto` queue — so
    /// a caller can snapshot/aggregate it across identities without a second
    /// lookup. Order is unspecified. Mirrors the identity set of
    /// [`Self::all_identities`] exactly.
    pub fn managed_identities(&self) -> impl Iterator<Item = &ManagedIdentity> {
        self.out_of_wallet_identities.values().chain(
            self.wallet_identities
                .values()
                .flat_map(|inner| inner.values()),
        )
    }

    /// Backwards-compatible name used by FFI / external callers — same
    /// as [`Self::managed_identity`].
    pub fn managed_identity(&self, identity_id: &Identifier) -> Option<&ManagedIdentity> {
        self.identity(identity_id)
    }

    /// Mutable counterpart to [`Self::managed_identity`].
    pub fn managed_identity_mut(
        &mut self,
        identity_id: &Identifier,
    ) -> Option<&mut ManagedIdentity> {
        self.identity_mut(identity_id)
    }

    /// Total credit balance across every identity in either bucket.
    pub fn total_credit_balance(&self) -> u64 {
        let out_of_wallet: u64 = self
            .out_of_wallet_identities
            .values()
            .map(|m| m.identity.balance())
            .sum();
        let in_wallet: u64 = self
            .wallet_identities
            .values()
            .flat_map(|inner| inner.values().map(|m| m.identity.balance()))
            .sum();
        out_of_wallet + in_wallet
    }

    /// Total number of managed identities across both buckets.
    pub fn identity_count(&self) -> usize {
        self.out_of_wallet_identities.len()
            + self
                .wallet_identities
                .values()
                .map(|m| m.len())
                .sum::<usize>()
    }

    /// Snapshot of every managed identity's `Identifier` across both
    /// buckets. Order is unspecified — callers that need a stable
    /// order should sort the returned `Vec`.
    pub fn identity_ids(&self) -> Vec<Identifier> {
        let mut out: Vec<Identifier> = Vec::with_capacity(self.identity_count());
        out.extend(self.out_of_wallet_identities.keys().copied());
        for inner in self.wallet_identities.values() {
            for managed in inner.values() {
                out.push(managed.identity.id());
            }
        }
        out
    }

    /// `true` iff both buckets are empty.
    pub fn is_empty(&self) -> bool {
        self.out_of_wallet_identities.is_empty() && self.wallet_identities.is_empty()
    }

    /// Highest BIP-9 identity index ever registered for `wallet_id`,
    /// or `None` if the wallet has no identities yet.
    ///
    /// Replaces the old `last_scanned_index` watermark — the gap-limit
    /// scan now resumes from `highest_registration_index(...).map_or(0, |i| i + 1)`
    /// rather than carrying its own counter on the manager.
    pub fn highest_registration_index(&self, wallet_id: &WalletId) -> Option<RegistrationIndex> {
        self.wallet_identities
            .get(wallet_id)
            .and_then(|m| m.keys().last().copied())
    }
}
