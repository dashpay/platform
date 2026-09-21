//! Elected moderation: the declaration that a contract's moderators are a team chosen by
//! masternodes and evonodes instead of by the contract owner (protocol version 14).
//!
//! The declaration is fixed at the contract's creation and never changes: the election
//! parameters, the document types the team moderates, the envelope a charter must fit in
//! (the abilities it may claim and the most it may charge the moderators part of each
//! document action), who moderates until the first team is seated, and whether the owner is
//! protected from the team. No election exists yet: until one does, the contract is in its
//! **interim**, moderated by the interim moderators the declaration names, or, with the
//! moderated types not yet usable, by nobody.

use crate::balances::credits::{Credits, MAX_CREDITS};
use crate::data_contract::config::moderation::ContractModerationConfig;
use crate::data_contract::DocumentName;
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonSafeFields;
use crate::state_transition::batch_transition::batched_transition::document_transition_action_type::DocumentTransitionActionType;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::{Identifier, Value};
use platform_version::version::PlatformVersion;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// One week: the join window and the vote window a declaration gets when it leaves them out.
pub const DEFAULT_ELECTION_WINDOW_SECONDS: u32 = 604_800;

/// The keys of an elected declaration on the wire, beside the `$type` of its variant.
pub mod property_names {
    /// The join window, in seconds
    pub const JOIN_WINDOW: &str = "joinWindow";
    /// The vote window, in seconds
    pub const VOTE_WINDOW: &str = "voteWindow";
    /// The challenge cool-down, in seconds
    pub const CHALLENGE_COOL_DOWN: &str = "challengeCoolDown";
    /// The moderated document type names
    pub const MODERATED_DOCUMENT_TYPES: &str = "moderatedDocumentTypes";
    /// The abilities a charter may claim
    pub const ABILITIES: &str = "abilities";
    /// The moderators action fee maximums, by document type
    pub const MODERATORS_ACTION_FEE_MAXIMUMS: &str = "moderatorsActionFeeMaximums";
    /// The interim moderators
    pub const INTERIM: &str = "interim";
    /// Whether the owner is protected from the team
    pub const OWNER_PROTECTED: &str = "ownerProtected";
    /// The `$type` of the variant and of the interim
    pub const TYPE: &str = "$type";
    /// The identities of an appointed interim set
    pub const IDENTITIES: &str = "identities";
}

/// What a moderation team may do to a contract's users and content. The contract declares
/// which of these a charter may claim; a seated team has what its charter claims.
///
/// Append-only: the discriminant is stored in every declaration.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Encode,
    Decode,
    DecodeUntrusted,
    Serialize,
    Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub enum ModerationAbility {
    /// Delete documents of the document types flagged `canBeDeletedByModerators`, within
    /// each type's window. Needs such a type.
    DeleteDocuments,
    /// Put identities on the banlist and take them off it. Needs the banlist.
    Ban,
    /// Put identities on the suspension list and take them off it. Needs the suspension
    /// list.
    Suspend,
    /// Warn identities and clear their warnings. Needs the warning list.
    Warn,
}

impl ModerationAbility {
    /// The wire name of the ability
    pub fn as_str(&self) -> &'static str {
        match self {
            ModerationAbility::DeleteDocuments => "deleteDocuments",
            ModerationAbility::Ban => "ban",
            ModerationAbility::Suspend => "suspend",
            ModerationAbility::Warn => "warn",
        }
    }
}

impl fmt::Display for ModerationAbility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Who moderates an elected contract until its first team is seated.
///
/// The first two are the merged kinds of [`ContractModerators`](super::ContractModerators),
/// with the same authority: the owner alone, or the owner and a fixed set. The third names
/// nobody: the moderated document types can not be used until a team is seated (their
/// transitions are refused), and nothing else is moderated in the meantime. A contract that
/// never attracts a team keeps those types unusable for good.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub enum InterimModerators {
    /// Only the contract owner moderates until a team is seated.
    ContractOwner,
    /// The identities the contract appoints moderate until a team is seated, beside its
    /// owner, who always may. Non-empty, at most `SystemLimits::max_contract_moderators`,
    /// each an identity that exists; a named owner counts toward the limit.
    AppointedModerators(BTreeSet<Identifier>),
    /// Nobody moderates until a team is seated, and the moderated document types can not be
    /// used until then.
    NotYetUsable,
}

impl InterimModerators {
    /// The identities the interim names, `None` unless a set is appointed.
    pub fn identity_ids(&self) -> Option<&BTreeSet<Identifier>> {
        match self {
            InterimModerators::AppointedModerators(ids) => Some(ids),
            InterimModerators::ContractOwner | InterimModerators::NotYetUsable => None,
        }
    }

    /// Whether `identity_id` may moderate a contract owned by `owner_id` during the interim.
    pub fn may_moderate(&self, owner_id: &Identifier, identity_id: &Identifier) -> bool {
        match self {
            InterimModerators::ContractOwner => owner_id == identity_id,
            InterimModerators::AppointedModerators(ids) => {
                owner_id == identity_id || ids.contains(identity_id)
            }
            InterimModerators::NotYetUsable => false,
        }
    }

    /// The interim team of a contract owned by `owner_id`: who shares its moderators fee pot
    /// until a team is seated. Nobody under [`InterimModerators::NotYetUsable`]: the pot
    /// accumulates for the team that gets seated.
    pub fn team(&self, owner_id: &Identifier) -> BTreeSet<Identifier> {
        match self {
            InterimModerators::ContractOwner => BTreeSet::from([*owner_id]),
            InterimModerators::AppointedModerators(ids) => ids.clone(),
            InterimModerators::NotYetUsable => BTreeSet::new(),
        }
    }

    /// Whether the moderated document types are unusable during the interim.
    pub fn blocks_moderated_document_types(&self) -> bool {
        matches!(self, InterimModerators::NotYetUsable)
    }

    /// The wire name of the kind
    fn type_name(&self) -> &'static str {
        match self {
            InterimModerators::ContractOwner => "contractOwner",
            InterimModerators::AppointedModerators(_) => "appointedModerators",
            InterimModerators::NotYetUsable => "notYetUsable",
        }
    }
}

// The wire shape is the moderators' own: a flat `{"$type": "contractOwner"}`,
// `{"$type": "appointedModerators", "identities": [...]}` or `{"$type": "notYetUsable"}` map.
impl Serialize for InterimModerators {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let identities = self.identity_ids();
        let mut m = serializer.serialize_map(Some(1 + usize::from(identities.is_some())))?;
        m.serialize_entry(property_names::TYPE, self.type_name())?;
        if let Some(ids) = identities {
            m.serialize_entry(property_names::IDENTITIES, ids)?;
        }
        m.end()
    }
}

impl<'de> Deserialize<'de> for InterimModerators {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::{self, MapAccess, Visitor};

        struct V;

        impl<'de> Visitor<'de> for V {
            type Value = InterimModerators;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(
                    "InterimModerators as a map with a `$type` discriminator, \
                     e.g. {\"$type\": \"contractOwner\"}, \
                     {\"$type\": \"appointedModerators\", \"identities\": [\"<base58>\"]} or \
                     {\"$type\": \"notYetUsable\"}",
                )
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut variant: Option<String> = None;
                let mut identities: Option<BTreeSet<Identifier>> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        property_names::TYPE => {
                            if variant.is_some() {
                                return Err(de::Error::duplicate_field(property_names::TYPE));
                            }
                            variant = Some(map.next_value()?);
                        }
                        property_names::IDENTITIES => {
                            if identities.is_some() {
                                return Err(de::Error::duplicate_field(property_names::IDENTITIES));
                            }
                            identities = Some(map.next_value()?);
                        }
                        // Refused rather than skipped: the declaration is frozen, so a
                        // misspelled key must not pass.
                        other => {
                            return Err(de::Error::unknown_field(
                                other,
                                &[property_names::TYPE, property_names::IDENTITIES],
                            ));
                        }
                    }
                }

                let variant =
                    variant.ok_or_else(|| de::Error::missing_field(property_names::TYPE))?;
                let without_identities = |kind: InterimModerators| {
                    if identities.is_some() {
                        return Err(de::Error::custom(
                            "`identities` is only valid for `appointedModerators`",
                        ));
                    }
                    Ok(kind)
                };
                match variant.as_str() {
                    "contractOwner" => without_identities(InterimModerators::ContractOwner),
                    "notYetUsable" => without_identities(InterimModerators::NotYetUsable),
                    "appointedModerators" => {
                        let ids = identities
                            .ok_or_else(|| de::Error::missing_field(property_names::IDENTITIES))?;
                        Ok(InterimModerators::AppointedModerators(ids))
                    }
                    other => Err(de::Error::unknown_variant(
                        other,
                        &["contractOwner", "appointedModerators", "notYetUsable"],
                    )),
                }
            }
        }

        deserializer.deserialize_map(V)
    }
}

impl fmt::Display for InterimModerators {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InterimModerators::ContractOwner => write!(f, "the contract owner"),
            InterimModerators::AppointedModerators(ids) => {
                write!(
                    f,
                    "the contract owner and {} appointed moderators",
                    ids.len()
                )
            }
            InterimModerators::NotYetUsable => {
                write!(f, "nobody, its moderated document types not yet usable")
            }
        }
    }
}

/// The most a charter may charge the moderators part of each action on the documents of one
/// document type, in the units the type's `actionFees` are declared in (so scaled by the
/// type's pricing where they are). An action left out lets a charter charge nothing for it.
/// The keys are the action keys of `actionFees`.
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    Hash,
    Encode,
    Decode,
    DecodeUntrusted,
    Serialize,
    Deserialize,
)]
#[serde(deny_unknown_fields)]
pub struct ModeratorsActionFeeMaximums {
    /// For creating a document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create: Option<Credits>,
    /// For replacing a document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replace: Option<Credits>,
    /// For deleting a document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete: Option<Credits>,
    /// For transferring a document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transfer: Option<Credits>,
    /// For updating the price of a document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_price: Option<Credits>,
    /// For purchasing a document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purchase: Option<Credits>,
}

impl ModeratorsActionFeeMaximums {
    /// The maximum of `action`, `None` when a charter may charge nothing for it. Deleting an
    /// index-only document is a deletion.
    pub fn maximum(&self, action: DocumentTransitionActionType) -> Option<Credits> {
        match action {
            DocumentTransitionActionType::Create => self.create,
            DocumentTransitionActionType::Replace => self.replace,
            DocumentTransitionActionType::Delete
            | DocumentTransitionActionType::IndexOnlyDelete => self.delete,
            DocumentTransitionActionType::Transfer => self.transfer,
            DocumentTransitionActionType::UpdatePrice => self.update_price,
            DocumentTransitionActionType::Purchase => self.purchase,
            DocumentTransitionActionType::IgnoreWhileBumpingRevision => None,
        }
    }

    /// Every declared maximum with its action key, in the order of the keys
    pub fn all(&self) -> impl Iterator<Item = (&'static str, Credits)> {
        [
            ("create", self.create),
            ("replace", self.replace),
            ("delete", self.delete),
            ("transfer", self.transfer),
            ("update_price", self.update_price),
            ("purchase", self.purchase),
        ]
        .into_iter()
        .filter_map(|(action, maximum)| maximum.map(|maximum| (action, maximum)))
    }
}

/// The declaration that a contract's moderators are an elected team. Every field is fixed
/// at the contract's creation.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub struct ElectedModerators {
    /// How long, in seconds, applicants may join an election once the first one applied.
    /// `SystemLimits::min_contract_moderation_election_window_seconds` to
    /// `SystemLimits::max_contract_moderation_election_window_seconds` (one day to four
    /// weeks); [`DEFAULT_ELECTION_WINDOW_SECONDS`] when the declaration leaves it out.
    pub join_window: u32,
    /// How long, in seconds, masternodes vote once the join window closed. The same bounds
    /// and default.
    pub vote_window: u32,
    /// How long, in seconds, a seated team is safe from a challenge after a seat change.
    /// `SystemLimits::min_contract_moderation_challenge_cool_down_seconds` to
    /// `SystemLimits::max_contract_moderation_challenge_cool_down_seconds` (two weeks to
    /// three years), always declared.
    pub challenge_cool_down: u32,
    /// The document types the team moderates: non-empty, each a document type of the
    /// contract. A type here need not be flagged `canBeDeletedByModerators`; deletions
    /// reach only the flagged ones. Bans and suspensions are contract-wide whatever this
    /// set says; it bounds deletions and the interim block.
    pub moderated_document_types: BTreeSet<DocumentName>,
    /// The abilities a charter may claim: non-empty, each backed by the contract (a list it
    /// keeps, or a document type moderators may delete from).
    pub abilities: BTreeSet<ModerationAbility>,
    /// The most a charter may charge the moderators part of each document action, by
    /// document type. A type left out lets a charter charge nothing on it. The owner part
    /// of every action stays what the type's `actionFees` declare.
    pub moderators_action_fee_maximums: BTreeMap<DocumentName, ModeratorsActionFeeMaximums>,
    /// Who moderates until the first team is seated.
    pub interim: InterimModerators,
    /// Whether the contract owner is protected from the team, as the owner and the
    /// moderators of the merged kinds are: it can then be neither banned nor suspended, and
    /// its documents can not be deleted. Not protected by default. During the interim the
    /// owner is protected whenever it moderates, flag or not.
    pub owner_protected: bool,
}

impl ElectedModerators {
    /// Whether the team moderates the document type
    pub fn moderates_document_type(&self, document_type_name: &str) -> bool {
        self.moderated_document_types.contains(document_type_name)
    }

    /// Whether a charter may claim the ability
    pub fn allows(&self, ability: ModerationAbility) -> bool {
        self.abilities.contains(&ability)
    }

    /// The most a charter may charge the moderators part of `action` on the documents of
    /// `document_type_name`: zero unless the declaration says otherwise.
    pub fn moderators_action_fee_maximum(
        &self,
        document_type_name: &str,
        action: DocumentTransitionActionType,
    ) -> Credits {
        self.moderators_action_fee_maximums
            .get(document_type_name)
            .and_then(|maximums| maximums.maximum(action))
            .unwrap_or_default()
    }

    /// Whether the interim refuses every document transition of the document type: the
    /// interim names nobody and the type is moderated. Once a team is seated (not yet
    /// possible) this ends.
    pub fn interim_blocks_document_type(&self, document_type_name: &str) -> bool {
        self.interim.blocks_moderated_document_types()
            && self.moderates_document_type(document_type_name)
    }

    /// The pure-data rules of the declaration beyond those every moderator kind shares (a
    /// named set non-empty and within the limit, checked on the interim set by the caller):
    /// the windows and the cool-down within the limits, the moderated set non-empty and
    /// naming document types of the contract, the envelope non-empty and each ability backed
    /// by the contract, and every fee maximum naming a document type of the contract,
    /// pricing at least one action, and non-zero within `MAX_CREDITS`. The first rule broken
    /// is the reason returned.
    pub(super) fn validation_error(
        &self,
        config: &ContractModerationConfig,
        has_document_type_deletable_by_moderators: bool,
        document_schemas: &BTreeMap<DocumentName, Value>,
        platform_version: &PlatformVersion,
    ) -> Option<String> {
        let limits = &platform_version.system_limits;
        let within = |what: &str, seconds: u32, min: u32, max: u32| {
            (seconds < min || seconds > max).then(|| {
                format!("the {what} of {seconds} seconds is outside {min} to {max} seconds")
            })
        };
        let window_min = limits.min_contract_moderation_election_window_seconds;
        let window_max = limits.max_contract_moderation_election_window_seconds;
        if let Some(reason) = within("join window", self.join_window, window_min, window_max)
            .or_else(|| within("vote window", self.vote_window, window_min, window_max))
            .or_else(|| {
                within(
                    "challenge cool-down",
                    self.challenge_cool_down,
                    limits.min_contract_moderation_challenge_cool_down_seconds,
                    limits.max_contract_moderation_challenge_cool_down_seconds,
                )
            })
        {
            return Some(reason);
        }

        if self.moderated_document_types.is_empty() {
            return Some("the moderated document type set is empty".to_string());
        }
        if let Some(unknown) = self
            .moderated_document_types
            .iter()
            .find(|name| !document_schemas.contains_key(*name))
        {
            return Some(format!(
                "the moderated document type \"{unknown}\" is not a document type of the contract"
            ));
        }

        if self.abilities.is_empty() {
            return Some("the ability envelope is empty".to_string());
        }
        for ability in &self.abilities {
            let unbacked = match ability {
                ModerationAbility::Ban if !config.banlist => {
                    Some("bans, but the contract keeps no banlist")
                }
                ModerationAbility::Suspend if !config.suspensions => {
                    Some("suspensions, but the contract keeps no suspension list")
                }
                ModerationAbility::Warn if !config.warnings => {
                    Some("warnings, but the contract keeps no warning list")
                }
                ModerationAbility::DeleteDocuments
                    if !has_document_type_deletable_by_moderators =>
                {
                    Some("document deletions, but no document type can be deleted by moderators")
                }
                _ => None,
            };
            if let Some(unbacked) = unbacked {
                return Some(format!("the ability envelope allows {unbacked}"));
            }
        }

        for (document_type_name, maximums) in &self.moderators_action_fee_maximums {
            if !document_schemas.contains_key(document_type_name) {
                return Some(format!(
                    "a moderators action fee maximum names \"{document_type_name}\", which is not \
                     a document type of the contract"
                ));
            }
            let mut any = false;
            for (action, maximum) in maximums.all() {
                any = true;
                if maximum == 0 {
                    return Some(format!(
                        "the moderators action fee maximum of \"{document_type_name}\".{action} is \
                         zero; leave the action out instead"
                    ));
                }
                if maximum > MAX_CREDITS {
                    return Some(format!(
                        "the moderators action fee maximum of \"{document_type_name}\".{action} of \
                         {maximum} credits is over the maximum of {MAX_CREDITS}"
                    ));
                }
            }
            if !any {
                return Some(format!(
                    "the moderators action fee maximums of \"{document_type_name}\" price no \
                     action; leave the type out instead"
                ));
            }
        }

        None
    }
}

impl fmt::Display for ElectedModerators {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "an elected moderation team, in its interim moderated by {}",
            self.interim
        )
    }
}

#[cfg(feature = "json-conversion")]
impl JsonSafeFields for ModerationAbility {}
#[cfg(feature = "json-conversion")]
impl JsonSafeFields for InterimModerators {}
#[cfg(feature = "json-conversion")]
impl JsonSafeFields for ElectedModerators {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::ConsensusError;
    use crate::data_contract::config::moderation::ContractModerators;
    use platform_value::platform_value;

    fn set(ids: &[u8]) -> BTreeSet<Identifier> {
        ids.iter().map(|b| Identifier::from([*b; 32])).collect()
    }

    /// `post`, which moderators may delete, and `like`, which they may not
    fn schemas() -> BTreeMap<DocumentName, Value> {
        BTreeMap::from([
            (
                "post".to_string(),
                platform_value!({ "type": "object", "canBeDeletedByModerators": true }),
            ),
            ("like".to_string(), platform_value!({ "type": "object" })),
        ])
    }

    /// A declaration within every bound: both lists, bans and suspensions allowed, `post`
    /// moderated, the owner in the interim.
    fn elected() -> ElectedModerators {
        ElectedModerators {
            join_window: DEFAULT_ELECTION_WINDOW_SECONDS,
            vote_window: DEFAULT_ELECTION_WINDOW_SECONDS,
            challenge_cool_down: 1_209_600,
            moderated_document_types: BTreeSet::from(["post".to_string()]),
            abilities: BTreeSet::from([ModerationAbility::Ban, ModerationAbility::Suspend]),
            moderators_action_fee_maximums: BTreeMap::new(),
            interim: InterimModerators::ContractOwner,
            owner_protected: false,
        }
    }

    fn config(elected: ElectedModerators) -> ContractModerationConfig {
        ContractModerationConfig {
            banlist: true,
            suspensions: true,
            warnings: false,
            moderators: ContractModerators::Elected(Box::new(elected)),
        }
    }

    /// The reason the declaration is refused for, `None` when it is accepted
    fn refusal(config: &ContractModerationConfig) -> Option<String> {
        let result = config
            .validate(&schemas(), PlatformVersion::latest())
            .expect("validate");
        (!result.is_valid()).then(|| rendered(&result.errors))
    }

    /// The errors' messages, one per line; `Debug` would escape the quotes the messages
    /// put around document type names.
    fn rendered(errors: &[ConsensusError]) -> String {
        errors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn should_accept_a_declaration_within_every_bound() {
        assert_eq!(refusal(&config(elected())), None);
    }

    #[test]
    fn should_accept_every_bound_and_refuse_one_second_outside_each() {
        let limits = &PlatformVersion::latest().system_limits;
        let window_min = limits.min_contract_moderation_election_window_seconds;
        let window_max = limits.max_contract_moderation_election_window_seconds;
        let cool_down_min = limits.min_contract_moderation_challenge_cool_down_seconds;
        let cool_down_max = limits.max_contract_moderation_challenge_cool_down_seconds;
        assert_eq!(window_min, 86_400);
        assert_eq!(window_max, 2_419_200);
        assert_eq!(cool_down_min, 1_209_600);
        assert_eq!(cool_down_max, 94_608_000);

        let with = |f: fn(&mut ElectedModerators, u32), seconds: u32| {
            let mut declaration = elected();
            f(&mut declaration, seconds);
            config(declaration)
        };
        let join = |d: &mut ElectedModerators, s: u32| d.join_window = s;
        let vote = |d: &mut ElectedModerators, s: u32| d.vote_window = s;
        let cool_down = |d: &mut ElectedModerators, s: u32| d.challenge_cool_down = s;
        for (name, field, min, max) in [
            (
                "join window",
                join as fn(&mut ElectedModerators, u32),
                window_min,
                window_max,
            ),
            ("vote window", vote, window_min, window_max),
            (
                "challenge cool-down",
                cool_down,
                cool_down_min,
                cool_down_max,
            ),
        ] {
            assert_eq!(refusal(&with(field, min)), None, "{name} at its minimum");
            assert_eq!(refusal(&with(field, max)), None, "{name} at its maximum");
            let below = refusal(&with(field, min - 1)).expect("refused below the minimum");
            assert!(below.contains(name), "{below}");
            let above = refusal(&with(field, max + 1)).expect("refused above the maximum");
            assert!(above.contains(name), "{above}");
        }
    }

    #[test]
    fn should_require_a_moderated_set_of_document_types_the_contract_has() {
        let mut empty = elected();
        empty.moderated_document_types.clear();
        assert!(refusal(&config(empty))
            .expect("refused")
            .contains("moderated document type set is empty"));

        let mut unknown = elected();
        unknown
            .moderated_document_types
            .insert("comment".to_string());
        assert!(refusal(&config(unknown))
            .expect("refused")
            .contains("\"comment\" is not a document type"));

        // A moderated type need not be deletable by moderators.
        let mut not_deletable = elected();
        not_deletable.moderated_document_types = BTreeSet::from(["like".to_string()]);
        assert_eq!(refusal(&config(not_deletable)), None);
    }

    #[test]
    fn should_require_a_non_empty_envelope_the_contract_can_back() {
        let mut empty = elected();
        empty.abilities.clear();
        assert!(refusal(&config(empty))
            .expect("refused")
            .contains("ability envelope is empty"));

        let mut bans_without_banlist = config(elected());
        bans_without_banlist.banlist = false;
        bans_without_banlist.suspensions = true;
        assert!(refusal(&bans_without_banlist)
            .expect("refused")
            .contains("keeps no banlist"));

        let mut suspensions_without_list = config(elected());
        suspensions_without_list.suspensions = false;
        assert!(refusal(&suspensions_without_list)
            .expect("refused")
            .contains("keeps no suspension list"));

        let mut warnings = elected();
        warnings.abilities.insert(ModerationAbility::Warn);
        let mut warnings_without_list = config(warnings);
        assert!(refusal(&warnings_without_list)
            .expect("refused")
            .contains("keeps no warning list"));
        warnings_without_list.warnings = true;
        assert_eq!(refusal(&warnings_without_list), None);

        // Deletions are backed by `post`; without it they are not.
        let mut deletions = elected();
        deletions.abilities = BTreeSet::from([ModerationAbility::DeleteDocuments]);
        let with_post = config(deletions);
        assert_eq!(refusal(&with_post), None);
        let no_deletable_type = with_post
            .validate(
                &BTreeMap::from([("like".to_string(), platform_value!({ "type": "object" }))]),
                PlatformVersion::latest(),
            )
            .expect("validate");
        assert!(!no_deletable_type.is_valid());
        // The moderated set is checked first: `post` is not in that contract.
        assert!(rendered(&no_deletable_type.errors).contains("\"post\" is not a document type"));
    }

    #[test]
    fn should_check_every_fee_maximum() {
        let maximum = |create: Option<Credits>| ModeratorsActionFeeMaximums {
            create,
            ..Default::default()
        };
        let with = |name: &str, maximums: ModeratorsActionFeeMaximums| {
            let mut declaration = elected();
            declaration
                .moderators_action_fee_maximums
                .insert(name.to_string(), maximums);
            config(declaration)
        };

        assert_eq!(refusal(&with("like", maximum(Some(1)))), None);
        assert_eq!(refusal(&with("post", maximum(Some(MAX_CREDITS)))), None);
        assert!(refusal(&with("comment", maximum(Some(1))))
            .expect("refused")
            .contains("names \"comment\""));
        assert!(refusal(&with("post", maximum(None)))
            .expect("refused")
            .contains("price no action"));
        assert!(refusal(&with("post", maximum(Some(0))))
            .expect("refused")
            .contains("\"post\".create is zero"));
        assert!(refusal(&with("post", maximum(Some(MAX_CREDITS + 1))))
            .expect("refused")
            .contains("over the maximum"));

        let declaration = with("post", maximum(Some(7)));
        let elected = declaration.moderators.elected().expect("elected");
        assert_eq!(
            elected.moderators_action_fee_maximum("post", DocumentTransitionActionType::Create),
            7
        );
        assert_eq!(
            elected.moderators_action_fee_maximum(
                "post",
                DocumentTransitionActionType::IndexOnlyDelete
            ),
            0
        );
        assert_eq!(
            elected.moderators_action_fee_maximum("like", DocumentTransitionActionType::Create),
            0
        );
    }

    #[test]
    fn should_bound_the_interim_set_like_an_appointed_set() {
        let max = PlatformVersion::latest()
            .system_limits
            .max_contract_moderators as u8;
        let with = |ids: &[u8]| {
            let mut declaration = elected();
            declaration.interim = InterimModerators::AppointedModerators(set(ids));
            config(declaration)
        };
        assert!(refusal(&with(&[])).expect("refused").contains("empty"));
        assert_eq!(refusal(&with(&(1..=max).collect::<Vec<u8>>())), None);
        assert!(refusal(&with(&(1..=max + 1).collect::<Vec<u8>>()))
            .expect("refused")
            .contains("at most"));
    }

    #[test]
    fn should_give_each_interim_kind_its_authority_team_and_block() {
        let owner = Identifier::from([9; 32]);
        let moderator = Identifier::from([1; 32]);
        let user = Identifier::from([2; 32]);
        let with = |interim: InterimModerators, owner_protected: bool| {
            let mut declaration = elected();
            declaration.interim = interim;
            declaration.owner_protected = owner_protected;
            ContractModerators::Elected(Box::new(declaration))
        };

        let owner_alone = with(InterimModerators::ContractOwner, false);
        assert!(owner_alone.may_moderate(&owner, &owner));
        assert!(!owner_alone.may_moderate(&owner, &moderator));
        assert_eq!(owner_alone.team(&owner), BTreeSet::from([owner]));
        assert_eq!(owner_alone.identity_ids(), None);
        assert!(owner_alone.protects(&owner, &owner));
        assert!(!owner_alone.protects(&owner, &user));
        assert!(!owner_alone.interim_blocks_document_type("post"));

        let appointed = with(InterimModerators::AppointedModerators(set(&[1])), false);
        assert!(appointed.may_moderate(&owner, &owner));
        assert!(appointed.may_moderate(&owner, &moderator));
        assert!(!appointed.may_moderate(&owner, &user));
        assert_eq!(appointed.team(&owner), set(&[1]));
        assert_eq!(appointed.identity_ids(), Some(&set(&[1])));
        assert!(appointed.protects(&owner, &moderator));

        let nobody = with(InterimModerators::NotYetUsable, false);
        assert!(!nobody.may_moderate(&owner, &owner));
        assert!(nobody.team(&owner).is_empty());
        assert!(!nobody.protects(&owner, &owner));
        assert!(nobody.interim_blocks_document_type("post"));
        assert!(!nobody.interim_blocks_document_type("like"));

        // The flag protects the owner where nothing else does.
        let protected = with(InterimModerators::NotYetUsable, true);
        assert!(!protected.may_moderate(&owner, &owner));
        assert!(protected.protects(&owner, &owner));
        assert!(!protected.protects(&owner, &user));
    }

    #[test]
    fn should_round_trip_through_json_and_platform_value() {
        let mut declaration = elected();
        declaration.interim = InterimModerators::AppointedModerators(set(&[1, 2]));
        declaration.moderators_action_fee_maximums.insert(
            "post".to_string(),
            ModeratorsActionFeeMaximums {
                create: Some(1_000),
                delete: Some(MAX_CREDITS),
                ..Default::default()
            },
        );
        declaration.owner_protected = true;
        let moderators = ContractModerators::Elected(Box::new(declaration));

        let json = serde_json::to_value(&moderators).expect("serialize");
        assert_eq!(json["$type"], "elected");
        assert_eq!(json["joinWindow"], 604_800);
        assert_eq!(json["moderatedDocumentTypes"], serde_json::json!(["post"]));
        assert_eq!(json["abilities"], serde_json::json!(["ban", "suspend"]));
        assert_eq!(json["interim"]["$type"], "appointedModerators");
        assert_eq!(
            json["interim"]["identities"].as_array().map(|a| a.len()),
            Some(2)
        );
        assert_eq!(json["moderatorsActionFeeMaximums"]["post"]["create"], 1_000);
        // Past 2^53 a credit amount travels as a string in JSON, and never in a value.
        assert_eq!(
            json["moderatorsActionFeeMaximums"]["post"]["delete"],
            MAX_CREDITS.to_string()
        );
        assert!(json["moderatorsActionFeeMaximums"]["post"]
            .get("replace")
            .is_none());
        assert_eq!(json["ownerProtected"], true);
        let back: ContractModerators = serde_json::from_value(json).expect("deserialize");
        assert_eq!(back, moderators);

        let value = platform_value::to_value(&moderators).expect("to value");
        let back: ContractModerators = platform_value::from_value(value).expect("from value");
        assert_eq!(back, moderators);

        let config = ContractModerationConfig {
            banlist: true,
            suspensions: true,
            warnings: false,
            moderators: moderators.clone(),
        };
        let value = platform_value::to_value(&config).expect("to value");
        let back: ContractModerationConfig = platform_value::from_value(value).expect("from value");
        assert_eq!(back, config);
    }

    #[test]
    fn should_default_the_windows_and_the_flag_and_refuse_a_misspelled_key() {
        let minimal = serde_json::json!({
            "$type": "elected",
            "challengeCoolDown": 1_209_600,
            "moderatedDocumentTypes": ["post"],
            "abilities": ["ban"],
            "interim": { "$type": "notYetUsable" },
        });
        let parsed: ContractModerators = serde_json::from_value(minimal).expect("deserialize");
        let elected = parsed.elected().expect("elected");
        assert_eq!(elected.join_window, DEFAULT_ELECTION_WINDOW_SECONDS);
        assert_eq!(elected.vote_window, DEFAULT_ELECTION_WINDOW_SECONDS);
        assert!(!elected.owner_protected);
        assert!(elected.moderators_action_fee_maximums.is_empty());
        assert_eq!(elected.interim, InterimModerators::NotYetUsable);

        let refused = [
            // The cool-down has no default.
            serde_json::json!({
                "$type": "elected",
                "moderatedDocumentTypes": ["post"],
                "abilities": ["ban"],
                "interim": { "$type": "contractOwner" },
            }),
            // A misspelled key is refused, not dropped.
            serde_json::json!({
                "$type": "elected",
                "challengeCoolDown": 1_209_600,
                "moderatedDocumentTypes": ["post"],
                "abilities": ["ban"],
                "interim": { "$type": "contractOwner" },
                "ownerProtcted": true,
            }),
            // So is one inside the interim, and an unknown interim kind.
            serde_json::json!({
                "$type": "elected",
                "challengeCoolDown": 1_209_600,
                "moderatedDocumentTypes": ["post"],
                "abilities": ["ban"],
                "interim": { "$type": "contractOwner", "identity": [] },
            }),
            serde_json::json!({
                "$type": "elected",
                "challengeCoolDown": 1_209_600,
                "moderatedDocumentTypes": ["post"],
                "abilities": ["ban"],
                "interim": { "$type": "seatedTeam" },
            }),
            // An unknown ability, and an unknown fee action.
            serde_json::json!({
                "$type": "elected",
                "challengeCoolDown": 1_209_600,
                "moderatedDocumentTypes": ["post"],
                "abilities": ["silence"],
                "interim": { "$type": "contractOwner" },
            }),
            serde_json::json!({
                "$type": "elected",
                "challengeCoolDown": 1_209_600,
                "moderatedDocumentTypes": ["post"],
                "abilities": ["ban"],
                "moderatorsActionFeeMaximums": { "post": { "updatePrice": 1 } },
                "interim": { "$type": "contractOwner" },
            }),
            // The elected keys under another kind, and `identities` under elected.
            serde_json::json!({ "$type": "contractOwner", "ownerProtected": true }),
            serde_json::json!({
                "$type": "elected",
                "challengeCoolDown": 1_209_600,
                "moderatedDocumentTypes": ["post"],
                "abilities": ["ban"],
                "interim": { "$type": "contractOwner" },
                "identities": [],
            }),
        ];
        for json in refused {
            assert!(
                serde_json::from_value::<ContractModerators>(json.clone()).is_err(),
                "{json}"
            );
        }
    }

    #[test]
    fn should_describe_itself() {
        let mut declaration = elected();
        assert_eq!(
            ContractModerators::Elected(Box::new(declaration.clone())).to_string(),
            "an elected moderation team, in its interim moderated by the contract owner"
        );
        declaration.interim = InterimModerators::NotYetUsable;
        assert_eq!(
            declaration.to_string(),
            "an elected moderation team, in its interim moderated by nobody, its moderated \
             document types not yet usable"
        );
        assert_eq!(
            ModerationAbility::DeleteDocuments.to_string(),
            "deleteDocuments"
        );
    }
}
