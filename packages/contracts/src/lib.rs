use dpp::data_contract::extra::common::json_document_to_value;
use dpp::data_contract::validation::data_contract_validator::DataContractValidator;
use dpp::data_contract::{DataContract, DataContractFactory};
use dpp::identifier::Identifier;
use dpp::version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION};
use dpp::ProtocolError;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::Arc;

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
    pub fn path_to_contract(&self) -> &str {
        match self {
            SystemContract::Withdrawals => {
                "/withdrawals-contract/schema/withdrawals-documents.json"
            }
            SystemContract::MasternodeRewards => {
                "/masternode-reward-shares-contract/schema/masternode-reward-shares-documents.json"
            }
            SystemContract::FeatureFlags => {
                "/feature-flags-contract/schema/feature-flags-documents.json"
            }
            SystemContract::DPNS => "/dpns-contract/schema/dpns-contract-documents.json",
            SystemContract::Dashpay => "/dashpay-contract/schema/dashpay.schema.json",
        }
    }
    pub fn load_contract(
        &self,
        system_contract: SystemContract,
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
        let value = json_document_to_value(system_contract.path_to_contract())?;
        factory.create(owner_id, value, None)
    }

    pub fn load_contracts(
        &self,
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
                let data_contract = factory.create(owner_id, value, None)?;
                Ok((system_contract, data_contract))
            })
            .collect()
    }
}
