use crate::data_contract::GroupContractPosition;
use bincode::{Decode, Encode};
use derive_more::Display;
use platform_value::Identifier;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

pub mod action_event;
pub mod group_action;

#[derive(Debug, Clone, Copy, Encode, Decode, Default, PartialEq, Display)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[display("ID: {}, Action ID: {}", "id", "action_id")]
pub struct GroupStateTransitionInfo {
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "$groupContractPosition")
    )]
    pub group_contract_position: GroupContractPosition,
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "$groupActionId")
    )]
    pub action_id: Identifier,
}
