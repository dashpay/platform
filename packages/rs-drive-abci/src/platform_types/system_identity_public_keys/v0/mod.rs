use crate::abci::config::Keys;
use crate::platform_types::required_identity_public_key_set;
use serde::{Deserialize, Serialize};

/// System identity public keys
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SystemIdentityPublicKeys {
    /// Required public key set for masternode reward shares contract owner identity
    pub masternode_reward_shares_contract_owner:
        required_identity_public_key_set::v0::RequiredIdentityPublicKeysSet,
    /// Required public key set for feature flags contract owner identity
    pub feature_flags_contract_owner:
        required_identity_public_key_set::v0::RequiredIdentityPublicKeysSet,
    /// Required public key set for dpns contract owner identity
    pub dpns_contract_owner: required_identity_public_key_set::v0::RequiredIdentityPublicKeysSet,
    /// Required public key set for withdrawals contract owner identity
    pub withdrawals_contract_owner:
        required_identity_public_key_set::v0::RequiredIdentityPublicKeysSet,
    /// Required public key set for dashpay contract owner identity
    pub dashpay_contract_owner: required_identity_public_key_set::v0::RequiredIdentityPublicKeysSet,
}

impl From<Keys> for SystemIdentityPublicKeys {
    fn from(keys: Keys) -> Self {
        Self {
            masternode_reward_shares_contract_owner:
                required_identity_public_key_set::v0::RequiredIdentityPublicKeysSet {
                    master: keys.masternode_reward_shares_master_public_key,
                    high: keys.masternode_reward_shares_second_public_key,
                },
            feature_flags_contract_owner:
                required_identity_public_key_set::v0::RequiredIdentityPublicKeysSet {
                    master: keys.feature_flags_master_public_key,
                    high: keys.feature_flags_second_public_key,
                },
            dpns_contract_owner:
                required_identity_public_key_set::v0::RequiredIdentityPublicKeysSet {
                    master: keys.dpns_master_public_key,
                    high: keys.dpns_second_public_key,
                },
            withdrawals_contract_owner:
                required_identity_public_key_set::v0::RequiredIdentityPublicKeysSet {
                    master: keys.withdrawals_master_public_key,
                    high: keys.withdrawals_second_public_key,
                },
            dashpay_contract_owner:
                required_identity_public_key_set::v0::RequiredIdentityPublicKeysSet {
                    master: keys.dashpay_master_public_key,
                    high: keys.dashpay_second_public_key,
                },
        }
    }
}
