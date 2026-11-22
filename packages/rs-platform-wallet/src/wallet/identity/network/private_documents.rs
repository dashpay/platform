//! The entries of any private document type (protocol version 15) a wallet's
//! identities can open.
//!
//! A private type's store holds fixed-size entries in the standard private
//! document envelope, each sealed to one key of one identity, and names nobody
//! in the clear. A wallet finds its own by trying every entry: with the keys
//! its identities may be sealed to, and with the outgoing viewing keys they
//! seal with, which recover what they sent.
//! [`IdentityWallet::read_private_document_entries`] reads a store that way,
//! from a position the caller keeps, and reads each opened entry's payload
//! against the type's schema.
//!
//! Which keys those are is decided here, once, for every private type
//! ([`PrivateDocumentKeyPaths::add_identity`]): DashPay's private contact
//! request sync scans with the same paths. The keys are derived inside the
//! [`ContactCryptoProvider`], which keeps them.
//!
//! The read keeps nothing: no position, no entry. What an entry means, and
//! whether the sender it names wrote it, is its type's rule; DashPay's is in
//! [`DashPayView::sync_private_contact_requests`](super::DashPayView::sync_private_contact_requests).

use std::collections::BTreeMap;

use dash_sdk::platform::dashpay::recipient_key_purpose_is_valid;
use dash_sdk::platform::private_document_entries::PrivateDocumentEntries;
use dash_sdk::platform::private_document_scan::{
    read_private_document_payload, PrivateDocumentPage, PrivateDocumentScanKeys,
    RejectedPrivateDocumentEntry, MAX_CONCURRENT_PRIVATE_DOCUMENT_PAGE_FETCHES,
};
use dpp::data_contract::DataContract;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, KeyType, Purpose};
use dpp::platform_value::Value;
use dpp::prelude::Identifier;
use dpp::secp256k1::SecretKey;
use dpp::version::PlatformVersion;
use futures::StreamExt;
use key_wallet::bip32::{DerivationPath, KeyDerivationType};
use key_wallet::Network;
use platform_encryption::{OpenedPrivateDocumentEntry, PrivateDocumentAuthMode};

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
    /// key as a sender's, one outgoing viewing key each. Whether it was added:
    /// an identity with no such key reads no store, and is left out.
    pub(super) fn add_identity(
        &mut self,
        network: Network,
        identity_index: u32,
        identity: &Identity,
    ) -> Result<bool, PlatformWalletError> {
        let identity_id = identity.id();
        let mut recipient_paths = Vec::new();
        let mut sender_paths = Vec::new();
        for (key_id, key) in identity.public_keys() {
            if !scanning_key(key) {
                continue;
            }
            let path = identity_auth_derivation_path_for_type(
                network,
                KeyDerivationType::ECDSA,
                identity_index,
                *key_id,
            )?;
            if key.purpose() == Purpose::ENCRYPTION {
                sender_paths.push((identity_id, *key_id, path.clone()));
            }
            recipient_paths.push((*key_id, path));
        }
        if recipient_paths.is_empty() {
            return Ok(false);
        }
        self.recipients.push((identity_id, recipient_paths));
        self.senders.extend(sender_paths);
        Ok(true)
    }
}

/// Whether `key` is one a private document entry can be sealed to, which a
/// wallet scans a private type's store with: an `ECDSA_SECP256K1` `DECRYPTION`
/// or `ENCRYPTION` key, disabled ones included, as an entry sealed before a
/// key was disabled still opens with it.
pub(super) fn scanning_key(key: &IdentityPublicKey) -> bool {
    key.key_type() == KeyType::ECDSA_SECP256K1 && recipient_key_purpose_is_valid(key.purpose())
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
    /// type's schema, or why they are refused.
    pub properties: Result<BTreeMap<String, Value>, String>,
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
    /// The store pages read.
    pub pages_read: usize,
    /// The position to read from next.
    pub next_position: u64,
    /// How many entries the store held, as the pages read proved it.
    pub total_count: u64,
    /// Whether the read reached the store's last entry. A later read from
    /// [`Self::next_position`] reads the entries appended since.
    pub complete: bool,
}

/// `entry`, opened by or recovered for `wallet_identity_id`, as the read
/// reports it.
#[allow(clippy::too_many_arguments)]
fn opened_private_document(
    contract: &DataContract,
    document_type_name: &str,
    position: u64,
    block_height: u64,
    block_time_ms: u64,
    direction: PrivateDocumentDirection,
    wallet_identity_id: Identifier,
    wallet_key_id: KeyID,
    entry: &OpenedPrivateDocumentEntry,
    platform_version: &PlatformVersion,
) -> OpenedPrivateDocument {
    let auth_mode = entry.auth_mode();
    OpenedPrivateDocument {
        position,
        block_height,
        block_time_ms,
        direction,
        wallet_identity_id,
        wallet_key_id,
        recipient_identity_id: Identifier::from(*entry.recipient_identity_id()),
        auth_mode,
        claimed_sender: (auth_mode != PrivateDocumentAuthMode::Anonymous).then(|| {
            (
                Identifier::from(*entry.sender_identity_id()),
                entry.sender_key_id(),
            )
        }),
        commitment: *entry.commitment(),
        properties: read_private_document_payload(
            contract,
            document_type_name,
            entry.payload(),
            platform_version,
        ),
    }
}

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// The paths of the keys the wallet's identities read a private document
    /// type's store with: those of every identity with an HD index, whose keys
    /// the signer can derive, and a key an entry can be sealed to
    /// ([`PrivateDocumentKeyPaths::add_identity`]). With them, the identities
    /// they are of.
    async fn private_document_key_paths(
        &self,
    ) -> Result<(Vec<Identifier>, PrivateDocumentKeyPaths), PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let info = wm
            .get_wallet_info(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        let mut identities = Vec::new();
        let mut paths = PrivateDocumentKeyPaths::default();
        for identity in info.identity_manager.all_identities() {
            let identity_id = identity.id();
            let Some(identity_index) = info
                .identity_manager
                .managed_identity(&identity_id)
                .and_then(|managed| managed.identity_index)
            else {
                continue;
            };
            if paths.add_identity(self.sdk.network, identity_index, identity)? {
                identities.push(identity_id);
            }
        }
        Ok((identities, paths))
    }

    /// Read the store of the private document type `document_type_name` of
    /// `data_contract_id` for the wallet's identities, at most `max_pages`
    /// proved pages from `start_position`: the entries sealed to them, opened,
    /// and the entries they sealed, recovered from the store alone, each with
    /// its payload read against the type's schema.
    ///
    /// The keys are derived inside `crypto` and stay there. Nothing is kept:
    /// the caller reads on from [`PrivateDocumentRead::next_position`], and a
    /// store only grows, so a later read from there holds what was appended
    /// since. A failed page fetch fails the read; the caller reads again from
    /// the position it had.
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
    pub async fn read_private_document_entries<C>(
        &self,
        crypto: &C,
        data_contract_id: Identifier,
        document_type_name: &str,
        start_position: u64,
        max_pages: usize,
    ) -> Result<PrivateDocumentRead, PlatformWalletError>
    where
        C: ContactCryptoProvider + Sync,
    {
        let (identities, paths) = self.private_document_key_paths().await?;
        let mut read = PrivateDocumentRead {
            identities,
            next_position: start_position,
            ..Default::default()
        };
        if read.identities.is_empty() {
            return Ok(read);
        }
        self.verify_seed_binds(crypto).await?;
        // The contract the pages are verified against, with the type, which
        // each payload is read against.
        let contract = self
            .sdk
            .fetch_private_document_contract(data_contract_id, document_type_name)
            .await?;
        let platform_version = self.sdk.version();

        // The pages are fetched many at once while earlier ones are scanned,
        // in batches of those ready, one key derivation a batch.
        let mut batches = std::pin::pin!(self
            .sdk
            .private_document_pages(
                data_contract_id,
                document_type_name,
                start_position,
                MAX_CONCURRENT_PRIVATE_DOCUMENT_PAGE_FETCHES,
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
                read.next_position = scan.next_position;
                // A later page from a node behind shows an older, smaller
                // store: the largest size proved is the store's.
                read.total_count = read.total_count.max(scan.total_count);
                let first = read.entries.len();
                read.entries.extend(scan.received.iter().map(|received| {
                    opened_private_document(
                        &contract,
                        document_type_name,
                        received.position,
                        received.block_height,
                        received.block_time_ms,
                        PrivateDocumentDirection::Received,
                        received.recipient_identity_id,
                        received.recipient_key_id,
                        &received.entry,
                        platform_version,
                    )
                }));
                read.entries.extend(scan.sent.iter().map(|sent| {
                    opened_private_document(
                        &contract,
                        document_type_name,
                        sent.position,
                        sent.block_height,
                        sent.block_time_ms,
                        PrivateDocumentDirection::Sent,
                        sent.sender_identity_id,
                        sent.outgoing_key_id,
                        &sent.entry,
                        platform_version,
                    )
                }));
                // Pages come in store order, so only this page's entries are
                // put in order: stable, the received one first at a position.
                read.entries[first..].sort_by_key(|entry| (entry.position, entry.direction));
                read.rejected.extend(scan.rejected);
            }
        }
        read.complete = read.pages_read > 0 && read.next_position >= read.total_count;
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
    use key_wallet::mnemonic::Mnemonic;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use platform_encryption::{
        private_document_outgoing_viewing_key, seal_private_document_entry,
        PrivateDocumentEnvelopeError, PrivateDocumentRecipient, PrivateDocumentSender,
        PrivateDocumentTarget,
    };
    use serde_json::json;
    use std::sync::Arc;

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
    struct HeldContract(Arc<DataContract>);

    impl ContextProvider for HeldContract {
        fn get_data_contract(
            &self,
            id: &Identifier,
            _platform_version: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
            Ok((*id == self.0.id()).then(|| Arc::clone(&self.0)))
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

    /// A contract holding the private `secret` type, a required `text` and an
    /// optional `count`, and the public `note` type.
    fn contract() -> DataContract {
        DataContract::from_value(
            to_value(json!({
                "$formatVersion": "1",
                "id": Identifier::from(CONTRACT_ID).to_string(Encoding::Base58),
                "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
                "version": 1,
                "documentSchemas": {
                    "secret": {
                        "type": "object",
                        "private": { "entrySize": ENTRY_SIZE },
                        "documentsMutable": false,
                        "canBeDeleted": false,
                        "properties": {
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
                        },
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
    /// whose context provider holds the test contract.
    async fn wallet(
        phrase: &str,
        own: &Identity,
    ) -> (PlatformWalletManager<NoPersistence>, Arc<PlatformWallet>) {
        let sdk = dash_sdk::SdkBuilder::new_mock()
            .with_network(Network::Testnet)
            .with_version(PlatformVersion::latest())
            .with_context_provider(HeldContract(Arc::new(contract())))
            .build()
            .expect("mock sdk");
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
        let contract = contract();
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
                0,
                usize::MAX,
            )
            .await
            .expect("the store reads");

        assert_eq!(read.identities, vec![Identifier::from(ALICE)]);
        assert_eq!(read.pages_read, 2);
        assert_eq!((read.next_position, read.total_count), (4, 4));
        assert!(read.complete);
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

    /// A read stops at `max_pages`, short of the store's end, and a read from
    /// where it stopped reaches the end.
    #[tokio::test]
    async fn should_read_on_from_the_position_a_bounded_read_stopped_at() {
        let alice = identity(ALICE_PHRASE, ALICE, true);
        let (_manager, wallet) = wallet(ALICE_PHRASE, &alice).await;
        let alice_decryption = identity_secret_key(ALICE_PHRASE, DECRYPTION_KEY_ID);
        let to_alice = |seed: u8| {
            seal(
                ALICE,
                &alice_decryption,
                &PrivateDocumentSender::Anonymous,
                &other_key(0xC0),
                &payload(json!({ "text": "hello Alice" })),
                seed,
            )
        };
        serve_page(&wallet, 0, vec![to_alice(1), to_alice(2)], 3).await;
        serve_page(&wallet, 2, vec![to_alice(3)], 3).await;
        let crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);
        let read_from = |start: u64, max_pages: usize| {
            let identity = wallet.identity();
            let crypto = &crypto;
            async move {
                identity
                    .read_private_document_entries(
                        crypto,
                        Identifier::from(CONTRACT_ID),
                        TYPE_NAME,
                        start,
                        max_pages,
                    )
                    .await
                    .expect("the store reads")
            }
        };

        let first = read_from(0, 1).await;
        assert_eq!(
            (first.pages_read, first.next_position, first.total_count),
            (1, 2, 3)
        );
        assert!(!first.complete);
        assert_eq!(first.entries.len(), 2);

        let rest = read_from(first.next_position, 1).await;
        assert_eq!((rest.pages_read, rest.next_position), (1, 3));
        assert!(rest.complete);
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
        let end = read_from(rest.next_position, usize::MAX).await;
        assert_eq!((end.pages_read, end.next_position), (1, 3));
        assert!(end.complete);
        assert_eq!(end.entries, vec![]);
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
                0,
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
                0,
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
    /// nothing, and says it read for no identity.
    #[tokio::test]
    async fn should_read_nothing_for_a_wallet_with_no_key_to_open_entries() {
        let alice = identity(ALICE_PHRASE, ALICE, false);
        let (_manager, wallet) = wallet(ALICE_PHRASE, &alice).await;
        let crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);

        let read = wallet
            .identity()
            .read_private_document_entries(
                &crypto,
                Identifier::from(CONTRACT_ID),
                TYPE_NAME,
                5,
                usize::MAX,
            )
            .await
            .expect("nothing to read");

        assert_eq!(
            read,
            PrivateDocumentRead {
                next_position: 5,
                ..Default::default()
            }
        );
    }

    /// A type that is not private has no store: the read is refused.
    #[tokio::test]
    async fn should_refuse_to_read_a_type_that_is_not_private() {
        let alice = identity(ALICE_PHRASE, ALICE, true);
        let (_manager, wallet) = wallet(ALICE_PHRASE, &alice).await;
        let mut sdk = (*wallet.identity().sdk).clone();
        sdk.mock()
            .expect_fetch(Identifier::from(CONTRACT_ID), Some(contract()))
            .await
            .expect("contract expectation");
        let crypto = SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet);

        let error = wallet
            .identity()
            .read_private_document_entries(
                &crypto,
                Identifier::from(CONTRACT_ID),
                "note",
                0,
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
        assert!(paths
            .add_identity(Network::Testnet, 0, &alice)
            .expect("the paths"));
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
        assert!(!none
            .add_identity(Network::Testnet, 0, &identity(ALICE_PHRASE, ALICE, false))
            .expect("no path"));
        assert_eq!(none, PrivateDocumentKeyPaths::default());
    }
}
