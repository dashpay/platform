//! Moderation charters.
//!
//! A data contract may declare that its moderation team is elected by the masternodes
//! (decentralized moderation teams, protocol version 14). The moderation charters system
//! contract holds how a team comes to be:
//!
//! - a `reason` is a ground for a moderation action, keyed by its owner and a three-letter code;
//! - a `submittedCharter` is a leader's proposal to moderate one contract on that contract's own
//!   terms: the reasons its actions may name, the share of the moderators fee it takes and how
//!   it splits the pay;
//! - a `joinRequest` is an identity's offer to serve on the team of a proposal, with a message
//!   only the leader can read;
//! - an `electedCharter` is a proposal put to the vote with its team, chosen from the identities
//!   that asked to join it. Creating one opens or joins the contest for the target contract;
//! - once a charter is seated, its leader may add members from the same join requests, up to
//!   the target's `maxAddedModerators` (`addedModerator`), and remove members
//!   (`removedModerator`); a member asks to leave with a `resignationRequest`, which the
//!   leader acts on with a removal and the member withdraws by deleting it.
//!
//! The team that acts is the leader plus [`ElectedCharter::active_members`]: the elected
//! members and the additions, less the removals.
//!
//! The schema carries almost every rule through its keywords (references, lookups, key
//! requirements, `distinctFrom`, `maxBytes` for the description's byte cap, and the
//! `propertyConstraints` rule holding the reward split to 100). What it cannot say is here:
//! [`SubmittedCharter`] and [`ElectedCharter`] read the documents' properties, and
//! [`validate_submitted_charter`] reads a proposal for the path that seats a team. Nothing here
//! reads state.

mod v0;

use crate::consensus::basic::moderation_charter::ModerationCharterMalformedFieldError;
use crate::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};
use crate::ProtocolError;
use platform_value::{Identifier, IdentifierBytes32, Value, ValueMap};
use platform_version::version::PlatformVersion;
use std::collections::{BTreeMap, BTreeSet};

/// The id of the moderation charters system contract, `EG7RGfV8fDTayC2FyVr8HwdpJh3fXDbVztcfE94UmN88`.
///
/// Spelled here so that consensus code can name the contract without the optional contract
/// crates; the crate's own constant is pinned to this one by a test.
pub const MODERATION_CHARTERS_CONTRACT_ID: Identifier = Identifier(IdentifierBytes32([
    197, 6, 230, 72, 106, 198, 82, 129, 253, 135, 43, 86, 185, 182, 17, 112, 164, 127, 96, 5, 107,
    185, 156, 46, 14, 10, 109, 237, 77, 228, 248, 129,
]));

/// The name of the reason document type.
pub const REASON_DOCUMENT_TYPE_NAME: &str = "reason";
/// The name of the proposal document type.
pub const SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME: &str = "submittedCharter";
/// The name of the join request document type.
pub const JOIN_REQUEST_DOCUMENT_TYPE_NAME: &str = "joinRequest";
/// The name of the elected charter document type, the one on the contested index.
pub const ELECTED_CHARTER_DOCUMENT_TYPE_NAME: &str = "electedCharter";
/// The name of the document type of a member the leader adds after the election.
pub const ADDED_MODERATOR_DOCUMENT_TYPE_NAME: &str = "addedModerator";
/// The name of the document type of a member the leader removes.
pub const REMOVED_MODERATOR_DOCUMENT_TYPE_NAME: &str = "removedModerator";
/// The name of the document type of a member asking to leave the team.
pub const RESIGNATION_REQUEST_DOCUMENT_TYPE_NAME: &str = "resignationRequest";

/// The moderators share a proposal takes when it declares none: the full declared fee.
pub const FULL_MODERATORS_SHARE: u8 = 100;

/// The properties of the charter document types.
pub mod property_names {
    pub const TARGET_CONTRACT_ID: &str = "targetContractId";
    pub const DESCRIPTION: &str = "description";
    pub const REASONS: &str = "reasons";
    pub const MODERATORS_SHARE: &str = "moderatorsShare";
    pub const REWARD_SPLIT: &str = "rewardSplit";
    pub const REWARD_SPLIT_LEADER: &str = "leader";
    pub const REWARD_SPLIT_EQUAL: &str = "equal";
    pub const REWARD_SPLIT_ACTIONS: &str = "actions";
    pub const SUBMITTED_CHARTER_ID: &str = "submittedCharterId";
    pub const MEMBERS: &str = "members";
    pub const ELECTED_CHARTER_ID: &str = "electedCharterId";
    pub const MEMBER_ID: &str = "memberId";
}

/// How a team splits every claim of the moderators pot: three percentages summing to 100.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModerationCharterRewardSplit {
    /// The share of the leader.
    pub leader: u8,
    /// The share split equally between the members other than the leader.
    pub equal: u8,
    /// The share split between the members by the moderation actions each signed since the
    /// last claim.
    pub actions: u8,
}

/// A proposal to moderate a contract, as read out of a `submittedCharter` document. Its owner
/// is the leader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmittedCharter {
    /// The contract the team proposes to moderate. Its elected moderation declaration is the
    /// team's whole mandate.
    pub target_contract_id: Identifier,
    /// What the team would moderate and how, for joiners and voters. Informational.
    pub description: String,
    /// The ids of the `reason` documents the team's actions may name, in declared order and
    /// never repeated. A team with none can take no action.
    pub reasons: Vec<Identifier>,
    /// The percentage, 0 to 100, of each moderated type's declared moderators fee the team
    /// takes. `None` is the full amount; 0 is a team that will not moderate and takes no
    /// rewards. See [`SubmittedCharter::moderators_share_or_full`].
    pub moderators_share: Option<u8>,
    /// How the team splits every claim of the moderators pot.
    pub reward_split: ModerationCharterRewardSplit,
}

/// A proposal put to the vote with its team, as read out of an `electedCharter` document. Its
/// owner is the leader, the owner of the proposal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElectedCharter {
    /// The contract contended for, the proposal's target.
    pub target_contract_id: Identifier,
    /// The `submittedCharter` document the team runs on.
    pub submitted_charter_id: Identifier,
    /// The team besides the leader, in declared order and never repeated: each filed a
    /// `joinRequest` for the proposal, which the schema's lookup reference checks.
    pub members: Vec<Identifier>,
}

fn malformed(field: &str, reason: impl Into<String>) -> ModerationCharterMalformedFieldError {
    ModerationCharterMalformedFieldError::for_field(field, reason)
}

fn get<'a>(
    properties: &'a BTreeMap<String, Value>,
    field: &'static str,
) -> Result<&'a Value, ModerationCharterMalformedFieldError> {
    properties
        .get(field)
        .ok_or_else(|| malformed(field, "missing"))
}

fn identifier_list(
    properties: &BTreeMap<String, Value>,
    field: &'static str,
) -> Result<Vec<Identifier>, ModerationCharterMalformedFieldError> {
    get(properties, field)?
        .as_array()
        .ok_or_else(|| malformed(field, "not a list"))?
        .iter()
        .map(|element| {
            element
                .to_identifier()
                .map_err(|e| malformed(field, e.to_string()))
        })
        .collect()
}

fn identifier_list_value(identifiers: &[Identifier]) -> Value {
    Value::Array(
        identifiers
            .iter()
            .map(|id| Value::Identifier(id.to_buffer()))
            .collect(),
    )
}

impl SubmittedCharter {
    /// The share of each moderated type's declared moderators fee the team takes, with an
    /// absent share read as the full amount.
    pub fn moderators_share_or_full(&self) -> u8 {
        self.moderators_share.unwrap_or(FULL_MODERATORS_SHARE)
    }

    /// Reads a proposal out of the properties of a `submittedCharter` document.
    ///
    /// The result carries a consensus error, never a proposal, when a property is missing or
    /// of the wrong type. The proposal's own rules are checked by
    /// [`SubmittedCharter::validate`]; [`validate_submitted_charter`] does both.
    pub fn from_document_properties(
        properties: &BTreeMap<String, Value>,
    ) -> ConsensusValidationResult<Self> {
        match Self::try_from_document_properties(properties) {
            Ok(charter) => ConsensusValidationResult::new_with_data(charter),
            Err(error) => ConsensusValidationResult::new_with_error(error.into()),
        }
    }

    fn try_from_document_properties(
        properties: &BTreeMap<String, Value>,
    ) -> Result<Self, ModerationCharterMalformedFieldError> {
        let target_contract_id = get(properties, property_names::TARGET_CONTRACT_ID)?
            .to_identifier()
            .map_err(|e| malformed(property_names::TARGET_CONTRACT_ID, e.to_string()))?;

        let description = get(properties, property_names::DESCRIPTION)?
            .to_str()
            .map_err(|e| malformed(property_names::DESCRIPTION, e.to_string()))?
            .to_string();

        let reasons = identifier_list(properties, property_names::REASONS)?;

        let moderators_share = properties
            .get(property_names::MODERATORS_SHARE)
            .map(|value| {
                value
                    .to_integer::<u8>()
                    .map_err(|e| malformed(property_names::MODERATORS_SHARE, e.to_string()))
            })
            .transpose()?;

        let reward_split = get(properties, property_names::REWARD_SPLIT)?;
        let share = |name: &str| {
            reward_split
                .get_integer::<u8>(name)
                .map_err(|e| malformed(property_names::REWARD_SPLIT, format!("{name}: {e}")))
        };
        let reward_split = ModerationCharterRewardSplit {
            leader: share(property_names::REWARD_SPLIT_LEADER)?,
            equal: share(property_names::REWARD_SPLIT_EQUAL)?,
            actions: share(property_names::REWARD_SPLIT_ACTIONS)?,
        };

        Ok(Self {
            target_contract_id,
            description,
            reasons,
            moderators_share,
            reward_split,
        })
    }

    /// The properties of the `submittedCharter` document that carries this proposal, the way
    /// [`SubmittedCharter::from_document_properties`] reads them back. An absent share stays
    /// absent.
    pub fn to_document_properties(&self) -> BTreeMap<String, Value> {
        let mut properties = BTreeMap::from([
            (
                property_names::TARGET_CONTRACT_ID.to_string(),
                Value::Identifier(self.target_contract_id.to_buffer()),
            ),
            (
                property_names::DESCRIPTION.to_string(),
                Value::Text(self.description.clone()),
            ),
            (
                property_names::REASONS.to_string(),
                identifier_list_value(&self.reasons),
            ),
            (
                property_names::REWARD_SPLIT.to_string(),
                Value::Map(ValueMap::from([
                    (
                        Value::Text(property_names::REWARD_SPLIT_LEADER.to_string()),
                        Value::U8(self.reward_split.leader),
                    ),
                    (
                        Value::Text(property_names::REWARD_SPLIT_EQUAL.to_string()),
                        Value::U8(self.reward_split.equal),
                    ),
                    (
                        Value::Text(property_names::REWARD_SPLIT_ACTIONS.to_string()),
                        Value::U8(self.reward_split.actions),
                    ),
                ])),
            ),
        ]);
        if let Some(share) = self.moderators_share {
            properties.insert(
                property_names::MODERATORS_SHARE.to_string(),
                Value::U8(share),
            );
        }
        properties
    }

    /// Checks the proposal's own rules. None is left: the reward split's sum is the
    /// contract's `propertyConstraints` rule `rewardSplitIsWhole` and the description's
    /// 4096-byte cap its `maxBytes`, both checked wherever the document is validated. The
    /// versioned step stays for the path that seats a team.
    pub fn validate(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .validation
            .data_contract
            .validate_moderation_charter
        {
            Some(0) => Ok(self.validate_v0()),
            Some(version) => Err(ProtocolError::UnknownVersionMismatch {
                method: "SubmittedCharter::validate".to_string(),
                known_versions: vec![0],
                received: version,
            }),
            None => Err(ProtocolError::NotSupported(format!(
                "moderation charters do not exist at protocol version {}",
                platform_version.protocol_version
            ))),
        }
    }
}

impl ElectedCharter {
    /// The members a seated team acts with besides its leader, `leader_id`: the elected
    /// members and those the leader added after the election, less those the leader removed.
    /// `added` and `removed` are the `memberId`s of the charter's `addedModerator` and
    /// `removedModerator` documents. A removal is final, so the order the documents were
    /// filed in does not matter. A `resignationRequest` changes nothing by itself: the leader
    /// acts on it with a removal. The leader is never among the result: neither list may
    /// name it.
    pub fn active_members<'a>(
        &self,
        leader_id: Identifier,
        added: impl IntoIterator<Item = &'a Identifier>,
        removed: impl IntoIterator<Item = &'a Identifier>,
    ) -> BTreeSet<Identifier> {
        let mut active: BTreeSet<Identifier> = self.members.iter().copied().collect();
        active.extend(added.into_iter().copied());
        for gone in removed {
            active.remove(gone);
        }
        active.remove(&leader_id);
        active
    }

    /// Reads an elected charter out of the properties of an `electedCharter` document. The
    /// result carries a consensus error, never a charter, when a property is missing or of the
    /// wrong type.
    pub fn from_document_properties(
        properties: &BTreeMap<String, Value>,
    ) -> ConsensusValidationResult<Self> {
        match Self::try_from_document_properties(properties) {
            Ok(charter) => ConsensusValidationResult::new_with_data(charter),
            Err(error) => ConsensusValidationResult::new_with_error(error.into()),
        }
    }

    fn try_from_document_properties(
        properties: &BTreeMap<String, Value>,
    ) -> Result<Self, ModerationCharterMalformedFieldError> {
        let target_contract_id = get(properties, property_names::TARGET_CONTRACT_ID)?
            .to_identifier()
            .map_err(|e| malformed(property_names::TARGET_CONTRACT_ID, e.to_string()))?;
        let submitted_charter_id = get(properties, property_names::SUBMITTED_CHARTER_ID)?
            .to_identifier()
            .map_err(|e| malformed(property_names::SUBMITTED_CHARTER_ID, e.to_string()))?;
        let members = identifier_list(properties, property_names::MEMBERS)?;
        Ok(Self {
            target_contract_id,
            submitted_charter_id,
            members,
        })
    }

    /// The properties of the `electedCharter` document that carries this charter, the way
    /// [`ElectedCharter::from_document_properties`] reads them back.
    pub fn to_document_properties(&self) -> BTreeMap<String, Value> {
        BTreeMap::from([
            (
                property_names::TARGET_CONTRACT_ID.to_string(),
                Value::Identifier(self.target_contract_id.to_buffer()),
            ),
            (
                property_names::SUBMITTED_CHARTER_ID.to_string(),
                Value::Identifier(self.submitted_charter_id.to_buffer()),
            ),
            (
                property_names::MEMBERS.to_string(),
                identifier_list_value(&self.members),
            ),
        ])
    }
}

/// Reads a proposal out of the properties of a `submittedCharter` document and checks its own
/// rules. The result carries the proposal when it passes, and the first error it fails on
/// otherwise.
pub fn validate_submitted_charter(
    properties: &BTreeMap<String, Value>,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<SubmittedCharter>, ProtocolError> {
    let result = SubmittedCharter::from_document_properties(properties);
    if !result.is_valid_with_data() {
        return Ok(ConsensusValidationResult::new_with_errors(result.errors));
    }
    let charter = result.into_data()?;
    let validation = charter.validate(platform_version)?;
    if validation.is_valid() {
        Ok(ConsensusValidationResult::new_with_data(charter))
    } else {
        Ok(ConsensusValidationResult::new_with_errors(
            validation.errors,
        ))
    }
}

#[cfg(test)]
mod tests;
