use crate::data_contract::group::accessors::v0::GroupV0Getters;
use crate::data_contract::group::{Group, GroupMemberPower};
use crate::data_contract::GroupContractPosition;
use crate::group::action_taker::{ActionGoal, ActionTaker};
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Decode, Encode, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Default)]
// Custom `Serialize` / `Deserialize` below — `derive(Serialize, Deserialize)`
// can't produce the desired flat wire shape because the `Identity` variant
// wraps `Identifier` (serializes as a base58 string, not a map) and `Group`
// wraps a bare `u16`, so internal tagging doesn't apply. The custom impl
// emits a flat `{"$type": ..., "identity"/"position": ...}` shape with
// synthesized field names (same pattern as `ResourceVoteChoice`). Bincode
// `Encode` / `Decode` derives are untouched (consensus binary format is
// unaffected).
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
pub enum AuthorizedActionTakers {
    #[default]
    NoOne,
    ContractOwner,
    Identity(Identifier),
    MainGroup,
    Group(GroupContractPosition),
}

impl Serialize for AuthorizedActionTakers {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        match self {
            AuthorizedActionTakers::NoOne => {
                let mut m = serializer.serialize_map(Some(1))?;
                m.serialize_entry("$type", "noOne")?;
                m.end()
            }
            AuthorizedActionTakers::ContractOwner => {
                let mut m = serializer.serialize_map(Some(1))?;
                m.serialize_entry("$type", "contractOwner")?;
                m.end()
            }
            AuthorizedActionTakers::Identity(id) => {
                let mut m = serializer.serialize_map(Some(2))?;
                m.serialize_entry("$type", "identity")?;
                m.serialize_entry("identity", id)?;
                m.end()
            }
            AuthorizedActionTakers::MainGroup => {
                let mut m = serializer.serialize_map(Some(1))?;
                m.serialize_entry("$type", "mainGroup")?;
                m.end()
            }
            AuthorizedActionTakers::Group(position) => {
                let mut m = serializer.serialize_map(Some(2))?;
                m.serialize_entry("$type", "group")?;
                m.serialize_entry("position", position)?;
                m.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for AuthorizedActionTakers {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::{self, MapAccess, Visitor};

        struct V;

        impl<'de> Visitor<'de> for V {
            type Value = AuthorizedActionTakers;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                // Mention the old shape: contract JSON authored before
                // 4.0.0-beta.4 used bare strings / externally-tagged maps, and
                // this message is the only hint users get on ingest failure.
                f.write_str(
                    "AuthorizedActionTakers as a map with a `type` discriminator, \
                     e.g. {\"type\": \"contractOwner\"} or {\"type\": \"identity\", \"identity\": \"<base58>\"} \
                     (the pre-4.0.0-beta.4 shapes \"ContractOwner\" / {\"Identity\": \"<base58>\"} are no longer accepted)",
                )
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut variant: Option<String> = None;
                let mut identity: Option<Identifier> = None;
                let mut position: Option<GroupContractPosition> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "$type" => {
                            if variant.is_some() {
                                return Err(de::Error::duplicate_field("$type"));
                            }
                            variant = Some(map.next_value()?);
                        }
                        "identity" => {
                            if identity.is_some() {
                                return Err(de::Error::duplicate_field("identity"));
                            }
                            identity = Some(map.next_value()?);
                        }
                        "position" => {
                            if position.is_some() {
                                return Err(de::Error::duplicate_field("position"));
                            }
                            position = Some(map.next_value()?);
                        }
                        _ => {
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                    }
                }

                let variant = variant.ok_or_else(|| de::Error::missing_field("$type"))?;
                match variant.as_str() {
                    "noOne" => Ok(AuthorizedActionTakers::NoOne),
                    "contractOwner" => Ok(AuthorizedActionTakers::ContractOwner),
                    "identity" => {
                        let id = identity.ok_or_else(|| de::Error::missing_field("identity"))?;
                        Ok(AuthorizedActionTakers::Identity(id))
                    }
                    "mainGroup" => Ok(AuthorizedActionTakers::MainGroup),
                    "group" => {
                        let position =
                            position.ok_or_else(|| de::Error::missing_field("position"))?;
                        Ok(AuthorizedActionTakers::Group(position))
                    }
                    other => Err(de::Error::unknown_variant(
                        other,
                        &["noOne", "contractOwner", "identity", "mainGroup", "group"],
                    )),
                }
            }
        }

        deserializer.deserialize_map(V)
    }
}

// Manual impl because AuthorizedActionTakers is a flat enum (not versioned V0/V1).
#[cfg(feature = "json-conversion")]
impl JsonConvertible for AuthorizedActionTakers {}

impl fmt::Display for AuthorizedActionTakers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthorizedActionTakers::NoOne => write!(f, "NoOne"),
            AuthorizedActionTakers::ContractOwner => write!(f, "ContractOwner"),
            AuthorizedActionTakers::MainGroup => write!(f, "MainGroup"),
            AuthorizedActionTakers::Group(position) => write!(f, "Group(Position: {})", position),
            AuthorizedActionTakers::Identity(identifier) => write!(f, "Identity({})", identifier),
        }
    }
}

impl AuthorizedActionTakers {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            AuthorizedActionTakers::NoOne => vec![0],
            AuthorizedActionTakers::ContractOwner => vec![1],
            AuthorizedActionTakers::Identity(identifier) => {
                let mut bytes = vec![2];
                bytes.extend_from_slice(identifier.as_bytes());
                bytes
            }
            AuthorizedActionTakers::MainGroup => vec![3],
            AuthorizedActionTakers::Group(position) => {
                let mut bytes = vec![4];
                bytes.extend_from_slice(&position.to_be_bytes());
                bytes
            }
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let Some(&tag) = bytes.first() else {
            return Err(ProtocolError::DecodingError(
                "empty bytes for AuthorizedActionTakers".to_string(),
            ));
        };
        match tag {
            0 => Ok(AuthorizedActionTakers::NoOne),
            1 => Ok(AuthorizedActionTakers::ContractOwner),
            2 => {
                if bytes.len() != 33 {
                    return Err(ProtocolError::DecodingError(format!(
                        "expected 33 bytes for AuthorizedActionTakers::Identity, got {}",
                        bytes.len()
                    )));
                }
                let identifier = Identifier::from_bytes(&bytes[1..])
                    .map_err(|e| ProtocolError::DecodingError(e.to_string()))?;
                Ok(AuthorizedActionTakers::Identity(identifier))
            }
            3 => Ok(AuthorizedActionTakers::MainGroup),
            4 => {
                if bytes.len() != 3 {
                    return Err(ProtocolError::DecodingError(format!(
                        "expected 3 bytes for AuthorizedActionTakers::Group, got {}",
                        bytes.len()
                    )));
                }
                let position = u16::from_be_bytes([bytes[1], bytes[2]]);
                Ok(AuthorizedActionTakers::Group(position))
            }
            other => Err(ProtocolError::DecodingError(format!(
                "unknown AuthorizedActionTakers tag: {}",
                other
            ))),
        }
    }
    pub fn allowed_for_action_taker(
        &self,
        contract_owner_id: &Identifier,
        main_group: Option<GroupContractPosition>,
        groups: &BTreeMap<GroupContractPosition, Group>,
        action_taker: &ActionTaker,
        goal: ActionGoal,
    ) -> bool {
        match self {
            // No one is allowed
            AuthorizedActionTakers::NoOne => false,

            // Only the contract owner is allowed
            AuthorizedActionTakers::ContractOwner => {
                if goal == ActionGoal::ActionParticipation {
                    false
                } else {
                    match action_taker {
                        ActionTaker::SingleIdentity(action_taker) => {
                            action_taker == contract_owner_id
                        }
                        ActionTaker::SpecifiedIdentities(action_takers) => {
                            action_takers.contains(contract_owner_id)
                        }
                    }
                }
            }

            // Only an identity is allowed
            AuthorizedActionTakers::Identity(identity) => {
                if goal == ActionGoal::ActionParticipation {
                    false
                } else {
                    match action_taker {
                        ActionTaker::SingleIdentity(action_taker) => action_taker == identity,
                        ActionTaker::SpecifiedIdentities(action_takers) => {
                            action_takers.contains(identity)
                        }
                    }
                }
            }

            // MainGroup allows multiparty actions with specific power requirements
            AuthorizedActionTakers::MainGroup => {
                if let Some(main_group_contract_position) = &main_group {
                    if let Some(group) = groups.get(main_group_contract_position) {
                        match goal {
                            ActionGoal::ActionCompletion => {
                                Self::is_action_taker_authorized(group, action_taker)
                            }
                            ActionGoal::ActionParticipation => {
                                Self::is_action_taker_participant(group, action_taker)
                            }
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            }

            // Group-specific permissions with power aggregation logic
            AuthorizedActionTakers::Group(group_contract_position) => {
                if let Some(group) = groups.get(group_contract_position) {
                    match goal {
                        ActionGoal::ActionCompletion => {
                            Self::is_action_taker_authorized(group, action_taker)
                        }
                        ActionGoal::ActionParticipation => {
                            Self::is_action_taker_participant(group, action_taker)
                        }
                    }
                } else {
                    false
                }
            }
        }
    }

    /// Helper method to check if action takers meet the group's required power threshold.
    fn is_action_taker_authorized(group: &Group, action_taker: &ActionTaker) -> bool {
        match action_taker {
            ActionTaker::SingleIdentity(member_id) => {
                let power = group.members().get(member_id).cloned().unwrap_or_default();
                power >= group.required_power()
            }
            ActionTaker::SpecifiedIdentities(action_takers) => {
                // Calculate the total power of action takers who are members of the group
                let total_power: GroupMemberPower = group
                    .members()
                    .iter()
                    .filter(|(member_id, _)| action_takers.contains(*member_id))
                    .map(|(_, power)| *power)
                    .sum();

                // Compare total power to the group's required power
                total_power >= group.required_power() as GroupMemberPower
            }
        }
    }

    /// Helper method to check if action takers are participants.
    fn is_action_taker_participant(group: &Group, action_taker: &ActionTaker) -> bool {
        match action_taker {
            ActionTaker::SingleIdentity(member_id) => group.members().get(member_id).is_some(),
            ActionTaker::SpecifiedIdentities(_) => {
                // this is made only for single identities
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::group::v0::GroupV0;
    use std::collections::BTreeSet;

    fn make_id(byte: u8) -> Identifier {
        Identifier::from([byte; 32])
    }

    fn make_group(members: Vec<(Identifier, u32)>, required_power: u32) -> Group {
        Group::V0(GroupV0 {
            members: members.into_iter().collect(),
            required_power,
        })
    }

    // --- Display tests ---

    #[test]
    fn display_no_one() {
        assert_eq!(format!("{}", AuthorizedActionTakers::NoOne), "NoOne");
    }

    #[test]
    fn display_contract_owner() {
        assert_eq!(
            format!("{}", AuthorizedActionTakers::ContractOwner),
            "ContractOwner"
        );
    }

    #[test]
    fn display_main_group() {
        assert_eq!(
            format!("{}", AuthorizedActionTakers::MainGroup),
            "MainGroup"
        );
    }

    #[test]
    fn display_group_position() {
        assert_eq!(
            format!("{}", AuthorizedActionTakers::Group(42)),
            "Group(Position: 42)"
        );
    }

    #[test]
    fn display_identity() {
        let id = make_id(0xAB);
        let display = format!("{}", AuthorizedActionTakers::Identity(id));
        assert!(display.starts_with("Identity("));
    }

    // --- to_bytes / from_bytes round-trip tests ---

    #[test]
    fn round_trip_no_one() {
        let original = AuthorizedActionTakers::NoOne;
        let bytes = original.to_bytes();
        assert_eq!(bytes, vec![0]);
        let recovered = AuthorizedActionTakers::from_bytes(&bytes).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn round_trip_contract_owner() {
        let original = AuthorizedActionTakers::ContractOwner;
        let bytes = original.to_bytes();
        assert_eq!(bytes, vec![1]);
        let recovered = AuthorizedActionTakers::from_bytes(&bytes).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn round_trip_identity() {
        let id = make_id(0x42);
        let original = AuthorizedActionTakers::Identity(id);
        let bytes = original.to_bytes();
        assert_eq!(bytes.len(), 33); // 1 tag + 32 identifier
        assert_eq!(bytes[0], 2);
        let recovered = AuthorizedActionTakers::from_bytes(&bytes).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn round_trip_main_group() {
        let original = AuthorizedActionTakers::MainGroup;
        let bytes = original.to_bytes();
        assert_eq!(bytes, vec![3]);
        let recovered = AuthorizedActionTakers::from_bytes(&bytes).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn round_trip_group() {
        let original = AuthorizedActionTakers::Group(1000);
        let bytes = original.to_bytes();
        assert_eq!(bytes.len(), 3); // 1 tag + 2 for u16
        assert_eq!(bytes[0], 4);
        let recovered = AuthorizedActionTakers::from_bytes(&bytes).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn round_trip_group_max_position() {
        let original = AuthorizedActionTakers::Group(u16::MAX);
        let bytes = original.to_bytes();
        let recovered = AuthorizedActionTakers::from_bytes(&bytes).unwrap();
        assert_eq!(original, recovered);
    }

    // --- from_bytes error path tests ---

    #[test]
    fn from_bytes_empty_returns_error() {
        let result = AuthorizedActionTakers::from_bytes(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn from_bytes_unknown_tag_returns_error() {
        let result = AuthorizedActionTakers::from_bytes(&[5]);
        assert!(result.is_err());
        let result = AuthorizedActionTakers::from_bytes(&[255]);
        assert!(result.is_err());
    }

    #[test]
    fn from_bytes_identity_wrong_length_returns_error() {
        // tag 2 needs exactly 33 bytes total
        let short = vec![2; 10]; // only 10 bytes
        let result = AuthorizedActionTakers::from_bytes(&short);
        assert!(result.is_err());
    }

    #[test]
    fn from_bytes_group_wrong_length_returns_error() {
        // tag 4 needs exactly 3 bytes total
        let short = vec![4, 0]; // only 2 bytes
        let result = AuthorizedActionTakers::from_bytes(&short);
        assert!(result.is_err());

        let long = vec![4, 0, 0, 0]; // 4 bytes
        let result = AuthorizedActionTakers::from_bytes(&long);
        assert!(result.is_err());
    }

    // --- allowed_for_action_taker tests ---

    #[test]
    fn no_one_always_returns_false() {
        let aat = AuthorizedActionTakers::NoOne;
        let owner = make_id(1);
        let taker = ActionTaker::SingleIdentity(owner);
        assert!(!aat.allowed_for_action_taker(
            &owner,
            None,
            &BTreeMap::new(),
            &taker,
            ActionGoal::ActionCompletion,
        ));
    }

    #[test]
    fn contract_owner_allows_matching_single_identity() {
        let aat = AuthorizedActionTakers::ContractOwner;
        let owner = make_id(1);
        let taker = ActionTaker::SingleIdentity(owner);
        assert!(aat.allowed_for_action_taker(
            &owner,
            None,
            &BTreeMap::new(),
            &taker,
            ActionGoal::ActionCompletion,
        ));
    }

    #[test]
    fn contract_owner_rejects_non_matching_single_identity() {
        let aat = AuthorizedActionTakers::ContractOwner;
        let owner = make_id(1);
        let other = make_id(2);
        let taker = ActionTaker::SingleIdentity(other);
        assert!(!aat.allowed_for_action_taker(
            &owner,
            None,
            &BTreeMap::new(),
            &taker,
            ActionGoal::ActionCompletion,
        ));
    }

    #[test]
    fn contract_owner_rejects_action_participation() {
        let aat = AuthorizedActionTakers::ContractOwner;
        let owner = make_id(1);
        let taker = ActionTaker::SingleIdentity(owner);
        assert!(!aat.allowed_for_action_taker(
            &owner,
            None,
            &BTreeMap::new(),
            &taker,
            ActionGoal::ActionParticipation,
        ));
    }

    #[test]
    fn contract_owner_allows_specified_identities_containing_owner() {
        let aat = AuthorizedActionTakers::ContractOwner;
        let owner = make_id(1);
        let mut set = BTreeSet::new();
        set.insert(owner);
        set.insert(make_id(2));
        let taker = ActionTaker::SpecifiedIdentities(set);
        assert!(aat.allowed_for_action_taker(
            &owner,
            None,
            &BTreeMap::new(),
            &taker,
            ActionGoal::ActionCompletion,
        ));
    }

    #[test]
    fn identity_allows_matching_identity() {
        let authorized_id = make_id(5);
        let aat = AuthorizedActionTakers::Identity(authorized_id);
        let taker = ActionTaker::SingleIdentity(authorized_id);
        assert!(aat.allowed_for_action_taker(
            &make_id(1),
            None,
            &BTreeMap::new(),
            &taker,
            ActionGoal::ActionCompletion,
        ));
    }

    #[test]
    fn identity_rejects_non_matching_identity() {
        let authorized_id = make_id(5);
        let aat = AuthorizedActionTakers::Identity(authorized_id);
        let taker = ActionTaker::SingleIdentity(make_id(6));
        assert!(!aat.allowed_for_action_taker(
            &make_id(1),
            None,
            &BTreeMap::new(),
            &taker,
            ActionGoal::ActionCompletion,
        ));
    }

    #[test]
    fn identity_rejects_action_participation() {
        let authorized_id = make_id(5);
        let aat = AuthorizedActionTakers::Identity(authorized_id);
        let taker = ActionTaker::SingleIdentity(authorized_id);
        assert!(!aat.allowed_for_action_taker(
            &make_id(1),
            None,
            &BTreeMap::new(),
            &taker,
            ActionGoal::ActionParticipation,
        ));
    }

    #[test]
    fn group_allows_single_member_with_enough_power() {
        let member = make_id(10);
        let group = make_group(vec![(member, 100)], 50);
        let mut groups = BTreeMap::new();
        groups.insert(0u16, group);

        let aat = AuthorizedActionTakers::Group(0);
        let taker = ActionTaker::SingleIdentity(member);
        assert!(aat.allowed_for_action_taker(
            &make_id(1),
            None,
            &groups,
            &taker,
            ActionGoal::ActionCompletion,
        ));
    }

    #[test]
    fn group_rejects_single_member_with_insufficient_power() {
        let member = make_id(10);
        let group = make_group(vec![(member, 10)], 50);
        let mut groups = BTreeMap::new();
        groups.insert(0u16, group);

        let aat = AuthorizedActionTakers::Group(0);
        let taker = ActionTaker::SingleIdentity(member);
        assert!(!aat.allowed_for_action_taker(
            &make_id(1),
            None,
            &groups,
            &taker,
            ActionGoal::ActionCompletion,
        ));
    }

    #[test]
    fn group_allows_participation_for_member() {
        let member = make_id(10);
        let group = make_group(vec![(member, 10)], 50);
        let mut groups = BTreeMap::new();
        groups.insert(0u16, group);

        let aat = AuthorizedActionTakers::Group(0);
        let taker = ActionTaker::SingleIdentity(member);
        assert!(aat.allowed_for_action_taker(
            &make_id(1),
            None,
            &groups,
            &taker,
            ActionGoal::ActionParticipation,
        ));
    }

    #[test]
    fn group_rejects_participation_for_non_member() {
        let member = make_id(10);
        let non_member = make_id(11);
        let group = make_group(vec![(member, 10)], 50);
        let mut groups = BTreeMap::new();
        groups.insert(0u16, group);

        let aat = AuthorizedActionTakers::Group(0);
        let taker = ActionTaker::SingleIdentity(non_member);
        assert!(!aat.allowed_for_action_taker(
            &make_id(1),
            None,
            &groups,
            &taker,
            ActionGoal::ActionParticipation,
        ));
    }

    #[test]
    fn group_rejects_when_group_not_found() {
        let aat = AuthorizedActionTakers::Group(99);
        let taker = ActionTaker::SingleIdentity(make_id(10));
        assert!(!aat.allowed_for_action_taker(
            &make_id(1),
            None,
            &BTreeMap::new(),
            &taker,
            ActionGoal::ActionCompletion,
        ));
    }

    #[test]
    fn group_allows_specified_identities_with_enough_combined_power() {
        let member_a = make_id(10);
        let member_b = make_id(11);
        let group = make_group(vec![(member_a, 30), (member_b, 30)], 50);
        let mut groups = BTreeMap::new();
        groups.insert(0u16, group);

        let mut set = BTreeSet::new();
        set.insert(member_a);
        set.insert(member_b);
        let taker = ActionTaker::SpecifiedIdentities(set);

        let aat = AuthorizedActionTakers::Group(0);
        assert!(aat.allowed_for_action_taker(
            &make_id(1),
            None,
            &groups,
            &taker,
            ActionGoal::ActionCompletion,
        ));
    }

    #[test]
    fn group_rejects_specified_identities_with_insufficient_combined_power() {
        let member_a = make_id(10);
        let member_b = make_id(11);
        let group = make_group(vec![(member_a, 10), (member_b, 10)], 50);
        let mut groups = BTreeMap::new();
        groups.insert(0u16, group);

        let mut set = BTreeSet::new();
        set.insert(member_a);
        set.insert(member_b);
        let taker = ActionTaker::SpecifiedIdentities(set);

        let aat = AuthorizedActionTakers::Group(0);
        assert!(!aat.allowed_for_action_taker(
            &make_id(1),
            None,
            &groups,
            &taker,
            ActionGoal::ActionCompletion,
        ));
    }

    #[test]
    fn main_group_allows_when_main_group_exists_and_power_sufficient() {
        let member = make_id(10);
        let group = make_group(vec![(member, 100)], 50);
        let mut groups = BTreeMap::new();
        groups.insert(7u16, group);

        let aat = AuthorizedActionTakers::MainGroup;
        let taker = ActionTaker::SingleIdentity(member);
        assert!(aat.allowed_for_action_taker(
            &make_id(1),
            Some(7),
            &groups,
            &taker,
            ActionGoal::ActionCompletion,
        ));
    }

    #[test]
    fn main_group_rejects_when_no_main_group_position() {
        let aat = AuthorizedActionTakers::MainGroup;
        let taker = ActionTaker::SingleIdentity(make_id(10));
        assert!(!aat.allowed_for_action_taker(
            &make_id(1),
            None,
            &BTreeMap::new(),
            &taker,
            ActionGoal::ActionCompletion,
        ));
    }

    #[test]
    fn main_group_rejects_when_group_not_in_map() {
        let aat = AuthorizedActionTakers::MainGroup;
        let taker = ActionTaker::SingleIdentity(make_id(10));
        assert!(!aat.allowed_for_action_taker(
            &make_id(1),
            Some(99),
            &BTreeMap::new(),
            &taker,
            ActionGoal::ActionCompletion,
        ));
    }

    #[test]
    fn main_group_participation_allows_member() {
        let member = make_id(10);
        let group = make_group(vec![(member, 10)], 100);
        let mut groups = BTreeMap::new();
        groups.insert(0u16, group);

        let aat = AuthorizedActionTakers::MainGroup;
        let taker = ActionTaker::SingleIdentity(member);
        assert!(aat.allowed_for_action_taker(
            &make_id(1),
            Some(0),
            &groups,
            &taker,
            ActionGoal::ActionParticipation,
        ));
    }

    #[test]
    fn participation_rejects_specified_identities() {
        let member = make_id(10);
        let group = make_group(vec![(member, 10)], 50);
        let mut groups = BTreeMap::new();
        groups.insert(0u16, group);

        let mut set = BTreeSet::new();
        set.insert(member);
        let taker = ActionTaker::SpecifiedIdentities(set);

        let aat = AuthorizedActionTakers::Group(0);
        // is_action_taker_participant returns false for SpecifiedIdentities
        assert!(!aat.allowed_for_action_taker(
            &make_id(1),
            None,
            &groups,
            &taker,
            ActionGoal::ActionParticipation,
        ));
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
    use platform_value::platform_value;
    use serde_json::json;

    fn id() -> Identifier {
        Identifier::from([0x42u8; 32])
    }

    /// Per-variant wire-shape coverage — the custom Serialize/Deserialize pair
    /// must stay in sync when variants are added (the maintenance trap of
    /// custom-impl enums).
    #[test]
    fn json_round_trip_with_full_wire_shape_all_variants() {
        use crate::serialization::JsonConvertible;
        let cases = vec![
            (AuthorizedActionTakers::NoOne, json!({"$type": "noOne"})),
            (
                AuthorizedActionTakers::ContractOwner,
                json!({"$type": "contractOwner"}),
            ),
            (
                AuthorizedActionTakers::Identity(id()),
                json!({"$type": "identity", "identity": "5TeWSsjg2gbxCyWVniXeCmwM7UtHTCK7svzJr5xYJzHf"}),
            ),
            (
                AuthorizedActionTakers::MainGroup,
                json!({"$type": "mainGroup"}),
            ),
            // `position` is u16 (GroupContractPosition); JSON erases the size —
            // the value-path test locks the typed variant.
            (
                AuthorizedActionTakers::Group(42),
                json!({"$type": "group", "position": 42}),
            ),
        ];
        for (original, expected) in cases {
            let json_v = original.to_json().expect("to_json");
            assert_eq!(json_v, expected, "json wire shape for {original}");
            let recovered = AuthorizedActionTakers::from_json(json_v).expect("from_json");
            assert_eq!(original, recovered);
        }
    }

    #[test]
    fn value_round_trip_with_full_wire_shape_all_variants() {
        use crate::serialization::ValueConvertible;
        let cases = vec![
            (
                AuthorizedActionTakers::NoOne,
                platform_value!({"$type": "noOne"}),
            ),
            (
                AuthorizedActionTakers::ContractOwner,
                platform_value!({"$type": "contractOwner"}),
            ),
            (
                AuthorizedActionTakers::Identity(id()),
                platform_value!({"$type": "identity", "identity": id()}),
            ),
            (
                AuthorizedActionTakers::MainGroup,
                platform_value!({"$type": "mainGroup"}),
            ),
            (
                AuthorizedActionTakers::Group(42),
                platform_value!({"$type": "group", "position": 42u16}),
            ),
        ];
        for (original, expected) in cases {
            let value = original.to_object().expect("to_object");
            assert_eq!(value, expected, "value wire shape for {original}");
            let recovered = AuthorizedActionTakers::from_object(value).expect("from_object");
            assert_eq!(original, recovered);
        }
    }
}
