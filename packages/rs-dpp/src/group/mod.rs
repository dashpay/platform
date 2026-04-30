use crate::data_contract::group::{Group, GroupMemberPower};
use crate::data_contract::GroupContractPosition;
use bincode::{Decode, Encode};
use derive_more::Display;
use platform_value::Identifier;
#[cfg(feature = "serde-conversion")]
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
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[display("ID: {}, Action ID: {}", "id", "action_id")]
pub struct GroupStateTransitionInfo {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "$groupContractPosition"))]
    pub group_contract_position: GroupContractPosition,
    #[cfg_attr(feature = "serde-conversion", serde(rename = "$groupActionId"))]
    pub action_id: Identifier,
    /// This is true if we are the proposer, otherwise we are just voting on a previous action.
    #[cfg_attr(feature = "serde-conversion", serde(rename = "$groupActionIsProposer"))]
    pub action_is_proposer: bool,
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for GroupStateTransitionInfo {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for GroupStateTransitionInfo {}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupStateTransitionResolvedInfo {
    pub group_contract_position: GroupContractPosition,
    pub group: Group,
    pub action_id: Identifier,
    /// This is true if we are the proposer, otherwise we are just voting on a previous action.
    pub action_is_proposer: bool,
    pub signer_power: GroupMemberPower,
}

#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests_groupstatetransitioninfo {
    use super::*;

    fn fixture() -> GroupStateTransitionInfo {
        GroupStateTransitionInfo {
            group_contract_position: 5,
            action_id: Identifier::new([0x33; 32]),
            action_is_proposer: true,
        }
    }

    fn assert_fields(g: &GroupStateTransitionInfo) {
        assert_eq!(g.group_contract_position, 5, "group_contract_position");
        assert_eq!(g.action_id, Identifier::new([0x33; 32]), "action_id");
        assert!(g.action_is_proposer, "action_is_proposer");
    }

    #[test]
    fn json_round_trip_with_per_property_assertions() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let recovered = GroupStateTransitionInfo::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
        assert_fields(&recovered);
    }

    #[test]
    fn value_round_trip_with_per_property_assertions() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let recovered = GroupStateTransitionInfo::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
        assert_fields(&recovered);
    }
}
