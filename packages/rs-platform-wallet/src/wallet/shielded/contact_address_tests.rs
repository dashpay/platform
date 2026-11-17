//! The shielded address a contact request carries, as the identity wallet derives it
//! from the sending identity's shielded account once the send has proved the
//! identity's slot through its crypto provider.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use dashcore::Network;
use dpp::data_contracts::SystemDataContract;
use dpp::identity::{Identity, IdentityV0};
use dpp::prelude::Identifier;
use dpp::secp256k1::{PublicKey, SecretKey};
use dpp::system_data_contracts::load_system_data_contract;
use dpp::version::PlatformVersion;
use key_wallet::bip32::{DerivationPath, ExtendedPrivKey, ExtendedPubKey};
use key_wallet_manager::WalletManager;
use tokio::sync::RwLock;

use super::viewing_key_bind_tests::{
    coordinator_at, seed_master_key, temp_dir, CapturingPersistence,
};
use crate::error::PlatformWalletError;
use crate::wallet::identity::network::{
    derive_identity_auth_key_hash_from_master, ContactCryptoProvider, ContactInfoOpened,
    ContactInfoSealed, SeedCryptoProvider, MASTER_KEY_INDEX,
};
use crate::wallet::persister::WalletPersister;
use crate::wallet::platform_wallet::{PlatformWallet, PlatformWalletInfo, WalletId};
use crate::wallet::shielded::keys::OrchardKeySet;
use crate::wallet::shielded::{shielded_identity_account_index, AccountViewingKeys};

/// The slot the fixture's identity holds.
const IDENTITY_INDEX: u32 = 2;
const BOB: [u8; 32] = [0x22; 32];
const CAROL: [u8; 32] = [0x33; 32];
const ACCOUNT_REFERENCE: u32 = 0x0123_4567;

/// A wallet whose identity holds slot 2, on a network that serves the DashPay contract
/// of `contract_protocol_version`. With `bind`, the wallet is bound to account 0, which
/// binds the identity's account too.
async fn fixture(
    tag: &str,
    contract_protocol_version: u32,
    bind: bool,
) -> (PlatformWallet, [u8; 64], Identifier) {
    let phrase = crate::test_support::MESSAGE_SIGNING_TEST_MNEMONIC;
    let seed = key_wallet::Mnemonic::from_phrase(phrase)
        .unwrap()
        .to_seed("");
    let (wallet_manager, wallet_id, _, _) =
        crate::test_support::mnemonic_wallet_manager(phrase).await;
    let generation = wallet_manager
        .read()
        .await
        .get_wallet_info(&wallet_id)
        .unwrap()
        .generation
        .clone();
    let spv = Arc::new(crate::spv::SpvRuntime::new(
        Arc::clone(&wallet_manager),
        Arc::new(crate::events::PlatformEventManager::new(Vec::new())),
    ));
    let mock_dir = temp_dir(tag);
    std::fs::create_dir_all(&mock_dir).unwrap();
    let mut sdk = dash_sdk::SdkBuilder::new_mock()
        .with_network(Network::Testnet)
        .with_dump_dir(&mock_dir)
        .with_version(PlatformVersion::latest())
        .build()
        .unwrap();
    let contract = load_system_data_contract(
        SystemDataContract::Dashpay,
        PlatformVersion::get(contract_protocol_version).unwrap(),
    )
    .unwrap();
    sdk.mock()
        .expect_fetch(SystemDataContract::Dashpay.id(), Some(contract))
        .await
        .unwrap();
    let wallet = PlatformWallet::new(
        Arc::new(sdk),
        wallet_id,
        wallet_manager,
        generation,
        Arc::new(tokio::sync::Notify::new()),
        Arc::new(CapturingPersistence::default()),
        Arc::new(crate::broadcaster::SpvBroadcaster::new(spv)),
        Arc::new(std::sync::atomic::AtomicBool::new(false)),
        crate::changeset::DurableCursors::default(),
    );
    let id = Identifier::from([0x66; 32]);
    {
        let mut wm = wallet.wallet_manager.write().await;
        wm.get_wallet_info_mut(&wallet.wallet_id())
            .unwrap()
            .identity_manager
            .add_identity(
                identity(id, &seed[..], IDENTITY_INDEX),
                IDENTITY_INDEX,
                wallet.wallet_id(),
                wallet.persister(),
            )
            .unwrap();
    }
    if bind {
        let coordinator = coordinator_at(&temp_dir(tag));
        wallet
            .bind_shielded(&seed[..], &[0], &coordinator)
            .await
            .unwrap();
    }
    (wallet, *seed, id)
}

/// The identity `id` whose MASTER key is the one `seed` derives at `key_slot`.
fn identity(id: Identifier, seed: &[u8], key_slot: u32) -> Identity {
    Identity::V0(IdentityV0 {
        id,
        public_keys: seed_master_key(seed, key_slot),
        balance: 0,
        revision: 0,
    })
}

/// Remove `id` and add it back at `slot`, carrying the MASTER key `seed` derives at
/// `key_slot`.
async fn add_back(
    wallet_manager: &RwLock<WalletManager<PlatformWalletInfo>>,
    wallet_id: &WalletId,
    persister: &WalletPersister,
    id: Identifier,
    seed: &[u8],
    slot: u32,
    key_slot: u32,
) {
    let mut wm = wallet_manager.write().await;
    let manager = &mut wm.get_wallet_info_mut(wallet_id).unwrap().identity_manager;
    manager.remove_identity(&id, persister).unwrap();
    manager
        .add_identity(identity(id, seed, key_slot), slot, *wallet_id, persister)
        .unwrap();
}

/// The provider of the wallet's own seed.
fn own_provider(seed: &[u8; 64]) -> SeedCryptoProvider {
    SeedCryptoProvider::from_seed(*seed, Network::Testnet)
}

async fn address_for<C: ContactCryptoProvider + Sync>(
    wallet: &PlatformWallet,
    owner: &Identifier,
    to: [u8; 32],
    account_reference: u32,
    crypto: &C,
) -> Option<[u8; 43]> {
    wallet
        .identity()
        .contact_shielded_address(owner, &Identifier::from(to), account_reference, crypto)
        .await
        .expect("the address is derived or left out")
        .map(|derived| derived.address)
}

/// A wallet's bound shielded viewing keys, by account.
type BoundKeys = Arc<RwLock<Option<BTreeMap<u32, AccountViewingKeys>>>>;

/// What [`SlotProvider`] does when it is asked for a public key, before it answers.
enum OnDerive {
    /// Answers as the seed provider does.
    Answer,
    /// Fails, as a host provider that cannot reach its seed does.
    Fail,
    /// Removes the identity and adds it back at `slot` with the key of `key_slot`
    /// before it answers, once.
    Replace {
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
        persister: WalletPersister,
        id: Identifier,
        seed: [u8; 64],
        slot: u32,
        key_slot: u32,
    },
    /// Binds `with` as the keys of `account`, or unbinds the account when `with` is
    /// `None`, before it answers, once: what a Clear and a bind again with other keys
    /// leave in the key map.
    ReplaceKeys {
        keys: BoundKeys,
        account: u32,
        with: Option<AccountViewingKeys>,
    },
}

/// A seed provider that does [`OnDerive`] when asked for a public key: the one wait in
/// which the derivation of the address holds no wallet-manager lock and has already
/// read the identity's slot.
struct SlotProvider {
    inner: SeedCryptoProvider,
    on_derive: Mutex<Option<OnDerive>>,
}

impl SlotProvider {
    fn new(seed: &[u8; 64], on_derive: OnDerive) -> Self {
        Self {
            inner: own_provider(seed),
            on_derive: Mutex::new(Some(on_derive)),
        }
    }
}

#[async_trait::async_trait]
impl ContactCryptoProvider for SlotProvider {
    async fn receiving_xpub(
        &self,
        path: &DerivationPath,
    ) -> Result<ExtendedPubKey, PlatformWalletError> {
        let on_derive = self.on_derive.lock().unwrap().take();
        match on_derive {
            None | Some(OnDerive::Answer) => {}
            Some(OnDerive::Fail) => {
                return Err(PlatformWalletError::InvalidIdentityData(
                    "the seed is not available".to_string(),
                ));
            }
            Some(OnDerive::Replace {
                wallet_manager,
                wallet_id,
                persister,
                id,
                seed,
                slot,
                key_slot,
            }) => {
                add_back(
                    &wallet_manager,
                    &wallet_id,
                    &persister,
                    id,
                    &seed,
                    slot,
                    key_slot,
                )
                .await;
            }
            Some(OnDerive::ReplaceKeys {
                keys,
                account,
                with,
            }) => {
                let mut bound = keys.write().await;
                let bound = bound.as_mut().expect("the wallet is bound");
                match with {
                    Some(with) => {
                        bound.insert(account, with);
                    }
                    None => {
                        bound.remove(&account);
                    }
                }
            }
        }
        self.inner.receiving_xpub(path).await
    }

    async fn ecdh_shared_secret(
        &self,
        path: &DerivationPath,
        peer: &PublicKey,
    ) -> Result<zeroize::Zeroizing<[u8; 32]>, PlatformWalletError> {
        self.inner.ecdh_shared_secret(path, peer).await
    }

    async fn export_auto_accept_private_key(
        &self,
        path: &DerivationPath,
    ) -> Result<SecretKey, PlatformWalletError> {
        self.inner.export_auto_accept_private_key(path).await
    }

    async fn account_reference(
        &self,
        path: &DerivationPath,
        compact_xpub: &[u8],
        account_index: u32,
        version: u32,
    ) -> Result<u32, PlatformWalletError> {
        self.inner
            .account_reference(path, compact_xpub, account_index, version)
            .await
    }

    async fn unmask_account_reference(
        &self,
        path: &DerivationPath,
        compact_xpub: &[u8],
        account_reference: u32,
    ) -> Result<(u32, u32), PlatformWalletError> {
        self.inner
            .unmask_account_reference(path, compact_xpub, account_reference)
            .await
    }

    async fn contact_info_seal(
        &self,
        root_path: &DerivationPath,
        derivation_index: u32,
        contact_id: &[u8; 32],
        private_data_plaintext: &[u8],
        private_data_iv: &[u8; 16],
    ) -> Result<ContactInfoSealed, PlatformWalletError> {
        self.inner
            .contact_info_seal(
                root_path,
                derivation_index,
                contact_id,
                private_data_plaintext,
                private_data_iv,
            )
            .await
    }

    async fn contact_info_open(
        &self,
        root_path: &DerivationPath,
        derivation_index: u32,
        enc_to_user_id: &[u8; 32],
        private_data_blob: &[u8],
    ) -> Result<ContactInfoOpened, PlatformWalletError> {
        self.inner
            .contact_info_open(
                root_path,
                derivation_index,
                enc_to_user_id,
                private_data_blob,
            )
            .await
    }
}

#[tokio::test]
async fn should_give_a_contact_the_address_of_the_identity_account_with_no_preparation() {
    let (wallet, seed, id) = fixture("contact_address_unprepared", 15, true).await;
    let crypto = own_provider(&seed);
    // Nothing in this session prepared the identity's shielded account.
    assert_eq!(wallet.prepared_shielded_identity_account(&id).await, None);

    let to_bob = address_for(&wallet, &id, BOB, ACCOUNT_REFERENCE, &crypto)
        .await
        .expect("an address for Bob");

    // It is the address the identity's own shielded account derives for the request.
    let account = shielded_identity_account_index(IDENTITY_INDEX).unwrap();
    let keys = OrchardKeySet::from_seed(&seed, Network::Testnet, account)
        .unwrap()
        .viewing_keys();
    assert_eq!(
        to_bob,
        keys.contact_address(id.as_bytes(), &BOB, u64::from(ACCOUNT_REFERENCE))
            .to_raw_address_bytes()
    );

    // The same request gives the same address; another contact, and the same contact
    // under another account reference, get another.
    assert_eq!(
        address_for(&wallet, &id, BOB, ACCOUNT_REFERENCE, &crypto).await,
        Some(to_bob)
    );
    let to_carol = address_for(&wallet, &id, CAROL, ACCOUNT_REFERENCE, &crypto)
        .await
        .expect("an address for Carol");
    assert_ne!(to_carol, to_bob);
    let rotated = address_for(&wallet, &id, BOB, ACCOUNT_REFERENCE + (1 << 28), &crypto)
        .await
        .expect("an address for the rotated request");
    assert_ne!(rotated, to_bob);
}

#[tokio::test]
async fn should_send_no_address_when_the_provider_is_of_another_seed() {
    let (wallet, _, id) = fixture("contact_address_other_seed", 15, true).await;
    // The identity does not carry the key this seed derives at its slot.
    let other = SeedCryptoProvider::from_seed([0x42; 64], Network::Testnet);

    assert_eq!(
        address_for(&wallet, &id, BOB, ACCOUNT_REFERENCE, &other).await,
        None
    );
}

#[tokio::test]
async fn should_send_no_address_from_a_wallet_never_bound() {
    let (wallet, seed, id) = fixture("contact_address_never_bound", 15, false).await;

    assert_eq!(
        address_for(&wallet, &id, BOB, ACCOUNT_REFERENCE, &own_provider(&seed)).await,
        None
    );
}

/// The identity moves to a slot whose shielded account the bind did not cover. It
/// carries the key the seed derives there, so the slot is proved, but the account's
/// viewing keys are not bound and no address is derived from another account's.
#[tokio::test]
async fn should_send_no_address_when_the_identity_account_is_not_bound() {
    let (wallet, seed, id) = fixture("contact_address_unbound_account", 15, true).await;
    let moved = IDENTITY_INDEX + 1;
    add_back(
        &wallet.wallet_manager,
        &wallet.wallet_id(),
        wallet.persister(),
        id,
        &seed,
        moved,
        moved,
    )
    .await;
    let account = shielded_identity_account_index(moved).unwrap();
    assert!(!wallet.shielded_account_indices().await.contains(&account));

    assert_eq!(
        address_for(&wallet, &id, BOB, ACCOUNT_REFERENCE, &own_provider(&seed)).await,
        None
    );
}

#[tokio::test]
async fn should_send_no_address_where_contact_requests_cannot_carry_one() {
    // Up to protocol version 14 the network's DashPay contract has no property for it.
    let (wallet, seed, id) = fixture("contact_address_before_v3", 14, true).await;

    assert_eq!(
        address_for(&wallet, &id, BOB, ACCOUNT_REFERENCE, &own_provider(&seed)).await,
        None
    );
}

#[tokio::test]
async fn should_send_no_address_for_an_identity_the_wallet_does_not_hold() {
    let (wallet, seed, _) = fixture("contact_address_other_identity", 15, true).await;

    assert_eq!(
        address_for(
            &wallet,
            &Identifier::from([0x77; 32]),
            BOB,
            ACCOUNT_REFERENCE,
            &own_provider(&seed)
        )
        .await,
        None
    );
}

/// An identity removed and added back at the same slot is another record under the same
/// id: with another key than the one the seed derives at the slot, the slot is not
/// proved and it gives no address; with that key, it gives the same one as before.
#[tokio::test]
async fn should_send_no_address_for_an_identity_added_back_without_the_proved_key() {
    let (wallet, seed, id) = fixture("contact_address_added_back", 15, true).await;
    let crypto = own_provider(&seed);
    let address = address_for(&wallet, &id, BOB, ACCOUNT_REFERENCE, &crypto).await;
    assert!(address.is_some());

    for (key_slot, expected) in [(IDENTITY_INDEX + 3, None), (IDENTITY_INDEX, address)] {
        add_back(
            &wallet.wallet_manager,
            &wallet.wallet_id(),
            wallet.persister(),
            id,
            &seed,
            IDENTITY_INDEX,
            key_slot,
        )
        .await;
        assert_eq!(
            address_for(&wallet, &id, BOB, ACCOUNT_REFERENCE, &crypto).await,
            expected
        );
    }
}

/// The derivation asks the crypto provider for the key of the identity's slot with the
/// wallet-manager lock released. An identity removed and added back during that wait,
/// without the proved key or at another slot, gets no address: the check is made again
/// as the address is derived. Added back as it was, it gets its address.
#[tokio::test]
async fn should_send_no_address_for_an_identity_replaced_while_the_address_is_derived() {
    let (wallet, seed, id) = fixture("contact_address_replaced", 15, true).await;
    let address = address_for(&wallet, &id, BOB, ACCOUNT_REFERENCE, &own_provider(&seed)).await;
    assert!(address.is_some());

    for (slot, key_slot, expected) in [
        // Without the proved key.
        (IDENTITY_INDEX, IDENTITY_INDEX + 3, None),
        // At another slot, with the key of that slot.
        (IDENTITY_INDEX + 1, IDENTITY_INDEX + 1, None),
        // As it was.
        (IDENTITY_INDEX, IDENTITY_INDEX, address),
    ] {
        let replacing = SlotProvider::new(
            &seed,
            OnDerive::Replace {
                wallet_manager: Arc::clone(&wallet.wallet_manager),
                wallet_id: wallet.wallet_id(),
                persister: wallet.persister().clone(),
                id,
                seed,
                slot,
                key_slot,
            },
        );
        assert_eq!(
            address_for(&wallet, &id, BOB, ACCOUNT_REFERENCE, &replacing).await,
            expected,
            "added back at slot {slot} with the key of slot {key_slot}"
        );
        // Back as the fixture holds it, for the next case.
        add_back(
            &wallet.wallet_manager,
            &wallet.wallet_id(),
            wallet.persister(),
            id,
            &seed,
            IDENTITY_INDEX,
            IDENTITY_INDEX,
        )
        .await;
    }
}

#[tokio::test]
async fn should_send_without_an_address_when_the_provider_cannot_derive_the_key() {
    let (wallet, seed, id) = fixture("contact_address_provider_fails", 15, true).await;
    let failing = SlotProvider::new(&seed, OnDerive::Fail);

    let derived = wallet
        .identity()
        .contact_shielded_address(&id, &Identifier::from(BOB), ACCOUNT_REFERENCE, &failing)
        .await
        .map(|derived| derived.map(|derived| derived.address));
    assert!(
        matches!(derived, Ok(None)),
        "the send goes on without an address: {derived:?}"
    );

    // The same provider, answering, gives the address.
    let answering = SlotProvider::new(&seed, OnDerive::Answer);
    assert!(
        address_for(&wallet, &id, BOB, ACCOUNT_REFERENCE, &answering)
            .await
            .is_some()
    );
}

#[tokio::test]
async fn should_prove_a_slot_with_the_key_hash_the_seed_derives_there() {
    let (wallet, seed, id) = fixture("contact_address_key_hash", 15, false).await;
    let crypto = own_provider(&seed);
    let master = ExtendedPrivKey::new_master(Network::Testnet, &seed).unwrap();

    for slot in [0, IDENTITY_INDEX, 7, 1000] {
        let from_provider = wallet
            .identity()
            .seed_master_key_hash(&id, slot, &crypto)
            .await
            .expect("the provider derives the key");
        let from_seed = derive_identity_auth_key_hash_from_master(
            &master,
            Network::Testnet,
            slot,
            MASTER_KEY_INDEX,
        )
        .unwrap();
        assert_eq!(from_provider, from_seed, "slot {slot}");
    }
}

/// The keys of the identity's account are replaced while the provider is asked for the
/// identity's key, as a Clear and a bind again with other keys would replace them. The
/// address is the one of the keys bound when it is derived, or none when the account is
/// no longer bound: never the one of the keys bound before the provider was asked.
#[tokio::test]
async fn should_derive_the_address_from_the_keys_bound_when_it_is_derived() {
    let (wallet, seed, id) = fixture("contact_address_keys_replaced", 15, true).await;
    let account = shielded_identity_account_index(IDENTITY_INDEX).unwrap();
    let before = address_for(&wallet, &id, BOB, ACCOUNT_REFERENCE, &own_provider(&seed))
        .await
        .expect("an address from the keys bound first");
    let other_keys = OrchardKeySet::from_seed(&[0x42; 64], Network::Testnet, account)
        .unwrap()
        .viewing_keys();
    let other_address = other_keys
        .contact_address(id.as_bytes(), &BOB, u64::from(ACCOUNT_REFERENCE))
        .to_raw_address_bytes();
    assert_ne!(other_address, before);

    for (with, expected) in [(Some(other_keys), Some(other_address)), (None, None)] {
        let replacing = SlotProvider::new(
            &seed,
            OnDerive::ReplaceKeys {
                keys: Arc::clone(&wallet.shielded_keys),
                account,
                with,
            },
        );
        assert_eq!(
            address_for(&wallet, &id, BOB, ACCOUNT_REFERENCE, &replacing).await,
            expected
        );
    }
}

/// A coordinator Clear drops the wallet's registration and its notes, and leaves the
/// viewing keys the wallet holds in memory: the address is still given from them, the
/// same as before, until the wallet is bound again.
#[tokio::test]
async fn should_still_give_the_address_after_a_clear_from_the_keys_held_in_memory() {
    let (wallet, seed, id) = fixture("contact_address_cleared", 15, false).await;
    let coordinator = coordinator_at(&temp_dir("contact_address_cleared"));
    wallet
        .bind_shielded(&seed[..], &[0], &coordinator)
        .await
        .unwrap();
    let crypto = own_provider(&seed);
    let before = address_for(&wallet, &id, BOB, ACCOUNT_REFERENCE, &crypto)
        .await
        .expect("an address while bound");

    coordinator.clear().await.unwrap();
    assert!(coordinator.registered_subwallets().await.is_empty());
    assert!(wallet.is_shielded_bound().await, "the keys stay in memory");
    assert_eq!(
        address_for(&wallet, &id, BOB, ACCOUNT_REFERENCE, &crypto).await,
        Some(before)
    );
}
