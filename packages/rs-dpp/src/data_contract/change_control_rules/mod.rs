pub mod authorized_action_takers;
pub mod v0;

use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use crate::data_contract::change_control_rules::v0::ChangeControlRulesV0;
use crate::data_contract::group::Group;
use crate::data_contract::GroupContractPosition;
use crate::group::action_taker::{ActionGoal, ActionTaker};
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

#[cfg_attr(feature = "json-conversion", derive(JsonConvertible))]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, From)]
#[serde(tag = "$formatVersion")]
pub enum ChangeControlRules {
    #[serde(rename = "0")]
    V0(ChangeControlRulesV0),
}

impl ChangeControlRules {
    pub fn admin_action_takers(&self) -> &AuthorizedActionTakers {
        match self {
            ChangeControlRules::V0(v0) => &v0.admin_action_takers,
        }
    }
    pub fn authorized_to_make_change_action_takers(&self) -> &AuthorizedActionTakers {
        match self {
            ChangeControlRules::V0(v0) => &v0.authorized_to_make_change,
        }
    }

    pub fn set_admin_action_takers(&mut self, admin_action_takers: AuthorizedActionTakers) {
        match self {
            ChangeControlRules::V0(v0) => {
                v0.admin_action_takers = admin_action_takers;
            }
        }
    }

    pub fn set_authorized_to_make_change_action_takers(
        &mut self,
        authorized_to_make_change: AuthorizedActionTakers,
    ) {
        match self {
            ChangeControlRules::V0(v0) => {
                v0.authorized_to_make_change = authorized_to_make_change;
            }
        }
    }

    pub fn can_make_change(
        &self,
        contract_owner_id: &Identifier,
        main_group: Option<GroupContractPosition>,
        groups: &BTreeMap<GroupContractPosition, Group>,
        action_taker: &ActionTaker,
        goal: ActionGoal,
    ) -> bool {
        match self {
            ChangeControlRules::V0(v0) => {
                v0.can_make_change(contract_owner_id, main_group, groups, action_taker, goal)
            }
        }
    }

    pub fn can_change_authorized_action_takers(
        &self,
        controlling_action_takers: &AuthorizedActionTakers,
        contract_owner_id: &Identifier,
        main_group: Option<GroupContractPosition>,
        groups: &BTreeMap<GroupContractPosition, Group>,
        action_taker: &ActionTaker,
        goal: ActionGoal,
    ) -> bool {
        match self {
            ChangeControlRules::V0(v0) => v0.can_change_authorized_action_takers(
                controlling_action_takers,
                contract_owner_id,
                main_group,
                groups,
                action_taker,
                goal,
            ),
        }
    }

    pub fn can_change_admin_action_takers(
        &self,
        admin_action_takers: &AuthorizedActionTakers,
        contract_owner_id: &Identifier,
        main_group: Option<GroupContractPosition>,
        groups: &BTreeMap<GroupContractPosition, Group>,
        action_taker: &ActionTaker,
        goal: ActionGoal,
    ) -> bool {
        match self {
            ChangeControlRules::V0(v0) => v0.can_change_admin_action_takers(
                admin_action_takers,
                contract_owner_id,
                main_group,
                groups,
                action_taker,
                goal,
            ),
        }
    }
    pub fn can_change_to(
        &self,
        other: &ChangeControlRules,
        contract_owner_id: &Identifier,
        main_group: Option<GroupContractPosition>,
        groups: &BTreeMap<GroupContractPosition, Group>,
        action_taker: &ActionTaker,
        goal: ActionGoal,
    ) -> bool {
        match (self, other) {
            (ChangeControlRules::V0(v0), ChangeControlRules::V0(v0_other)) => v0.can_change_to(
                v0_other,
                contract_owner_id,
                main_group,
                groups,
                action_taker,
                goal,
            ),
        }
    }
}

impl fmt::Display for ChangeControlRules {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChangeControlRules::V0(v0) => {
                write!(f, "{}", v0) //just pass through
            }
        }
    }
}

#[cfg(all(test, feature = "json-conversion"))]
mod tests {
    use super::*;
    use crate::serialization::JsonConvertible;

    #[test]
    fn change_control_rules_json_round_trip() {
        let rules = ChangeControlRules::V0(ChangeControlRulesV0 {
            authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
            admin_action_takers: AuthorizedActionTakers::NoOne,
            changing_authorized_action_takers_to_no_one_allowed: true,
            changing_admin_action_takers_to_no_one_allowed: false,
            self_changing_admin_action_takers_allowed: true,
        });

        let json = rules.to_json().expect("to_json should succeed");

        // Verify boolean fields
        assert!(json["changingAuthorizedActionTakersToNoOneAllowed"]
            .as_bool()
            .unwrap());
        assert!(!json["changingAdminActionTakersToNoOneAllowed"]
            .as_bool()
            .unwrap());
        assert!(json["selfChangingAdminActionTakersAllowed"]
            .as_bool()
            .unwrap());

        // round-trip
        let restored = ChangeControlRules::from_json(json).expect("from_json should succeed");
        assert_eq!(rules, restored);
    }

    #[test]
    fn change_control_rules_with_group_json_round_trip() {
        let rules = ChangeControlRules::V0(ChangeControlRulesV0 {
            authorized_to_make_change: AuthorizedActionTakers::Group(3),
            admin_action_takers: AuthorizedActionTakers::Identity(Identifier::from([0xFFu8; 32])),
            changing_authorized_action_takers_to_no_one_allowed: false,
            changing_admin_action_takers_to_no_one_allowed: false,
            self_changing_admin_action_takers_allowed: false,
        });

        let json = rules.to_json().expect("to_json should succeed");
        let restored = ChangeControlRules::from_json(json).expect("from_json should succeed");
        assert_eq!(rules, restored);
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
    use crate::data_contract::change_control_rules::v0::ChangeControlRulesV0;
    use platform_value::platform_value;
    use serde_json::json;

    /// Non-default values per field so the wire-shape assertion catches any
    /// silent zero-out / flip on round-trip.
    fn fixture() -> ChangeControlRules {
        ChangeControlRules::V0(ChangeControlRulesV0 {
            authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
            admin_action_takers: AuthorizedActionTakers::MainGroup,
            changing_authorized_action_takers_to_no_one_allowed: true,
            changing_admin_action_takers_to_no_one_allowed: false,
            self_changing_admin_action_takers_allowed: true,
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // `AuthorizedActionTakers` uses a custom internally-tagged serde impl
        // (`{"$type": ...}` maps — see authorized_action_takers.rs); unit and
        // payload variants share the same flat map shape.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "authorizedToMakeChange": {"$type": "contractOwner"},
                "adminActionTakers": {"$type": "mainGroup"},
                "changingAuthorizedActionTakersToNoOneAllowed": true,
                "changingAdminActionTakersToNoOneAllowed": false,
                "selfChangingAdminActionTakersAllowed": true,
            })
        );
        let recovered = ChangeControlRules::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // No sized integers in this fixture — only Text + Bool. The custom
        // `AuthorizedActionTakers` impl emits `{"$type": ...}` maps on both
        // wire formats.
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "authorizedToMakeChange": {"$type": "contractOwner"},
                "adminActionTakers": {"$type": "mainGroup"},
                "changingAuthorizedActionTakersToNoOneAllowed": true,
                "changingAdminActionTakersToNoOneAllowed": false,
                "selfChangingAdminActionTakersAllowed": true,
            })
        );
        let recovered = ChangeControlRules::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
