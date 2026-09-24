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
//! - once a charter is seated, its leader may add members from the same join requests, at most
//!   the target's `maxAddedModerators` at a time (`addedModerator`, taken back by deleting it),
//!   and remove elected members (`removedModerator`, undone by deleting it); a member asks to
//!   leave with a `resignationRequest`, which the leader acts on and the member withdraws by
//!   deleting it.
//!
//! The team that acts is the leader plus [`ElectedCharter::active_members`]: the elected
//! members and the additions, less the removals.
//!
//! Seating writes nothing. Awarding the contest for a target writes the winning
//! `electedCharter` to the contract's storage, the only one ever written there for that target
//! (contenders live in the contest, and in protocol version 14 a seat is never replaced), so the
//! charter seated on a contract is the one its `byTargetContract` index finds. The moderation
//! paths of the target read it from there: its team moderates, its proposal's
//! [`SubmittedCharter::moderators_share`] discounts the moderators part of an action fee
//! ([`moderators_share_of`]), and its additions are capped by the target's
//! `maxAddedModerators`.
//!
//! The schema carries almost every rule through its keywords (references, lookups, key
//! requirements, `distinctFrom`, `maxBytes` for the description's byte cap, and the
//! `propertyConstraints` rule holding the reward split to 100). What is here only reads:
//! [`SubmittedCharter`] and [`ElectedCharter`] read the documents' properties. Nothing here
//! reads state.

mod reward_split;

use crate::balances::credits::Credits;
use crate::consensus::basic::moderation_charter::ModerationCharterMalformedFieldError;
use crate::data_contract::document_type::contested_index_identifier;
use crate::validation::ConsensusValidationResult;
use platform_value::{Identifier, IdentifierBytes32, Value, ValueMap};
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

/// Whether a contest on the contested index of `document_type_name` in the contract
/// `contract_id` is a moderation election: an `electedCharter` of the moderation charters
/// contract, contending for the seat of its target contract. A moderation election runs on the
/// join and vote windows its target declares and is prefunded with the moderation fund; every
/// other contest keeps the generic windows and fund.
pub fn is_charter_election(contract_id: &Identifier, document_type_name: &str) -> bool {
    *contract_id == MODERATION_CHARTERS_CONTRACT_ID
        && document_type_name == ELECTED_CHARTER_DOCUMENT_TYPE_NAME
}

/// The contract a moderation election contends for: the single value of the contested index's
/// key, `targetContractId`, in any form validation accepts for an identifier (from protocol
/// version 14 a contest's index values are written as `Value::Identifier` anyway, see
/// `Index::extract_contested_values`). `None` for every other contest, and for index values
/// that do not name one contract, a base58 string included.
pub fn charter_election_target(
    contract_id: &Identifier,
    document_type_name: &str,
    index_values: &[Value],
) -> Option<Identifier> {
    if !is_charter_election(contract_id, document_type_name) {
        return None;
    }
    match index_values {
        [target] => contested_index_identifier(target).map(Identifier::new),
        _ => None,
    }
}

/// The moderators part a seated charter's team charges for an action whose document type
/// declares `declared_moderators`: `moderators_share` percent of it, rounded down to the credit.
/// A document action on a type the target moderates may agree to exactly this amount instead
/// of the declared one; it is then charged this amount, and nothing else below the declared
/// amount is accepted. A share of 100 (or none declared) gives the declared amount itself.
pub fn moderators_share_of(declared_moderators: Credits, moderators_share: u8) -> Credits {
    let share = (declared_moderators as u128) * (moderators_share as u128)
        / (FULL_MODERATORS_SHARE as u128);
    // At most 100 percent of an amount that fits, so the share fits; a stored share over 100
    // is refused by the schema, and is held at the declared amount if one ever got through.
    Credits::try_from(share)
        .unwrap_or(declared_moderators)
        .min(declared_moderators)
}

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

/// How a team splits every settle of the moderators pot, a claim or a change of the team: three
/// percentages summing to 100.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModerationCharterRewardSplit {
    /// The share of the leader.
    pub leader: u8,
    /// The share split equally between the members other than the leader; the leader's when
    /// it has no member.
    pub equal: u8,
    /// The share split between the team, the leader included, by the moderation actions each
    /// signed since the pot was last settled, or equally when nobody acted. See
    /// [`ModerationCharterRewardSplit::payouts`].
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
    /// of the wrong type. The proposal's rules are the contract's own keywords (the reward
    /// split's `propertyConstraints` rule `rewardSplitIsWhole`, the description's `maxBytes`),
    /// checked wherever the document is validated, not here.
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
}

impl ElectedCharter {
    /// The members a seated team acts with besides its leader, `leader_id`: the elected
    /// members and those the leader added after the election, less those the leader removed.
    /// `added` and `removed` are the `memberId`s of the charter's `addedModerator` and
    /// `removedModerator` documents that exist now: the leader takes an addition back by
    /// deleting it, and a removal, which only names an elected member, puts the member back
    /// when it is deleted. A removal wins over an addition of the same member, so the order
    /// the documents were filed in does not matter. A `resignationRequest` changes nothing by
    /// itself: the leader acts on it by deleting the member's addition or removing an elected
    /// member. The leader is never among the result: neither list may name it.
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

#[cfg(test)]
mod tests;
