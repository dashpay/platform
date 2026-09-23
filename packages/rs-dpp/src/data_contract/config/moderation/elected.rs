//! Elected moderation: the declaration that a contract's moderators are a team chosen by
//! masternodes and evonodes instead of by the contract owner (protocol version 14).
//!
//! The declaration is fixed at the contract's creation and never changes: the election
//! parameters, the document types the team moderates with the abilities it holds on each,
//! who moderates until the first team is seated, and whether the owner is protected from the
//! team. A charter does not price the moderators part of a document action: the
//! type's own `actionFees.moderators` amount is the most a team may charge, and a charter
//! charges a share of it. No election exists yet: until one does, the contract is in its
//! **interim**,
//! moderated by the interim moderators the declaration names, or by nobody: with the
//! moderated types not yet usable, or usable and unmoderated meanwhile.

use crate::data_contract::config::moderation::{
    document_schema_lets_moderators_delete, ContractModerationConfig,
};
use crate::data_contract::DocumentName;
use crate::prelude::TimestampMillis;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonSafeFields;
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
    /// The election delay, in seconds after the contract's creation
    pub const ELECTION_DELAY: &str = "electionDelay";
    /// The moderated document types, each with the abilities the seated team holds on it
    pub const MODERATED_DOCUMENT_TYPES: &str = "moderatedDocumentTypes";
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
/// which of these a seated team holds on each moderated document type. A team holds every
/// ability the declaration gives it; its charter narrows what it may act on only through the
/// moderation reasons it lists, since every action names one.
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
    /// Delete documents of the type, within its `canBeDeletedByModeratorsFor` window. Needs
    /// the type flagged `canBeDeletedByModerators`.
    DeleteDocuments,
    /// Put identities on the banlist and take them off it, over their documents of the
    /// type. Needs the banlist.
    Ban,
    /// Put identities on the suspension list and take them off it, over their documents of
    /// the type. Needs the suspension list.
    Suspend,
    /// Warn identities and clear their warnings, over their documents of the type. Needs
    /// the warning list.
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
/// with the same authority: the owner alone, or the owner and a fixed set. The other two
/// name nobody, so nothing is moderated in the meantime and nobody claims the moderators
/// pot: under `NotYetUsable` the moderated document types can not be used until a team is
/// seated (their transitions are refused), and a contract that never attracts a team keeps
/// them unusable for good; under `NoModeration` they are used unmoderated until then.
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
    /// Nobody moderates until a team is seated, and the moderated document types are used
    /// unmoderated until then.
    NoModeration,
}

impl InterimModerators {
    /// The identities the interim names, `None` unless a set is appointed.
    pub fn identity_ids(&self) -> Option<&BTreeSet<Identifier>> {
        match self {
            InterimModerators::AppointedModerators(ids) => Some(ids),
            InterimModerators::ContractOwner
            | InterimModerators::NotYetUsable
            | InterimModerators::NoModeration => None,
        }
    }

    /// Whether `identity_id` may moderate a contract owned by `owner_id` during the interim.
    pub fn may_moderate(&self, owner_id: &Identifier, identity_id: &Identifier) -> bool {
        match self {
            InterimModerators::ContractOwner => owner_id == identity_id,
            InterimModerators::AppointedModerators(ids) => {
                owner_id == identity_id || ids.contains(identity_id)
            }
            InterimModerators::NotYetUsable | InterimModerators::NoModeration => false,
        }
    }

    /// The interim team of a contract owned by `owner_id`: who shares its moderators fee pot
    /// until a team is seated. Nobody under [`InterimModerators::NotYetUsable`] or
    /// [`InterimModerators::NoModeration`]: the pot accumulates for the team that gets seated.
    pub fn team(&self, owner_id: &Identifier) -> BTreeSet<Identifier> {
        match self {
            InterimModerators::ContractOwner => BTreeSet::from([*owner_id]),
            InterimModerators::AppointedModerators(ids) => ids.clone(),
            InterimModerators::NotYetUsable | InterimModerators::NoModeration => BTreeSet::new(),
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
            InterimModerators::NoModeration => "noModeration",
        }
    }
}

// The wire shape is the moderators' own: a flat `{"$type": "contractOwner"}`,
// `{"$type": "appointedModerators", "identities": [...]}`, `{"$type": "notYetUsable"}` or
// `{"$type": "noModeration"}` map.
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
                     {\"$type\": \"appointedModerators\", \"identities\": [\"<base58>\"]}, \
                     {\"$type\": \"notYetUsable\"} or {\"$type\": \"noModeration\"}",
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
                    "noModeration" => without_identities(InterimModerators::NoModeration),
                    "appointedModerators" => {
                        let ids = identities
                            .ok_or_else(|| de::Error::missing_field(property_names::IDENTITIES))?;
                        Ok(InterimModerators::AppointedModerators(ids))
                    }
                    other => Err(de::Error::unknown_variant(
                        other,
                        &[
                            "contractOwner",
                            "appointedModerators",
                            "notYetUsable",
                            "noModeration",
                        ],
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
            InterimModerators::NoModeration => {
                write!(
                    f,
                    "nobody, its moderated document types unmoderated meanwhile"
                )
            }
        }
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
    /// How long, in seconds after the contract's creation, before the first charter may be
    /// filed against the contract: the notice the contract gives before its first election
    /// can be called. Unbounded, and `None` when the declaration leaves it out, in which
    /// case the election may be called at once. A reference declaring
    /// `contractRequirements: { "moderation": "electionOpen" }` is what reads it.
    pub election_delay: Option<u32>,
    /// The document types the team moderates, each with the abilities the seated team holds
    /// on it: non-empty, each type a document type of the contract, each ability set
    /// non-empty and backed by the contract (`Ban`, `Suspend` and `Warn` by the list the
    /// contract keeps, `DeleteDocuments` by the type being flagged
    /// `canBeDeletedByModerators`). The lists themselves stay contract-wide: an ability on
    /// a type is what a team may do over the documents of that type. The set also bounds
    /// the interim block. A charter does not price the moderators part of an action: the
    /// type's `actionFees.moderators` amount is the most a team may charge, and a charter
    /// charges a share of it.
    pub moderated_document_types: BTreeMap<DocumentName, BTreeSet<ModerationAbility>>,
    /// Who moderates until the first team is seated.
    pub interim: InterimModerators,
    /// Whether the contract owner is protected from the team, as the owner and the
    /// moderators of the merged kinds are: it can then be neither banned nor suspended, and
    /// its documents can not be deleted. Not protected by default. During the interim the
    /// owner is protected whenever it moderates, flag or not.
    pub owner_protected: bool,
}

impl ElectedModerators {
    /// Whether the first election may be called at `block_time_ms` on a contract created at
    /// `contract_created_at`: the declaration has no election delay, or the delay has passed
    /// since the creation. An elected declaration is made at the contract's creation and
    /// never changes, so the creation is the declaration's own time. A contract without a
    /// recorded creation time and with a delay is of unknown age, and its election is not
    /// open.
    pub fn election_is_open(
        &self,
        contract_created_at: Option<TimestampMillis>,
        block_time_ms: TimestampMillis,
    ) -> bool {
        match self.election_delay {
            None => true,
            Some(delay) => contract_created_at.is_some_and(|created_at| {
                block_time_ms
                    >= created_at.saturating_add(TimestampMillis::from(delay).saturating_mul(1000))
            }),
        }
    }

    /// Whether the team moderates the document type
    pub fn moderates_document_type(&self, document_type_name: &str) -> bool {
        self.moderated_document_types
            .contains_key(document_type_name)
    }

    /// Whether the seated team holds the ability on the document type
    pub fn allows(&self, document_type_name: &str, ability: ModerationAbility) -> bool {
        self.moderated_document_types
            .get(document_type_name)
            .is_some_and(|abilities| abilities.contains(&ability))
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
    /// the windows and the cool-down within the limits, the moderated set non-empty, each
    /// of its types a document type of the contract with a non-empty ability set the
    /// contract backs. The first rule broken is the reason returned.
    pub(super) fn validation_error(
        &self,
        config: &ContractModerationConfig,
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
        for (document_type_name, abilities) in &self.moderated_document_types {
            let Some(schema) = document_schemas.get(document_type_name) else {
                return Some(format!(
                    "the moderated document type \"{document_type_name}\" is not a document \
                     type of the contract"
                ));
            };
            if abilities.is_empty() {
                return Some(format!(
                    "the ability set of the moderated document type \"{document_type_name}\" \
                     is empty"
                ));
            }
            for ability in abilities {
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
                        if !document_schema_lets_moderators_delete(schema) =>
                    {
                        Some("document deletions, but the type can not be deleted by moderators")
                    }
                    _ => None,
                };
                if let Some(unbacked) = unbacked {
                    return Some(format!(
                        "the moderated document type \"{document_type_name}\" allows {unbacked}"
                    ));
                }
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
        )?;
        if let Some(delay) = self.election_delay {
            write!(
                f,
                ", its first election open {delay} seconds after the contract's creation"
            )?;
        }
        Ok(())
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
            moderated_document_types: BTreeMap::from([(
                "post".to_string(),
                moderated(&[ModerationAbility::Ban, ModerationAbility::Suspend]),
            )]),
            interim: InterimModerators::ContractOwner,
            election_delay: None,
            owner_protected: false,
        }
    }

    /// The ability set of a moderated type
    fn moderated(abilities: &[ModerationAbility]) -> BTreeSet<ModerationAbility> {
        abilities.iter().copied().collect()
    }

    /// The ability set of a moderated type, to edit
    fn abilities_of<'a>(
        declaration: &'a mut ElectedModerators,
        name: &str,
    ) -> &'a mut BTreeSet<ModerationAbility> {
        declaration
            .moderated_document_types
            .get_mut(name)
            .expect("the type is moderated")
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
            .insert("comment".to_string(), moderated(&[ModerationAbility::Ban]));
        assert!(refusal(&config(unknown))
            .expect("refused")
            .contains("\"comment\" is not a document type"));

        // A moderated type need not be deletable by moderators.
        let mut not_deletable = elected();
        not_deletable.moderated_document_types =
            BTreeMap::from([("like".to_string(), moderated(&[ModerationAbility::Ban]))]);
        assert_eq!(refusal(&config(not_deletable)), None);
    }

    #[test]
    fn should_require_a_non_empty_envelope_the_contract_can_back() {
        let mut empty = elected();
        abilities_of(&mut empty, "post").clear();
        assert!(refusal(&config(empty))
            .expect("refused")
            .contains("ability set of the moderated document type \"post\" is empty"));

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
        abilities_of(&mut warnings, "post").insert(ModerationAbility::Warn);
        let mut warnings_without_list = config(warnings);
        assert!(refusal(&warnings_without_list)
            .expect("refused")
            .contains("keeps no warning list"));
        warnings_without_list.warnings = true;
        assert_eq!(refusal(&warnings_without_list), None);

        // Deletions on `post` are backed by its flag; on `like` they are not.
        let mut deletions = elected();
        *abilities_of(&mut deletions, "post") =
            BTreeSet::from([ModerationAbility::DeleteDocuments]);
        assert_eq!(refusal(&config(deletions)), None);
        let mut deletions_on_like = elected();
        deletions_on_like.moderated_document_types.insert(
            "like".to_string(),
            moderated(&[ModerationAbility::DeleteDocuments]),
        );
        assert!(refusal(&config(deletions_on_like))
            .expect("refused")
            .contains("\"like\" allows document deletions, but the type can not be deleted"));
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
        declaration.owner_protected = true;
        declaration.election_delay = Some(86_400);
        let moderators = ContractModerators::Elected(Box::new(declaration));

        let json = serde_json::to_value(&moderators).expect("serialize");
        assert_eq!(json["$type"], "elected");
        assert_eq!(json["joinWindow"], 604_800);
        assert_eq!(json["electionDelay"], 86_400);
        assert_eq!(
            json["moderatedDocumentTypes"],
            serde_json::json!({ "post": ["ban", "suspend"] })
        );
        assert_eq!(json["interim"]["$type"], "appointedModerators");
        assert_eq!(
            json["interim"]["identities"].as_array().map(|a| a.len()),
            Some(2)
        );
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
            "moderatedDocumentTypes": { "post": ["ban"] },
            "interim": { "$type": "notYetUsable" },
        });
        let parsed: ContractModerators = serde_json::from_value(minimal).expect("deserialize");
        let elected = parsed.elected().expect("elected");
        assert_eq!(elected.join_window, DEFAULT_ELECTION_WINDOW_SECONDS);
        assert_eq!(elected.election_delay, None);
        let json = serde_json::to_value(&parsed).expect("serialize");
        assert!(
            json.get("electionDelay").is_none(),
            "a declaration without a delay serializes none: {json}"
        );
        assert_eq!(elected.vote_window, DEFAULT_ELECTION_WINDOW_SECONDS);
        assert!(!elected.owner_protected);
        assert_eq!(elected.interim, InterimModerators::NotYetUsable);
        let no_moderation: InterimModerators =
            serde_json::from_value(serde_json::json!({ "$type": "noModeration" }))
                .expect("deserialize");
        assert_eq!(no_moderation, InterimModerators::NoModeration);
        assert_eq!(
            serde_json::to_value(&no_moderation).expect("serialize"),
            serde_json::json!({ "$type": "noModeration" })
        );

        let refused = [
            // The cool-down has no default.
            serde_json::json!({
                "$type": "elected",
                "moderatedDocumentTypes": { "post": ["ban"] },
                "interim": { "$type": "contractOwner" },
            }),
            // A misspelled key is refused, not dropped.
            serde_json::json!({
                "$type": "elected",
                "challengeCoolDown": 1_209_600,
                "moderatedDocumentTypes": { "post": ["ban"] },
                "interim": { "$type": "contractOwner" },
                "ownerProtcted": true,
            }),
            // So is one inside the interim, and an unknown interim kind.
            serde_json::json!({
                "$type": "elected",
                "challengeCoolDown": 1_209_600,
                "moderatedDocumentTypes": { "post": ["ban"] },
                "interim": { "$type": "contractOwner", "identity": [] },
            }),
            serde_json::json!({
                "$type": "elected",
                "challengeCoolDown": 1_209_600,
                "moderatedDocumentTypes": { "post": ["ban"] },
                "interim": { "$type": "seatedTeam" },
            }),
            // An unknown ability.
            serde_json::json!({
                "$type": "elected",
                "challengeCoolDown": 1_209_600,
                "moderatedDocumentTypes": { "post": ["silence"] },
                "interim": { "$type": "contractOwner" },
            }),
            // A charter does not price actions: fee maximums are no key of the declaration.
            serde_json::json!({
                "$type": "elected",
                "challengeCoolDown": 1_209_600,
                "moderatedDocumentTypes": { "post": ["ban"] },
                "moderatorsActionFeeMaximums": { "post": { "create": 1 } },
                "interim": { "$type": "contractOwner" },
            }),
            // The abilities live under each moderated type, not beside them.
            serde_json::json!({
                "$type": "elected",
                "challengeCoolDown": 1_209_600,
                "moderatedDocumentTypes": ["post"],
                "abilities": ["ban"],
                "interim": { "$type": "contractOwner" },
            }),
            // The elected keys under another kind, and `identities` under elected.
            serde_json::json!({ "$type": "contractOwner", "ownerProtected": true }),
            serde_json::json!({
                "$type": "elected",
                "challengeCoolDown": 1_209_600,
                "moderatedDocumentTypes": { "post": ["ban"] },
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
    fn should_open_the_election_after_the_delay_from_the_contract_creation() {
        let created_at: TimestampMillis = 1_700_000_000_000;
        let mut declaration = elected();

        // No delay: open at once, whether or not the creation time is recorded
        assert!(declaration.election_is_open(Some(created_at), created_at));
        assert!(declaration.election_is_open(None, 0));

        declaration.election_delay = Some(3600);
        assert!(!declaration.election_is_open(Some(created_at), created_at + 3_599_999));
        assert!(declaration.election_is_open(Some(created_at), created_at + 3_600_000));
        assert!(declaration.election_is_open(Some(created_at), TimestampMillis::MAX));
        // A delay on a contract of unknown age never opens
        assert!(!declaration.election_is_open(None, TimestampMillis::MAX));
        // The bound saturates rather than wrapping around into the past
        declaration.election_delay = Some(u32::MAX);
        assert!(
            !declaration.election_is_open(Some(TimestampMillis::MAX - 1), TimestampMillis::MAX - 1)
        );
    }

    #[test]
    fn should_describe_itself() {
        let mut declaration = elected();
        assert_eq!(
            ContractModerators::Elected(Box::new(declaration.clone())).to_string(),
            "an elected moderation team, in its interim moderated by the contract owner"
        );
        declaration.election_delay = Some(86_400);
        assert_eq!(
            declaration.to_string(),
            "an elected moderation team, in its interim moderated by the contract owner, its \
             first election open 86400 seconds after the contract's creation"
        );
        declaration.election_delay = None;
        declaration.interim = InterimModerators::NotYetUsable;
        assert_eq!(
            declaration.to_string(),
            "an elected moderation team, in its interim moderated by nobody, its moderated \
             document types not yet usable"
        );
        declaration.interim = InterimModerators::NoModeration;
        assert_eq!(
            declaration.to_string(),
            "an elected moderation team, in its interim moderated by nobody, its moderated \
             document types unmoderated meanwhile"
        );
        assert_eq!(
            ModerationAbility::DeleteDocuments.to_string(),
            "deleteDocuments"
        );
    }

    #[test]
    fn should_leave_the_moderated_types_usable_and_unmoderated_under_no_moderation() {
        let mut declaration = elected();
        declaration.interim = InterimModerators::NoModeration;
        let owner = Identifier::from([9u8; 32]);
        assert!(!declaration.interim_blocks_document_type("post"));
        assert!(!declaration.interim.may_moderate(&owner, &owner));
        assert!(declaration.interim.team(&owner).is_empty());
        assert_eq!(declaration.interim.identity_ids(), None);
        assert!(declaration.allows("post", ModerationAbility::Ban));
        assert!(!declaration.allows("post", ModerationAbility::Warn));
        assert!(!declaration.allows("like", ModerationAbility::Ban));
        assert_eq!(refusal(&config(declaration)), None);
    }
}
