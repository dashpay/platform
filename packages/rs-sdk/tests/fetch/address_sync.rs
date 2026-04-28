use async_trait::async_trait;
use dash_sdk::platform::address_sync::{
    sync_address_balances, AddressFunds, AddressIndex, AddressProvider,
};
use dpp::address_funds::PlatformAddress;
use std::collections::{BTreeMap, BTreeSet};

use super::{
    common::setup_logs,
    config::Config,
    generated_data::{
        PLATFORM_ADDRESS_1, PLATFORM_ADDRESS_1_BALANCE, PLATFORM_ADDRESS_2,
        PLATFORM_ADDRESS_2_BALANCE, UNKNOWN_PLATFORM_ADDRESS,
    },
};

struct TestAddressProvider {
    gap_limit: AddressIndex,
    pending: BTreeMap<AddressIndex, PlatformAddress>,
    found: BTreeMap<(AddressIndex, PlatformAddress), AddressFunds>,
    absent: BTreeSet<(AddressIndex, PlatformAddress)>,
}

impl TestAddressProvider {
    fn new(pending: Vec<(AddressIndex, PlatformAddress)>) -> Self {
        Self {
            gap_limit: 0,
            pending: pending.into_iter().collect(),
            found: BTreeMap::new(),
            absent: BTreeSet::new(),
        }
    }
}

#[async_trait]
impl AddressProvider for TestAddressProvider {
    type Tag = AddressIndex;
    type Address = PlatformAddress;

    fn gap_limit(&self) -> AddressIndex {
        self.gap_limit
    }

    fn pending_addresses(&self) -> impl Iterator<Item = (AddressIndex, PlatformAddress)> + '_ {
        self.pending
            .iter()
            .map(|(index, address)| (*index, *address))
    }

    fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    async fn on_address_found(
        &mut self,
        index: AddressIndex,
        address: &PlatformAddress,
        funds: AddressFunds,
    ) {
        self.found.insert((index, *address), funds);
        self.pending.remove(&index);
    }

    async fn on_address_absent(&mut self, index: AddressIndex, address: &PlatformAddress) {
        self.absent.insert((index, *address));
        self.pending.remove(&index);
    }

    fn current_balances(
        &self,
    ) -> impl Iterator<Item = (AddressIndex, PlatformAddress, AddressFunds)> + '_ {
        std::iter::empty()
    }
}

/// Given several platform addresses, when I sync balances, I get found and absent entries.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_sync_address_balances() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_sync_address_balances").await;

    let addr_1 = PLATFORM_ADDRESS_1;
    let addr_2 = PLATFORM_ADDRESS_2;
    let addr_unknown = UNKNOWN_PLATFORM_ADDRESS;

    let mut provider = TestAddressProvider::new(vec![(0, addr_1), (1, addr_2), (2, addr_unknown)]);

    let result = sync_address_balances(&sdk, &mut provider, None, None)
        .await
        .expect("sync address balances");

    assert_eq!(result.found.len(), 2);
    assert_eq!(result.absent.len(), 1);

    assert_eq!(
        result
            .found
            .get(&(0, addr_1))
            .expect("found address 0")
            .balance,
        PLATFORM_ADDRESS_1_BALANCE
    );
    assert_eq!(
        result
            .found
            .get(&(1, addr_2))
            .expect("found address 1")
            .balance,
        PLATFORM_ADDRESS_2_BALANCE
    );
    assert!(result.absent.contains(&(2, addr_unknown)));

    assert_eq!(
        result.total_balance(),
        PLATFORM_ADDRESS_1_BALANCE + PLATFORM_ADDRESS_2_BALANCE
    );
}
