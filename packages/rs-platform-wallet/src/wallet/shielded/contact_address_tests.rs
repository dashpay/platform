//! The shielded address a contact request carries, as the identity wallet derives it
//! from the sending identity's shielded account.

use std::sync::Arc;

use dashcore::Network;
use dpp::data_contracts::SystemDataContract;
use dpp::identity::{Identity, IdentityV0};
use dpp::prelude::Identifier;
use dpp::system_data_contracts::load_system_data_contract;
use dpp::version::PlatformVersion;

use super::viewing_key_bind_tests::{
    coordinator_at, seed_master_key, temp_dir, CapturingPersistence,
};
use crate::wallet::platform_wallet::PlatformWallet;
use crate::wallet::shielded::keys::OrchardKeySet;
use crate::wallet::shielded::{shielded_identity_account_index, NetworkShieldedCoordinator};

/// The slot the fixture's identity holds.
const IDENTITY_INDEX: u32 = 2;
const BOB: [u8; 32] = [0x22; 32];
const CAROL: [u8; 32] = [0x33; 32];
const ACCOUNT_REFERENCE: u32 = 0x0123_4567;

/// A wallet bound to account 0 whose identity holds slot 2, on a network that serves the
/// DashPay contract of `contract_protocol_version`.
async fn fixture(
    tag: &str,
    contract_protocol_version: u32,
) -> (
    PlatformWallet,
    [u8; 64],
    Identifier,
    Arc<NetworkShieldedCoordinator>,
) {
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
                Identity::V0(IdentityV0 {
                    id,
                    public_keys: seed_master_key(&seed[..], IDENTITY_INDEX),
                    balance: 0,
                    revision: 0,
                }),
                IDENTITY_INDEX,
                wallet.wallet_id(),
                wallet.persister(),
            )
            .unwrap();
    }
    let coordinator = coordinator_at(&temp_dir(tag));
    wallet
        .bind_shielded(&seed[..], &[0], &coordinator)
        .await
        .unwrap();
    (wallet, *seed, id, coordinator)
}

async fn address_for(
    wallet: &PlatformWallet,
    owner: &Identifier,
    to: [u8; 32],
    account_reference: u32,
) -> Option<[u8; 43]> {
    wallet
        .identity()
        .contact_shielded_address(owner, &Identifier::from(to), account_reference)
        .await
        .expect("the address is derived or left out")
}

#[tokio::test]
async fn should_give_a_contact_the_address_of_the_prepared_identity_account() {
    let (wallet, seed, id, coordinator) = fixture("contact_address_prepared", 15).await;
    wallet
        .prepare_shielded_identity_address(&seed, &id, &coordinator)
        .await
        .unwrap();

    let to_bob = address_for(&wallet, &id, BOB, ACCOUNT_REFERENCE)
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
        address_for(&wallet, &id, BOB, ACCOUNT_REFERENCE).await,
        Some(to_bob)
    );
    let to_carol = address_for(&wallet, &id, CAROL, ACCOUNT_REFERENCE)
        .await
        .expect("an address for Carol");
    assert_ne!(to_carol, to_bob);
    let rotated = address_for(&wallet, &id, BOB, ACCOUNT_REFERENCE + (1 << 28))
        .await
        .expect("an address for the rotated request");
    assert_ne!(rotated, to_bob);
}

#[tokio::test]
async fn should_send_no_address_until_the_identity_account_is_prepared() {
    let (wallet, seed, id, coordinator) = fixture("contact_address_unprepared", 15).await;
    // The account's keys are bound, but nothing in this session proved with the seed
    // that the identity holds the slot the account belongs to.
    let account = shielded_identity_account_index(IDENTITY_INDEX).unwrap();
    wallet
        .bind_shielded(&seed[..], &[0, account], &coordinator)
        .await
        .unwrap();
    assert_eq!(
        address_for(&wallet, &id, BOB, ACCOUNT_REFERENCE).await,
        None
    );

    wallet
        .prepare_shielded_identity_address(&seed, &id, &coordinator)
        .await
        .unwrap();
    assert!(address_for(&wallet, &id, BOB, ACCOUNT_REFERENCE)
        .await
        .is_some());
}

#[tokio::test]
async fn should_send_no_address_once_the_preparation_is_cleared() {
    let (wallet, seed, id, coordinator) = fixture("contact_address_cleared", 15).await;
    wallet
        .prepare_shielded_identity_address(&seed, &id, &coordinator)
        .await
        .unwrap();
    assert!(address_for(&wallet, &id, BOB, ACCOUNT_REFERENCE)
        .await
        .is_some());

    coordinator.clear().await.unwrap();
    assert_eq!(
        address_for(&wallet, &id, BOB, ACCOUNT_REFERENCE).await,
        None
    );
}

#[tokio::test]
async fn should_send_no_address_where_contact_requests_cannot_carry_one() {
    // Up to protocol version 14 the network's DashPay contract has no property for it.
    let (wallet, seed, id, coordinator) = fixture("contact_address_before_v3", 14).await;
    wallet
        .prepare_shielded_identity_address(&seed, &id, &coordinator)
        .await
        .unwrap();

    assert_eq!(
        address_for(&wallet, &id, BOB, ACCOUNT_REFERENCE).await,
        None
    );
}

#[tokio::test]
async fn should_send_no_address_for_an_identity_the_wallet_does_not_hold() {
    let (wallet, seed, id, coordinator) = fixture("contact_address_other_identity", 15).await;
    wallet
        .prepare_shielded_identity_address(&seed, &id, &coordinator)
        .await
        .unwrap();

    assert_eq!(
        address_for(
            &wallet,
            &Identifier::from([0x77; 32]),
            BOB,
            ACCOUNT_REFERENCE
        )
        .await,
        None
    );
}
