use serde_json::{Error, Value};

pub use dashpay_contract;
pub use dpns_contract;
pub use feature_flags_contract;
pub use masternode_reward_shares_contract;
use platform_value::Identifier;
pub use withdrawals_contract;

#[repr(u8)]
#[derive(PartialEq, Eq, Clone, Copy, Debug, Ord, PartialOrd)]
pub enum SystemDataContract {
    Withdrawals = 0,
    MasternodeRewards = 1,
    FeatureFlags = 2,
    DPNS = 3,
    Dashpay = 4,
}

pub struct DataContractSource {
    pub id_bytes: [u8; 32],
    pub owner_id_bytes: [u8; 32],
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
        };
        Identifier::new(bytes)
    }
    /// Returns [DataContractSource]
    pub fn source(self) -> Result<DataContractSource, Error> {
        let data = match self {
            SystemDataContract::Withdrawals => DataContractSource {
                id_bytes: withdrawals_contract::ID_BYTES,
                owner_id_bytes: withdrawals_contract::OWNER_ID_BYTES,
                definitions: None,
                document_schemas: withdrawals_contract::load_documents_schemas()?,
            },
            SystemDataContract::MasternodeRewards => DataContractSource {
                id_bytes: masternode_reward_shares_contract::ID_BYTES,
                owner_id_bytes: masternode_reward_shares_contract::OWNER_ID_BYTES,
                definitions: None,
                document_schemas: masternode_reward_shares_contract::load_documents_schemas()?,
            },
            SystemDataContract::FeatureFlags => DataContractSource {
                id_bytes: feature_flags_contract::ID_BYTES,
                owner_id_bytes: feature_flags_contract::OWNER_ID_BYTES,
                definitions: None,
                document_schemas: feature_flags_contract::load_documents_schemas()?,
            },
            SystemDataContract::DPNS => DataContractSource {
                id_bytes: dpns_contract::ID_BYTES,
                owner_id_bytes: dpns_contract::OWNER_ID_BYTES,
                definitions: None,
                document_schemas: dpns_contract::load_documents_schemas()?,
            },
            SystemDataContract::Dashpay => DataContractSource {
                id_bytes: dashpay_contract::ID_BYTES,
                owner_id_bytes: dashpay_contract::OWNER_ID_BYTES,
                definitions: None,
                document_schemas: dashpay_contract::load_documents_schemas()?,
            },
        };

        Ok(data)
    }
}
