mod error;

use serde_json::Value;

use crate::error::Error;

#[cfg(feature = "app-connect")]
pub use app_connect_contract;

#[cfg(feature = "dashpay")]
pub use dashpay_contract;

#[cfg(feature = "dpns")]
pub use dpns_contract;

#[cfg(feature = "keyword-search")]
pub use keyword_search_contract;

#[cfg(feature = "masternode-rewards")]
pub use masternode_reward_shares_contract;

#[cfg(feature = "moderation-charters")]
pub use moderation_charters_contract;

use platform_value::Identifier;
use platform_version::version::PlatformVersion;

#[cfg(feature = "document-history")]
pub use document_history_contract;

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
    /// Reserved slot — the feature-flags contract was never deployed at genesis
    /// and its implementation has been removed. The discriminant `2` is kept to
    /// preserve the stable numbering of subsequent variants.
    FeatureFlags = 2,
    DPNS = 3,
    Dashpay = 4,
    WalletUtils = 5,
    TokenHistory = 6,
    KeywordSearch = 7,
    DocumentHistory = 8,
    AppConnect = 9,
    /// The charters of elected moderation teams (protocol version 14). Registered from
    /// protocol version 14 on: registered at genesis by chains born at 14 and inserted by the
    /// upgrade to 14.
    ModerationCharters = 10,
}

pub struct DataContractSource {
    pub id_bytes: [u8; 32],
    pub owner_id_bytes: [u8; 32],
    pub version: u32,
    pub definitions: Option<Value>,
    pub document_schemas: Value,
}

impl SystemDataContract {
    /// Every system data contract, including the reserved `FeatureFlags` slot.
    ///
    /// Deliberately kept beside the enum so that adding a variant and adding it here are the
    /// same edit. `assert_every_variant_is_listed` below makes that mechanical rather than
    /// remembered: a new variant makes its match non-exhaustive and the crate stops compiling.
    pub const ALL: [SystemDataContract; 11] = [
        SystemDataContract::Withdrawals,
        SystemDataContract::MasternodeRewards,
        SystemDataContract::FeatureFlags,
        SystemDataContract::DPNS,
        SystemDataContract::Dashpay,
        SystemDataContract::WalletUtils,
        SystemDataContract::TokenHistory,
        SystemDataContract::KeywordSearch,
        SystemDataContract::DocumentHistory,
        SystemDataContract::AppConnect,
        SystemDataContract::ModerationCharters,
    ];

    /// A new variant must also be added to [`SystemDataContract::ALL`]; this match is where the
    /// compiler stops you, but it cannot check that list for you.
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

            // Reserved: feature-flags contract was removed but the ID is kept for
            // discriminant stability and to prevent reuse of a potentially meaningful
            // Identifier on-chain.
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
                161, 147, 167, 153, 40, 225, 219, 101, 50, 156, 28, 146, 150, 52, 114, 213, 56,
                154, 106, 15, 79, 66, 18, 156, 94, 146, 216, 104, 140, 93, 170, 215,
            ],

            #[cfg(feature = "document-history")]
            SystemDataContract::DocumentHistory => document_history_contract::ID_BYTES,
            #[cfg(not(feature = "document-history"))]
            SystemDataContract::DocumentHistory => [
                88, 18, 140, 208, 179, 231, 242, 57, 225, 203, 4, 210, 245, 95, 136, 92, 160, 167,
                112, 118, 173, 238, 83, 62, 234, 230, 222, 16, 231, 30, 99, 98,
            ],

            #[cfg(feature = "app-connect")]
            SystemDataContract::AppConnect => app_connect_contract::ID_BYTES,
            #[cfg(not(feature = "app-connect"))]
            SystemDataContract::AppConnect => [
                239, 150, 14, 165, 105, 114, 235, 173, 190, 248, 162, 126, 247, 218, 92, 129, 255,
                75, 179, 138, 2, 150, 151, 69, 126, 36, 218, 66, 183, 155, 84, 183,
            ],

            #[cfg(feature = "moderation-charters")]
            SystemDataContract::ModerationCharters => moderation_charters_contract::ID_BYTES,
            #[cfg(not(feature = "moderation-charters"))]
            SystemDataContract::ModerationCharters => [
                197, 6, 230, 72, 106, 198, 82, 129, 253, 135, 43, 86, 185, 182, 17, 112, 164, 127,
                96, 5, 107, 185, 156, 46, 14, 10, 109, 237, 77, 228, 248, 129,
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

            // Reserved: feature-flags contract was removed. The variant exists only
            // to preserve discriminant stability; loading it is not supported.
            SystemDataContract::FeatureFlags => Err(Error::ContractReserved("feature-flags")),

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

            #[cfg(feature = "document-history")]
            SystemDataContract::DocumentHistory => Ok(DataContractSource {
                id_bytes: document_history_contract::ID_BYTES,
                owner_id_bytes: document_history_contract::OWNER_ID_BYTES,
                version: platform_version.system_data_contracts.document_history as u32,
                definitions: document_history_contract::load_definitions(platform_version)?,
                document_schemas: document_history_contract::load_documents_schemas(
                    platform_version,
                )?,
            }),
            #[cfg(not(feature = "document-history"))]
            SystemDataContract::DocumentHistory => {
                Err(Error::ContractNotIncluded("document-history"))
            }

            #[cfg(feature = "app-connect")]
            SystemDataContract::AppConnect => Ok(DataContractSource {
                id_bytes: app_connect_contract::ID_BYTES,
                owner_id_bytes: app_connect_contract::OWNER_ID_BYTES,
                version: platform_version.system_data_contracts.app_connect as u32,
                definitions: app_connect_contract::load_definitions(platform_version)?,
                document_schemas: app_connect_contract::load_documents_schemas(platform_version)?,
            }),
            #[cfg(not(feature = "app-connect"))]
            SystemDataContract::AppConnect => Err(Error::ContractNotIncluded("app-connect")),

            #[cfg(feature = "moderation-charters")]
            SystemDataContract::ModerationCharters => Ok(DataContractSource {
                id_bytes: moderation_charters_contract::ID_BYTES,
                owner_id_bytes: moderation_charters_contract::OWNER_ID_BYTES,
                version: platform_version.system_data_contracts.moderation_charters as u32,
                definitions: moderation_charters_contract::load_definitions(platform_version)?,
                document_schemas: moderation_charters_contract::load_documents_schemas(
                    platform_version,
                )?,
            }),
            #[cfg(not(feature = "moderation-charters"))]
            SystemDataContract::ModerationCharters => {
                Err(Error::ContractNotIncluded("moderation-charters"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SystemDataContract;
    use base58::FromBase58;

    /// `id()` spells the published identifier of every system contract under both feature
    /// settings: with a contract's feature on it reads that crate's constant, without it the
    /// bytes copied above. A copy that drifts from the published id would give feature-less
    /// builds a different contract id, which is exactly what happened to `KeywordSearch`
    /// before this test existed. `FeatureFlags` has no crate and no published id; its bytes
    /// are only a reserved slot.
    #[test]
    fn every_system_contract_id_matches_the_published_id() {
        let published = |base58: &str| base58.from_base58().expect("the published id is base58");

        for contract in SystemDataContract::ALL {
            let expected = match contract {
                SystemDataContract::Withdrawals => {
                    published("4fJLR2GYTPFdomuTVvNy3VRrvWgvkKPzqehEBpNf2nk6")
                }
                SystemDataContract::MasternodeRewards => {
                    published("rUnsWrFu3PKyRMGk2mxmZVBPbQuZx2qtHeFjURoQevX")
                }
                SystemDataContract::FeatureFlags => continue,
                SystemDataContract::DPNS => {
                    published("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec")
                }
                SystemDataContract::Dashpay => {
                    published("Bwr4WHCPz5rFVAD87RqTs3izo4zpzwsEdKPWUT1NS1C7")
                }
                SystemDataContract::WalletUtils => {
                    published("7CSFGeF4WNzgDmx94zwvHkYaG3Dx4XEe5LFsFgJswLbm")
                }
                SystemDataContract::TokenHistory => {
                    published("43gujrzZgXqcKBiScLa4T8XTDnRhenR9BLx8GWVHjPxF")
                }
                SystemDataContract::KeywordSearch => {
                    published("BsjE6tQxG47wffZCRQCovFx5rYrAYYC3rTVRWKro27LA")
                }
                SystemDataContract::DocumentHistory => {
                    published("6voHRaoiPcfmMhbqCA9dixH98xcgPQ9UEcuaXjpVu3LD")
                }
                SystemDataContract::AppConnect => {
                    published("H8F9mP1BM55TE1ShsxPZHzhyinaMdY9bMmP85mkDhcJJ")
                }
                SystemDataContract::ModerationCharters => {
                    published("EG7RGfV8fDTayC2FyVr8HwdpJh3fXDbVztcfE94UmN88")
                }
            };

            assert_eq!(
                contract.id().to_buffer().to_vec(),
                expected,
                "{contract:?} id does not match its published id"
            );
        }
    }
}
