use crate::data_contract::config::v0::DataContractConfigGettersV0;
use crate::data_contract::config::DataContractConfig;
use crate::identity::state_transition::asset_lock_proof::{Decode, Encode};
use crate::identity::SecurityLevel;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentTypeConfigV0 {
    pub keep_history: Option<bool>,
    pub mutable: Option<bool>,
    pub security_level_requirement: SecurityLevel,
}

impl Default for DocumentTypeConfigV0 {
    fn default() -> Self {
        Self {
            keep_history: None,
            mutable: None,
            security_level_requirement: SecurityLevel::HIGH,
        }
    }
}

pub trait DocumentTypeConfigAccessorsV0 {
    fn keep_history(&self) -> Option<bool>;

    fn set_keep_history(&mut self, keep_history: Option<bool>);

    fn mutable(&self) -> Option<bool>;

    fn set_mutable(&mut self, mutable: Option<bool>);

    fn security_level_requirement(&self) -> SecurityLevel;

    fn set_security_level_requirement(&mut self, security_level_requirement: SecurityLevel);
}
