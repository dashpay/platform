use crate::util::storage_flags::StorageFlags;
use dpp::data_contract::DataContract;
#[cfg(feature = "fixtures-and-mocks")]
use dpp::data_contracts;
use dpp::fee::fee_result::FeeResult;
#[cfg(feature = "fixtures-and-mocks")]
use dpp::system_data_contracts::load_system_data_contract;
#[cfg(feature = "fixtures-and-mocks")]
use dpp::tests::fixtures::get_dashpay_contract_fixture;
#[cfg(feature = "fixtures-and-mocks")]
use dpp::tests::fixtures::get_dpns_data_contract_fixture;
#[cfg(feature = "fixtures-and-mocks")]
use dpp::tests::fixtures::get_masternode_reward_shares_data_contract_fixture;
use grovedb_costs::OperationCost;
#[cfg(feature = "fixtures-and-mocks")]
use platform_version::version::PlatformVersion;
use platform_version::version::ProtocolVersion;
/// DataContract and fetch information
#[derive(PartialEq, Debug, Clone)]
pub struct DataContractFetchInfo {
    /// The contract
    pub contract: DataContract,
    /// The contract's potential storage flags
    pub storage_flags: Option<StorageFlags>,
    /// These are the operations that are used to fetch a contract
    /// This is only used on epoch change
    pub(crate) cost: OperationCost,
    /// The fee of the read that built this entry, when it was built with an epoch, which a cache
    /// hit of `get_contract_with_fetch_info` 0 (protocol version 14 and earlier) bills again. A
    /// read's fee depends only on the fee schedule, and from protocol version 14 the contract
    /// cache is cleared on the first block of every protocol change, the only time the schedule
    /// can change. Generation 1 (protocol version 15 on) never bills it: a hit there prices the
    /// entry's `cost` under the reader's protocol version. Entries are cached with and without a
    /// fee, so callers bill the fee `Drive::get_contract_with_fetch_info_and_fee` returns, never
    /// this.
    pub(crate) fee: Option<FeeResult>,
    /// The protocol version whose platform version parsed `contract`. The parsed contract
    /// depends on it as well as on the stored bytes, so from protocol version 15 a cache read
    /// returns an entry only to a reader at this protocol version
    /// (`DataContractCache::get_parsed_under`).
    pub(crate) parsed_under_protocol_version: ProtocolVersion,
}

#[cfg(feature = "fixtures-and-mocks")]
impl DataContractFetchInfo {
    /// This should ONLY be used for tests: whether this entry carries the fee of the read that
    /// built it. Never bill it.
    pub fn has_fee_for_tests(&self) -> bool {
        self.fee.is_some()
    }

    /// This should ONLY be used for tests
    pub fn dpns_contract_fixture(protocol_version: u32) -> Self {
        let dpns = get_dpns_data_contract_fixture(None, 0, protocol_version);
        DataContractFetchInfo {
            contract: dpns.data_contract_owned(),
            storage_flags: None,
            cost: OperationCost::with_seek_count(1), //Just so there's a cost
            fee: Some(FeeResult::new_from_processing_fee(30000)),
            parsed_under_protocol_version: protocol_version,
        }
    }

    /// This should ONLY be used for tests
    pub fn dashpay_contract_fixture(protocol_version: u32) -> Self {
        let dashpay = get_dashpay_contract_fixture(None, 0, protocol_version);
        DataContractFetchInfo {
            contract: dashpay.data_contract_owned(),
            storage_flags: None,
            cost: OperationCost::with_seek_count(1), //Just so there's a cost
            fee: Some(FeeResult::new_from_processing_fee(30000)),
            parsed_under_protocol_version: protocol_version,
        }
    }

    /// This should ONLY be used for tests
    pub fn masternode_rewards_contract_fixture(protocol_version: u32) -> Self {
        let masternode_rewards =
            get_masternode_reward_shares_data_contract_fixture(protocol_version);
        DataContractFetchInfo {
            contract: masternode_rewards,
            storage_flags: None,
            cost: OperationCost::with_seek_count(1), //Just so there's a cost
            fee: Some(FeeResult::new_from_processing_fee(30000)),
            parsed_under_protocol_version: protocol_version,
        }
    }

    /// This should ONLY be used for tests
    pub fn withdrawals_contract_fixture(protocol_version: u32) -> Self {
        let platform_version =
            PlatformVersion::get(protocol_version).expect("expected to get version");

        let contract = load_system_data_contract(
            data_contracts::SystemDataContract::Withdrawals,
            platform_version,
        )
        .expect("to load system data contract");

        DataContractFetchInfo {
            contract,
            storage_flags: None,
            cost: OperationCost::with_seek_count(1), //Just so there's a cost
            fee: Some(FeeResult::new_from_processing_fee(30000)),
            parsed_under_protocol_version: protocol_version,
        }
    }

    /// This should ONLY be used for tests
    pub fn moderation_charters_contract_fixture(protocol_version: u32) -> Self {
        let platform_version =
            PlatformVersion::get(protocol_version).expect("expected to get version");

        let contract = load_system_data_contract(
            data_contracts::SystemDataContract::ModerationCharters,
            platform_version,
        )
        .expect("to load system data contract");

        DataContractFetchInfo {
            contract,
            storage_flags: None,
            cost: OperationCost::with_seek_count(1), //Just so there's a cost
            fee: Some(FeeResult::new_from_processing_fee(30000)),
            parsed_under_protocol_version: protocol_version,
        }
    }
}
