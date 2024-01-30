use crate::drive::flags::StorageFlags;
use dpp::data_contract::DataContract;
use dpp::data_contracts;
use dpp::fee::fee_result::FeeResult;
use dpp::system_data_contracts::load_system_data_contract;
#[cfg(feature = "fixtures-and-mocks")]
use dpp::tests::fixtures::get_dashpay_contract_fixture;
#[cfg(feature = "fixtures-and-mocks")]
use dpp::tests::fixtures::get_dpns_data_contract_fixture;
#[cfg(feature = "fixtures-and-mocks")]
use dpp::tests::fixtures::get_masternode_reward_shares_data_contract_fixture;
use grovedb_costs::OperationCost;
use platform_version::version::PlatformVersion;

#[cfg(any(feature = "full", feature = "verify"))]
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
    /// The fee is updated every epoch based on operation costs
    /// Except if protocol version has changed in which case all the cache is cleared
    pub fee: Option<FeeResult>,
}

#[cfg(feature = "fixtures-and-mocks")]
impl DataContractFetchInfo {
    /// This should ONLY be used for tests
    pub fn dpns_contract_fixture(protocol_version: u32) -> Self {
        let dpns = get_dpns_data_contract_fixture(None, protocol_version);
        DataContractFetchInfo {
            contract: dpns.data_contract_owned(),
            storage_flags: None,
            cost: OperationCost::with_seek_count(1), //Just so there's a cost
            fee: Some(FeeResult::new_from_processing_fee(30000)),
        }
    }

    /// This should ONLY be used for tests
    pub fn dashpay_contract_fixture(protocol_version: u32) -> Self {
        let dashpay = get_dashpay_contract_fixture(None, protocol_version);
        DataContractFetchInfo {
            contract: dashpay.data_contract_owned(),
            storage_flags: None,
            cost: OperationCost::with_seek_count(1), //Just so there's a cost
            fee: Some(FeeResult::new_from_processing_fee(30000)),
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
        }
    }
}
