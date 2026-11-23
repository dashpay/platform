//! Creating an entry of any private document type (protocol version 15).
//!
//! One of the wallet's identities seals an entry to a key of any identity
//! ([`IdentityWallet::seal_private_document_entry`]), and the wallet appends
//! it to the type's store, paying from a shielded account
//! (`PlatformWallet::create_private_document_entry`). The keys follow the
//! rule the wallet reads a private type's store with
//! ([`super::private_documents`]): the entry is sealed to a `DECRYPTION` or
//! `ENCRYPTION` key of the recipient, and with the outgoing viewing key of the
//! sender's `ENCRYPTION` key at its identity key slot, so a read of the store
//! recovers it as sent. The sender's secret key stays in the
//! [`ContactCryptoProvider`].
//!
//! An entry names its sender or nobody
//! ([`PrivateDocumentSenderAuth`]). A signed entry is not built: which keys
//! may sign for a type other than DashPay's contact requests is not decided.

use std::collections::BTreeMap;

use dash_sdk::platform::private_document_seal::{
    private_document_recipient_keys, private_document_sender_key, PrivateDocumentEntryRequest,
    PrivateDocumentSenderAuth,
};
use dash_sdk::platform::Fetch;
use dash_sdk::Sdk;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::data_contract::DataContract;
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{Identity, KeyID, Purpose};
use dpp::platform_value::{BinaryData, Value};
use dpp::prelude::Identifier;
use dpp::secp256k1::rand::rngs::StdRng;
use dpp::secp256k1::rand::{RngCore, SeedableRng};
use dpp::secp256k1::PublicKey;
use dpp::shielded::compute_private_document_create_from_shielded_pool_fee;
use dpp::version::PlatformVersion;
use key_wallet::bip32::{DerivationPath, KeyDerivationType};
use zeroize::Zeroizing;

use super::document::document_properties_from_json;
use super::{identity_auth_derivation_path_for_type, ContactCryptoProvider, IdentityWallet};
use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;

/// The Orchard actions the fee of a private document create is previewed at
/// ([`private_document_create_fee`]): a spend of one or two notes with its
/// change, the fewest a spend reserves its fee at. A spend of more notes
/// takes one action each, and a higher fee.
pub const PRIVATE_DOCUMENT_CREATE_FEE_PREVIEW_ACTIONS: usize = 2;

/// An entry of a private document type to create: what it holds, for which
/// type, from which of the wallet's identities, to which key of which
/// identity, and whether it names its sender.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewPrivateDocumentEntry {
    /// The contract declaring the private type.
    pub data_contract_id: Identifier,
    /// The private type's name.
    pub document_type_name: String,
    /// The wallet's identity that seals the entry: it needs an enabled
    /// `ECDSA_SECP256K1` `ENCRYPTION` key at its identity key slot.
    pub sender_identity_id: Identifier,
    /// The identity the entry is for.
    pub recipient_identity_id: Identifier,
    /// The recipient's key the entry is sealed to: one of
    /// [`private_document_recipient_key_choices`].
    pub recipient_key_id: KeyID,
    /// Whether the entry names the sender.
    pub auth: PrivateDocumentSenderAuth,
    /// The entry's properties, a JSON object keyed by property name, as a
    /// document create takes them: byte arrays as hex (or base64) strings,
    /// identifiers as base58 (or hex) strings.
    pub properties_json: String,
}

/// An entry sealed for the store of its type
/// ([`IdentityWallet::seal_private_document_entry`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SealedPrivateDocumentEntry {
    /// The entry, of the type's entry size.
    pub entry: BinaryData,
    /// Its commitment, the entry's first 32 bytes: a read of the store
    /// reports the entry with it.
    pub commitment: [u8; 32],
    /// The sender's key the entry was sealed with: its outgoing viewing key
    /// recovers the entry, and a deniable MAC names it.
    pub sender_key_id: KeyID,
}

/// What a created entry is known by
/// (`PlatformWallet::create_private_document_entry`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrivateDocumentEntryCreated {
    /// The entry's commitment, which a read of the store reports it with.
    pub commitment: [u8; 32],
    /// The sender's key the entry was sealed with.
    pub sender_key_id: KeyID,
}

/// A key of an identity an entry of a private document type may be sealed
/// to now ([`private_document_recipient_key_choices`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrivateDocumentRecipientKey {
    /// The key's id.
    pub key_id: KeyID,
    /// Its purpose: `DECRYPTION` or `ENCRYPTION`.
    pub purpose: Purpose,
}

/// The keys of the identity `identity_id`, as the network proves it now, that
/// an entry of a private document type may be sealed to: its enabled
/// `ECDSA_SECP256K1` `DECRYPTION` and `ENCRYPTION` keys
/// ([`dash_sdk::platform::private_document_seal::private_document_recipient_key`]),
/// by key id. An identity the network does not hold is refused with
/// [`PlatformWalletError::IdentityNotFound`]; one with no such key gives none.
pub async fn private_document_recipient_key_choices(
    sdk: &Sdk,
    identity_id: Identifier,
) -> Result<Vec<PrivateDocumentRecipientKey>, PlatformWalletError> {
    let identity = Identity::fetch(sdk, identity_id)
        .await?
        .ok_or(PlatformWalletError::IdentityNotFound(identity_id))?;
    Ok(recipient_key_choices(&identity))
}

/// [`private_document_recipient_key_choices`] of a fetched identity.
fn recipient_key_choices(identity: &Identity) -> Vec<PrivateDocumentRecipientKey> {
    private_document_recipient_keys(identity)
        .into_iter()
        .map(|key| PrivateDocumentRecipientKey {
            key_id: key.id(),
            purpose: key.purpose(),
        })
        .collect()
}

/// The fee a private document create of `entry_count` entries of the type
/// `document_type_name` of `data_contract_id`, as the network proves the
/// contract now, reserves from the notes when it spends one or two of them
/// ([`PRIVATE_DOCUMENT_CREATE_FEE_PREVIEW_ACTIONS`]), at the protocol version
/// the SDK runs: what the spend pays at least. See
/// [`private_document_create_fee_for`].
pub async fn private_document_create_fee(
    sdk: &Sdk,
    data_contract_id: Identifier,
    document_type_name: &str,
    entry_count: usize,
) -> Result<Credits, PlatformWalletError> {
    let contract = sdk
        .fetch_private_document_contract(data_contract_id, document_type_name)
        .await?;
    private_document_create_fee_for(&contract, document_type_name, entry_count, sdk.version())
}

/// [`private_document_create_fee`] for `contract`, at `platform_version`.
/// Refuses a type that is not private, and an entry count of 0 or above what
/// one transition appends (`max_private_document_entries_per_transition`),
/// with [`PlatformWalletError::InvalidParameter`].
pub fn private_document_create_fee_for(
    contract: &DataContract,
    document_type_name: &str,
    entry_count: usize,
    platform_version: &PlatformVersion,
) -> Result<Credits, PlatformWalletError> {
    let entry_size = private_entry_size(contract, document_type_name)?;
    let Some(max_entries) = platform_version
        .system_limits
        .max_private_document_entries_per_transition
    else {
        return Err(PlatformWalletError::InvalidParameter(format!(
            "protocol version {} has no private document types",
            platform_version.protocol_version
        )));
    };
    if entry_count == 0 || entry_count > usize::from(max_entries) {
        return Err(PlatformWalletError::InvalidParameter(format!(
            "a private document create appends 1 to {max_entries} entries, not {entry_count}"
        )));
    }
    Ok(compute_private_document_create_from_shielded_pool_fee(
        PRIVATE_DOCUMENT_CREATE_FEE_PREVIEW_ACTIONS,
        entry_count,
        entry_size,
        platform_version,
    )
    .map_err(dash_sdk::Error::Protocol)?)
}

/// The entry size of the private type `document_type_name` of `contract`.
fn private_entry_size(
    contract: &DataContract,
    document_type_name: &str,
) -> Result<u32, PlatformWalletError> {
    let document_type = contract
        .document_type_for_name(document_type_name)
        .map_err(|error| {
            PlatformWalletError::InvalidParameter(format!(
                "the contract {} has no document type \"{document_type_name}\": {error}",
                contract.id()
            ))
        })?;
    document_type
        .private_document()
        .map(|config| config.entry_size())
        .ok_or_else(|| {
            PlatformWalletError::InvalidParameter(format!(
                "the document type \"{document_type_name}\" of contract {} is not private",
                contract.id()
            ))
        })
}

/// A refusal of the SDK's check of an entry as the caller's: its text alone.
fn refused(error: dash_sdk::Error) -> PlatformWalletError {
    match error {
        dash_sdk::Error::Generic(message) => PlatformWalletError::InvalidParameter(message),
        error => error.into(),
    }
}

/// The sender identity of a create, as the wallet holds it, and the keys it
/// may seal with: each enabled `ECDSA_SECP256K1` `ENCRYPTION` key whose id
/// names an identity key slot, with that slot's path, in key id order.
struct SenderCandidates {
    identity: Identity,
    keys: Vec<(KeyID, PublicKey, DerivationPath)>,
}

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// The wallet's identity `sender_identity_id` and the keys it may seal a
    /// private document entry with ([`SenderCandidates`]).
    async fn private_document_sender_candidates(
        &self,
        sender_identity_id: &Identifier,
    ) -> Result<SenderCandidates, PlatformWalletError> {
        let (identity, identity_index) = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            // The identities this wallet owns, not those of another wallet
            // the manager holds or the contacts it observes.
            let managed = info
                .identity_manager
                .wallet_managed_identities(&self.wallet_id)
                .find(|managed| managed.identity.id() == *sender_identity_id)
                .ok_or(PlatformWalletError::IdentityNotFound(*sender_identity_id))?;
            (managed.identity.clone(), managed.identity_index)
        };
        let identity_index = identity_index.ok_or_else(|| {
            PlatformWalletError::InvalidParameter(format!(
                "the identity {sender_identity_id} has no HD index in this wallet, so the wallet \
                 derives none of its keys to seal a private document entry with"
            ))
        })?;
        let mut keys = Vec::new();
        for key_id in identity.public_keys().keys() {
            let Ok(public_key) = private_document_sender_key(&identity, *key_id) else {
                continue;
            };
            // A key id past the hardened range names no slot.
            let Ok(path) = identity_auth_derivation_path_for_type(
                self.sdk.network,
                KeyDerivationType::ECDSA,
                identity_index,
                *key_id,
            ) else {
                continue;
            };
            keys.push((*key_id, public_key, path));
        }
        if keys.is_empty() {
            return Err(PlatformWalletError::InvalidParameter(format!(
                "the identity {sender_identity_id} has no enabled ECDSA_SECP256K1 ENCRYPTION \
                 key, which a private document entry is sealed with; add one to send entries"
            )));
        }
        Ok(SenderCandidates { identity, keys })
    }

    /// Seal `new`, an entry of any private document type, from the wallet's
    /// identity `new.sender_identity_id` to the key `new.recipient_key_id`
    /// of the identity `new.recipient_identity_id`, for the wallet to append
    /// to the type's store.
    ///
    /// Before the signer is asked, the contract is fetched as a read of the
    /// store fetches it (refused below protocol version 15 with the SDK's
    /// `ProtocolVersionNotActive`, and when the type is not private), the
    /// recipient is fetched, and the entry is checked
    /// ([`PrivateDocumentEntryRequest::check`]): the recipient key is an
    /// enabled `ECDSA_SECP256K1` `DECRYPTION` or `ENCRYPTION` key, the
    /// properties satisfy the type's schema and fit an entry. Then the
    /// signer's seed is checked to be the wallet's
    /// ([`PlatformWalletError::SeedMismatch`]), and the entry is sealed with
    /// the sender's first enabled `ECDSA_SECP256K1` `ENCRYPTION` key whose
    /// identity key slot derives, inside `crypto`, the key it registered:
    /// the key a read of the store recovers the entry with, as sent. Its
    /// secret key stays in `crypto`.
    ///
    /// Every refusal of what was asked is a
    /// [`PlatformWalletError::InvalidParameter`] with the reason; a sender
    /// the wallet does not hold, or a recipient the network does not, is
    /// [`PlatformWalletError::IdentityNotFound`].
    pub async fn seal_private_document_entry<C>(
        &self,
        crypto: &C,
        new: &NewPrivateDocumentEntry,
    ) -> Result<SealedPrivateDocumentEntry, PlatformWalletError>
    where
        C: ContactCryptoProvider + Sync,
    {
        let SenderCandidates {
            identity: sender,
            keys,
        } = self
            .private_document_sender_candidates(&new.sender_identity_id)
            .await?;
        let contract = self
            .sdk
            .fetch_private_document_contract(new.data_contract_id, &new.document_type_name)
            .await?;
        let document_type = contract
            .document_type_for_name(&new.document_type_name)
            .map_err(|error| PlatformWalletError::InvalidParameter(error.to_string()))?;
        let properties: BTreeMap<String, Value> =
            document_properties_from_json(document_type, &new.properties_json)?;
        let recipient = Identity::fetch(&self.sdk, new.recipient_identity_id)
            .await?
            .ok_or(PlatformWalletError::IdentityNotFound(
                new.recipient_identity_id,
            ))?;
        let platform_version = self.sdk.version();
        let request = |sender_encryption_key_id| PrivateDocumentEntryRequest {
            contract: &contract,
            document_type_name: &new.document_type_name,
            sender: &sender,
            sender_encryption_key_id,
            auth: new.auth,
            recipient: &recipient,
            recipient_key_id: new.recipient_key_id,
            properties: &properties,
        };
        // Every candidate passes the sender key check alike: the check
        // refuses what was asked before the signer is.
        request(keys[0].0)
            .check(platform_version)
            .map_err(refused)?;

        self.verify_seed_binds(crypto).await?;
        let paths: Vec<DerivationPath> = keys.iter().map(|(_, _, path)| path.clone()).collect();
        let derived = crypto.extended_public_keys(&paths).await?;
        let (sender_key_id, _, path) = keys
            .iter()
            .zip(derived.iter())
            .find(|((_, registered, _), derived)| {
                registered.serialize()[..] == derived.public_key.to_compressed()[..]
            })
            .map(|(key, _)| key)
            .ok_or_else(|| {
                PlatformWalletError::InvalidParameter(format!(
                    "no ENCRYPTION key of the identity {} derives at its identity key slot, \
                     such as one registered by another app: the wallet could not recover \
                     an entry sealed with it",
                    new.sender_identity_id
                ))
            })?;

        // Wiped on drop, so a seal dropped while it is awaited wipes it too.
        let mut esk_seed = Zeroizing::new([0u8; 32]);
        StdRng::try_from_os_rng()
            .map_err(|error| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "the OS RNG failed while sealing a private document entry: {error}"
                ))
            })?
            .fill_bytes(&mut *esk_seed);
        let entry = crypto
            .seal_private_document_entry(
                path,
                &request(*sender_key_id),
                &esk_seed,
                platform_version,
            )
            .await?;
        drop(esk_seed);
        let commitment: [u8; 32] = entry
            .as_slice()
            .get(..32)
            .and_then(|commitment| commitment.try_into().ok())
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(
                    "the provider sealed an entry shorter than its commitment".to_string(),
                )
            })?;
        Ok(SealedPrivateDocumentEntry {
            entry,
            commitment,
            sender_key_id: *sender_key_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::identity::network::private_documents::tests::{
        contract, identity, identity_secret_key, seed, serve_page, wallet, ALICE, ALICE_PHRASE,
        BOB_PHRASE, CONTRACT_ID, DECRYPTION_KEY_ID, ENCRYPTION_KEY_ID, ENTRY_SIZE, TYPE_NAME,
    };
    use crate::wallet::identity::network::{
        ContactInfoOpened, ContactInfoSealed, PrivateDocumentDirection, SeedCryptoProvider,
    };
    use crate::wallet::platform_wallet::PlatformWallet;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::{IdentityPublicKey, KeyType, SecurityLevel};
    use key_wallet::Network;
    use platform_encryption::{
        PrivateDocumentAuthMode, PrivateDocumentRecipientKeys, PrivateDocumentTarget,
    };

    const BOB: [u8; 32] = [0xB0; 32];

    /// The entry to create: `text` from Alice to Bob's decryption key.
    fn new_entry(auth: PrivateDocumentSenderAuth, text: &str) -> NewPrivateDocumentEntry {
        NewPrivateDocumentEntry {
            data_contract_id: Identifier::from(CONTRACT_ID),
            document_type_name: TYPE_NAME.to_string(),
            sender_identity_id: Identifier::from(ALICE),
            recipient_identity_id: Identifier::from(BOB),
            recipient_key_id: DECRYPTION_KEY_ID,
            auth,
            properties_json: serde_json::json!({ "text": text, "count": 4 }).to_string(),
        }
    }

    /// `wallet`'s mock answers a fetch of Bob with `bob`.
    async fn serve_bob(wallet: &PlatformWallet, bob: Identity) {
        let mut sdk = (*wallet.identity().sdk).clone();
        sdk.mock()
            .expect_fetch(Identifier::from(BOB), Some(bob))
            .await
            .expect("identity expectation");
    }

    fn alice_crypto() -> SeedCryptoProvider {
        SeedCryptoProvider::from_seed(seed(ALICE_PHRASE), Network::Testnet)
    }

    /// An entry Alice seals to Bob opens with Bob's decryption key, names
    /// Alice's encryption key with a MAC that verifies, and a read of the
    /// store by Alice's wallet recovers it as sent, with its properties.
    #[tokio::test]
    async fn should_seal_an_entry_the_recipient_opens_and_the_sender_s_read_recovers() {
        let alice = identity(ALICE_PHRASE, ALICE, true);
        let (_manager, wallet) = wallet(ALICE_PHRASE, &alice).await;
        serve_bob(&wallet, identity(BOB_PHRASE, BOB, true)).await;

        let sealed = wallet
            .identity()
            .seal_private_document_entry(
                &alice_crypto(),
                &new_entry(PrivateDocumentSenderAuth::DeniableMac, "hello Bob"),
            )
            .await
            .expect("the entry seals");
        assert_eq!(sealed.entry.len(), ENTRY_SIZE);
        assert_eq!(sealed.sender_key_id, ENCRYPTION_KEY_ID);
        assert_eq!(&sealed.entry.as_slice()[..32], &sealed.commitment);

        let target = PrivateDocumentTarget {
            contract_id: CONTRACT_ID,
            document_type_name: TYPE_NAME,
        };
        let opened = PrivateDocumentRecipientKeys::new(&[identity_secret_key(
            BOB_PHRASE,
            DECRYPTION_KEY_ID,
        )])
        .open(sealed.entry.as_slice(), &target, &BOB)
        .expect("the entry opens")
        .expect("for Bob");
        assert_eq!(opened.auth_mode(), PrivateDocumentAuthMode::DeniableMac);
        assert_eq!(opened.sender_identity_id(), &ALICE);
        assert_eq!(opened.sender_key_id(), ENCRYPTION_KEY_ID);
        PrivateDocumentRecipientKeys::new(&[identity_secret_key(BOB_PHRASE, DECRYPTION_KEY_ID)])
            .verify_sender(
                &opened,
                &PublicKey::from_secret_key(&identity_secret_key(ALICE_PHRASE, ENCRYPTION_KEY_ID)),
            )
            .expect("the MAC is Alice's encryption key's");

        serve_page(&wallet, 0, vec![sealed.entry.to_vec()], 1).await;
        let read = wallet
            .identity()
            .read_private_document_entries(
                &alice_crypto(),
                Identifier::from(CONTRACT_ID),
                TYPE_NAME,
                None,
                usize::MAX,
            )
            .await
            .expect("the store reads");
        assert_eq!(read.entries.len(), 1);
        let sent = &read.entries[0];
        assert_eq!(sent.direction, PrivateDocumentDirection::Sent);
        assert_eq!(sent.wallet_key_id, ENCRYPTION_KEY_ID);
        assert_eq!(sent.recipient_identity_id, Identifier::from(BOB));
        assert_eq!(sent.commitment, sealed.commitment);
        assert_eq!(
            sent.claimed_sender,
            Some((Identifier::from(ALICE), ENCRYPTION_KEY_ID))
        );
        let properties = sent.properties.as_ref().expect("the payload reads");
        assert_eq!(
            properties.get("text"),
            Some(&Value::Text("hello Bob".to_string()))
        );
    }

    /// An anonymous entry names nobody.
    #[tokio::test]
    async fn should_seal_an_anonymous_entry() {
        let alice = identity(ALICE_PHRASE, ALICE, true);
        let (_manager, wallet) = wallet(ALICE_PHRASE, &alice).await;
        serve_bob(&wallet, identity(BOB_PHRASE, BOB, true)).await;

        let sealed = wallet
            .identity()
            .seal_private_document_entry(
                &alice_crypto(),
                &new_entry(PrivateDocumentSenderAuth::Anonymous, "anonymous"),
            )
            .await
            .expect("the entry seals");
        let opened = PrivateDocumentRecipientKeys::new(&[identity_secret_key(
            BOB_PHRASE,
            DECRYPTION_KEY_ID,
        )])
        .open(
            sealed.entry.as_slice(),
            &PrivateDocumentTarget {
                contract_id: CONTRACT_ID,
                document_type_name: TYPE_NAME,
            },
            &BOB,
        )
        .expect("the entry opens")
        .expect("for Bob");
        assert_eq!(opened.auth_mode(), PrivateDocumentAuthMode::Anonymous);
    }

    /// A sender without an encryption key is refused before the signer is
    /// asked, and so is a recipient key an entry cannot be sealed to and
    /// properties the type's schema refuses.
    #[tokio::test]
    async fn should_refuse_what_cannot_be_sealed_before_the_signer() {
        // Alice with her signing key only.
        let alice = identity(ALICE_PHRASE, ALICE, false);
        let (_bare_manager, bare) = wallet(ALICE_PHRASE, &alice).await;
        let refused = bare
            .identity()
            .seal_private_document_entry(
                &alice_crypto(),
                &new_entry(PrivateDocumentSenderAuth::Anonymous, "hello"),
            )
            .await
            .expect_err("refused");
        assert!(
            matches!(&refused, PlatformWalletError::InvalidParameter(message)
                if message.contains("no enabled ECDSA_SECP256K1 ENCRYPTION key")),
            "{refused}"
        );

        let alice = identity(ALICE_PHRASE, ALICE, true);
        let (_manager, wallet) = wallet(ALICE_PHRASE, &alice).await;
        serve_bob(&wallet, identity(BOB_PHRASE, BOB, true)).await;
        let mut signing_key = new_entry(PrivateDocumentSenderAuth::Anonymous, "hello");
        signing_key.recipient_key_id = 0;
        let mut too_short = new_entry(PrivateDocumentSenderAuth::Anonymous, "no");
        too_short.recipient_key_id = ENCRYPTION_KEY_ID;
        let mut not_json = new_entry(PrivateDocumentSenderAuth::Anonymous, "hello");
        not_json.properties_json = "[1]".to_string();
        for (new, expected) in [
            (
                signing_key,
                "not an ECDSA_SECP256K1 DECRYPTION or ENCRYPTION key",
            ),
            (too_short, "the properties do not satisfy \"secret\""),
            (not_json, "must be a JSON object"),
        ] {
            // A provider that derives nothing: a refusal comes before it.
            let refused = wallet
                .identity()
                .seal_private_document_entry(&NoKeys, &new)
                .await
                .expect_err("refused");
            assert!(refused.to_string().contains(expected), "{refused}");
        }
    }

    /// A sender the wallet does not hold is not found.
    #[tokio::test]
    async fn should_refuse_a_sender_the_wallet_does_not_hold() {
        let alice = identity(ALICE_PHRASE, ALICE, true);
        let (_manager, wallet) = wallet(ALICE_PHRASE, &alice).await;
        let mut new = new_entry(PrivateDocumentSenderAuth::Anonymous, "hello");
        new.sender_identity_id = Identifier::from(BOB);
        assert!(matches!(
            wallet
                .identity()
                .seal_private_document_entry(&NoKeys, &new)
                .await,
            Err(PlatformWalletError::IdentityNotFound(id)) if id == Identifier::from(BOB)
        ));
    }

    /// The choices are the enabled decryption and encryption keys.
    #[test]
    fn should_offer_only_the_keys_an_entry_can_be_sealed_to() {
        let mut bob = identity(BOB_PHRASE, BOB, true);
        let disabled = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 7,
            purpose: Purpose::DECRYPTION,
            security_level: SecurityLevel::MEDIUM,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(
                PublicKey::from_secret_key(&identity_secret_key(BOB_PHRASE, 7))
                    .serialize()
                    .to_vec(),
            ),
            disabled_at: Some(1),
        });
        bob.add_public_key(disabled);
        assert_eq!(
            recipient_key_choices(&bob),
            vec![
                PrivateDocumentRecipientKey {
                    key_id: ENCRYPTION_KEY_ID,
                    purpose: Purpose::ENCRYPTION,
                },
                PrivateDocumentRecipientKey {
                    key_id: DECRYPTION_KEY_ID,
                    purpose: Purpose::DECRYPTION,
                },
            ]
        );
        assert_eq!(
            recipient_key_choices(&identity(BOB_PHRASE, BOB, false)),
            vec![]
        );
    }

    /// The preview is the fee of the spend at two actions, for the type's
    /// entry size, and refuses what one create cannot append.
    #[test]
    fn should_preview_the_fee_a_create_reserves_at_two_actions() {
        let contract = contract();
        let version = PlatformVersion::latest();
        for entry_count in [1, 3, 16] {
            assert_eq!(
                private_document_create_fee_for(&contract, TYPE_NAME, entry_count, version)
                    .expect("a fee"),
                compute_private_document_create_from_shielded_pool_fee(
                    2,
                    entry_count,
                    ENTRY_SIZE as u32,
                    version
                )
                .expect("a fee")
            );
        }
        for entry_count in [0, 17] {
            assert!(matches!(
                private_document_create_fee_for(&contract, TYPE_NAME, entry_count, version),
                Err(PlatformWalletError::InvalidParameter(_))
            ));
        }
        assert!(matches!(
            private_document_create_fee_for(&contract, "note", 1, version),
            Err(PlatformWalletError::InvalidParameter(message)) if message.contains("not private")
        ));
    }

    /// A provider that derives no key: any use of it fails the test.
    struct NoKeys;

    #[async_trait::async_trait]
    impl ContactCryptoProvider for NoKeys {
        async fn receiving_xpub(
            &self,
            _path: &DerivationPath,
        ) -> Result<key_wallet::bip32::ExtendedPubKey, PlatformWalletError> {
            panic!("the signer is not asked")
        }

        async fn ecdh_shared_secret(
            &self,
            _path: &DerivationPath,
            _peer: &PublicKey,
        ) -> Result<Zeroizing<[u8; 32]>, PlatformWalletError> {
            panic!("the signer is not asked")
        }

        async fn export_auto_accept_private_key(
            &self,
            _path: &DerivationPath,
        ) -> Result<dpp::secp256k1::SecretKey, PlatformWalletError> {
            panic!("the signer is not asked")
        }

        async fn account_reference(
            &self,
            _path: &DerivationPath,
            _compact_xpub: &[u8],
            _account_index: u32,
            _version: u32,
        ) -> Result<u32, PlatformWalletError> {
            panic!("the signer is not asked")
        }

        async fn unmask_account_reference(
            &self,
            _path: &DerivationPath,
            _compact_xpub: &[u8],
            _account_reference: u32,
        ) -> Result<(u32, u32), PlatformWalletError> {
            panic!("the signer is not asked")
        }

        async fn contact_info_seal(
            &self,
            _root_path: &DerivationPath,
            _derivation_index: u32,
            _contact_id: &[u8; 32],
            _private_data_plaintext: &[u8],
            _private_data_iv: &[u8; 16],
        ) -> Result<ContactInfoSealed, PlatformWalletError> {
            panic!("the signer is not asked")
        }

        async fn contact_info_open(
            &self,
            _root_path: &DerivationPath,
            _derivation_index: u32,
            _enc_to_user_id: &[u8; 32],
            _private_data_blob: &[u8],
        ) -> Result<ContactInfoOpened, PlatformWalletError> {
            panic!("the signer is not asked")
        }
    }
}
