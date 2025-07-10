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
    pub fn id(&self) -> Result<Identifier, Error> {
        let bytes = match self {
            #[cfg(feature = "withdrawals")]
            SystemDataContract::Withdrawals => withdrawals_contract::ID_BYTES,
            #[cfg(not(feature = "withdrawals"))]
            SystemDataContract::Withdrawals => {
                return Err(Error::ContractNotIncluded("withdrawals"))
            }

            #[cfg(feature = "masternode-rewards")]
            SystemDataContract::MasternodeRewards => masternode_reward_shares_contract::ID_BYTES,
            #[cfg(not(feature = "masternode-rewards"))]
            SystemDataContract::MasternodeRewards => {
                return Err(Error::ContractNotIncluded("masternode-rewards"))
            }

            #[cfg(feature = "feature-flags")]
            SystemDataContract::FeatureFlags => feature_flags_contract::ID_BYTES,
            #[cfg(not(feature = "feature-flags"))]
            SystemDataContract::FeatureFlags => {
                return Err(Error::ContractNotIncluded("feature-flags"))
            }

            #[cfg(feature = "dpns")]
            SystemDataContract::DPNS => dpns_contract::ID_BYTES,
            #[cfg(not(feature = "dpns"))]
            SystemDataContract::DPNS => return Err(Error::ContractNotIncluded("dpns")),

            #[cfg(feature = "dashpay")]
            SystemDataContract::Dashpay => dashpay_contract::ID_BYTES,
            #[cfg(not(feature = "dashpay"))]
            SystemDataContract::Dashpay => return Err(Error::ContractNotIncluded("dashpay")),

            #[cfg(feature = "wallet-utils")]
            SystemDataContract::WalletUtils => wallet_utils_contract::ID_BYTES,
            #[cfg(not(feature = "wallet-utils"))]
            SystemDataContract::WalletUtils => {
                return Err(Error::ContractNotIncluded("wallet-utils"))
            }

            #[cfg(feature = "token-history")]
            SystemDataContract::TokenHistory => token_history_contract::ID_BYTES,
            #[cfg(not(feature = "token-history"))]
            SystemDataContract::TokenHistory => {
                return Err(Error::ContractNotIncluded("token-history"))
            }

            #[cfg(feature = "keyword-search")]
            SystemDataContract::KeywordSearch => keyword_search_contract::ID_BYTES,
            #[cfg(not(feature = "keyword-search"))]
            SystemDataContract::KeywordSearch => {
                return Err(Error::ContractNotIncluded("keyword-search"))
            }
        };
        Ok(Identifier::new(bytes))
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
