//! Contract moderation: the declaration, inside a data contract's config, that the contract
//! keeps a banlist and/or a suspension list of identities, and who may edit them.
//!
//! An identity on the banlist, or on the suspension list with a suspension that has not lapsed,
//! cannot act on the contract at the document level: every document transition it signs against
//! the contract is refused. Token transitions are not affected. The lists live under the
//! contract's own subtree in Drive (keys `3` and `4`) and are edited by the
//! `ContractUserModeration` state transition.

use crate::consensus::basic::contract_moderation::InvalidContractModerationConfigError;
use crate::identity::TimestampMillis;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonSafeFields;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

/// Who may send a `ContractUserModeration` transition for the contract.
///
/// The contract owner always may. A moderator set is fixed in the config and changed only by a
/// contract update.
#[derive(Debug, Clone, PartialEq, Eq, Default, Encode, Decode, DecodeUntrusted)]
pub enum ContractModerators {
    /// Only the contract owner moderates.
    #[default]
    ContractOwner,
    /// The contract owner and a fixed set of identities moderate. Non-empty, at most
    /// `SystemLimits::max_contract_moderators`, the owner not among them.
    OwnerAndIdentities(BTreeSet<Identifier>),
}

impl ContractModerators {
    /// The identities that moderate besides the owner. Empty for `ContractOwner`.
    pub fn identity_ids(&self) -> Option<&BTreeSet<Identifier>> {
        match self {
            ContractModerators::ContractOwner => None,
            ContractModerators::OwnerAndIdentities(ids) => Some(ids),
        }
    }

    /// Whether `identity_id` is one of the moderator identities named besides the owner.
    pub fn names(&self, identity_id: &Identifier) -> bool {
        self.identity_ids()
            .is_some_and(|ids| ids.contains(identity_id))
    }

    /// Whether `identity_id` may moderate a contract owned by `owner_id`.
    pub fn may_moderate(&self, owner_id: &Identifier, identity_id: &Identifier) -> bool {
        owner_id == identity_id || self.names(identity_id)
    }
}

// The wire shape is a flat `{"$type": "contractOwner"}` or
// `{"$type": "ownerAndIdentities", "identities": [...]}` map, the style of
// `AuthorizedActionTakers`. Bincode is untouched.
impl Serialize for ContractModerators {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        match self {
            ContractModerators::ContractOwner => {
                let mut m = serializer.serialize_map(Some(1))?;
                m.serialize_entry("$type", "contractOwner")?;
                m.end()
            }
            ContractModerators::OwnerAndIdentities(ids) => {
                let mut m = serializer.serialize_map(Some(2))?;
                m.serialize_entry("$type", "ownerAndIdentities")?;
                m.serialize_entry("identities", ids)?;
                m.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for ContractModerators {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::{self, MapAccess, Visitor};

        struct V;

        impl<'de> Visitor<'de> for V {
            type Value = ContractModerators;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(
                    "ContractModerators as a map with a `$type` discriminator, \
                     e.g. {\"$type\": \"contractOwner\"} or \
                     {\"$type\": \"ownerAndIdentities\", \"identities\": [\"<base58>\"]}",
                )
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut variant: Option<String> = None;
                let mut identities: Option<BTreeSet<Identifier>> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "$type" => {
                            if variant.is_some() {
                                return Err(de::Error::duplicate_field("$type"));
                            }
                            variant = Some(map.next_value()?);
                        }
                        "identities" => {
                            if identities.is_some() {
                                return Err(de::Error::duplicate_field("identities"));
                            }
                            identities = Some(map.next_value()?);
                        }
                        _ => {
                            let _: de::IgnoredAny = map.next_value()?;
                        }
                    }
                }

                let variant = variant.ok_or_else(|| de::Error::missing_field("$type"))?;
                match variant.as_str() {
                    "contractOwner" => Ok(ContractModerators::ContractOwner),
                    "ownerAndIdentities" => {
                        let ids =
                            identities.ok_or_else(|| de::Error::missing_field("identities"))?;
                        Ok(ContractModerators::OwnerAndIdentities(ids))
                    }
                    other => Err(de::Error::unknown_variant(
                        other,
                        &["contractOwner", "ownerAndIdentities"],
                    )),
                }
            }
        }

        deserializer.deserialize_map(V)
    }
}

impl fmt::Display for ContractModerators {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContractModerators::ContractOwner => write!(f, "contract owner"),
            ContractModerators::OwnerAndIdentities(ids) => {
                write!(f, "contract owner and {} identities", ids.len())
            }
        }
    }
}

/// Which of the two moderation lists an action or a query refers to.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Encode,
    Decode,
    DecodeUntrusted,
    Hash,
    Serialize,
    Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub enum ContractModerationList {
    /// The banlist: identities barred until an unban.
    Banlist,
    /// The suspension list: identities barred until a block time.
    Suspensions,
}

impl fmt::Display for ContractModerationList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContractModerationList::Banlist => write!(f, "banlist"),
            ContractModerationList::Suspensions => write!(f, "suspensions"),
        }
    }
}

/// The moderation a data contract declares in its config.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractModerationConfig {
    /// The contract keeps a banlist (Drive key `3` under the contract).
    #[serde(default)]
    pub banlist: bool,
    /// The contract keeps a suspension list (Drive key `4` under the contract).
    #[serde(default)]
    pub suspensions: bool,
    /// Who may edit the lists.
    #[serde(default)]
    pub moderators: ContractModerators,
}

impl ContractModerationConfig {
    /// Whether the contract keeps `list`.
    pub fn keeps(&self, list: ContractModerationList) -> bool {
        match list {
            ContractModerationList::Banlist => self.banlist,
            ContractModerationList::Suspensions => self.suspensions,
        }
    }

    /// The lists the contract keeps, in tree key order.
    pub fn lists(&self) -> impl Iterator<Item = ContractModerationList> + '_ {
        [
            ContractModerationList::Banlist,
            ContractModerationList::Suspensions,
        ]
        .into_iter()
        .filter(|list| self.keeps(*list))
    }

    /// Whether `identity_id` may moderate a contract owned by `owner_id`.
    pub fn may_moderate(&self, owner_id: &Identifier, identity_id: &Identifier) -> bool {
        self.moderators.may_moderate(owner_id, identity_id)
    }

    /// Whether `identity_id` is the owner or a named moderator, and so can never be a target.
    pub fn is_owner_or_moderator(&self, owner_id: &Identifier, identity_id: &Identifier) -> bool {
        self.may_moderate(owner_id, identity_id)
    }

    /// The pure-data rules of the declaration: at least one list is kept, and a moderator set
    /// is non-empty, within `SystemLimits::max_contract_moderators`, and does not name the
    /// owner.
    pub fn validate(
        &self,
        owner_id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .methods
            .validate_moderation_config
        {
            0 => Ok(self.validate_v0(owner_id, platform_version)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "ContractModerationConfig::validate".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    #[inline(always)]
    fn validate_v0(
        &self,
        owner_id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        if !self.banlist && !self.suspensions {
            return SimpleConsensusValidationResult::new_with_error(
                InvalidContractModerationConfigError::new(
                    "moderation declares neither a banlist nor a suspension list".to_string(),
                )
                .into(),
            );
        }
        if let Some(ids) = self.moderators.identity_ids() {
            if ids.is_empty() {
                return SimpleConsensusValidationResult::new_with_error(
                    InvalidContractModerationConfigError::new(
                        "the moderator identity set is empty".to_string(),
                    )
                    .into(),
                );
            }
            let max = platform_version.system_limits.max_contract_moderators as usize;
            if ids.len() > max {
                return SimpleConsensusValidationResult::new_with_error(
                    InvalidContractModerationConfigError::new(format!(
                        "{} moderator identities named, at most {} allowed",
                        ids.len(),
                        max
                    ))
                    .into(),
                );
            }
            if ids.contains(owner_id) {
                return SimpleConsensusValidationResult::new_with_error(
                    InvalidContractModerationConfigError::new(
                        "the contract owner is named as a moderator identity".to_string(),
                    )
                    .into(),
                );
            }
        }
        SimpleConsensusValidationResult::new()
    }
}

#[cfg(feature = "json-conversion")]
impl JsonSafeFields for ContractModerators {}
#[cfg(feature = "json-conversion")]
impl JsonSafeFields for ContractModerationConfig {}
#[cfg(feature = "json-conversion")]
impl JsonSafeFields for ContractModerationList {}

/// What a contract's moderation lists say about one identity.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    Encode,
    Decode,
    DecodeUntrusted,
    Serialize,
    Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct ContractModerationStatus {
    /// The identity is on the banlist.
    pub banned: bool,
    /// The identity is on the suspension list, until this block time in milliseconds. A
    /// lapsed suspension (at or before the block time) still appears here until it is swept.
    pub suspended_until: Option<TimestampMillis>,
}

impl ContractModerationStatus {
    /// Whether the identity may act on the contract at the document level at `block_time_ms`.
    pub fn is_barred_at(&self, block_time_ms: TimestampMillis) -> bool {
        self.banned
            || self
                .suspended_until
                .is_some_and(|until| until > block_time_ms)
    }

    /// Whether the identity carries a suspension that has lapsed at `block_time_ms`.
    pub fn has_lapsed_suspension_at(&self, block_time_ms: TimestampMillis) -> bool {
        self.suspended_until
            .is_some_and(|until| until <= block_time_ms)
    }
}

/// What one of a contract's moderation lists says about one identity, and nothing about the
/// other list. It is what the proof of a moderation transition's execution shows: that proof
/// holds the edited entry only, so the other list stays unknown rather than being reported as
/// empty (an identity unsuspended a moment ago may well be banned).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "list", rename_all = "camelCase")]
pub enum ContractModerationListStatus {
    /// The banlist entry
    #[serde(rename_all = "camelCase")]
    Banlist {
        /// The identity is on the banlist.
        banned: bool,
    },
    /// The suspension list entry
    #[serde(rename_all = "camelCase")]
    Suspensions {
        /// The identity is on the suspension list, until this block time in milliseconds. A
        /// lapsed suspension still appears here until it is swept.
        suspended_until: Option<TimestampMillis>,
    },
}

impl ContractModerationListStatus {
    /// The part of a full `status` that `list` holds.
    pub fn from_status(list: ContractModerationList, status: &ContractModerationStatus) -> Self {
        match list {
            ContractModerationList::Banlist => Self::Banlist {
                banned: status.banned,
            },
            ContractModerationList::Suspensions => Self::Suspensions {
                suspended_until: status.suspended_until,
            },
        }
    }

    /// The list this status was read from.
    pub fn list(&self) -> ContractModerationList {
        match self {
            Self::Banlist { .. } => ContractModerationList::Banlist,
            Self::Suspensions { .. } => ContractModerationList::Suspensions,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(ids: &[u8]) -> BTreeSet<Identifier> {
        ids.iter().map(|b| Identifier::from([*b; 32])).collect()
    }

    #[test]
    fn should_round_trip_moderators_through_json() {
        for moderators in [
            ContractModerators::ContractOwner,
            ContractModerators::OwnerAndIdentities(set(&[1, 2])),
        ] {
            let json = serde_json::to_value(&moderators).expect("serialize");
            let back: ContractModerators = serde_json::from_value(json).expect("deserialize");
            assert_eq!(moderators, back);
        }
        let json = serde_json::to_value(ContractModerators::OwnerAndIdentities(set(&[1])))
            .expect("serialize");
        assert_eq!(json["$type"], "ownerAndIdentities");
        assert_eq!(json["identities"].as_array().map(|a| a.len()), Some(1));
    }

    #[test]
    fn should_reject_a_config_with_no_list() {
        let owner = Identifier::from([9; 32]);
        let config = ContractModerationConfig {
            banlist: false,
            suspensions: false,
            moderators: ContractModerators::ContractOwner,
        };
        let result = config
            .validate(&owner, PlatformVersion::latest())
            .expect("validate");
        assert!(!result.is_valid());
    }

    #[test]
    fn should_reject_the_owner_among_the_moderators() {
        let owner = Identifier::from([9; 32]);
        let config = ContractModerationConfig {
            banlist: true,
            suspensions: false,
            moderators: ContractModerators::OwnerAndIdentities(set(&[9, 1])),
        };
        let result = config
            .validate(&owner, PlatformVersion::latest())
            .expect("validate");
        assert!(!result.is_valid());
    }

    #[test]
    fn should_reject_an_empty_or_oversized_moderator_set() {
        let owner = Identifier::from([9; 32]);
        let platform_version = PlatformVersion::latest();
        let empty = ContractModerationConfig {
            banlist: true,
            suspensions: true,
            moderators: ContractModerators::OwnerAndIdentities(BTreeSet::new()),
        };
        assert!(!empty
            .validate(&owner, platform_version)
            .expect("validate")
            .is_valid());
        let too_many: Vec<u8> =
            (1..=(platform_version.system_limits.max_contract_moderators as u8 + 1)).collect();
        let oversized = ContractModerationConfig {
            banlist: true,
            suspensions: true,
            moderators: ContractModerators::OwnerAndIdentities(set(&too_many)),
        };
        assert!(!oversized
            .validate(&owner, platform_version)
            .expect("validate")
            .is_valid());
    }

    #[test]
    fn should_accept_a_well_formed_config() {
        let owner = Identifier::from([9; 32]);
        let config = ContractModerationConfig {
            banlist: true,
            suspensions: true,
            moderators: ContractModerators::OwnerAndIdentities(set(&[1, 2, 3])),
        };
        assert!(config
            .validate(&owner, PlatformVersion::latest())
            .expect("validate")
            .is_valid());
        assert!(config.may_moderate(&owner, &owner));
        assert!(config.may_moderate(&owner, &Identifier::from([2; 32])));
        assert!(!config.may_moderate(&owner, &Identifier::from([7; 32])));
        assert_eq!(config.lists().count(), 2);
    }

    #[test]
    fn should_tell_barred_from_lapsed() {
        let banned = ContractModerationStatus {
            banned: true,
            suspended_until: None,
        };
        assert!(banned.is_barred_at(0));
        let suspended = ContractModerationStatus {
            banned: false,
            suspended_until: Some(100),
        };
        assert!(suspended.is_barred_at(99));
        assert!(!suspended.is_barred_at(100));
        assert!(suspended.has_lapsed_suspension_at(100));
        assert!(!suspended.has_lapsed_suspension_at(99));
    }
}
