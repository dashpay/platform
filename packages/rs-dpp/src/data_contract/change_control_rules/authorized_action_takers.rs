use crate::data_contract::group::accessors::v0::GroupV0Getters;
use crate::data_contract::group::{Group, GroupMemberPower};
use crate::data_contract::GroupContractPosition;
use crate::group::action_taker::{ActionGoal, ActionTaker};
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

#[derive(
    Serialize, Deserialize, Decode, Encode, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Default,
)]
pub enum AuthorizedActionTakers {
    #[default]
    NoOne,
    ContractOwner,
    Identity(Identifier),
    MainGroup,
    Group(GroupContractPosition),
}

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
