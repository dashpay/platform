use crate::data_contract::group::{Group, GroupMemberPower};
use crate::data_contract::GroupContractPosition;
use bincode::{Decode, Encode};
use derive_more::Display;
use platform_value::Identifier;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

pub mod action_event;
pub mod action_taker;
pub mod group_action;
pub mod group_action_status;

#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq)]
pub enum GroupStateTransitionInfoStatus {
    GroupStateTransitionInfoProposer(GroupContractPosition),
    GroupStateTransitionInfoOtherSigner(GroupStateTransitionInfo),
}

impl From<GroupStateTransitionInfoStatus> for GroupStateTransitionInfo {
    fn from(value: GroupStateTransitionInfoStatus) -> Self {
        match value {
            GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(
                group_contract_position,
            ) => GroupStateTransitionInfo {
                group_contract_position,
                action_id: Default::default(),
                action_is_proposer: true,
            },
            GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(info) => info,
        }
    }
}

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
    /// This is true if we are the proposer, otherwise we are just voting on a previous action.
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "$groupActionIsProposer")
    )]
    pub action_is_proposer: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupStateTransitionResolvedInfo {
    pub group_contract_position: GroupContractPosition,
    pub group: Group,
    pub action_id: Identifier,
    /// This is true if we are the proposer, otherwise we are just voting on a previous action.
    pub action_is_proposer: bool,
    pub signer_power: GroupMemberPower,
}
