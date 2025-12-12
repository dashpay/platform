use dash_sdk::platform::{Fetch, FetchMany};
use drive_proof_verifier::types::AddressInfo;
use std::collections::BTreeSet;

use super::{
    common::setup_logs,
    config::Config,
    generated_data::{
        PLATFORM_ADDRESS_1, PLATFORM_ADDRESS_1_BALANCE, PLATFORM_ADDRESS_1_NONCE,
        PLATFORM_ADDRESS_2, PLATFORM_ADDRESS_2_BALANCE, PLATFORM_ADDRESS_2_NONCE,
        UNKNOWN_PLATFORM_ADDRESS,
    },
};

/// Given an existing platform address, when I fetch address info, I get balance and nonce.
/// TODO: Lukazs please fix
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[ignore]
async fn test_fetch_address_info() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_fetch_address_info").await;

    let info = AddressInfo::fetch(&sdk, PLATFORM_ADDRESS_1)
        .await
        .expect("fetch address info")
        .expect("address info present");

    assert_eq!(info.address, PLATFORM_ADDRESS_1);
    assert_eq!(info.nonce, PLATFORM_ADDRESS_1_NONCE);
    assert_eq!(info.balance, PLATFORM_ADDRESS_1_BALANCE);
}

/// Given multiple platform addresses, when I fetch infos, I get them indexed by address.
/// TODO: Lukazs please fix
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[ignore]
async fn test_fetch_addresses_infos() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_fetch_addresses_infos").await;

    let addresses = BTreeSet::from([
        PLATFORM_ADDRESS_1,
        PLATFORM_ADDRESS_2,
        UNKNOWN_PLATFORM_ADDRESS,
    ]);

    let infos = AddressInfo::fetch_many(&sdk, addresses.clone())
        .await
        .expect("fetch addresses infos");

    assert_eq!(infos.len(), addresses.len());

    let info_one = infos
        .get(&PLATFORM_ADDRESS_1)
        .expect("entry for address 1")
        .as_ref()
        .expect("address 1 should exist");
    assert_eq!(info_one.nonce, PLATFORM_ADDRESS_1_NONCE);
    assert_eq!(info_one.balance, PLATFORM_ADDRESS_1_BALANCE);

    let info_two = infos
        .get(&PLATFORM_ADDRESS_2)
        .expect("entry for address 2")
        .as_ref()
        .expect("address 2 should exist");
    assert_eq!(info_two.nonce, PLATFORM_ADDRESS_2_NONCE);
    assert_eq!(info_two.balance, PLATFORM_ADDRESS_2_BALANCE);

    assert!(
        infos
            .get(&UNKNOWN_PLATFORM_ADDRESS)
            .expect("entry for unknown address")
            .is_none(),
        "unknown address should be absent"
    );
}
