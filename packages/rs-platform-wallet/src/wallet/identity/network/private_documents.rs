//! The entries of any private document type (protocol version 15) a wallet's
//! identities can open.
//!
//! A private type's store holds fixed-size entries in the standard private
//! document envelope, each sealed to one key of one identity, and names nobody
//! in the clear. A wallet finds its own by trying every entry: with the keys
//! its identities may be sealed to, and with the outgoing viewing keys they
//! seal with, which recover what they sent.
//! [`IdentityWallet::read_private_document_entries`] reads a store that way,
//! from a [`PrivateDocumentCursor`] the caller keeps, and reads each opened
//! entry's payload against the type's schema.
//!
//! Which keys those are is decided here for every private type
//! ([`PrivateDocumentKeyPaths::add_identity`]), and each is checked to derive
//! its registered key before it is scanned with. DashPay's private contact
//! request sync keeps its own key rule, as its reads gate sends and replies.
//! The keys are derived inside the [`ContactCryptoProvider`], which keeps
//! them.
//!
//! The read keeps nothing: no cursor, no entry. What an entry means, and
//! whether the sender it names wrote it, is its type's rule; DashPay's is in
//! [`DashPayView::sync_private_contact_requests`](super::DashPayView::sync_private_contact_requests).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use dash_sdk::platform::private_document_entries::PrivateDocumentEntries;
use dash_sdk::platform::private_document_scan::{
    read_private_document_payload, PrivateDocumentPage, PrivateDocumentScanKeys,
    RejectedPrivateDocumentEntry, MAX_CONCURRENT_PRIVATE_DOCUMENT_PAGE_FETCHES,
};
use dash_sdk::platform::{ContextProvider, Fetch};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::data_contract::DataContract;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, KeyType, Purpose};
use dpp::platform_value::Value;
use dpp::prelude::Identifier;
use dpp::secp256k1::{PublicKey, SecretKey};
use futures::StreamExt;
use key_wallet::bip32::{DerivationPath, KeyDerivationType};
use key_wallet::Network;
use platform_encryption::{OpenedPrivateDocumentEntry, PrivateDocumentAuthMode};
use sha2::{Digest, Sha256};

use super::{identity_auth_derivation_path_for_type, ContactCryptoProvider, IdentityWallet};
use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;

/// The paths of the keys a wallet's identities read a private document type's
/// store with, for a [`ContactCryptoProvider`] to derive inside the signer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PrivateDocumentKeyPaths {
    /// Each identity entries may be sealed to, with the path of each of its
    /// keys an entry may be sealed to, by key id.
    pub recipients: Vec<(Identifier, Vec<(KeyID, DerivationPath)>)>,
    /// Each identity that may have sealed entries, with the id and path of
    /// each of its encryption keys: one outgoing viewing key each.
    pub senders: Vec<(Identifier, KeyID, DerivationPath)>,
}

impl PrivateDocumentKeyPaths {
    /// The scan keys these paths derive to, `derive` deriving the secret key at
    /// a path: for a provider implementation to build inside the signer and
    /// drop there.
    pub fn scan_keys<E>(
        &self,
        derive: impl FnMut(&DerivationPath) -> Result<SecretKey, E>,
    ) -> Result<PrivateDocumentScanKeys, E> {
        PrivateDocumentScanKeys::derive(&self.recipients, &self.senders, derive)
    }

    /// Adds `identity`, held at the HD index `identity_index` of a wallet on
    /// `network`, with the paths of its keys: every key an entry can be sealed
    /// to ([`scanning_key`]) as a recipient key, and every such `ENCRYPTION`
    /// key as a sender's, one outgoing viewing key each. A key whose id names
    /// no hardened slot (`2^31` or above) has no path and is left out, and
    /// reported: the wallet derives no key there. An identity index that
    /// names no hardened slot fails, as it is the wallet's own record.
    pub(super) fn add_identity(
        &mut self,
        network: Network,
        identity_index: u32,
        identity: &Identity,
    ) -> Result<AddedIdentity, PlatformWalletError> {
        let identity_id = identity.id();
        // The identity's own slot: a failure here is the wallet's record, not
        // a key the read can leave out.
        identity_auth_derivation_path_for_type(
            network,
            KeyDerivationType::ECDSA,
            identity_index,
            0,
        )?;
        let mut recipient_paths = Vec::new();
        let mut sender_paths = Vec::new();
        let mut keys_without_slot = Vec::new();
        for (key_id, key) in identity.public_keys() {
            if !scanning_key(key) {
                continue;
            }
            let Ok(path) = identity_auth_derivation_path_for_type(
                network,
                KeyDerivationType::ECDSA,
                identity_index,
                *key_id,
            ) else {
                keys_without_slot.push(*key_id);
                continue;
            };
            if key.purpose() == Purpose::ENCRYPTION {
                sender_paths.push((identity_id, *key_id, path.clone()));
            }
            recipient_paths.push((*key_id, path));
        }
        let added = !recipient_paths.is_empty();
        if added {
            self.recipients.push((identity_id, recipient_paths));
            self.senders.extend(sender_paths);
        }
        Ok(AddedIdentity {
            added,
            keys_without_slot,
        })
    }

    /// A fingerprint of these paths: every identity, key id and path, as a
    /// recipient and as a sender. Paths with the same fingerprint open the
    /// same entries, so a position read with one set holds nothing more for
    /// the other.
    pub fn fingerprint(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update((self.recipients.len() as u64).to_le_bytes());
        for (identity_id, keys) in &self.recipients {
            hasher.update(identity_id.as_bytes());
            hasher.update((keys.len() as u64).to_le_bytes());
            for (key_id, path) in keys {
                hasher.update(key_id.to_le_bytes());
                hash_path(&mut hasher, path);
            }
        }
        hasher.update((self.senders.len() as u64).to_le_bytes());
        for (identity_id, key_id, path) in &self.senders {
            hasher.update(identity_id.as_bytes());
            hasher.update(key_id.to_le_bytes());
            hash_path(&mut hasher, path);
        }
        hasher.finalize().into()
    }
}

/// Adds `path` to `hasher`, length-prefixed so two paths cannot run together.
fn hash_path(hasher: &mut Sha256, path: &DerivationPath) {
    let path = path.to_string();
    hasher.update((path.len() as u64).to_le_bytes());
    hasher.update(path.as_bytes());
}

/// What [`PrivateDocumentKeyPaths::add_identity`] did with an identity.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct AddedIdentity {
    /// Whether it was added: it has a key, with a path, an entry can be
    /// sealed to.
    pub(super) added: bool,
    /// Its keys an entry can be sealed to whose id names no hardened slot:
    /// they have no path and were left out.
    pub(super) keys_without_slot: Vec<KeyID>,
}

/// Whether `key` is one a private document entry can be sealed to, which a
/// wallet scans a private type's store with: an `ECDSA_SECP256K1` `DECRYPTION`
/// or `ENCRYPTION` key, disabled ones included, as an entry sealed before a
/// key was disabled still opens with it. The rule of this read, for every
/// private type: not the policy for the contact requests DashPay seals, which
/// may change on its own.
fn scanning_key(key: &IdentityPublicKey) -> bool {
    key.key_type() == KeyType::ECDSA_SECP256K1
        && matches!(key.purpose(), Purpose::DECRYPTION | Purpose::ENCRYPTION)
}

/// The public key data of each key of the wallet's identities an entry may be
/// sealed to, by identity and key id.
type RegisteredKeys = BTreeMap<(Identifier, KeyID), Vec<u8>>;

/// The keys the wallet's identities read a private document type's store
/// with, before any is checked against its registered key.
struct KeyPathsOfWallet {
    /// The identities with a key that has a path.
    identities: Vec<Identifier>,
    paths: PrivateDocumentKeyPaths,
    registered: RegisteredKeys,
    /// The keys an entry can be sealed to whose id names no slot.
    keys_without_slot: Vec<(Identifier, KeyID)>,
}

/// How many fetched pages a read scans at once, one key derivation for them
/// all: at most the pages in flight.
const PRIVATE_DOCUMENT_SCAN_BATCH_PAGES: usize = MAX_CONCURRENT_PRIVATE_DOCUMENT_PAGE_FETCHES;

/// Which way an opened entry went.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrivateDocumentDirection {
    /// Sealed to a key of one of the wallet's identities, which opened it.
    Received,
    /// Sealed by one of the wallet's identities, whose outgoing viewing key
    /// recovered it.
    Sent,
}

/// An entry of a private document type's store one of the wallet's identities
/// opened or recovered. An entry from one of the wallet's identities to
/// another is read twice, once each way.
#[derive(Clone, Debug, PartialEq)]
pub struct OpenedPrivateDocument {
    /// The entry's position in the store.
    pub position: u64,
    /// The height of the block that appended it.
    pub block_height: u64,
    /// The time of the block that appended it, in milliseconds.
    pub block_time_ms: u64,
    /// Which way it went.
    pub direction: PrivateDocumentDirection,
    /// The wallet's identity that opened it, when received, or whose outgoing
    /// viewing key recovered it, when sent.
    pub wallet_identity_id: Identifier,
    /// That identity's key: the one the entry is sealed to, when received, or
    /// the one the outgoing viewing key is derived from, when sent.
    pub wallet_key_id: KeyID,
    /// The identity the entry is sealed to.
    pub recipient_identity_id: Identifier,
    /// How the entry tells its recipient who wrote it.
    pub auth_mode: PrivateDocumentAuthMode,
    /// The identity and key the entry names as its sender, `None` when it is
    /// anonymous. A claim: this read does not check it against the sender
    /// identity's keys, which is the type's rule to set.
    pub claimed_sender: Option<(Identifier, KeyID)>,
    /// The entry's commitment: two entries with the same one are copies.
    pub commitment: [u8; 32],
    /// The properties its payload holds, decoded and checked against the
    /// type's schema, or why they are refused. The contract is fetched once a
    /// read, before its first page: a refusal of an entry appended after that
    /// fetch may come from a newer version of the type, which a later read
    /// from the store's start reads under the newer contract.
    pub properties: Result<BTreeMap<String, Value>, String>,
}

/// Where reading a private document type's store goes on from: the caller
/// keeps it between reads ([`IdentityWallet::read_private_document_entries`])
/// and hands it back as it got it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PrivateDocumentCursor {
    /// The position the next read starts at.
    pub position: u64,
    /// The largest store size a page proved, in the reads this cursor comes
    /// from; `None` before any page was read. A later page from a node
    /// behind shows an older, smaller store: the store holds at least this
    /// many entries, so a read reaches its end only there.
    pub proved_total_count: Option<u64>,
    /// The fingerprint of the keys those reads scanned with
    /// ([`PrivateDocumentKeyPaths::fingerprint`]). When the wallet's keys no
    /// longer match it, the next read starts over from the store's start:
    /// the positions read before may hold entries the new keys open.
    pub key_set: [u8; 32],
    /// The store those reads read, on its network
    /// ([`private_document_store_fingerprint`]): a cursor is refused by a
    /// read of another store, whose position and size it does not describe.
    pub store: [u8; 32],
}

/// A fingerprint of the store of the private type `document_type_name` of
/// `data_contract_id` on `network`, which a [`PrivateDocumentCursor`] names.
/// The network is named: a system contract such as DashPay has the same id on
/// every network.
pub fn private_document_store_fingerprint(
    network: Network,
    data_contract_id: &Identifier,
    document_type_name: &str,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"Dash/PrivateDocumentStore");
    let network = network.to_string();
    hasher.update((network.len() as u64).to_le_bytes());
    hasher.update(network.as_bytes());
    hasher.update(data_contract_id.as_bytes());
    hasher.update((document_type_name.len() as u64).to_le_bytes());
    hasher.update(document_type_name.as_bytes());
    hasher.finalize().into()
}

/// What a read of a private document type's store found for the wallet
/// ([`IdentityWallet::read_private_document_entries`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PrivateDocumentRead {
    /// The wallet's identities the store was read for: those with an HD index
    /// and a key an entry can be sealed to. With none, nothing was read.
    pub identities: Vec<Identifier>,
    /// The entries opened or recovered, in store order; at one position, the
    /// received one first.
    pub entries: Vec<OpenedPrivateDocument>,
    /// Entries that decrypted under one of the wallet's keys but are refused.
    pub rejected: Vec<RejectedPrivateDocumentEntry>,
    /// Keys of the wallet's identities an entry could be sealed to that the
    /// wallet does not derive at their identity key slot, such as a DashPay
    /// Connect application key: the read did not scan with them, so no entry
    /// sealed to them is in it, whether or not the read is complete.
    pub unscanned_keys: Vec<(Identifier, KeyID)>,
    /// The store pages read.
    pub pages_read: usize,
    /// Where to read on from: hand it to the next read.
    pub cursor: PrivateDocumentCursor,
    /// The wallet's keys changed since the cursor the read was given, so it
    /// read from the store's start: its entries replace those of the earlier
    /// reads rather than follow them.
    pub restarted: bool,
    /// Whether the read reached the store's last entry, as large a store as
    /// any page proved, with every key but [`Self::unscanned_keys`]. A later
    /// read from [`Self::cursor`] reads the entries appended since. `false`
    /// with no page read means nothing was read, and reading again changes
    /// nothing until the wallet's keys or the budget do: the page budget was
    /// 0, or no identity has a key the wallet can scan with. See
    /// [`IdentityWallet::read_private_document_entries`] for which of those
    /// reads asked the signer.
    pub complete: bool,
}

/// An entry of a page one of the wallet's identities opened or recovered,
/// before its payload is read.
struct PageEntry<'a> {
    position: u64,
    block_height: u64,
    block_time_ms: u64,
    direction: PrivateDocumentDirection,
    wallet_identity_id: Identifier,
    wallet_key_id: KeyID,
    entry: &'a OpenedPrivateDocumentEntry,
}

impl PageEntry<'_> {
    /// The entry as the read reports it, with `properties` read from its
    /// payload.
    fn into_document(
        self,
        properties: Result<BTreeMap<String, Value>, String>,
    ) -> OpenedPrivateDocument {
        let auth_mode = self.entry.auth_mode();
        OpenedPrivateDocument {
            position: self.position,
            block_height: self.block_height,
            block_time_ms: self.block_time_ms,
            direction: self.direction,
            wallet_identity_id: self.wallet_identity_id,
            wallet_key_id: self.wallet_key_id,
            recipient_identity_id: Identifier::from(*self.entry.recipient_identity_id()),
            auth_mode,
            claimed_sender: (auth_mode != PrivateDocumentAuthMode::Anonymous).then(|| {
                (
                    Identifier::from(*self.entry.sender_identity_id()),
                    self.entry.sender_key_id(),
                )
            }),
            commitment: *self.entry.commitment(),
            properties,
        }
    }
}

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// The paths of the keys the wallet's identities read a private document
    /// type's store with: those of every identity with an HD index, whose keys
    /// the signer can derive, and a key an entry can be sealed to
    /// ([`PrivateDocumentKeyPaths::add_identity`]). With them, the identities
    /// they are of.
    async fn private_document_key_paths(&self) -> Result<KeyPathsOfWallet, PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let info = wm
            .get_wallet_info(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        let mut identities = Vec::new();
        let mut paths = PrivateDocumentKeyPaths::default();
        let mut registered = RegisteredKeys::new();
        let mut keys_without_slot = Vec::new();
        // The identities this wallet owns, not those of another wallet the
        // manager holds or the contacts it observes.
        for managed in info
            .identity_manager
            .wallet_managed_identities(&self.wallet_id)
        {
            let Some(identity_index) = managed.identity_index else {
                continue;
            };
            let identity = &managed.identity;
            let identity_id = identity.id();
            for (key_id, key) in identity.public_keys() {
                if scanning_key(key) {
                    registered.insert((identity_id, *key_id), key.data().to_vec());
                }
            }
            let added = paths.add_identity(self.sdk.network, identity_index, identity)?;
            keys_without_slot.extend(
                added
                    .keys_without_slot
                    .into_iter()
                    .map(|key_id| (identity_id, key_id)),
            );
            if added.added {
                identities.push(identity_id);
            }
        }
        Ok(KeyPathsOfWallet {
            identities,
            paths,
            registered,
            keys_without_slot,
        })
    }

    /// `paths` with only the keys whose path derives, inside `crypto`, the
    /// public key the identity registered under that key id, and every key of
    /// `registered` left out: one registered from another path, such as a
    /// DashPay Connect application key, derives some other key at its
    /// identity key slot, which would open nothing sealed to it, and one whose
    /// id names no slot has no path at all. They are reported, not scanned
    /// with. An identity left with no key is dropped. The public keys are
    /// derived in one call, so a provider resolves its seed once.
    async fn keys_the_wallet_derives<C>(
        crypto: &C,
        paths: PrivateDocumentKeyPaths,
        registered: &RegisteredKeys,
    ) -> Result<(PrivateDocumentKeyPaths, Vec<(Identifier, KeyID)>), PlatformWalletError>
    where
        C: ContactCryptoProvider + Sync,
    {
        let candidate_paths: Vec<DerivationPath> = paths
            .recipients
            .iter()
            .flat_map(|(_, keys)| keys.iter().map(|(_, path)| path.clone()))
            .collect();
        let public_keys = crypto.extended_public_keys(&candidate_paths).await?;
        if public_keys.len() != candidate_paths.len() {
            return Err(PlatformWalletError::InvalidIdentityData(format!(
                "the crypto provider derived {} public keys for {} paths",
                public_keys.len(),
                candidate_paths.len()
            )));
        }
        let mut public_keys = public_keys.into_iter();
        let mut derived = PrivateDocumentKeyPaths::default();
        let mut scanned: BTreeSet<(Identifier, KeyID)> = BTreeSet::new();
        for (identity_id, keys) in paths.recipients {
            let mut kept = Vec::with_capacity(keys.len());
            for (key_id, path) in keys {
                let Some(public_key) = public_keys.next() else {
                    break;
                };
                let derived_key = public_key.public_key.to_compressed();
                let matches = registered
                    .get(&(identity_id, key_id))
                    .and_then(|data| PublicKey::from_slice(data).ok())
                    .is_some_and(|registered| registered.serialize()[..] == derived_key[..]);
                if matches {
                    scanned.insert((identity_id, key_id));
                    kept.push((key_id, path));
                }
            }
            if !kept.is_empty() {
                derived.recipients.push((identity_id, kept));
            }
        }
        let unscanned: Vec<(Identifier, KeyID)> = registered
            .keys()
            .filter(|key| !scanned.contains(key))
            .copied()
            .collect();
        derived.senders = paths
            .senders
            .into_iter()
            .filter(|(identity_id, key_id, _)| scanned.contains(&(*identity_id, *key_id)))
            .collect();
        Ok((derived, unscanned))
    }

    /// The contract `data_contract_id` as the network proves it now, when it is
    /// newer than `held` and still declares `document_type_name` private, and
    /// `None` otherwise. A type can gain optional properties, and a payload
    /// written under the newer type does not read under an older one, which
    /// DPP refuses for the reader to fetch the contract again. The context
    /// provider is given the newer contract, so it verifies the next pages.
    async fn newer_private_document_contract(
        &self,
        held: &DataContract,
        data_contract_id: Identifier,
        document_type_name: &str,
    ) -> Result<Option<Arc<DataContract>>, PlatformWalletError> {
        let Some(fetched) = DataContract::fetch(&self.sdk, data_contract_id).await? else {
            return Ok(None);
        };
        let declares_type = fetched
            .document_type_for_name(document_type_name)
            .is_ok_and(|document_type| document_type.is_private());
        if fetched.version() <= held.version() || !declares_type {
            return Ok(None);
        }
        let fetched = Arc::new(fetched);
        if let Some(provider) = self.sdk.context_provider() {
            provider.register_data_contract(Arc::clone(&fetched));
        }
        Ok(Some(fetched))
    }

    /// Read the store of the private document type `document_type_name` of
    /// `data_contract_id` for the wallet's identities, at most `max_pages`
    /// proved pages from `from` (the store's start when `None`): the entries
    /// sealed to them, opened, and the entries they sealed, recovered from the
    /// store alone, each with its payload read against the type's schema.
    ///
    /// The keys are derived inside `crypto` and stay there. Nothing is kept:
    /// the caller hands [`PrivateDocumentRead::cursor`] to the next read, and
    /// a store only grows, so a later read from there holds what was appended
    /// since. The cursor carries the largest store size proved so far, so a
    /// later page from a node behind does not end the reading early, and the
    /// keys it was read with: when they changed, the read starts over from the
    /// store's start and says so ([`PrivateDocumentRead::restarted`]). A failed
    /// page fetch fails the read; the caller reads again from the cursor it
    /// had.
    ///
    /// Every read that reads a page fetches the contract once, before the
    /// first page, and reads every payload under the newer of it and the one
    /// held: the type may have gained properties since. It is fetched on every
    /// such read, whatever the pages hold for the wallet, so the requests a
    /// node sees do not show whether an entry opened.
    ///
    /// Each key is scanned with only once the public key its identity key
    /// slot derives inside `crypto` is the one registered: a key registered
    /// from another path, such as a DashPay Connect application key, is left
    /// out and listed in [`PrivateDocumentRead::unscanned_keys`], rather than
    /// scanned with a key that opens nothing sealed to it.
    ///
    /// A cursor of another store is refused with
    /// [`PlatformWalletError::InvalidParameter`].
    ///
    /// An entry's sender is what the entry claims
    /// ([`OpenedPrivateDocument::claimed_sender`]): whether that identity
    /// wrote it is not checked here.
    ///
    /// Refused with [`PlatformWalletError::SeedMismatch`], before any page is
    /// read, when `crypto` does not resolve this wallet's seed: another seed
    /// derives valid but unrelated keys, which open nothing, so the read
    /// would show a store holding nothing for the wallet. Refused with the
    /// SDK's `ProtocolVersionNotActive` below protocol version 15, and when
    /// the contract declares no such private type.
    ///
    /// Two reads read no page. With `max_pages` 0, or no identity holding a
    /// key with a path, it returns before the signer, the seed check, the
    /// protocol version check and any request: its success shows none of
    /// them. When keys have paths but none derives its registered key, it runs
    /// the seed check and the key derivation in the signer first, so it fails
    /// as they fail (a locked resolver, another seed), and then returns before
    /// the protocol version check and any request to the store.
    pub async fn read_private_document_entries<C>(
        &self,
        crypto: &C,
        data_contract_id: Identifier,
        document_type_name: &str,
        from: Option<PrivateDocumentCursor>,
        max_pages: usize,
    ) -> Result<PrivateDocumentRead, PlatformWalletError>
    where
        C: ContactCryptoProvider + Sync,
    {
        let store = private_document_store_fingerprint(
            self.sdk.network,
            &data_contract_id,
            document_type_name,
        );
        if from.is_some_and(|cursor| cursor.store != store) {
            return Err(PlatformWalletError::InvalidParameter(format!(
                "the cursor was read from another store than the private type \
                 \"{document_type_name}\" of contract {data_contract_id}"
            )));
        }
        let KeyPathsOfWallet {
            identities,
            paths,
            registered,
            keys_without_slot,
        } = self.private_document_key_paths().await?;
        if identities.is_empty() || max_pages == 0 {
            // Nothing is read, and the signer is not asked: the cursor goes
            // back as it came, with the keys known unscanned without it.
            return Ok(PrivateDocumentRead {
                identities,
                unscanned_keys: keys_without_slot,
                cursor: from.unwrap_or(PrivateDocumentCursor {
                    key_set: paths.fingerprint(),
                    store,
                    ..Default::default()
                }),
                ..Default::default()
            });
        }
        self.verify_seed_binds(crypto).await?;
        let (paths, unscanned_keys) =
            Self::keys_the_wallet_derives(crypto, paths, &registered).await?;
        let key_set = paths.fingerprint();
        let restarted = from.is_some_and(|cursor| cursor.key_set != key_set);
        let start_position = from
            .filter(|cursor| cursor.key_set == key_set)
            .map_or(0, |cursor| cursor.position);
        let mut read = PrivateDocumentRead {
            identities: paths
                .recipients
                .iter()
                .map(|(identity_id, _)| *identity_id)
                .collect(),
            unscanned_keys,
            cursor: PrivateDocumentCursor {
                position: start_position,
                // The store's size does not depend on the keys.
                proved_total_count: from.and_then(|cursor| cursor.proved_total_count),
                key_set,
                store,
            },
            restarted,
            ..Default::default()
        };
        if read.identities.is_empty() {
            return Ok(read);
        }
        // The contract the pages are verified against, with the type, which
        // each payload is read against.
        let mut contract = self
            .sdk
            .fetch_private_document_contract(data_contract_id, document_type_name)
            .await?;
        // Fetched again on every read, whatever opens: a fetch only when a
        // payload is refused would tell the node that the reader opened an
        // entry. Also when the resolver above has just fetched it, on a cold
        // cache: skipping it then would rest on a look at the cache that
        // another operation, or the resolver's own protocol version gate, can
        // change before the resolver looks.
        if let Some(newer) = self
            .newer_private_document_contract(&contract, data_contract_id, document_type_name)
            .await?
        {
            contract = newer;
        }
        let platform_version = self.sdk.version();

        // The pages are fetched many at once while earlier ones are scanned,
        // in batches of those ready, one key derivation a batch; no more at
        // once than the read takes, so few are fetched only to be dropped.
        let mut batches = std::pin::pin!(self
            .sdk
            .private_document_pages(
                data_contract_id,
                document_type_name,
                start_position,
                max_pages.min(MAX_CONCURRENT_PRIVATE_DOCUMENT_PAGE_FETCHES),
            )
            .take(max_pages)
            .ready_chunks(PRIVATE_DOCUMENT_SCAN_BATCH_PAGES));
        while let Some(batch) = batches.next().await {
            let fetched = batch
                .into_iter()
                .collect::<Result<Vec<PrivateDocumentPage>, _>>()?;
            let pages: Vec<(u64, &PrivateDocumentEntries)> = fetched
                .iter()
                .map(|page| (page.start_position, &page.entries))
                .collect();
            let scans = crypto
                .scan_private_document_pages(&paths, data_contract_id, document_type_name, &pages)
                .await?;
            if scans.len() != fetched.len() {
                return Err(PlatformWalletError::InvalidIdentityData(format!(
                    "the crypto provider sorted {} pages into {} scans",
                    fetched.len(),
                    scans.len()
                )));
            }
            for scan in scans {
                if !scan.is_of_store(&data_contract_id, document_type_name) {
                    return Err(PlatformWalletError::InvalidIdentityData(format!(
                        "the crypto provider scanned a page of the store of \"{}\" of contract \
                         {}, not of \"{document_type_name}\" of contract {data_contract_id}",
                        scan.document_type_name, scan.contract_id
                    )));
                }
                read.pages_read += 1;
                read.cursor.position = scan.next_position;
                // A later page from a node behind shows an older, smaller
                // store: the largest size proved is the store's.
                read.cursor.proved_total_count = Some(
                    read.cursor
                        .proved_total_count
                        .map_or(scan.total_count, |proved| proved.max(scan.total_count)),
                );
                let received = scan.received.iter().map(|received| PageEntry {
                    position: received.position,
                    block_height: received.block_height,
                    block_time_ms: received.block_time_ms,
                    direction: PrivateDocumentDirection::Received,
                    wallet_identity_id: received.recipient_identity_id,
                    wallet_key_id: received.recipient_key_id,
                    entry: &received.entry,
                });
                let sent = scan.sent.iter().map(|sent| PageEntry {
                    position: sent.position,
                    block_height: sent.block_height,
                    block_time_ms: sent.block_time_ms,
                    direction: PrivateDocumentDirection::Sent,
                    wallet_identity_id: sent.sender_identity_id,
                    wallet_key_id: sent.outgoing_key_id,
                    entry: &sent.entry,
                });
                let mut page_entries: Vec<PageEntry> = received.chain(sent).collect();
                // Pages come in store order, so only this page's entries are
                // put in order: the received one first at a position.
                page_entries.sort_by_key(|entry| (entry.position, entry.direction));
                for page_entry in page_entries {
                    let properties = read_private_document_payload(
                        &contract,
                        document_type_name,
                        page_entry.entry.payload(),
                        platform_version,
                    );
                    read.entries.push(page_entry.into_document(properties));
                }
                read.rejected.extend(scan.rejected);
            }
        }
        read.complete = read.pages_read > 0
            && read
                .cursor
                .proved_total_count
                .is_some_and(|total| read.cursor.position >= total);
        Ok(read)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changeset::{
        ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
    };
    use crate::events::{EventHandler, PlatformEventHandler};
    use crate::wallet::identity::network::SeedCryptoProvider;
    use crate::wallet::platform_wallet::{PlatformWallet, WalletId};
    use crate::PlatformWalletManager;
    use dash_sdk::error::ContextProviderError;
    use dash_sdk::platform::private_document_entries::{
        PrivateDocumentEntriesQuery, PrivateDocumentEntry,
    };
    use dash_sdk::platform::ContextProvider;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use dpp::data_contract::document_type::encode_private_document_payload;
    use dpp::data_contract::TokenConfiguration;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::SecurityLevel;
    use dpp::platform_value::string_encoding::Encoding;
    use dpp::platform_value::{to_value, BinaryData};
    use dpp::secp256k1::PublicKey;
    use dpp::version::PlatformVersion;
    use key_wallet::mnemonic::Mnemonic;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use platform_encryption::{
        private_document_outgoing_viewing_key, seal_private_document_entry,
        PrivateDocumentEnvelopeError, PrivateDocumentRecipient, PrivateDocumentSender,
        PrivateDocumentTarget,
    };
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    const ALICE_PHRASE: &str = "abandon abandon abandon abandon abandon abandon abandon abandon \
                                abandon abandon abandon about";
    const BOB_PHRASE: &str =
        "legal winner thank year wave sausage worth useful legal winner thank yellow";
    const ALICE: [u8; 32] = [0xA1; 32];
    const CAROL: [u8; 32] = [0xC2; 32];
    const DAVE: [u8; 32] = [0xD3; 32];
    const CONTRACT_ID: [u8; 32] = [7; 32];
    const TYPE_NAME: &str = "secret";
    const ENTRY_SIZE: usize = 400;
    const BLOCK_TIME_MS: u64 = 1_700_000_000_000;
    const SIGNING_KEY_ID: KeyID = 0;
    const ENCRYPTION_KEY_ID: KeyID = 1;
    const DECRYPTION_KEY_ID: KeyID = 2;

    /// A backend that keeps nothing.
    struct NoPersistence;

    impl PlatformWalletPersistence for NoPersistence {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    struct NoopEventHandler;
    impl EventHandler for NoopEventHandler {}
    impl PlatformEventHandler for NoopEventHandler {}

    /// A context provider holding the test contract.
    /// What it holds, or nothing; a contract it is given takes its place.
    struct HeldContract(Mutex<Option<Arc<DataContract>>>);

    impl ContextProvider for HeldContract {
        fn get_data_contract(
            &self,
            id: &Identifier,
            _platform_version: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
            Ok(self
                .0
                .lock()
                .expect("the lock")
                .as_ref()
                .filter(|held| held.id() == *id)
                .cloned())
        }

        fn register_data_contract(&self, contract: Arc<DataContract>) {
            *self.0.lock().expect("the lock") = Some(contract);
        }

        fn get_token_configuration(
            &self,
            _token_id: &Identifier,
        ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
            Ok(None)
        }

        fn get_quorum_public_key(
            &self,
            _quorum_type: u32,
            _quorum_hash: [u8; 32],
            _core_chain_locked_height: u32,
        ) -> Result<[u8; 48], ContextProviderError> {
            Err(ContextProviderError::Config(
                "no quorum keys in this test".into(),
            ))
        }

        fn get_platform_activation_height(&self) -> Result<u32, ContextProviderError> {
            Ok(1)
        }
    }

    /// The test contract, version 1: the private `secret` type, a required
    /// `text` and an optional `count`, and the public `note` type.
    fn contract() -> DataContract {
        contract_at(1)
    }

    /// The test contract at `version`. From version 2 the `secret` type has an
    /// optional `mood` too, appended as a compatible update appends one.
    fn contract_at(version: u32) -> DataContract {
        let mut secret_properties = json!({
            "text": {
                "type": "string",
                "minLength": 3,
                "maxLength": 40,
                "position": 0
            },
            "count": {
                "type": "integer",
                "minimum": 0,
                "maximum": 255,
                "position": 1
            }
        });
        if version >= 2 {
            secret_properties["mood"] = json!({
                "type": "string",
                "maxLength": 10,
                "position": 2
            });
        }
        DataContract::from_value(
            to_value(json!({
                "$formatVersion": "1",
                "id": Identifier::from(CONTRACT_ID).to_string(Encoding::Base58),
                "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
                "version": version,
                "documentSchemas": {
                    "secret": {
                        "type": "object",
                        "private": { "entrySize": ENTRY_SIZE },
                        "documentsMutable": false,
                        "canBeDeleted": false,
                        "properties": secret_properties,
                        "required": ["text"],
                        "additionalProperties": false
                    },
                    "note": {
                        "type": "object",
                        "properties": {
                            "text": { "type": "string", "maxLength": 63, "position": 0 }
                        },
                        "additionalProperties": false
                    }
                }
            }))
            .expect("the contract converts"),
            true,
            PlatformVersion::latest(),
        )
        .expect("the contract registers")
    }

    fn seed(phrase: &str) -> [u8; 64] {
        *Mnemonic::from_phrase(phrase)
            .expect("test mnemonic")
            .to_seed("")
    }

    /// The secret key `key_id` of identity index 0 of `phrase`'s seed.
    fn identity_secret_key(phrase: &str, key_id: KeyID) -> SecretKey {
        let wallet = key_wallet::wallet::Wallet::from_seed_bytes(
            seed(phrase),
            Network::Testnet,
            WalletAccountCreationOptions::None,
        )
        .expect("test wallet");
        let path = identity_auth_derivation_path_for_type(
            Network::Testnet,
            KeyDerivationType::ECDSA,
            0,
            key_id,
        )
        .expect("identity key path");
        let xprv = wallet
            .derive_extended_private_key(&path)
            .expect("identity key");
        SecretKey::from_secret_bytes(*xprv.private_key.to_bytes()).expect("secret key")
    }

    /// The identity `id` whose keys are those of identity index 0 of
    /// `phrase`'s seed: its signing key 0 and, with `scanning_keys`, its
    /// encryption key 1 and decryption key 2.
    fn identity(phrase: &str, id: [u8; 32], scanning_keys: bool) -> Identity {
        let key = |key_id: KeyID, purpose: Purpose, security_level: SecurityLevel| {
            IdentityPublicKey::V0(IdentityPublicKeyV0 {
                id: key_id,
                purpose,
                security_level,
                contract_bounds: None,
                key_type: KeyType::ECDSA_SECP256K1,
                read_only: false,
                data: BinaryData::new(
                    PublicKey::from_secret_key(&identity_secret_key(phrase, key_id))
                        .serialize()
                        .to_vec(),
                ),
                disabled_at: None,
            })
        };
        let mut keys = vec![key(
            SIGNING_KEY_ID,
            Purpose::AUTHENTICATION,
            SecurityLevel::HIGH,
        )];
        if scanning_keys {
            keys.push(key(
                ENCRYPTION_KEY_ID,
                Purpose::ENCRYPTION,
                SecurityLevel::MEDIUM,
            ));
            keys.push(key(
                DECRYPTION_KEY_ID,
                Purpose::DECRYPTION,
                SecurityLevel::MEDIUM,
            ));
        }
        Identity::V0(IdentityV0 {
            id: Identifier::from(id),
            public_keys: keys.into_iter().map(|key| (key.id(), key)).collect(),
            balance: 0,
            revision: 0,
        })
    }

    /// A wallet of `phrase` holding `own` at identity index 0, on a mock SDK
    /// whose context provider holds the test contract, which a fetch of it
    /// also answers.
    async fn wallet(
        phrase: &str,
        own: &Identity,
    ) -> (PlatformWalletManager<NoPersistence>, Arc<PlatformWallet>) {
        wallet_with(phrase, own, Some(contract()), Some(contract())).await
    }

    /// [`wallet`], its context provider holding `held`, or nothing, and a fetch
    /// of the contract answered with `proved`, or not answered at all.
    async fn wallet_with(
        phrase: &str,
        own: &Identity,
        held: Option<DataContract>,
        proved: Option<DataContract>,
    ) -> (PlatformWalletManager<NoPersistence>, Arc<PlatformWallet>) {
        let mut sdk = dash_sdk::SdkBuilder::new_mock()
            .with_network(Network::Testnet)
            .with_version(PlatformVersion::latest())
            .with_context_provider(HeldContract(Mutex::new(held.map(Arc::new))))
            .build()
            .expect("mock sdk");
        if let Some(proved) = proved {
            sdk.mock()
                .expect_fetch(Identifier::from(CONTRACT_ID), Some(proved))
                .await
                .expect("contract expectation");
        }
        let manager = PlatformWalletManager::new(
            Arc::new(sdk),
            Arc::new(NoPersistence),
            Arc::new(NoopEventHandler),
        );
        let wallet = manager
            .create_wallet_from_seed_bytes(
                Network::Testnet,
                &seed(phrase),
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("wallet");
        {
            let iw = wallet.identity();
            let mut wm = iw.wallet_manager.write().await;
            let info = wm
                .get_wallet_info_mut(&wallet.wallet_id())
                .expect("wallet info");
            info.identity_manager
                .add_identity(own.clone(), 0, wallet.wallet_id(), &iw.persister)
                .expect("own identity");
        }
        (manager, wallet)
    }

    /// `wallet`'s mock answers the page of the test store from `start` with
    /// `entries`, of a store of `total` entries in chunks of two.
    async fn serve_page(wallet: &PlatformWallet, start: u64, entries: Vec<Vec<u8>>, total: u64) {
        let mut sdk = (*wallet.identity().sdk).clone();
        let page = PrivateDocumentEntries {
            chunk_power: Some(1),
            entries: entries
                .into_iter()
                .zip(start..)
                .map(|(data, position)| PrivateDocumentEntry {
                    position,
                    block_height: 10 + position,
                    block_time_ms: BLOCK_TIME_MS + position,
                    data,
                })
                .collect(),
            total_count: total,
        };
        sdk.mock()
            .expect_fetch(
                PrivateDocumentEntriesQuery::new(Identifier::from(CONTRACT_ID), TYPE_NAME, start),
                Some(page),
            )
            .await
            .expect("page expectation");
    }

    /// The payload of a `secret` entry holding `properties`, unchecked against
    /// the schema.
    fn payload(properties: serde_json::Value) -> Vec<u8> {
        payload_under(&contract(), properties)
    }

    /// [`payload`], encoded as `contract`'s `secret` type writes it.
    fn payload_under(contract: &DataContract, properties: serde_json::Value) -> Vec<u8> {
        encode_private_document_payload(
            contract
                .document_type_for_name(TYPE_NAME)
                .expect("the type"),
            to_value(properties)
                .expect("the properties convert")
                .into_btree_string_map()
                .expect("a map"),
        )
        .expect("the payload encodes")
    }

    /// An entry of the test store carrying `payload` to the key
    /// `recipient_key` of `recipient`, from `sender`, sealed with the outgoing
    /// viewing key of `outgoing_secret`.
    fn seal(
        recipient: [u8; 32],
        recipient_key: &SecretKey,
        sender: &PrivateDocumentSender,
        outgoing_secret: &SecretKey,
        payload: &[u8],
        seed: u8,
    ) -> Vec<u8> {
        seal_private_document_entry(
            &PrivateDocumentTarget {
                contract_id: CONTRACT_ID,
                document_type_name: TYPE_NAME,
            },
            ENTRY_SIZE,
            &PrivateDocumentRecipient {
                identity_id: recipient,
                public_key: &PublicKey::from_secret_key(recipient_key),
            },
            sender,
            payload,
            &private_document_outgoing_viewing_key(&outgoing_secret.to_secret_bytes()),
            &[seed; 32],
        )
        .expect("the payload fits")
    }

    fn other_key(byte: u8) -> SecretKey {
        SecretKey::from_secret_bytes([byte; 32]).expect("a valid secret key")
    }

    /// A read opens what is sealed to the wallet's identity, recovers what it
    /// sealed, reads each payload against the type's schema, and passes over
    /// the rest.
    #[tokio::test]
    async fn should_read_the_entries_a_wallet_s_identity_received_and_sent() {
        let alice = identity(ALICE_PHRASE, ALICE, true);
        let (_manager, wallet) = wallet(ALICE_PHRASE, &alice).await;
        let alice_decryption = identity_secret_key(ALICE_PHRASE, DECRYPTION_KEY_ID);
        let alice_encryption = identity_secret_key(ALICE_PHRASE, ENCRYPTION_KEY_ID);
        let carol_key = other_key(0xC0);
        let entries = vec![
            // 0: to Alice's decryption key, from Carol, who names herself.
            seal(
                ALICE,
                &alice_decryption,
                &PrivateDocumentSender::DeniableMac {
                    identity_id: CAROL,
                    key_id: 4,
                    secret_key: &carol_key,
                },
                &carol_key,
                &payload(json!({ "text": "hello Alice", "count": 3 })),
                1,
            ),
            // 1: between two others.
            seal(
                DAVE,
                &other_key(0xD0),
                &PrivateDocumentSender::Anonymous,
                &carol_key,
                &payload(json!({ "text": "not for Alice" })),
                2,
            ),
        ];
        serve_page(&wallet, 0, entries, 4).await;
        let entries = vec![
            // 2: from Alice to Dave, sealed with her encryption key's
            // outgoing viewing key.
            seal(
                DAVE,
                &other_key(0xD0),
                &PrivateDocumentSender::Anonymous,
                &alice_encryption,
                &payload(json!({ "text": "hello Dave" })),
                3,
            ),
            // 3: to Alice's encryption key, anonymous, its text shorter than
            // the schema allows.
            seal(
                ALICE,
                &alice_encryption,
                &PrivateDocumentSender::Anonymous,
                &carol_key,
                &payload(json!({ "text": "no" })),
                4,
            ),
        ];
        serve_page(&wallet, 2, entries, 4).await;
        let crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);

        let read = wallet
            .identity()
            .read_private_document_entries(
                &crypto,
                Identifier::from(CONTRACT_ID),
                TYPE_NAME,
                None,
                usize::MAX,
            )
            .await
            .expect("the store reads");

        assert_eq!(read.identities, vec![Identifier::from(ALICE)]);
        assert_eq!(read.pages_read, 2);
        assert_eq!(
            (read.cursor.position, read.cursor.proved_total_count),
            (4, Some(4))
        );
        assert!(read.complete);
        assert!(!read.restarted);
        assert_eq!(read.rejected, vec![]);
        assert_eq!(
            read.entries
                .iter()
                .map(|entry| (
                    entry.position,
                    entry.direction,
                    entry.wallet_key_id,
                    entry.recipient_identity_id,
                    entry.claimed_sender,
                ))
                .collect::<Vec<_>>(),
            vec![
                (
                    0,
                    PrivateDocumentDirection::Received,
                    DECRYPTION_KEY_ID,
                    Identifier::from(ALICE),
                    Some((Identifier::from(CAROL), 4)),
                ),
                (
                    2,
                    PrivateDocumentDirection::Sent,
                    ENCRYPTION_KEY_ID,
                    Identifier::from(DAVE),
                    None,
                ),
                (
                    3,
                    PrivateDocumentDirection::Received,
                    ENCRYPTION_KEY_ID,
                    Identifier::from(ALICE),
                    None,
                ),
            ]
        );
        let received = &read.entries[0];
        assert_eq!(received.wallet_identity_id, Identifier::from(ALICE));
        assert_eq!(received.auth_mode, PrivateDocumentAuthMode::DeniableMac);
        assert_eq!(
            (received.block_height, received.block_time_ms),
            (10, BLOCK_TIME_MS)
        );
        let properties = received.properties.as_ref().expect("a valid payload");
        assert_eq!(
            properties.get("text"),
            Some(&Value::Text("hello Alice".to_string()))
        );
        assert_eq!(
            properties
                .get("count")
                .map(|count| count.to_integer::<u64>().expect("an integer")),
            Some(3)
        );
        assert_eq!(properties.len(), 2);
        assert_eq!(
            read.entries[1].properties,
            Ok(BTreeMap::from([(
                "text".to_string(),
                Value::Text("hello Dave".to_string())
            )]))
        );
        let refused = read.entries[2]
            .properties
            .as_ref()
            .expect_err("a text shorter than the schema's minimum");
        assert!(refused.contains("text"), "{refused}");
    }

    /// The entry from Carol, anonymous, to Alice's decryption key, carrying
    /// `payload`.
    fn to_alice(payload: &[u8], seed: u8) -> Vec<u8> {
        seal(
            ALICE,
            &identity_secret_key(ALICE_PHRASE, DECRYPTION_KEY_ID),
            &PrivateDocumentSender::Anonymous,
            &other_key(0xC0),
            payload,
            seed,
        )
    }

    /// Reads the test store for `wallet` with Alice's seed from `from`, at
    /// most `max_pages` pages.
    async fn read_from(
        wallet: &PlatformWallet,
        from: Option<PrivateDocumentCursor>,
        max_pages: usize,
    ) -> PrivateDocumentRead {
        let crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        wallet
            .identity()
            .read_private_document_entries(
                &crypto,
                Identifier::from(CONTRACT_ID),
                TYPE_NAME,
                from,
                max_pages,
            )
            .await
            .expect("the store reads")
    }

    /// A read stops at `max_pages`, short of the store's end, and a read from
    /// its cursor reaches the end.
    #[tokio::test]
    async fn should_read_on_from_the_cursor_a_bounded_read_stopped_at() {
        let alice = identity(ALICE_PHRASE, ALICE, true);
        let (_manager, wallet) = wallet(ALICE_PHRASE, &alice).await;
        let hello = payload(json!({ "text": "hello Alice" }));
        serve_page(
            &wallet,
            0,
            vec![to_alice(&hello, 1), to_alice(&hello, 2)],
            3,
        )
        .await;
        serve_page(&wallet, 2, vec![to_alice(&hello, 3)], 3).await;

        let first = read_from(&wallet, None, 1).await;
        assert_eq!(
            (
                first.pages_read,
                first.cursor.position,
                first.cursor.proved_total_count
            ),
            (1, 2, Some(3))
        );
        assert!(!first.complete);
        assert_eq!(first.entries.len(), 2);

        let rest = read_from(&wallet, Some(first.cursor), 1).await;
        assert_eq!((rest.pages_read, rest.cursor.position), (1, 3));
        assert!(rest.complete);
        assert!(!rest.restarted);
        assert_eq!(
            rest.entries
                .iter()
                .map(|entry| entry.position)
                .collect::<Vec<_>>(),
            vec![2]
        );

        // Nothing was appended since: the read from the end is complete and
        // holds nothing.
        serve_page(&wallet, 3, vec![], 3).await;
        let end = read_from(&wallet, Some(rest.cursor), usize::MAX).await;
        assert_eq!((end.pages_read, end.cursor.position), (1, 3));
        assert!(end.complete);
        assert_eq!(end.entries, vec![]);
    }

    /// A read from a cursor that proved a store of 5 entries, served a page
    /// from a node behind whose store ends at 4, does not take 4 for the end:
    /// the store is still known to hold position 4.
    #[tokio::test]
    async fn should_keep_a_read_incomplete_below_a_store_size_an_earlier_read_proved() {
        let alice = identity(ALICE_PHRASE, ALICE, true);
        let (_manager, wallet) = wallet(ALICE_PHRASE, &alice).await;
        let hello = payload(json!({ "text": "hello Alice" }));
        serve_page(
            &wallet,
            0,
            vec![to_alice(&hello, 1), to_alice(&hello, 2)],
            5,
        )
        .await;
        serve_page(
            &wallet,
            2,
            vec![to_alice(&hello, 3), to_alice(&hello, 4)],
            4,
        )
        .await;

        let first = read_from(&wallet, None, 1).await;
        assert_eq!(first.cursor.proved_total_count, Some(5));
        assert!(!first.complete);

        let behind = read_from(&wallet, Some(first.cursor), 1).await;
        assert_eq!(behind.cursor.position, 4);
        assert_eq!(behind.cursor.proved_total_count, Some(5));
        assert!(!behind.complete, "position 4 is known to exist");
    }

    /// Once the wallet's keys differ from those a cursor was read with, the
    /// next read starts over from the store's start and says so: the
    /// positions read before may hold entries the new keys open.
    #[tokio::test]
    async fn should_read_from_the_start_again_once_the_wallet_s_keys_change() {
        let alice = identity(ALICE_PHRASE, ALICE, true);
        let (_manager, wallet) = wallet(ALICE_PHRASE, &alice).await;
        let hello = payload(json!({ "text": "hello Alice" }));
        serve_page(
            &wallet,
            0,
            vec![to_alice(&hello, 1), to_alice(&hello, 2)],
            2,
        )
        .await;
        let read_with_other_keys = PrivateDocumentCursor {
            position: 2,
            proved_total_count: Some(2),
            key_set: [9; 32],
            store: test_store(),
        };

        let read = read_from(&wallet, Some(read_with_other_keys), usize::MAX).await;

        assert!(read.restarted);
        assert_eq!(
            read.entries
                .iter()
                .map(|entry| entry.position)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert_eq!(read.cursor.position, 2);
        assert_ne!(read.cursor.key_set, [9; 32]);
        assert!(read.complete);
    }

    /// A payload written under a newer version of the type, which gained an
    /// optional property, does not read under the contract the provider
    /// holds: the read fetches the contract and reads it under the newer one.
    /// A payload the newer schema refuses is still refused.
    #[tokio::test]
    async fn should_read_a_payload_written_under_a_newer_version_of_the_type() {
        let alice = identity(ALICE_PHRASE, ALICE, true);
        let (_manager, wallet) =
            wallet_with(ALICE_PHRASE, &alice, Some(contract()), Some(contract_at(2))).await;
        let newer = contract_at(2);
        serve_page(
            &wallet,
            0,
            vec![
                to_alice(
                    &payload_under(&newer, json!({ "text": "hello Alice", "mood": "happy" })),
                    1,
                ),
                to_alice(&payload_under(&newer, json!({ "text": "no" })), 2),
            ],
            2,
        )
        .await;

        let read = read_from(&wallet, None, usize::MAX).await;

        let properties = read.entries[0]
            .properties
            .as_ref()
            .expect("read under the newer type");
        assert_eq!(
            properties.get("mood"),
            Some(&Value::Text("happy".to_string()))
        );
        let refused = read.entries[1]
            .properties
            .as_ref()
            .expect_err("a text shorter than the schema's minimum");
        assert!(refused.contains("text"), "{refused}");
    }

    /// The fingerprint of the test store.
    fn test_store() -> [u8; 32] {
        private_document_store_fingerprint(
            Network::Testnet,
            &Identifier::from(CONTRACT_ID),
            TYPE_NAME,
        )
    }

    /// The contract is fetched on every read that reads a page, whatever the
    /// pages hold for the wallet: a read whose pages open nothing still asks
    /// for it, here failing for want of an answer, so a node cannot tell from
    /// the requests whether an entry opened.
    #[tokio::test]
    async fn should_fetch_the_contract_on_every_read_whatever_opens() {
        let alice = identity(ALICE_PHRASE, ALICE, true);
        let not_for_alice = seal(
            DAVE,
            &other_key(0xD0),
            &PrivateDocumentSender::Anonymous,
            &other_key(0xC0),
            &payload(json!({ "text": "for Dave" })),
            1,
        );

        let (_manager, unanswered) =
            wallet_with(ALICE_PHRASE, &alice, Some(contract()), None).await;
        serve_page(&unanswered, 0, vec![not_for_alice.clone()], 1).await;
        let crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        unanswered
            .identity()
            .read_private_document_entries(
                &crypto,
                Identifier::from(CONTRACT_ID),
                TYPE_NAME,
                None,
                usize::MAX,
            )
            .await
            .expect_err("the contract fetch has no answer");

        let (_manager, answered) = wallet(ALICE_PHRASE, &alice).await;
        serve_page(&answered, 0, vec![not_for_alice], 1).await;
        let read = read_from(&answered, None, usize::MAX).await;
        assert_eq!(read.entries, vec![]);
        assert!(read.complete);
    }

    /// With no contract held, the read fetches it once to verify the pages
    /// and reads under it.
    #[tokio::test]
    async fn should_read_with_no_contract_held_yet() {
        let alice = identity(ALICE_PHRASE, ALICE, true);
        let (_manager, wallet) = wallet_with(ALICE_PHRASE, &alice, None, Some(contract())).await;
        serve_page(
            &wallet,
            0,
            vec![to_alice(&payload(json!({ "text": "hello Alice" })), 1)],
            1,
        )
        .await;

        let read = read_from(&wallet, None, usize::MAX).await;

        assert_eq!(read.entries.len(), 1);
        assert!(read.entries[0].properties.is_ok());
        assert!(read.complete);
    }

    /// A key the identity registered that its identity key slot does not
    /// derive, as a DashPay Connect application key, is not scanned with and
    /// is listed as such; the identity's other keys still open its entries.
    #[tokio::test]
    async fn should_list_a_key_its_identity_key_slot_does_not_derive_as_unscanned() {
        let mut alice = identity(ALICE_PHRASE, ALICE, true);
        let application_key = other_key(0x33);
        alice.public_keys_mut().insert(
            3,
            IdentityPublicKey::V0(IdentityPublicKeyV0 {
                id: 3,
                purpose: Purpose::DECRYPTION,
                security_level: SecurityLevel::MEDIUM,
                contract_bounds: None,
                key_type: KeyType::ECDSA_SECP256K1,
                read_only: false,
                data: BinaryData::new(
                    PublicKey::from_secret_key(&application_key)
                        .serialize()
                        .to_vec(),
                ),
                disabled_at: None,
            }),
        );
        let (_manager, wallet) = wallet(ALICE_PHRASE, &alice).await;
        let hello = payload(json!({ "text": "hello Alice" }));
        let to_application_key = seal(
            ALICE,
            &application_key,
            &PrivateDocumentSender::Anonymous,
            &other_key(0xC0),
            &hello,
            2,
        );
        serve_page(&wallet, 0, vec![to_alice(&hello, 1), to_application_key], 2).await;

        let read = read_from(&wallet, None, usize::MAX).await;

        assert_eq!(read.unscanned_keys, vec![(Identifier::from(ALICE), 3)]);
        assert_eq!(read.identities, vec![Identifier::from(ALICE)]);
        assert_eq!(
            read.entries
                .iter()
                .map(|entry| (entry.position, entry.wallet_key_id))
                .collect::<Vec<_>>(),
            vec![(0, DECRYPTION_KEY_ID)]
        );
    }

    /// A key whose id names no hardened slot (`2^31` or above) has no path: it
    /// is listed as not scanned, and the read goes on with the identity's
    /// other keys, derived in one call to the provider.
    #[tokio::test]
    async fn should_list_a_key_whose_id_names_no_slot_as_unscanned_and_read_on() {
        let mut alice = identity(ALICE_PHRASE, ALICE, true);
        let far_key_id: KeyID = 0x8000_0001;
        alice.public_keys_mut().insert(
            far_key_id,
            IdentityPublicKey::V0(IdentityPublicKeyV0 {
                id: far_key_id,
                purpose: Purpose::DECRYPTION,
                security_level: SecurityLevel::MEDIUM,
                contract_bounds: None,
                key_type: KeyType::ECDSA_SECP256K1,
                read_only: false,
                data: BinaryData::new(
                    PublicKey::from_secret_key(&other_key(0x44))
                        .serialize()
                        .to_vec(),
                ),
                disabled_at: None,
            }),
        );
        let (_manager, wallet) = wallet(ALICE_PHRASE, &alice).await;
        serve_page(
            &wallet,
            0,
            vec![to_alice(&payload(json!({ "text": "hello Alice" })), 1)],
            1,
        )
        .await;
        let crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);

        let read = wallet
            .identity()
            .read_private_document_entries(
                &crypto,
                Identifier::from(CONTRACT_ID),
                TYPE_NAME,
                None,
                usize::MAX,
            )
            .await
            .expect("the store reads");

        assert_eq!(
            read.unscanned_keys,
            vec![(Identifier::from(ALICE), far_key_id)]
        );
        assert_eq!(
            read.entries
                .iter()
                .map(|entry| (entry.position, entry.wallet_key_id))
                .collect::<Vec<_>>(),
            vec![(0, DECRYPTION_KEY_ID)]
        );
        assert_eq!(
            crypto
                .public_key_batches
                .load(std::sync::atomic::Ordering::SeqCst),
            1,
            "every key's public key in one call"
        );
    }

    /// An identity whose only key an entry can be sealed to names no slot
    /// reads nothing, without asking the signer, and the key is still listed
    /// as not scanned.
    #[tokio::test]
    async fn should_list_a_key_with_no_slot_even_when_nothing_is_read() {
        let mut alice = identity(ALICE_PHRASE, ALICE, false);
        let far_key_id: KeyID = 0x8000_0002;
        alice.public_keys_mut().insert(
            far_key_id,
            IdentityPublicKey::V0(IdentityPublicKeyV0 {
                id: far_key_id,
                purpose: Purpose::DECRYPTION,
                security_level: SecurityLevel::MEDIUM,
                contract_bounds: None,
                key_type: KeyType::ECDSA_SECP256K1,
                read_only: false,
                data: BinaryData::new(
                    PublicKey::from_secret_key(&other_key(0x45))
                        .serialize()
                        .to_vec(),
                ),
                disabled_at: None,
            }),
        );
        let (_manager, wallet) = wallet(ALICE_PHRASE, &alice).await;
        let crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);

        let read = wallet
            .identity()
            .read_private_document_entries(
                &crypto,
                Identifier::from(CONTRACT_ID),
                TYPE_NAME,
                None,
                usize::MAX,
            )
            .await
            .expect("nothing to read");

        assert_eq!(
            read.unscanned_keys,
            vec![(Identifier::from(ALICE), far_key_id)]
        );
        assert_eq!((read.pages_read, read.identities.len()), (0, 0));
        assert!(!read.complete);
        assert_eq!(
            crypto
                .public_key_batches
                .load(std::sync::atomic::Ordering::SeqCst),
            0,
            "the signer is not asked"
        );
    }

    /// A cursor of another store is refused: its position and size describe
    /// that store, not this one.
    #[tokio::test]
    async fn should_refuse_a_cursor_of_another_store() {
        let alice = identity(ALICE_PHRASE, ALICE, true);
        let (_manager, wallet) = wallet(ALICE_PHRASE, &alice).await;
        let crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let of_another_store = PrivateDocumentCursor {
            position: 3,
            proved_total_count: Some(3),
            key_set: [0; 32],
            store: private_document_store_fingerprint(
                Network::Testnet,
                &Identifier::from(CONTRACT_ID),
                "other",
            ),
        };

        let error = wallet
            .identity()
            .read_private_document_entries(
                &crypto,
                Identifier::from(CONTRACT_ID),
                TYPE_NAME,
                Some(of_another_store),
                usize::MAX,
            )
            .await
            .expect_err("another store's cursor");
        assert!(
            matches!(error, PlatformWalletError::InvalidParameter(_)),
            "{error:?}"
        );
    }

    /// An entry that opens under the wallet's key but names another identity
    /// is listed as refused, not as received.
    #[tokio::test]
    async fn should_list_an_entry_replayed_to_the_wallet_s_key_as_refused() {
        let alice = identity(ALICE_PHRASE, ALICE, true);
        let (_manager, wallet) = wallet(ALICE_PHRASE, &alice).await;
        let replayed = seal(
            DAVE,
            &identity_secret_key(ALICE_PHRASE, DECRYPTION_KEY_ID),
            &PrivateDocumentSender::Anonymous,
            &other_key(0xC0),
            &payload(json!({ "text": "for Dave" })),
            1,
        );
        serve_page(&wallet, 0, vec![replayed], 1).await;
        let crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);

        let read = wallet
            .identity()
            .read_private_document_entries(
                &crypto,
                Identifier::from(CONTRACT_ID),
                TYPE_NAME,
                None,
                usize::MAX,
            )
            .await
            .expect("the store reads");

        assert_eq!(read.entries, vec![]);
        assert_eq!(
            read.rejected,
            vec![RejectedPrivateDocumentEntry {
                position: 0,
                block_height: 10,
                error: PrivateDocumentEnvelopeError::WrongRecipient,
            }]
        );
        assert!(read.complete);
    }

    /// A provider resolving another seed is refused before any page is read:
    /// its keys would open nothing, and the store would read as empty.
    #[tokio::test]
    async fn should_refuse_a_read_with_another_wallet_s_seed() {
        let alice = identity(ALICE_PHRASE, ALICE, true);
        let (_manager, wallet) = wallet(ALICE_PHRASE, &alice).await;
        let crypto = SeedCryptoProvider::from_seed(seed(BOB_PHRASE), Network::Testnet);

        // No page is served: a read that reached the store would fail on the
        // mock instead.
        let error = wallet
            .identity()
            .read_private_document_entries(
                &crypto,
                Identifier::from(CONTRACT_ID),
                TYPE_NAME,
                None,
                usize::MAX,
            )
            .await
            .expect_err("another seed");
        assert!(
            matches!(error, PlatformWalletError::SeedMismatch { .. }),
            "{error:?}"
        );
    }

    /// A wallet whose identities hold no key an entry can be sealed to reads
    /// nothing, says it read for no identity, and hands back the cursor it was
    /// given, its size unchanged and nothing complete.
    #[tokio::test]
    async fn should_read_nothing_for_a_wallet_with_no_key_to_open_entries() {
        let alice = identity(ALICE_PHRASE, ALICE, false);
        let (_manager, wallet) = wallet(ALICE_PHRASE, &alice).await;
        let cursor = PrivateDocumentCursor {
            position: 5,
            proved_total_count: Some(9),
            key_set: PrivateDocumentKeyPaths::default().fingerprint(),
            store: test_store(),
        };

        let read = read_from(&wallet, Some(cursor), usize::MAX).await;

        assert_eq!(
            read,
            PrivateDocumentRead {
                cursor,
                ..Default::default()
            }
        );

        // Never read, the store's size is unknown.
        let read = read_from(&wallet, None, usize::MAX).await;
        assert_eq!(read.cursor.proved_total_count, None);
        assert_eq!(read.pages_read, 0);
        assert!(!read.complete);
    }

    /// A type that is not private has no store: the read is refused.
    #[tokio::test]
    async fn should_refuse_to_read_a_type_that_is_not_private() {
        let alice = identity(ALICE_PHRASE, ALICE, true);
        let (_manager, wallet) = wallet(ALICE_PHRASE, &alice).await;
        let crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);

        let error = wallet
            .identity()
            .read_private_document_entries(
                &crypto,
                Identifier::from(CONTRACT_ID),
                "note",
                None,
                usize::MAX,
            )
            .await
            .expect_err("a public type");
        assert!(
            error.to_string().contains("declares no private type"),
            "{error}"
        );
    }

    /// The paths an identity scans with: every decryption and encryption key
    /// as a recipient key, each encryption key as a sender's, and no other.
    #[test]
    fn should_scan_with_an_identity_s_decryption_and_encryption_keys() {
        let alice = identity(ALICE_PHRASE, ALICE, true);
        let mut paths = PrivateDocumentKeyPaths::default();
        assert!(
            paths
                .add_identity(Network::Testnet, 0, &alice)
                .expect("the paths")
                .added
        );
        let path = |key_id| {
            identity_auth_derivation_path_for_type(
                Network::Testnet,
                KeyDerivationType::ECDSA,
                0,
                key_id,
            )
            .expect("the path")
        };
        assert_eq!(
            paths,
            PrivateDocumentKeyPaths {
                recipients: vec![(
                    Identifier::from(ALICE),
                    vec![
                        (ENCRYPTION_KEY_ID, path(ENCRYPTION_KEY_ID)),
                        (DECRYPTION_KEY_ID, path(DECRYPTION_KEY_ID)),
                    ]
                )],
                senders: vec![(
                    Identifier::from(ALICE),
                    ENCRYPTION_KEY_ID,
                    path(ENCRYPTION_KEY_ID)
                )],
            }
        );

        let mut none = PrivateDocumentKeyPaths::default();
        assert_eq!(
            none.add_identity(Network::Testnet, 0, &identity(ALICE_PHRASE, ALICE, false))
                .expect("no path"),
            AddedIdentity::default()
        );
        assert_eq!(none, PrivateDocumentKeyPaths::default());
        // Another key set, another fingerprint: a cursor read with one is
        // read again from the start with the other.
        assert_ne!(paths.fingerprint(), none.fingerprint());
        // An identity index that names no slot is the wallet's record, not a
        // key to leave out: it fails, whatever its key ids.
        let mut invalid_index = PrivateDocumentKeyPaths::default();
        invalid_index
            .add_identity(Network::Testnet, 0x8000_0000, &alice)
            .expect_err("an identity index with no hardened slot");
        assert_eq!(invalid_index, PrivateDocumentKeyPaths::default());
    }
}
