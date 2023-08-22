use crate::platform_types::required_identity_public_key_set::v0::RequiredIdentityPublicKeysSet;
use crate::platform_types::system_identity_public_keys::v0::{
    SystemIdentityPublicKeysV0, SystemIdentityPublicKeysV0Getters,
    SystemIdentityPublicKeysV0Setters,
};
use derive_more::From;
use serde::{Deserialize, Serialize};

/// Version 0
pub mod v0;

/// System identity public keys
#[derive(Serialize, Deserialize, Clone, Debug, From)]
pub enum SystemIdentityPublicKeys {
    /// Version 0
    V0(SystemIdentityPublicKeysV0),
}

impl SystemIdentityPublicKeysV0Getters for SystemIdentityPublicKeys {
    fn masternode_reward_shares_contract_owner(&self) -> &RequiredIdentityPublicKeysSet {
        match self {
            SystemIdentityPublicKeys::V0(v0) => &v0.masternode_reward_shares_contract_owner,
        }
    }

    fn feature_flags_contract_owner(&self) -> &RequiredIdentityPublicKeysSet {
        match self {
            SystemIdentityPublicKeys::V0(v0) => &v0.feature_flags_contract_owner,
        }
    }

    fn dpns_contract_owner(&self) -> &RequiredIdentityPublicKeysSet {
        match self {
            SystemIdentityPublicKeys::V0(v0) => &v0.dpns_contract_owner,
        }
    }

    fn withdrawals_contract_owner(&self) -> &RequiredIdentityPublicKeysSet {
        match self {
            SystemIdentityPublicKeys::V0(v0) => &v0.withdrawals_contract_owner,
        }
    }

    fn dashpay_contract_owner(&self) -> &RequiredIdentityPublicKeysSet {
        match self {
            SystemIdentityPublicKeys::V0(v0) => &v0.dashpay_contract_owner,
        }
    }
}

impl SystemIdentityPublicKeysV0Setters for SystemIdentityPublicKeys {
    fn set_masternode_reward_shares_contract_owner(&mut self, keys: RequiredIdentityPublicKeysSet) {
        match self {
            SystemIdentityPublicKeys::V0(v0) => {
                v0.masternode_reward_shares_contract_owner = keys;
            }
        }
    }

    fn set_feature_flags_contract_owner(&mut self, keys: RequiredIdentityPublicKeysSet) {
        match self {
            SystemIdentityPublicKeys::V0(v0) => {
                v0.feature_flags_contract_owner = keys;
            }
        }
    }

    fn set_dpns_contract_owner(&mut self, keys: RequiredIdentityPublicKeysSet) {
        match self {
            SystemIdentityPublicKeys::V0(v0) => {
                v0.dpns_contract_owner = keys;
            }
        }
    }

    fn set_withdrawals_contract_owner(&mut self, keys: RequiredIdentityPublicKeysSet) {
        match self {
            SystemIdentityPublicKeys::V0(v0) => {
                v0.withdrawals_contract_owner = keys;
            }
        }
    }

    fn set_dashpay_contract_owner(&mut self, keys: RequiredIdentityPublicKeysSet) {
        match self {
            SystemIdentityPublicKeys::V0(v0) => {
                v0.dashpay_contract_owner = keys;
            }
        }
    }
}
