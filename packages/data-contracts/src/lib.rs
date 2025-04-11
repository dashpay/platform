mod error;

use serde_json::Value;

use crate::error::Error;
pub use dashpay_contract;
pub use dpns_contract;
pub use feature_flags_contract;
pub use keyword_search_contract;
pub use masternode_reward_shares_contract;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
pub use token_history_contract;
pub use wallet_utils_contract;
pub use withdrawals_contract;

#[repr(u8)]
#[derive(PartialEq, Eq, Clone, Copy, Debug, Ord, PartialOrd, Hash)]
pub enum SystemDataContract {
    Withdrawals = 0,
    MasternodeRewards = 1,
    FeatureFlags = 2,
    DPNS = 3,
    Dashpay = 4,
    WalletUtils = 5,
    TokenHistory = 6,
    KeywordSearch = 7,
}

pub struct DataContractSource {
    pub id_bytes: [u8; 32],
    pub owner_id_bytes: [u8; 32],
    pub version: u32,
    pub definitions: Option<Value>,
    pub document_schemas: Value,
}

impl SystemDataContract {
    pub fn id(&self) -> Identifier {
        let bytes = match self {
            SystemDataContract::Withdrawals => withdrawals_contract::ID_BYTES,
            SystemDataContract::MasternodeRewards => masternode_reward_shares_contract::ID_BYTES,
            SystemDataContract::FeatureFlags => feature_flags_contract::ID_BYTES,
            SystemDataContract::DPNS => dpns_contract::ID_BYTES,
            SystemDataContract::Dashpay => dashpay_contract::ID_BYTES,
            SystemDataContract::WalletUtils => wallet_utils_contract::ID_BYTES,
            SystemDataContract::TokenHistory => token_history_contract::ID_BYTES,
            SystemDataContract::KeywordSearch => keyword_search_contract::ID_BYTES,
        };
        Identifier::new(bytes)
    }
    /// Returns [DataContractSource]
    pub fn source(self, platform_version: &PlatformVersion) -> Result<DataContractSource, Error> {
        let data = match self {
            SystemDataContract::Withdrawals => DataContractSource {
                id_bytes: withdrawals_contract::ID_BYTES,
                owner_id_bytes: withdrawals_contract::OWNER_ID_BYTES,
                version: platform_version.system_data_contracts.withdrawals as u32,
                definitions: withdrawals_contract::load_definitions(platform_version)?,
                document_schemas: withdrawals_contract::load_documents_schemas(platform_version)?,
            },
            SystemDataContract::MasternodeRewards => DataContractSource {
                id_bytes: masternode_reward_shares_contract::ID_BYTES,
                owner_id_bytes: masternode_reward_shares_contract::OWNER_ID_BYTES,
                version: platform_version
                    .system_data_contracts
                    .masternode_reward_shares as u32,
                definitions: withdrawals_contract::load_definitions(platform_version)?,
                document_schemas: masternode_reward_shares_contract::load_documents_schemas(
                    platform_version,
                )?,
            },
            SystemDataContract::FeatureFlags => DataContractSource {
                id_bytes: feature_flags_contract::ID_BYTES,
                owner_id_bytes: feature_flags_contract::OWNER_ID_BYTES,
                version: platform_version.system_data_contracts.feature_flags as u32,
                definitions: feature_flags_contract::load_definitions(platform_version)?,
                document_schemas: feature_flags_contract::load_documents_schemas(platform_version)?,
            },
            SystemDataContract::DPNS => DataContractSource {
                id_bytes: dpns_contract::ID_BYTES,
                owner_id_bytes: dpns_contract::OWNER_ID_BYTES,
                version: platform_version.system_data_contracts.dpns as u32,
                definitions: dpns_contract::load_definitions(platform_version)?,
                document_schemas: dpns_contract::load_documents_schemas(platform_version)?,
            },
            SystemDataContract::Dashpay => DataContractSource {
                id_bytes: dashpay_contract::ID_BYTES,
                owner_id_bytes: dashpay_contract::OWNER_ID_BYTES,
                version: platform_version.system_data_contracts.dashpay as u32,
                definitions: dashpay_contract::load_definitions(platform_version)?,
                document_schemas: dashpay_contract::load_documents_schemas(platform_version)?,
            },
            SystemDataContract::WalletUtils => DataContractSource {
                id_bytes: wallet_utils_contract::ID_BYTES,
                owner_id_bytes: wallet_utils_contract::OWNER_ID_BYTES,
                version: platform_version.system_data_contracts.wallet as u32,
                definitions: wallet_utils_contract::load_definitions(platform_version)?,
                document_schemas: wallet_utils_contract::load_documents_schemas(platform_version)?,
            },
            SystemDataContract::TokenHistory => DataContractSource {
                id_bytes: token_history_contract::ID_BYTES,
                owner_id_bytes: token_history_contract::OWNER_ID_BYTES,
                version: platform_version.system_data_contracts.token_history as u32,
                definitions: token_history_contract::load_definitions(platform_version)?,
                document_schemas: token_history_contract::load_documents_schemas(platform_version)?,
            },
            SystemDataContract::KeywordSearch => DataContractSource {
                id_bytes: keyword_search_contract::ID_BYTES,
                owner_id_bytes: keyword_search_contract::OWNER_ID_BYTES,
                version: platform_version.system_data_contracts.keyword_search as u32,
                definitions: keyword_search_contract::load_definitions(platform_version)?,
                document_schemas: keyword_search_contract::load_documents_schemas(
                    platform_version,
                )?,
            },
        };

        Ok(data)
    }
}
