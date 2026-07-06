//! Lean startup snapshot for [`IdentityManager`](crate::wallet::identity::IdentityManager).
//!
//! Mirrors the persistable buckets of `IdentityManager` as a plain data
//! struct — no methods, no invariants, no live handles — so persisters
//! can round-trip it without dragging in the manager's business logic.

use std::collections::{BTreeMap, HashMap};

use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::Identifier;

use crate::changeset::{ContactChangeSet, IdentityKeysChangeSet};
use crate::wallet::identity::ManagedIdentity;
use crate::wallet::identity::RegistrationIndex;
use crate::wallet::platform_wallet::WalletId;

/// Restored [`IdentityManager`](crate::wallet::identity::IdentityManager)
/// state carried in [`ClientWalletStartState`](crate::changeset::ClientWalletStartState).
///
/// Two-bucket shape — see
/// [`IdentityManager`](crate::wallet::identity::IdentityManager) for
/// the layout rationale.
#[derive(Debug, Default)]
pub struct IdentityManagerStartState {
    /// Observed identities the client doesn't own keys for, keyed by
    /// identity id.
    pub out_of_wallet_identities: BTreeMap<Identifier, ManagedIdentity>,
    /// Wallet-owned identities, outer-keyed by wallet id and
    /// inner-keyed by BIP-9 registration index.
    pub wallet_identities: BTreeMap<WalletId, BTreeMap<RegistrationIndex, ManagedIdentity>>,
}

impl IdentityManagerStartState {
    /// Fold persisted PUBLIC keys and contact state onto the already-built
    /// managed identities so `Identity.public_keys` and the contact maps
    /// are populated at load time — the FFI persister's pre-keyed shape,
    /// with no separate changeset layered on afterwards.
    ///
    /// Entries route by owner `identity_id` across BOTH buckets; one whose
    /// owner is absent (e.g. a tombstoned identity's orphaned rows) is
    /// logged and skipped, never fatal. Only key `upserts` and the
    /// `sent` / `incoming` / `established` maps are routed; `removed_*`
    /// (insert-only feed) and `ignored` / `unignored` (restored in the
    /// identity reader from the `ignored_senders` table) are skipped. No
    /// `Network` needed — key insert is network-independent.
    pub fn merge_contacts_and_keys(
        &mut self,
        contacts: ContactChangeSet,
        identity_keys: IdentityKeysChangeSet,
    ) {
        // One transient id → &mut ManagedIdentity view over both buckets so
        // routing is O(1) per entry rather than a per-entry bucket scan. The
        // two buckets are disjoint fields, so their mutable borrows coexist.
        let mut by_id: HashMap<Identifier, &mut ManagedIdentity> = HashMap::new();
        for managed in self.out_of_wallet_identities.values_mut() {
            by_id.insert(managed.identity.id(), managed);
        }
        for inner in self.wallet_identities.values_mut() {
            for managed in inner.values_mut() {
                by_id.insert(managed.identity.id(), managed);
            }
        }

        for (_key, entry) in identity_keys.upserts {
            match by_id.get_mut(&entry.identity_id) {
                Some(managed) => managed.identity.add_public_key(entry.public_key),
                None => tracing::warn!(
                    identity = %entry.identity_id,
                    key_id = entry.key_id,
                    "skipping identity key during rehydration merge: owner identity not loaded"
                ),
            }
        }
        for (key, entry) in contacts.sent_requests {
            match by_id.get_mut(&key.owner_id) {
                Some(managed) => managed.apply_sent_contact_request(entry.request),
                None => tracing::warn!(
                    owner = %key.owner_id,
                    "skipping sent contact request during rehydration merge: owner identity not loaded"
                ),
            }
        }
        for (key, entry) in contacts.incoming_requests {
            match by_id.get_mut(&key.owner_id) {
                Some(managed) => managed.apply_incoming_contact_request(entry.request),
                None => tracing::warn!(
                    owner = %key.owner_id,
                    "skipping incoming contact request during rehydration merge: owner identity not loaded"
                ),
            }
        }
        for (key, established) in contacts.established {
            match by_id.get_mut(&key.owner_id) {
                Some(managed) => managed.apply_established_contact(established),
                None => tracing::warn!(
                    owner = %key.owner_id,
                    "skipping established contact during rehydration merge: owner identity not loaded"
                ),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changeset::{
        ContactRequestEntry, IdentityKeyEntry, ReceivedContactRequestKey, SentContactRequestKey,
    };
    use crate::wallet::identity::{ContactRequest, EstablishedContact};
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;

    fn identity(id_byte: u8) -> Identity {
        Identity::V0(IdentityV0 {
            id: Identifier::from([id_byte; 32]),
            public_keys: BTreeMap::new(),
            balance: 1_000,
            revision: 1,
        })
    }

    fn wallet_identity(state: &mut IdentityManagerStartState, w: WalletId, idx: u32, id_byte: u8) {
        let mut managed = ManagedIdentity::new(identity(id_byte), idx);
        managed.wallet_id = Some(w);
        state
            .wallet_identities
            .entry(w)
            .or_default()
            .insert(idx, managed);
    }

    fn key_entry(
        id_byte: u8,
        key_id: u32,
        data_byte: u8,
        security: SecurityLevel,
    ) -> IdentityKeyEntry {
        IdentityKeyEntry {
            identity_id: Identifier::from([id_byte; 32]),
            key_id,
            public_key: IdentityPublicKey::V0(IdentityPublicKeyV0 {
                id: key_id,
                purpose: Purpose::AUTHENTICATION,
                security_level: security,
                contract_bounds: None,
                key_type: KeyType::ECDSA_SECP256K1,
                read_only: false,
                data: BinaryData::new(vec![data_byte; 33]),
                disabled_at: None,
            }),
            public_key_hash: [data_byte; 20],
            wallet_id: None,
            derivation_indices: None,
        }
    }

    fn keys(entries: impl IntoIterator<Item = IdentityKeyEntry>) -> IdentityKeysChangeSet {
        let mut cs = IdentityKeysChangeSet::default();
        for e in entries {
            cs.upserts.insert((e.identity_id, e.key_id), e);
        }
        cs
    }

    fn request(sender: u8, recipient: u8) -> ContactRequest {
        ContactRequest {
            sender_id: Identifier::from([sender; 32]),
            recipient_id: Identifier::from([recipient; 32]),
            sender_key_index: 0,
            recipient_key_index: 0,
            account_reference: 0,
            encrypted_account_label: None,
            encrypted_public_key: vec![7; 96],
            auto_accept_proof: None,
            core_height_created_at: 11,
            created_at: 22,
        }
    }

    /// A key upsert whose owner is a wallet-bucket identity lands in that
    /// identity's `public_keys` map, keyed by `KeyID`.
    #[test]
    fn merge_routes_key_into_wallet_identity() {
        let w: WalletId = [0xAA; 32];
        let mut state = IdentityManagerStartState::default();
        wallet_identity(&mut state, w, 5, 0x01);

        state.merge_contacts_and_keys(
            ContactChangeSet::default(),
            keys([key_entry(0x01, 0, 0xAB, SecurityLevel::HIGH)]),
        );

        let managed = &state.wallet_identities[&w][&5];
        let pk = managed
            .identity
            .public_keys()
            .get(&0)
            .expect("key routed onto identity");
        use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
        assert_eq!(pk.data().as_slice(), &[0xAB; 33]);
    }

    /// Out-of-wallet identities are covered by the merge too — a naive
    /// implementation that only walked `wallet_identities` would drop
    /// their keys.
    #[test]
    fn merge_routes_key_into_out_of_wallet_identity() {
        let id = Identifier::from([0x02; 32]);
        let mut state = IdentityManagerStartState::default();
        state
            .out_of_wallet_identities
            .insert(id, ManagedIdentity::new_out_of_wallet(identity(0x02)));

        state.merge_contacts_and_keys(
            ContactChangeSet::default(),
            keys([key_entry(0x02, 3, 0xCD, SecurityLevel::CRITICAL)]),
        );

        assert!(state.out_of_wallet_identities[&id]
            .identity
            .public_keys()
            .contains_key(&3));
    }

    /// Sent / incoming / established contacts each route to their own map.
    #[test]
    fn merge_routes_contacts_to_correct_maps() {
        let w: WalletId = [0xAA; 32];
        let owner = 0x01;
        let mut state = IdentityManagerStartState::default();
        wallet_identity(&mut state, w, 0, owner);

        let owner_id = Identifier::from([owner; 32]);
        let mut contacts = ContactChangeSet::default();
        contacts.sent_requests.insert(
            SentContactRequestKey {
                owner_id,
                recipient_id: Identifier::from([0x22; 32]),
            },
            ContactRequestEntry {
                request: request(owner, 0x22),
            },
        );
        contacts.incoming_requests.insert(
            ReceivedContactRequestKey {
                owner_id,
                sender_id: Identifier::from([0x33; 32]),
            },
            ContactRequestEntry {
                request: request(0x33, owner),
            },
        );
        let contact_c = Identifier::from([0x44; 32]);
        contacts.established.insert(
            SentContactRequestKey {
                owner_id,
                recipient_id: contact_c,
            },
            EstablishedContact {
                contact_identity_id: contact_c,
                outgoing_request: request(owner, 0x44),
                incoming_request: request(0x44, owner),
                alias: Some("c".into()),
                note: None,
                is_hidden: false,
                accepted_accounts: vec![1],
                payment_channel_broken: false,
                contact_account_label: None,
                external_account_reference: None,
            },
        );

        state.merge_contacts_and_keys(contacts, IdentityKeysChangeSet::default());

        let managed = &state.wallet_identities[&w][&0];
        assert!(managed
            .dashpay()
            .sent_contact_requests()
            .contains_key(&Identifier::from([0x22; 32])));
        assert!(managed
            .dashpay()
            .incoming_contact_requests()
            .contains_key(&Identifier::from([0x33; 32])));
        assert!(managed
            .dashpay()
            .established_contacts()
            .contains_key(&contact_c));
    }

    /// Two identities with the same numeric `KeyID` but different owners
    /// keep disjoint key maps — the group-by must not misattribute.
    #[test]
    fn merge_does_not_leak_keys_across_identities() {
        let w: WalletId = [0xAA; 32];
        let mut state = IdentityManagerStartState::default();
        wallet_identity(&mut state, w, 0, 0x01);
        wallet_identity(&mut state, w, 1, 0x02);

        state.merge_contacts_and_keys(
            ContactChangeSet::default(),
            keys([
                key_entry(0x01, 0, 0xA0, SecurityLevel::HIGH),
                key_entry(0x02, 0, 0xB0, SecurityLevel::HIGH),
            ]),
        );

        use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
        let a = &state.wallet_identities[&w][&0];
        let b = &state.wallet_identities[&w][&1];
        assert_eq!(a.identity.public_keys().len(), 1);
        assert_eq!(b.identity.public_keys().len(), 1);
        assert_eq!(a.identity.public_keys()[&0].data().as_slice(), &[0xA0; 33]);
        assert_eq!(b.identity.public_keys()[&0].data().as_slice(), &[0xB0; 33]);
    }

    /// An entry whose owner is absent from both buckets is logged and
    /// skipped — never a panic.
    #[test]
    fn merge_skips_orphan_entries() {
        let w: WalletId = [0xAA; 32];
        let mut state = IdentityManagerStartState::default();
        wallet_identity(&mut state, w, 0, 0x01);

        let mut contacts = ContactChangeSet::default();
        contacts.sent_requests.insert(
            SentContactRequestKey {
                owner_id: Identifier::from([0xEE; 32]),
                recipient_id: Identifier::from([0xFF; 32]),
            },
            ContactRequestEntry {
                request: request(0xEE, 0xFF),
            },
        );
        // Key for an identity that isn't present in either bucket.
        state.merge_contacts_and_keys(
            contacts,
            keys([key_entry(0xEE, 0, 0x01, SecurityLevel::HIGH)]),
        );

        let managed = &state.wallet_identities[&w][&0];
        assert!(managed.identity.public_keys().is_empty());
        assert!(managed.dashpay().sent_contact_requests().is_empty());
    }

    /// Empty changesets are a no-op.
    #[test]
    fn merge_empty_changesets_is_noop() {
        let w: WalletId = [0xAA; 32];
        let mut state = IdentityManagerStartState::default();
        wallet_identity(&mut state, w, 0, 0x01);

        state.merge_contacts_and_keys(
            ContactChangeSet::default(),
            IdentityKeysChangeSet::default(),
        );

        let managed = &state.wallet_identities[&w][&0];
        assert!(managed.identity.public_keys().is_empty());
        assert!(managed.dashpay().sent_contact_requests().is_empty());
        assert!(managed.dashpay().incoming_contact_requests().is_empty());
        assert!(managed.dashpay().established_contacts().is_empty());
    }
}
