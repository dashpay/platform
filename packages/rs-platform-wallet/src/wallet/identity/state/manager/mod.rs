//! Identity management for platform wallets.
//!
//! Storage and management of Dash Platform identities associated
//! with a single wallet. Implementation is split across several
//! sibling files by responsibility:
//!
//! - [`lifecycle`] — construction, add / remove, HD index lookup.
//! - [`accessors`] — read/write access to managed identities
//!   (getters, total balance).
//! - [`apply`] — replay path for persisted
//!   [`IdentityEntry`](crate::changeset::IdentityEntry)s.
//!
//! # Storage shape
//!
//! Identities are bucketed by ownership:
//!
//! - `wallet_identities[wallet_id][registration_index]` — wallet-owned,
//!   signing-capable. Inner key is the BIP-9 HD identity index used
//!   during registration; outer key is the wallet identifier.
//! - `out_of_wallet_identities[identity_id]` — observed read-only.
//!   No wallet → no signing keys derivable. Replaces the previous
//!   `WatchedIdentity` collection; the read-only-ness is enforced
//!   naturally by the absence of a `wallet_id`.
//!
//! Each `ManagedIdentity` denormalizes both keys (`identity_index`,
//! `wallet_id`) onto its own struct so callers that iterate values
//! don't need to re-thread the bucket key through.

mod accessors;
mod apply;
mod lifecycle;

use super::managed_identity::ManagedIdentity;
use crate::changeset::{IdentityManagerStartState, IdentityScanStateEntry};
use crate::wallet::platform_wallet::WalletId;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::Identifier;
use std::collections::BTreeMap;

/// Plain alias for the BIP-9 HD identity index used as the inner-bucket
/// key for wallet-owned identities. Keeps the type signatures readable
/// without adding a newtype.
pub type RegistrationIndex = u32;

/// Where in [`IdentityManager`] an identity lives.
///
/// The two-bucket invariant is encoded in the type — there's no way to
/// construct an `InWallet` without both pieces, and `OutOfWallet` carries
/// nothing. Replaces the previous `(Option<WalletId>, RegistrationIndex)`
/// tuple, which had two non-meaningful representable states (a `None`
/// wallet paired with a non-zero index, or a `Some` wallet paired with
/// the sentinel `0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdentityLocation {
    /// The identity lives in `out_of_wallet_identities` — no wallet,
    /// no derivation context.
    OutOfWallet,
    /// The identity lives in `wallet_identities[wallet_id][registration_index]`.
    InWallet {
        wallet_id: WalletId,
        registration_index: RegistrationIndex,
    },
}

/// Manages identities for a platform wallet.
///
/// See the module docs for the bucket layout.
#[derive(Debug, Clone, Default)]
pub struct IdentityManager {
    /// Identities the wallet observes but cannot sign for, keyed by
    /// identity id. Replaces the old `WatchedIdentity` bucket — the
    /// "read-only" property is implied by `wallet_id == None`.
    pub out_of_wallet_identities: BTreeMap<Identifier, ManagedIdentity>,

    /// Wallet-owned, signing-capable identities. Outer map keyed by
    /// `wallet_id`; inner map keyed by the BIP-9 registration index.
    /// Both keys are denormalized onto each `ManagedIdentity` value
    /// (`wallet_id`, `identity_index`) so iterating values is enough.
    pub wallet_identities: BTreeMap<WalletId, BTreeMap<RegistrationIndex, ManagedIdentity>>,

    /// Reverse index for O(log n) lookup by [`Identifier`]. Maintained
    /// as an invariant alongside `out_of_wallet_identities` and
    /// `wallet_identities` — every mutation that adds/removes an
    /// identity must update this map. See [`IdentityLocation`] for the
    /// shape; the enum makes the bucket discriminant explicit and
    /// removes the placeholder-vs-real-index ambiguity the previous
    /// `(Option<WalletId>, u32)` tuple carried.
    ///
    /// Private on purpose — this is derived state, and any code path
    /// outside this module that mutated it directly would risk
    /// breaking the invariant. All bucket-mutating helpers in the
    /// module (`add_identity`, `add_out_of_wallet_identity`,
    /// `remove_identity`, `apply_identity_entry`, `remove_for_apply`)
    /// update this index in lockstep with the buckets, and external
    /// callers that need to drop an identity reach the buckets through
    /// `remove_for_apply` so the index stays in sync.
    location_index: BTreeMap<Identifier, IdentityLocation>,

    /// Per-wallet verdict of the last gap-limit identity scan, keyed by wallet
    /// id because a scan is a wallet-scoped act even though its result is a
    /// set of identities.
    ///
    /// Consulted by the startup sequence before it takes the warm-launch
    /// shortcut: a scan that could not answer every index must not let a
    /// later launch conclude the identity set is settled. An absent entry
    /// means no verdict is known — see
    /// [`IdentityManagerStartState::scan_states`] for why that is deliberately
    /// not read as "complete".
    identity_scan_states: BTreeMap<WalletId, IdentityScanStateEntry>,
}

impl From<IdentityManagerStartState> for IdentityManager {
    fn from(state: IdentityManagerStartState) -> Self {
        let IdentityManagerStartState {
            out_of_wallet_identities,
            wallet_identities,
            scan_states,
        } = state;

        // Rebuild the side-index from the two buckets — `IdentityManagerStartState`
        // is the persistable two-bucket shape and intentionally doesn't carry the
        // index, since it's runtime-only computed state.
        let mut location_index: BTreeMap<Identifier, IdentityLocation> = BTreeMap::new();
        for id in out_of_wallet_identities.keys() {
            location_index.insert(*id, IdentityLocation::OutOfWallet);
        }
        for (wallet_id, inner) in &wallet_identities {
            for (registration_index, managed) in inner {
                location_index.insert(
                    managed.identity.id(),
                    IdentityLocation::InWallet {
                        wallet_id: *wallet_id,
                        registration_index: *registration_index,
                    },
                );
            }
        }

        Self {
            out_of_wallet_identities,
            wallet_identities,
            location_index,
            identity_scan_states: scan_states,
        }
    }
}

impl IdentityManager {
    /// Read-only view onto the location index. Exposed to sibling
    /// modules (`accessors`, `lifecycle`, `apply`) so the lookup helpers
    /// can hop directly into the right bucket without re-scanning. Not
    /// `pub` — see the field doc for why.
    #[inline]
    pub(super) fn location_index(&self) -> &BTreeMap<Identifier, IdentityLocation> {
        &self.location_index
    }

    /// Insert a new entry in the location index. Must be called from
    /// the same code path that just inserted into one of the buckets
    /// (or that's about to). Restricted to sibling modules.
    #[inline]
    pub(super) fn location_index_insert(&mut self, id: Identifier, location: IdentityLocation) {
        self.location_index.insert(id, location);
    }

    /// Drop an entry from the location index. Returns the prior
    /// location if any, so the caller can use it to know which bucket
    /// to remove from. Restricted to sibling modules.
    #[inline]
    pub(super) fn location_index_remove(&mut self, id: &Identifier) -> Option<IdentityLocation> {
        self.location_index.remove(id)
    }

    /// Helper for the apply path — find a managed identity in either
    /// bucket. Uses the side-index for O(log n) lookup. Internal use
    /// only; public callers go through [`Self::identity`] /
    /// [`Self::identity_mut`].
    pub(crate) fn locate_mut(&mut self, identity_id: &Identifier) -> Option<&mut ManagedIdentity> {
        match self.location_index.get(identity_id).copied()? {
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

    /// Tombstone-only removal for the apply path
    /// ([`crate::wallet::apply`]). Removes the identity from whichever
    /// bucket holds it AND drops the corresponding entry from the
    /// side-index, without persisting (the persister has already
    /// recorded the removal that triggered this replay).
    ///
    /// Returns `true` iff something was removed.
    pub(crate) fn remove_for_apply(&mut self, identity_id: &Identifier) -> bool {
        let Some(location) = self.location_index.remove(identity_id) else {
            return false;
        };
        match location {
            IdentityLocation::InWallet {
                wallet_id,
                registration_index,
            } => {
                if let Some(inner) = self.wallet_identities.get_mut(&wallet_id) {
                    inner.remove(&registration_index);
                    if inner.is_empty() {
                        self.wallet_identities.remove(&wallet_id);
                    }
                }
            }
            IdentityLocation::OutOfWallet => {
                self.out_of_wallet_identities.remove(identity_id);
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::persister::{NoPlatformPersistence, WalletPersister};
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use std::sync::Arc;

    fn noop_persister() -> WalletPersister {
        WalletPersister::new([0u8; 32], Arc::new(NoPlatformPersistence))
    }

    fn create_test_identity(id: Identifier) -> Identity {
        use dpp::identity::v0::IdentityV0;
        use std::collections::BTreeMap;

        let identity_v0 = IdentityV0 {
            id,
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        };

        Identity::V0(identity_v0)
    }

    #[test]
    fn test_add_identity_into_wallet_bucket() {
        let mut manager = IdentityManager::new();
        let wallet_id: WalletId = [9u8; 32];
        let identity_id = Identifier::from([1u8; 32]);
        let identity = create_test_identity(identity_id);
        let p = noop_persister();

        manager
            .add_identity(identity.clone(), 0, wallet_id, &p)
            .unwrap();

        assert_eq!(
            manager.wallet_identities.get(&wallet_id).map(|m| m.len()),
            Some(1)
        );
        assert!(manager.out_of_wallet_identities.is_empty());
        let managed = manager.identity(&identity_id).expect("present");
        assert_eq!(managed.identity_index, Some(0));
        assert_eq!(managed.wallet_id, Some(wallet_id));
        assert_eq!(manager.identity_index(&identity_id), Some(0));
        assert_eq!(manager.highest_registration_index(&wallet_id), Some(0));
    }

    #[test]
    fn test_add_out_of_wallet_identity() {
        let mut manager = IdentityManager::new();
        let identity_id = Identifier::from([2u8; 32]);
        let identity = create_test_identity(identity_id);
        let p = noop_persister();

        manager.add_out_of_wallet_identity(identity, &p).unwrap();

        assert_eq!(manager.out_of_wallet_identities.len(), 1);
        assert!(manager.wallet_identities.is_empty());
        let managed = manager.identity(&identity_id).expect("present");
        assert!(managed.wallet_id.is_none());
    }

    #[test]
    fn out_of_wallet_identity_has_no_identity_index() {
        let mut manager = IdentityManager::new();
        let identity_id = Identifier::from([0xEFu8; 32]);
        let identity = create_test_identity(identity_id);
        let p = noop_persister();

        manager
            .add_out_of_wallet_identity(identity, &p)
            .expect("add observed");

        // The whole point of `Option<u32>` on `identity_index` — observed
        // identities have no derivation context, and the type forces every
        // signing/derivation path to handle that explicitly.
        let managed = manager.identity(&identity_id).expect("present");
        assert!(managed.identity_index.is_none());
        // The manager-level helper agrees.
        assert_eq!(manager.identity_index(&identity_id), None);
    }

    #[test]
    fn test_remove_identity_from_wallet_bucket() {
        let mut manager = IdentityManager::new();
        let wallet_id: WalletId = [9u8; 32];
        let identity_id = Identifier::from([1u8; 32]);
        let identity = create_test_identity(identity_id);
        let p = noop_persister();

        manager.add_identity(identity, 0, wallet_id, &p).unwrap();
        let removed = manager.remove_identity(&identity_id, &p).unwrap();

        assert_eq!(removed.id(), identity_id);
        assert!(!manager.wallet_identities.contains_key(&wallet_id));
        assert_eq!(manager.identity_count(), 0);
    }

    #[test]
    fn test_remove_identity_from_out_of_wallet_bucket() {
        let mut manager = IdentityManager::new();
        let identity_id = Identifier::from([2u8; 32]);
        let identity = create_test_identity(identity_id);
        let p = noop_persister();

        manager.add_out_of_wallet_identity(identity, &p).unwrap();
        let removed = manager.remove_identity(&identity_id, &p).unwrap();

        assert_eq!(removed.id(), identity_id);
        assert!(manager.out_of_wallet_identities.is_empty());
    }

    #[test]
    fn add_identity_updates_location_index() {
        let mut manager = IdentityManager::new();
        let wallet_id: WalletId = [9u8; 32];
        let identity_id = Identifier::from([1u8; 32]);
        let identity = create_test_identity(identity_id);
        let p = noop_persister();

        manager
            .add_identity(identity, 7, wallet_id, &p)
            .expect("add ok");

        // The side-index records the bucket discriminant + inner key
        // so the lookup can hop straight to the right slot in the
        // wallet bucket.
        assert_eq!(
            manager.location_index().get(&identity_id).copied(),
            Some(IdentityLocation::InWallet {
                wallet_id,
                registration_index: 7,
            })
        );
        // Cross-check the public lookup hits the same value.
        let managed = manager.identity(&identity_id).expect("present");
        assert_eq!(managed.identity_index, Some(7));
        assert_eq!(managed.wallet_id, Some(wallet_id));
    }

    #[test]
    fn add_out_of_wallet_identity_updates_location_index() {
        let mut manager = IdentityManager::new();
        let identity_id = Identifier::from([2u8; 32]);
        let identity = create_test_identity(identity_id);
        let p = noop_persister();

        manager
            .add_out_of_wallet_identity(identity, &p)
            .expect("add ok");

        // Out-of-wallet entries record the `OutOfWallet` discriminant —
        // no wallet, no derivation index. The enum makes the absence
        // of those fields impossible to misread.
        assert_eq!(
            manager.location_index().get(&identity_id).copied(),
            Some(IdentityLocation::OutOfWallet)
        );
        assert!(manager.identity(&identity_id).is_some());
    }

    #[test]
    fn remove_identity_drops_from_location_index() {
        let mut manager = IdentityManager::new();
        let wallet_id: WalletId = [9u8; 32];

        // Seed one entry in each bucket so removing one doesn't empty
        // the manager and we can be sure the surviving entry is left
        // alone.
        let wallet_owned_id = Identifier::from([1u8; 32]);
        let observed_id = Identifier::from([2u8; 32]);
        let p = noop_persister();
        manager
            .add_identity(create_test_identity(wallet_owned_id), 0, wallet_id, &p)
            .expect("add wallet-owned");
        manager
            .add_out_of_wallet_identity(create_test_identity(observed_id), &p)
            .expect("add observed");

        // Wallet-owned removal: index entry gone, lookup misses, the
        // other identity still there.
        manager
            .remove_identity(&wallet_owned_id, &p)
            .expect("remove wallet-owned");
        assert!(manager.location_index().get(&wallet_owned_id).is_none());
        assert!(manager.identity(&wallet_owned_id).is_none());
        assert!(manager.identity(&observed_id).is_some());

        // Out-of-wallet removal: same invariant on the other bucket.
        manager
            .remove_identity(&observed_id, &p)
            .expect("remove observed");
        assert!(manager.location_index().get(&observed_id).is_none());
        assert!(manager.identity(&observed_id).is_none());
        assert!(manager.location_index().is_empty());
    }

    /// The cross-launch half of dashpay/platform#4365: an incomplete scan
    /// verdict restored from the start state must still say "incomplete", or
    /// the next launch takes the warm shortcut over an identity set that was
    /// never fully probed.
    #[test]
    fn an_incomplete_scan_verdict_survives_a_restore() {
        use crate::changeset::{IdentityManagerStartState, IdentityScanStateEntry};

        let wallet: WalletId = [10u8; 32];
        let mut state = IdentityManagerStartState::default();
        state
            .scan_states
            .insert(wallet, IdentityScanStateEntry::incomplete(0, 5, vec![1]));

        let manager = IdentityManager::from(state);

        assert!(
            manager.identity_scan_is_incomplete(&wallet),
            "a restored partial scan must still force a rescan"
        );
        assert_eq!(
            manager
                .identity_scan_state(&wallet)
                .expect("verdict restored")
                .failed_indices,
            vec![1]
        );
    }

    /// The other side of it: a scan that answered everything restores as
    /// complete, so the warm-launch shortcut keeps working and a healthy
    /// wallet pays for no probes.
    #[test]
    fn a_complete_scan_verdict_permits_the_warm_shortcut() {
        use crate::changeset::{IdentityManagerStartState, IdentityScanStateEntry};

        let wallet: WalletId = [10u8; 32];
        let mut state = IdentityManagerStartState::default();
        state
            .scan_states
            .insert(wallet, IdentityScanStateEntry::completed(0, 6));

        let manager = IdentityManager::from(state);

        assert!(!manager.identity_scan_is_incomplete(&wallet));
    }

    /// "No verdict" is not "incomplete". Every wallet that predates this
    /// bookkeeping, and every host that does not persist the verdict yet,
    /// lands here — and forcing them all to rescan on every launch would cost
    /// a full gap-limit scan plus a Keychain round trip before every Core SPV
    /// start, which is the cost the warm shortcut exists to avoid.
    #[test]
    fn an_unknown_scan_verdict_does_not_force_a_rescan() {
        let manager = IdentityManager::new();

        assert!(!manager.identity_scan_is_incomplete(&[42u8; 32]));
        assert!(manager.identity_scan_state(&[42u8; 32]).is_none());
    }

    /// A rescan that covers an earlier scan's unanswered indices clears
    /// them — that is what lets a clean rescan hand the shortcut back.
    #[test]
    fn a_clean_rescan_clears_an_earlier_partial_verdict() {
        use crate::changeset::IdentityScanStateEntry;

        let wallet: WalletId = [10u8; 32];
        let mut manager = IdentityManager::new();

        manager.record_identity_scan(wallet, IdentityScanStateEntry::incomplete(0, 5, vec![1]));
        assert!(manager.identity_scan_is_incomplete(&wallet));

        // Clearing takes coverage: this rescan walked index 1 and answered it.
        manager.record_identity_scan(wallet, IdentityScanStateEntry::completed(0, 6));
        assert!(!manager.identity_scan_is_incomplete(&wallet));
    }

    /// The same gap, erased from the other side: a later scan may not clear
    /// an index it never probed.
    ///
    /// Discovery's default options resume one past the highest registered
    /// identity, so a wallet with identities at 0 and 2 and no answer at index
    /// 1 resumes at 3. Publishing that suffix scan's clean verdict over the
    /// recorded one dropped index 1, and the next launch took the warm
    /// shortcut and reported a settled identity set with the identity at 1 —
    /// and every contact it owns — still missing. That is the silent,
    /// permanent gap the verdict exists to record, reached through the
    /// bookkeeping meant to close it.
    #[test]
    fn a_clean_suffix_scan_cannot_erase_an_unresolved_index() {
        use crate::changeset::IdentityScanStateEntry;

        let wallet: WalletId = [10u8; 32];
        let mut manager = IdentityManager::new();

        // Identities at 0 and 2, index 1 never answered.
        manager.record_identity_scan(wallet, IdentityScanStateEntry::incomplete(0, 3, vec![1]));
        assert!(manager.identity_scan_is_incomplete(&wallet));

        // A later default scan resumes at 3 and answers 3..9 cleanly.
        let recorded =
            manager.record_identity_scan(wallet, IdentityScanStateEntry::completed(3, 9));

        assert!(
            manager.identity_scan_is_incomplete(&wallet),
            "a scan that never probed index 1 must not clear it"
        );
        assert_eq!(
            recorded.failed_indices,
            vec![1],
            "the gap must survive by name, so a later scan knows what to cover"
        );
        assert_eq!(
            manager
                .identity_scan_state(&wallet)
                .expect("verdict on record"),
            &recorded,
            "what is persisted is what is in memory"
        );

        // And a scan that DOES cover it hands the shortcut back — without
        // this the assertion above would pass against a verdict stuck on
        // incomplete forever.
        manager.record_identity_scan(wallet, IdentityScanStateEntry::completed(0, 9));
        assert!(!manager.identity_scan_is_incomplete(&wallet));
    }

    /// A scan abandoned before it answered anything records no index, so
    /// nothing carries its gap by name. Only a scan that starts at the bottom
    /// of the index space can be said to have covered whatever it never
    /// reached; a suffix scan must not hand the shortcut back over it.
    #[test]
    fn an_abandoned_scan_is_not_cleared_by_a_suffix_scan() {
        use crate::changeset::IdentityScanStateEntry;

        let wallet: WalletId = [10u8; 32];
        let mut manager = IdentityManager::new();

        // What the startup budget records when discovery is dropped mid-await.
        manager.record_identity_scan(wallet, IdentityScanStateEntry::incomplete(0, 0, Vec::new()));
        assert!(manager.identity_scan_is_incomplete(&wallet));

        manager.record_identity_scan(wallet, IdentityScanStateEntry::completed(3, 9));
        assert!(
            manager.identity_scan_is_incomplete(&wallet),
            "a scan that started at 3 says nothing about the indices below it"
        );

        manager.record_identity_scan(wallet, IdentityScanStateEntry::completed(0, 9));
        assert!(!manager.identity_scan_is_incomplete(&wallet));
    }

    /// An unlocated gap has no name, so nothing in `failed_indices` can carry
    /// it — and a fold that reads the fact back off that list loses it the
    /// moment an intermediate scan puts a name in there.
    ///
    /// The abandoned scan records no index at all. An incomplete suffix scan
    /// then folds in an unanswered index of its own, and what comes out is
    /// indistinguishable from an ordinary located gap: the next suffix scan
    /// covers that named index, finds a previous verdict whose failed list is
    /// non-empty, and hands the shortcut back — while no scan beginning at
    /// index 0 ever superseded the abandoned one. The launch after that skips
    /// discovery over an identity nobody has looked for, which is the gap the
    /// verdict exists to keep open.
    #[test]
    fn an_abandoned_gap_survives_an_incomplete_suffix_scan() {
        use crate::changeset::IdentityScanStateEntry;

        let wallet: WalletId = [10u8; 32];
        let mut manager = IdentityManager::new();

        // What the startup budget records when discovery is dropped
        // mid-await: no coverage, no named index, the gap could be anywhere.
        manager.record_identity_scan(wallet, IdentityScanStateEntry::incomplete(0, 0, Vec::new()));
        assert!(manager.identity_scan_is_incomplete(&wallet));

        // A suffix scan resumes at 3 and leaves index 4 unanswered. It says
        // nothing about the indices below it, so the abandoned gap has to ride
        // along — even though this fold now has a name of its own to carry.
        manager.record_identity_scan(wallet, IdentityScanStateEntry::incomplete(3, 5, vec![4]));
        assert!(manager.identity_scan_is_incomplete(&wallet));

        // The next suffix scan covers index 4 and answers it. The named gap is
        // legitimately gone; the unlocated one below index 3 is not.
        manager.record_identity_scan(wallet, IdentityScanStateEntry::completed(4, 6));
        assert!(
            manager.identity_scan_is_incomplete(&wallet),
            "nothing has started at index 0 since the abandoned scan, so this hands \
             the warm shortcut back over a region no scan ever covered"
        );

        // And a scan that DOES start at the bottom hands the shortcut back —
        // without this the assertion above would pass against a verdict stuck
        // on incomplete forever.
        manager.record_identity_scan(wallet, IdentityScanStateEntry::completed(0, 9));
        assert!(!manager.identity_scan_is_incomplete(&wallet));
    }

    /// Starting at index 0 is not the same as having covered the region an
    /// unlocated gap could be hiding in. A scan that begins at the bottom and
    /// is itself cut short walked only as far as it got, and everything above
    /// that is still the same unlooked-at space the abandoned scan left.
    ///
    /// Only the scan's own window can be credited: an incomplete from-zero
    /// scan carries the gap on, and a later suffix scan that answers the names
    /// it did leave behind must not read the list going empty as the unknown
    /// region having been covered.
    #[test]
    fn an_abandoned_gap_survives_an_incomplete_scan_from_index_zero() {
        use crate::changeset::IdentityScanStateEntry;

        let wallet: WalletId = [11u8; 32];
        let mut manager = IdentityManager::new();

        // Discovery dropped mid-await: no coverage, no named index, the gap
        // could be anywhere.
        manager.record_identity_scan(wallet, IdentityScanStateEntry::incomplete(0, 0, Vec::new()));
        assert!(manager.identity_scan_is_incomplete(&wallet));

        // A rescan does start at the bottom, but is cut off at index 2 with
        // index 1 unanswered. It covered 0..2 and nothing above, so the
        // abandoned gap is exactly as unlocated as it was.
        let recorded =
            manager.record_identity_scan(wallet, IdentityScanStateEntry::incomplete(0, 2, vec![1]));
        assert!(
            recorded.unlocated_gap,
            "a from-zero scan that never finished covered only the window it walked, \
             so the region above it is still the one nobody can point at"
        );
        assert!(manager.identity_scan_is_incomplete(&wallet));

        // A suffix scan answers index 1. That named gap is legitimately gone —
        // the unlocated one is not, and the failed list going empty must not
        // be read as it having been covered.
        manager.record_identity_scan(wallet, IdentityScanStateEntry::completed(1, 3));
        assert!(
            manager.identity_scan_is_incomplete(&wallet),
            "no scan has both started at index 0 and finished clean, so this hands \
             the warm shortcut back over a region no scan ever covered"
        );

        // A scan that starts at the bottom AND finishes clean does supersede
        // it — without this the assertion above would pass against a verdict
        // stuck on incomplete forever.
        manager.record_identity_scan(wallet, IdentityScanStateEntry::completed(0, 9));
        assert!(!manager.identity_scan_is_incomplete(&wallet));
    }

    #[test]
    fn from_start_state_rebuilds_location_index() {
        use crate::changeset::IdentityManagerStartState;
        use crate::wallet::identity::state::managed_identity::ManagedIdentity;

        let wallet_a: WalletId = [10u8; 32];
        let wallet_b: WalletId = [11u8; 32];

        let owned_a0 = Identifier::from([0xA0u8; 32]);
        let owned_a3 = Identifier::from([0xA3u8; 32]);
        let owned_b1 = Identifier::from([0xB1u8; 32]);
        let observed = Identifier::from([0xC0u8; 32]);

        let mk_owned = |id: Identifier, idx: u32, wid: WalletId| {
            let mut m = ManagedIdentity::new(create_test_identity(id), idx);
            m.wallet_id = Some(wid);
            m
        };
        let mk_observed =
            |id: Identifier| ManagedIdentity::new_out_of_wallet(create_test_identity(id));

        let mut start = IdentityManagerStartState::default();
        start
            .out_of_wallet_identities
            .insert(observed, mk_observed(observed));
        start.wallet_identities.insert(wallet_a, {
            let mut inner = std::collections::BTreeMap::new();
            inner.insert(0u32, mk_owned(owned_a0, 0, wallet_a));
            inner.insert(3u32, mk_owned(owned_a3, 3, wallet_a));
            inner
        });
        start.wallet_identities.insert(wallet_b, {
            let mut inner = std::collections::BTreeMap::new();
            inner.insert(1u32, mk_owned(owned_b1, 1, wallet_b));
            inner
        });

        let manager: IdentityManager = start.into();

        // Each entry in the buckets has a matching side-index row…
        assert_eq!(
            manager.location_index().get(&owned_a0).copied(),
            Some(IdentityLocation::InWallet {
                wallet_id: wallet_a,
                registration_index: 0,
            })
        );
        assert_eq!(
            manager.location_index().get(&owned_a3).copied(),
            Some(IdentityLocation::InWallet {
                wallet_id: wallet_a,
                registration_index: 3,
            })
        );
        assert_eq!(
            manager.location_index().get(&owned_b1).copied(),
            Some(IdentityLocation::InWallet {
                wallet_id: wallet_b,
                registration_index: 1,
            })
        );
        assert_eq!(
            manager.location_index().get(&observed).copied(),
            Some(IdentityLocation::OutOfWallet)
        );
        // …and lookups round-trip through the index into the right bucket.
        assert!(manager.identity(&owned_a0).is_some());
        assert!(manager.identity(&owned_a3).is_some());
        assert!(manager.identity(&owned_b1).is_some());
        assert!(manager.identity(&observed).is_some());
        assert_eq!(manager.identity_count(), 4);
    }

    #[test]
    fn lookup_by_id_finds_in_either_bucket() {
        let mut manager = IdentityManager::new();
        let wallet_id: WalletId = [42u8; 32];
        let owned = Identifier::from([1u8; 32]);
        let observed = Identifier::from([2u8; 32]);
        let p = noop_persister();

        manager
            .add_identity(create_test_identity(owned), 5, wallet_id, &p)
            .expect("add wallet-owned");
        manager
            .add_out_of_wallet_identity(create_test_identity(observed), &p)
            .expect("add observed");

        // Wallet-owned hits the wallet bucket and pulls back the right
        // denormalized fields.
        let owned_managed = manager.identity(&owned).expect("owned present");
        assert_eq!(owned_managed.wallet_id, Some(wallet_id));
        assert_eq!(owned_managed.identity_index, Some(5));
        assert_eq!(owned_managed.identity.id(), owned);

        // Out-of-wallet hits the other bucket — no wallet attached.
        let observed_managed = manager.identity(&observed).expect("observed present");
        assert!(observed_managed.wallet_id.is_none());
        assert_eq!(observed_managed.identity.id(), observed);

        // Wallet-scoped accessors are the signing/ownership boundary: they
        // include the owned identity and categorically exclude an observed
        // contact or an identity queried under another wallet id.
        assert!(manager.wallet_identity(&wallet_id, &owned).is_some());
        assert!(manager.wallet_identity(&wallet_id, &observed).is_none());
        assert!(manager.wallet_identity(&[43u8; 32], &owned).is_none());
        assert_eq!(manager.wallet_identity_ids(&wallet_id), vec![owned]);
        assert!(manager.wallet_identity_mut(&wallet_id, &observed).is_none());

        // Unknown ids miss cleanly.
        let unknown = Identifier::from([0xFFu8; 32]);
        assert!(manager.identity(&unknown).is_none());
    }

    #[test]
    fn test_highest_registration_index_advances() {
        let mut manager = IdentityManager::new();
        let wallet_id: WalletId = [7u8; 32];
        let p = noop_persister();

        manager
            .add_identity(
                create_test_identity(Identifier::from([1u8; 32])),
                0,
                wallet_id,
                &p,
            )
            .unwrap();
        manager
            .add_identity(
                create_test_identity(Identifier::from([2u8; 32])),
                5,
                wallet_id,
                &p,
            )
            .unwrap();
        manager
            .add_identity(
                create_test_identity(Identifier::from([3u8; 32])),
                3,
                wallet_id,
                &p,
            )
            .unwrap();

        assert_eq!(manager.highest_registration_index(&wallet_id), Some(5));
    }
}
