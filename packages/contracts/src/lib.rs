use dpp::data_contract::extra::common::json_document_to_value;
use dpp::data_contract::validation::data_contract_validator::DataContractValidator;
use dpp::data_contract::{DataContract, DataContractFactory};
use dpp::identifier::Identifier;
use dpp::version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION};
use dpp::ProtocolError;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::Arc;
use crate::SystemContract::{Dashpay, DPNS, FeatureFlags, MasternodeRewards, Withdrawals};

/// Masternode reward shares contract ID
pub const MN_REWARD_SHARES_CONTRACT_ID: [u8; 32] = [
    0x0c, 0xac, 0xe2, 0x05, 0x24, 0x66, 0x93, 0xa7, 0xc8, 0x15, 0x65, 0x23, 0x62, 0x0d, 0xaa, 0x93,
    0x7d, 0x2f, 0x22, 0x47, 0x93, 0x44, 0x63, 0xee, 0xb0, 0x1f, 0xf7, 0x21, 0x95, 0x90, 0x95, 0x8c,
];


#[repr(u8)]
#[derive(PartialEq, Eq, Clone, Copy, Debug, Ord, PartialOrd)]
pub enum SystemContract {
    Withdrawals = 0,
    MasternodeRewards = 1,
    FeatureFlags = 2,
    DPNS = 3,
    Dashpay = 4,
}

impl SystemContract {
    pub fn contract_id() -> [u8;32] {
        match self {
            Withdrawals => {
                "../contracts/withdrawals-contract/schema/withdrawals-documents.json"
            }
            MasternodeRewards => {
                MN_REWARD_SHARES_CONTRACT_ID
            }
            FeatureFlags => {
                "../contracts/feature-flags-contract/schema/feature-flags-documents.json"
            }
            DPNS => "../contracts/dpns-contract/schema/dpns-contract-documents.json",
            Dashpay => "../contracts/dashpay-contract/schema/dashpay.schema.json",
        }
    }

    pub fn all_contracts() -> BTreeSet<Self> {
        BTreeSet::from([Withdrawals, MasternodeRewards, FeatureFlags, DPNS, Dashpay])
    }

    pub fn path_to_contract(&self) -> &str {
        match self {
            Withdrawals => {
                "/withdrawals-contract/schema/withdrawals-documents.json"
            }
            MasternodeRewards => {
                "/masternode-reward-shares-contract/schema/masternode-reward-shares-documents.json"
            }
            FeatureFlags => {
                "/feature-flags-contract/schema/feature-flags-documents.json"
            }
            DPNS => "/dpns-contract/schema/dpns-contract-documents.json",
            Dashpay => "/dashpay-contract/schema/dashpay.schema.json",
        }
    }
    pub fn load_contract(
        &self,
        owner_id: Identifier,
    ) -> Result<DataContract, ProtocolError> {
        let protocol_version_validator = ProtocolVersionValidator::new(
            LATEST_VERSION,
            LATEST_VERSION,
            COMPATIBILITY_MAP.clone(),
        );
        let data_contract_validator =
            DataContractValidator::new(Arc::new(protocol_version_validator));
        let factory = DataContractFactory::new(1, data_contract_validator);
        let value = serde_json::from_;
        let mut contract = factory.create(owner_id, value, None)?;
        contract.id = self.contract_id()
    }

    pub fn load_contracts(
        system_contracts: BTreeSet<SystemContract>,
        owner_id: Identifier,
    ) -> Result<BTreeMap<SystemContract, DataContract>, ProtocolError> {
        let protocol_version_validator = ProtocolVersionValidator::new(
            LATEST_VERSION,
            LATEST_VERSION,
            COMPATIBILITY_MAP.clone(),
        );
        let data_contract_validator =
            DataContractValidator::new(Arc::new(protocol_version_validator));
        let factory = DataContractFactory::new(1, data_contract_validator);
        system_contracts
            .into_iter()
            .map(|system_contract| {
                let value = json_document_to_value(system_contract.path_to_contract())?;
                let mut data_contract = factory.create(owner_id, value, None)?;
                Ok((system_contract, data_contract))
            })
            .collect()
    }
}
