mod error;

use serde_json::Value;

use crate::error::Error;

#[cfg(feature = "dashpay")]
pub use dashpay_contract;

#[cfg(feature = "dpns")]
pub use dpns_contract;

#[cfg(feature = "feature-flags")]
pub use feature_flags_contract;

#[cfg(feature = "keyword-search")]
pub use keyword_search_contract;

#[cfg(feature = "masternode-rewards")]
pub use masternode_reward_shares_contract;

use platform_value::Identifier;
use platform_version::version::PlatformVersion;

#[cfg(feature = "token-history")]
pub use token_history_contract;

#[cfg(feature = "wallet-utils")]
pub use wallet_utils_contract;

#[cfg(feature = "withdrawals")]
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
            #[cfg(feature = "withdrawals")]
            SystemDataContract::Withdrawals => withdrawals_contract::ID_BYTES,
            #[cfg(not(feature = "withdrawals"))]
            SystemDataContract::Withdrawals => [
                54, 98, 187, 97, 225, 127, 174, 62, 162, 148, 207, 96, 49, 151, 251, 10, 171, 109,
                81, 24, 11, 216, 182, 16, 76, 73, 68, 166, 47, 226, 217, 127,
            ],

            #[cfg(feature = "masternode-rewards")]
            SystemDataContract::MasternodeRewards => masternode_reward_shares_contract::ID_BYTES,
            #[cfg(not(feature = "masternode-rewards"))]
            SystemDataContract::MasternodeRewards => [
                12, 172, 226, 5, 36, 102, 147, 167, 200, 21, 101, 35, 98, 13, 170, 147, 125, 47,
                34, 71, 147, 68, 99, 238, 176, 31, 247, 33, 149, 144, 149, 140,
            ],

            #[cfg(feature = "feature-flags")]
            SystemDataContract::FeatureFlags => feature_flags_contract::ID_BYTES,
            #[cfg(not(feature = "feature-flags"))]
            SystemDataContract::FeatureFlags => [
                245, 172, 216, 200, 193, 110, 185, 172, 40, 110, 7, 132, 190, 86, 127, 80, 9, 244,
                86, 26, 243, 212, 255, 2, 91, 7, 90, 243, 68, 55, 152, 34,
            ],

            #[cfg(feature = "dpns")]
            SystemDataContract::DPNS => dpns_contract::ID_BYTES,
            #[cfg(not(feature = "dpns"))]
            SystemDataContract::DPNS => [
                230, 104, 198, 89, 175, 102, 174, 225, 231, 44, 24, 109, 222, 123, 91, 126, 10, 29,
                113, 42, 9, 196, 13, 87, 33, 246, 34, 191, 83, 197, 49, 85,
            ],

            #[cfg(feature = "dashpay")]
            SystemDataContract::Dashpay => dashpay_contract::ID_BYTES,
            #[cfg(not(feature = "dashpay"))]
            SystemDataContract::Dashpay => [
                162, 161, 180, 172, 111, 239, 34, 234, 42, 26, 104, 232, 18, 54, 68, 179, 87, 135,
                95, 107, 65, 44, 24, 16, 146, 129, 193, 70, 231, 178, 113, 188,
            ],

            #[cfg(feature = "wallet-utils")]
            SystemDataContract::WalletUtils => wallet_utils_contract::ID_BYTES,
            #[cfg(not(feature = "wallet-utils"))]
            SystemDataContract::WalletUtils => [
                92, 20, 14, 101, 92, 2, 101, 187, 194, 168, 8, 113, 109, 225, 132, 121, 133, 19,
                89, 24, 173, 81, 205, 253, 11, 118, 102, 75, 169, 91, 163, 124,
            ],

            #[cfg(feature = "token-history")]
            SystemDataContract::TokenHistory => token_history_contract::ID_BYTES,
            #[cfg(not(feature = "token-history"))]
            SystemDataContract::TokenHistory => [
                45, 67, 89, 21, 34, 216, 145, 78, 156, 243, 17, 58, 202, 190, 13, 92, 61, 40, 122,
                201, 84, 99, 187, 110, 233, 128, 63, 48, 172, 29, 210, 108,
            ],

            #[cfg(feature = "keyword-search")]
            SystemDataContract::KeywordSearch => keyword_search_contract::ID_BYTES,
            #[cfg(not(feature = "keyword-search"))]
            SystemDataContract::KeywordSearch => [
                92, 20, 14, 101, 92, 2, 101, 187, 194, 168, 8, 113, 109, 225, 132, 121, 133, 19,
                89, 24, 173, 81, 205, 253, 11, 118, 102, 75, 169, 91, 163, 124,
            ],
        };
        Identifier::new(bytes)
    }
    /// Returns [DataContractSource]
    pub fn source(self, platform_version: &PlatformVersion) -> Result<DataContractSource, Error> {
        match self {
            #[cfg(feature = "withdrawals")]
            SystemDataContract::Withdrawals => Ok(DataContractSource {
                id_bytes: withdrawals_contract::ID_BYTES,
                owner_id_bytes: withdrawals_contract::OWNER_ID_BYTES,
                version: platform_version.system_data_contracts.withdrawals as u32,
                definitions: withdrawals_contract::load_definitions(platform_version)?,
                document_schemas: withdrawals_contract::load_documents_schemas(platform_version)?,
            }),
            #[cfg(not(feature = "withdrawals"))]
            SystemDataContract::Withdrawals => Err(Error::ContractNotIncluded("withdrawals")),

            #[cfg(feature = "masternode-rewards")]
            SystemDataContract::MasternodeRewards => Ok(DataContractSource {
                id_bytes: masternode_reward_shares_contract::ID_BYTES,
                owner_id_bytes: masternode_reward_shares_contract::OWNER_ID_BYTES,
                version: platform_version
                    .system_data_contracts
                    .masternode_reward_shares as u32,
                definitions: masternode_reward_shares_contract::load_definitions(platform_version)?,
                document_schemas: masternode_reward_shares_contract::load_documents_schemas(
                    platform_version,
                )?,
            }),
            #[cfg(not(feature = "masternode-rewards"))]
            SystemDataContract::MasternodeRewards => {
                Err(Error::ContractNotIncluded("masternode-rewards"))
            }

            #[cfg(feature = "feature-flags")]
            SystemDataContract::FeatureFlags => Ok(DataContractSource {
                id_bytes: feature_flags_contract::ID_BYTES,
                owner_id_bytes: feature_flags_contract::OWNER_ID_BYTES,
                version: platform_version.system_data_contracts.feature_flags as u32,
                definitions: feature_flags_contract::load_definitions(platform_version)?,
                document_schemas: feature_flags_contract::load_documents_schemas(platform_version)?,
            }),
            #[cfg(not(feature = "feature-flags"))]
            SystemDataContract::FeatureFlags => Err(Error::ContractNotIncluded("feature-flags")),

            #[cfg(feature = "dpns")]
            SystemDataContract::DPNS => Ok(DataContractSource {
                id_bytes: dpns_contract::ID_BYTES,
                owner_id_bytes: dpns_contract::OWNER_ID_BYTES,
                version: platform_version.system_data_contracts.dpns as u32,
                definitions: dpns_contract::load_definitions(platform_version)?,
                document_schemas: dpns_contract::load_documents_schemas(platform_version)?,
            }),
            #[cfg(not(feature = "dpns"))]
            SystemDataContract::DPNS => Err(Error::ContractNotIncluded("dpns")),

            #[cfg(feature = "dashpay")]
            SystemDataContract::Dashpay => Ok(DataContractSource {
                id_bytes: dashpay_contract::ID_BYTES,
                owner_id_bytes: dashpay_contract::OWNER_ID_BYTES,
                version: platform_version.system_data_contracts.dashpay as u32,
                definitions: dashpay_contract::load_definitions(platform_version)?,
                document_schemas: dashpay_contract::load_documents_schemas(platform_version)?,
            }),
            #[cfg(not(feature = "dashpay"))]
            SystemDataContract::Dashpay => Err(Error::ContractNotIncluded("dashpay")),

            #[cfg(feature = "wallet-utils")]
            SystemDataContract::WalletUtils => Ok(DataContractSource {
                id_bytes: wallet_utils_contract::ID_BYTES,
                owner_id_bytes: wallet_utils_contract::OWNER_ID_BYTES,
                version: platform_version.system_data_contracts.wallet as u32,
                definitions: wallet_utils_contract::load_definitions(platform_version)?,
                document_schemas: wallet_utils_contract::load_documents_schemas(platform_version)?,
            }),
            #[cfg(not(feature = "wallet-utils"))]
            SystemDataContract::WalletUtils => Err(Error::ContractNotIncluded("wallet-utils")),

            #[cfg(feature = "token-history")]
            SystemDataContract::TokenHistory => Ok(DataContractSource {
                id_bytes: token_history_contract::ID_BYTES,
                owner_id_bytes: token_history_contract::OWNER_ID_BYTES,
                version: platform_version.system_data_contracts.token_history as u32,
                definitions: token_history_contract::load_definitions(platform_version)?,
                document_schemas: token_history_contract::load_documents_schemas(platform_version)?,
            }),
            #[cfg(not(feature = "token-history"))]
            SystemDataContract::TokenHistory => Err(Error::ContractNotIncluded("token-history")),

            #[cfg(feature = "keyword-search")]
            SystemDataContract::KeywordSearch => Ok(DataContractSource {
                id_bytes: keyword_search_contract::ID_BYTES,
                owner_id_bytes: keyword_search_contract::OWNER_ID_BYTES,
                version: platform_version.system_data_contracts.keyword_search as u32,
                definitions: keyword_search_contract::load_definitions(platform_version)?,
                document_schemas: keyword_search_contract::load_documents_schemas(
                    platform_version,
                )?,
            }),
            #[cfg(not(feature = "keyword-search"))]
            SystemDataContract::KeywordSearch => Err(Error::ContractNotIncluded("keyword-search")),
        }
    }
}
