//! Contract moderation: the declaration, inside a data contract's config, that the contract
//! keeps a banlist, a suspension list and/or a warning list of identities, and who may edit
//! them.
//!
//! An identity on the banlist, or on the suspension list with a suspension that has not lapsed,
//! cannot act on the contract at the document level: every document transition it signs against
//! the contract is refused. Token transitions are not affected. A warning bars nothing: it is
//! a record, with a reason and a time, that the identity and everyone else can read, and that
//! accumulates until a moderator clears it. The lists live under the contract's own subtree in
//! Drive (keys `128`, `192` and `224` of its other tree, `[64, id, 2]`) and are edited by the
//! `ContractUserModeration` state transition.
//!
//! The same moderators may delete the documents of the document types that say so
//! (`canBeDeletedByModerators`), with the same transition. Each removal leaves a
//! [`ContractDocumentRemoval`] under the contract (key `16` of its other tree).
//!
//! A contract may instead declare that its moderators are an [elected team](elected): until
//! one is seated, the interim moderators the declaration names moderate as the merged kinds
//! do, or nobody does and the moderated document types wait.

use crate::consensus::basic::contract_moderation::InvalidContractModerationConfigError;
use crate::data_contract::document_type::property_names::CAN_BE_DELETED_BY_MODERATORS;
use crate::data_contract::DocumentName;
use crate::identity::TimestampMillis;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonSafeFields;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::{Identifier, Value};
use platform_version::version::PlatformVersion;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

mod document_removal;
pub mod elected;
mod reason;
pub use document_removal::{ContractDocumentRemoval, ContractDocumentRestoration};
pub use elected::{
    ElectedModerators, InterimModerators, ModerationAbility, DEFAULT_ELECTION_WINDOW_SECONDS,
};
pub use reason::{ContractModerationDocument, ContractModerationReason};

/// Whether a raw document type schema sets `canBeDeletedByModerators: true`.
pub fn document_schema_lets_moderators_delete(schema: &Value) -> bool {
    schema
        .get_optional_bool(CAN_BE_DELETED_BY_MODERATORS)
        .ok()
        .flatten()
        .unwrap_or(false)
}

/// Who may send a `ContractUserModeration` transition for the contract.
///
/// With the first two kinds the contract owner always may, named or not; a moderator set is
/// fixed in the config and changed only by a contract update. With the third the moderators
/// are an elected team, and until one is seated the interim moderators the declaration
/// names; the declaration is fixed at creation and never changes.
#[derive(Debug, Clone, PartialEq, Eq, Default, Encode, Decode, DecodeUntrusted)]
pub enum ContractModerators {
    /// Only the contract owner moderates.
    #[default]
    ContractOwner,
    /// The moderators the contract appoints, beside its owner, who always may moderate.
    /// Non-empty, at most `SystemLimits::max_contract_moderators`, each an identity that
    /// exists. The owner may be appointed too, and then counts toward that limit; appointing
    /// it changes nothing about who may moderate.
    AppointedModerators(BTreeSet<Identifier>),
    /// A team elected by masternodes moderates, once one is seated; until then the interim
    /// moderators of the declaration do. Declarable only when the contract is created, and
    /// never left or changed by an update. Boxed: the declaration is the largest kind by
    /// far, and a contract's config is embedded by value wherever a contract is.
    Elected(Box<ElectedModerators>),
}

impl ContractModerators {
    /// The identities the kind names, the owner among them only when it is named: the
    /// appointed set, or the appointed interim set of an elected declaration. `None` when
    /// nobody is named.
    pub fn identity_ids(&self) -> Option<&BTreeSet<Identifier>> {
        match self {
            ContractModerators::ContractOwner => None,
            ContractModerators::AppointedModerators(ids) => Some(ids),
            ContractModerators::Elected(elected) => elected.interim.identity_ids(),
        }
    }

    /// Whether `identity_id` is one of the identities the kind names.
    pub fn names(&self, identity_id: &Identifier) -> bool {
        self.identity_ids()
            .is_some_and(|ids| ids.contains(identity_id))
    }

    /// The elected declaration, `None` for the merged kinds.
    pub fn elected(&self) -> Option<&ElectedModerators> {
        match self {
            ContractModerators::Elected(elected) => Some(elected.as_ref()),
            ContractModerators::ContractOwner | ContractModerators::AppointedModerators(_) => None,
        }
    }

    /// Whether `identity_id` may moderate a contract owned by `owner_id`. Under an elected
    /// declaration, whether it may during the interim: the owner alone, the owner and the
    /// appointed interim set, or nobody, the moderated types unusable or unmoderated meanwhile.
    ///
    /// Once a charter is seated on an elected contract its team moderates instead, and the
    /// interim moderators no longer may. Who is on the team is read from the moderation
    /// charters contract, so that is decided where state is read, not here.
    pub fn may_moderate(&self, owner_id: &Identifier, identity_id: &Identifier) -> bool {
        match self {
            ContractModerators::ContractOwner | ContractModerators::AppointedModerators(_) => {
                owner_id == identity_id || self.names(identity_id)
            }
            ContractModerators::Elected(elected) => {
                elected.interim.may_moderate(owner_id, identity_id)
            }
        }
    }

    /// Whether `identity_id` is protected from moderation on a contract owned by `owner_id`:
    /// it can be neither banned nor suspended, and its documents can not be deleted. Whoever
    /// may moderate is, and so is the owner of an elected contract whose declaration says so.
    /// Under an elected declaration this is the interim's protection; once a charter is
    /// seated, the leader and the active members of its team are protected instead, with the
    /// owner when the declaration says so, as state says.
    pub fn protects(&self, owner_id: &Identifier, identity_id: &Identifier) -> bool {
        self.may_moderate(owner_id, identity_id)
            || (owner_id == identity_id
                && self
                    .elected()
                    .is_some_and(|elected| elected.owner_protected))
    }

    /// Whether every document transition of the document type is refused while no team is
    /// seated: an elected declaration in its interim with nobody moderating blocks its
    /// moderated types until one is. Whether one is, is state's to say.
    pub fn interim_blocks_document_type(&self, document_type_name: &str) -> bool {
        self.elected()
            .is_some_and(|elected| elected.interim_blocks_document_type(document_type_name))
    }

    /// The moderation team of a contract owned by `owner_id`: the identities that share its
    /// moderators fee pot. It is the set the contract appoints, the owner among them only
    /// when appointed, and the owner alone when nobody is appointed. Under an elected
    /// declaration it is the interim's team: the same by kind, and nobody while the
    /// moderated types are not yet usable, so that the pot accumulates for the team to come.
    ///
    /// The team is about earnings, not authority: an owner who is not on it still may
    /// moderate ([`Self::may_moderate`]).
    ///
    /// Like [`Self::may_moderate`] and [`Self::protects`], this reads the config alone, so for
    /// an elected contract it describes the interim only. Once a charter is seated its claim of
    /// the moderators pot is refused and its moderations too; who is on the seated team is read
    /// from the moderation charters contract, which a client asks rather than this.
    pub fn team(&self, owner_id: &Identifier) -> BTreeSet<Identifier> {
        match self {
            ContractModerators::ContractOwner => BTreeSet::from([*owner_id]),
            ContractModerators::AppointedModerators(ids) => ids.clone(),
            ContractModerators::Elected(elected) => elected.interim.team(owner_id),
        }
    }
}

// The wire shape is a flat `{"$type": "contractOwner"}`,
// `{"$type": "appointedModerators", "identities": [...]}` or `{"$type": "elected", ...}` map
// with the declaration's keys beside its `$type`, the style of `AuthorizedActionTakers`.
// Bincode is untouched.
impl Serialize for ContractModerators {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use elected::property_names as elected_names;
        use serde::ser::SerializeMap;
        match self {
            ContractModerators::ContractOwner => {
                let mut m = serializer.serialize_map(Some(1))?;
                m.serialize_entry("$type", "contractOwner")?;
                m.end()
            }
            ContractModerators::AppointedModerators(ids) => {
                let mut m = serializer.serialize_map(Some(2))?;
                m.serialize_entry("$type", "appointedModerators")?;
                m.serialize_entry("identities", ids)?;
                m.end()
            }
            ContractModerators::Elected(elected) => {
                let entries = 7
                    + usize::from(elected.challenge_cool_down.is_some())
                    + usize::from(elected.election_delay.is_some())
                    + usize::from(elected.max_added_moderators > 0);
                let mut m = serializer.serialize_map(Some(entries))?;
                m.serialize_entry("$type", "elected")?;
                m.serialize_entry(elected_names::JOIN_WINDOW, &elected.join_window)?;
                m.serialize_entry(elected_names::VOTE_WINDOW, &elected.vote_window)?;
                m.serialize_entry(elected_names::SEAT_CONTESTABLE, &elected.seat_contestable())?;
                // Only a contestable seat has a cool-down
                if let Some(cool_down) = elected.challenge_cool_down {
                    m.serialize_entry(elected_names::CHALLENGE_COOL_DOWN, &cool_down)?;
                }
                // Absent, not null, when the declaration has no delay: the wire form of a
                // declaration that left it out is unchanged
                if let Some(delay) = elected.election_delay {
                    m.serialize_entry(elected_names::ELECTION_DELAY, &delay)?;
                }
                // Absent when no member may be added, for the same reason
                if elected.max_added_moderators > 0 {
                    m.serialize_entry(
                        elected_names::MAX_ADDED_MODERATORS,
                        &elected.max_added_moderators,
                    )?;
                }
                m.serialize_entry(
                    elected_names::MODERATED_DOCUMENT_TYPES,
                    &elected.moderated_document_types,
                )?;
                m.serialize_entry(elected_names::INTERIM, &elected.interim)?;
                m.serialize_entry(elected_names::OWNER_PROTECTED, &elected.owner_protected)?;
                m.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for ContractModerators {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use elected::property_names as elected_names;
        use serde::de::{self, MapAccess, Visitor};

        const KEYS: &[&str] = &[
            "$type",
            "identities",
            elected_names::JOIN_WINDOW,
            elected_names::VOTE_WINDOW,
            elected_names::SEAT_CONTESTABLE,
            elected_names::CHALLENGE_COOL_DOWN,
            elected_names::ELECTION_DELAY,
            elected_names::MAX_ADDED_MODERATORS,
            elected_names::MODERATED_DOCUMENT_TYPES,
            elected_names::INTERIM,
            elected_names::OWNER_PROTECTED,
        ];

        /// The keys of an elected declaration, each read at most once.
        #[derive(Default)]
        struct ElectedKeys {
            join_window: Option<u32>,
            vote_window: Option<u32>,
            seat_contestable: Option<bool>,
            challenge_cool_down: Option<u32>,
            election_delay: Option<u32>,
            max_added_moderators: Option<u16>,
            moderated_document_types: Option<BTreeMap<DocumentName, BTreeSet<ModerationAbility>>>,
            interim: Option<InterimModerators>,
            owner_protected: Option<bool>,
        }

        impl ElectedKeys {
            fn any(&self) -> bool {
                self.join_window.is_some()
                    || self.vote_window.is_some()
                    || self.seat_contestable.is_some()
                    || self.challenge_cool_down.is_some()
                    || self.election_delay.is_some()
                    || self.max_added_moderators.is_some()
                    || self.moderated_document_types.is_some()
                    || self.interim.is_some()
                    || self.owner_protected.is_some()
            }

            /// The cool-down of the seat, `None` for a seat that can not be contested.
            /// `seatContestable` is required with no default: false would make every team
            /// permanent, true would opt every contract into challenges unasked. The
            /// cool-down comes with a contestable seat and only with one.
            fn challenge_cool_down<E: de::Error>(&self) -> Result<Option<u32>, E> {
                match (self.seat_contestable, self.challenge_cool_down) {
                    (None, _) => Err(E::missing_field(elected_names::SEAT_CONTESTABLE)),
                    (Some(true), None) => Err(E::missing_field(elected_names::CHALLENGE_COOL_DOWN)),
                    (Some(false), Some(_)) => Err(E::custom(
                        "`challengeCoolDown` is only valid with `seatContestable: true`: a seat \
                         that can not be contested has no cool-down",
                    )),
                    (Some(_), cool_down) => Ok(cool_down),
                }
            }
        }

        /// Reads the value of `key` into `slot`, refusing a second occurrence.
        fn read_once<'de, A: MapAccess<'de>, T: Deserialize<'de>>(
            map: &mut A,
            key: &'static str,
            slot: &mut Option<T>,
        ) -> Result<(), A::Error> {
            if slot.is_some() {
                return Err(de::Error::duplicate_field(key));
            }
            *slot = Some(map.next_value()?);
            Ok(())
        }

        struct V;

        impl<'de> Visitor<'de> for V {
            type Value = ContractModerators;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(
                    "ContractModerators as a map with a `$type` discriminator, \
                     e.g. {\"$type\": \"contractOwner\"}, \
                     {\"$type\": \"appointedModerators\", \"identities\": [\"<base58>\"]} or \
                     {\"$type\": \"elected\", \"seatContestable\": true, \
                     \"challengeCoolDown\": 1209600, \
                     \"moderatedDocumentTypes\": {\"post\": [\"ban\"]}, \
                     \"interim\": {\"$type\": \"contractOwner\"}}",
                )
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut variant: Option<String> = None;
                let mut identities: Option<BTreeSet<Identifier>> = None;
                let mut elected = ElectedKeys::default();

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "$type" => read_once(&mut map, "$type", &mut variant)?,
                        "identities" => read_once(&mut map, "identities", &mut identities)?,
                        elected_names::JOIN_WINDOW => read_once(
                            &mut map,
                            elected_names::JOIN_WINDOW,
                            &mut elected.join_window,
                        )?,
                        elected_names::VOTE_WINDOW => read_once(
                            &mut map,
                            elected_names::VOTE_WINDOW,
                            &mut elected.vote_window,
                        )?,
                        elected_names::SEAT_CONTESTABLE => read_once(
                            &mut map,
                            elected_names::SEAT_CONTESTABLE,
                            &mut elected.seat_contestable,
                        )?,
                        elected_names::CHALLENGE_COOL_DOWN => read_once(
                            &mut map,
                            elected_names::CHALLENGE_COOL_DOWN,
                            &mut elected.challenge_cool_down,
                        )?,
                        elected_names::ELECTION_DELAY => read_once(
                            &mut map,
                            elected_names::ELECTION_DELAY,
                            &mut elected.election_delay,
                        )?,
                        elected_names::MAX_ADDED_MODERATORS => read_once(
                            &mut map,
                            elected_names::MAX_ADDED_MODERATORS,
                            &mut elected.max_added_moderators,
                        )?,
                        elected_names::MODERATED_DOCUMENT_TYPES => read_once(
                            &mut map,
                            elected_names::MODERATED_DOCUMENT_TYPES,
                            &mut elected.moderated_document_types,
                        )?,
                        elected_names::INTERIM => {
                            read_once(&mut map, elected_names::INTERIM, &mut elected.interim)?
                        }
                        elected_names::OWNER_PROTECTED => read_once(
                            &mut map,
                            elected_names::OWNER_PROTECTED,
                            &mut elected.owner_protected,
                        )?,
                        // Refused rather than skipped: the declaration can hardly be changed
                        // after the contract is created, so a misspelled key must not pass.
                        other => return Err(de::Error::unknown_field(other, KEYS)),
                    }
                }

                let variant = variant.ok_or_else(|| de::Error::missing_field("$type"))?;
                if variant != "elected" && elected.any() {
                    return Err(de::Error::custom(
                        "the keys of an elected declaration are only valid for `elected`",
                    ));
                }
                match variant.as_str() {
                    "contractOwner" => {
                        if identities.is_some() {
                            return Err(de::Error::custom(
                                "`identities` is only valid for `appointedModerators`",
                            ));
                        }
                        Ok(ContractModerators::ContractOwner)
                    }
                    "appointedModerators" => {
                        let ids =
                            identities.ok_or_else(|| de::Error::missing_field("identities"))?;
                        Ok(ContractModerators::AppointedModerators(ids))
                    }
                    "elected" => {
                        if identities.is_some() {
                            return Err(de::Error::custom(
                                "`identities` is only valid for `appointedModerators`; an \
                                 elected declaration names its interim set under `interim`",
                            ));
                        }
                        let required = |key: &'static str| move || de::Error::missing_field(key);
                        Ok(ContractModerators::Elected(Box::new(ElectedModerators {
                            join_window: elected
                                .join_window
                                .unwrap_or(DEFAULT_ELECTION_WINDOW_SECONDS),
                            vote_window: elected
                                .vote_window
                                .unwrap_or(DEFAULT_ELECTION_WINDOW_SECONDS),
                            challenge_cool_down: elected.challenge_cool_down()?,
                            election_delay: elected.election_delay,
                            max_added_moderators: elected.max_added_moderators.unwrap_or(0),
                            moderated_document_types: elected
                                .moderated_document_types
                                .ok_or_else(required(elected_names::MODERATED_DOCUMENT_TYPES))?,
                            interim: elected
                                .interim
                                .ok_or_else(required(elected_names::INTERIM))?,
                            owner_protected: elected.owner_protected.unwrap_or(false),
                        })))
                    }
                    other => Err(de::Error::unknown_variant(
                        other,
                        &["contractOwner", "appointedModerators", "elected"],
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
            ContractModerators::AppointedModerators(ids) => {
                write!(f, "contract owner and {} appointed moderators", ids.len())
            }
            ContractModerators::Elected(elected) => elected.fmt(f),
        }
    }
}

/// Which of the moderation lists an action or a query refers to.
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
    /// The warning list: identities warned, and why, barred from nothing.
    Warnings,
}

impl ContractModerationList {
    /// Whether an entry on the list bars the identity from the contract's documents. A
    /// warning does not.
    pub fn bars(&self) -> bool {
        match self {
            ContractModerationList::Banlist | ContractModerationList::Suspensions => true,
            ContractModerationList::Warnings => false,
        }
    }
}

impl fmt::Display for ContractModerationList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContractModerationList::Banlist => write!(f, "banlist"),
            ContractModerationList::Suspensions => write!(f, "suspensions"),
            ContractModerationList::Warnings => write!(f, "warning list"),
        }
    }
}

/// The moderation a data contract declares in its config.
///
/// An unknown key is refused: which lists a contract keeps is fixed when it is created, so a
/// misspelled `suspensions` must not quietly leave the contract without the list for good.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContractModerationConfig {
    /// The contract keeps a banlist (Drive key `128` of the contract's other tree).
    #[serde(default)]
    pub banlist: bool,
    /// The contract keeps a suspension list (Drive key `192` of the contract's other tree).
    #[serde(default)]
    pub suspensions: bool,
    /// Who may edit the lists.
    #[serde(default)]
    pub moderators: ContractModerators,
    /// The contract keeps a warning list (Drive key `224` of the contract's other tree).
    /// Last in the bincode layout: it joined after the rest, and a declaration stored by a
    /// 4.2 beta without it fails to decode at its end rather than misreading the moderators.
    /// Such networks are reset.
    #[serde(default)]
    pub warnings: bool,
}

impl ContractModerationConfig {
    /// Whether the contract keeps `list`.
    pub fn keeps(&self, list: ContractModerationList) -> bool {
        match list {
            ContractModerationList::Banlist => self.banlist,
            ContractModerationList::Suspensions => self.suspensions,
            ContractModerationList::Warnings => self.warnings,
        }
    }

    /// The lists the contract keeps, in tree key order: the banlist, the suspension list, the
    /// warning list.
    pub fn lists(&self) -> impl Iterator<Item = ContractModerationList> + '_ {
        [
            ContractModerationList::Banlist,
            ContractModerationList::Suspensions,
            ContractModerationList::Warnings,
        ]
        .into_iter()
        .filter(|list| self.keeps(*list))
    }

    /// The lists the contract keeps whose entries bar an identity from its documents: the
    /// banlist and the suspension list, never the warning list. What the document gate reads,
    /// and what a ban's proof covers.
    pub fn barring_lists(&self) -> impl Iterator<Item = ContractModerationList> + '_ {
        self.lists().filter(ContractModerationList::bars)
    }

    /// Whether `identity_id` may moderate a contract owned by `owner_id`. Whoever may moderate
    /// cannot be put on a list either, though an entry it already carries may be removed.
    pub fn may_moderate(&self, owner_id: &Identifier, identity_id: &Identifier) -> bool {
        self.moderators.may_moderate(owner_id, identity_id)
    }

    /// Whether `identity_id` is protected from moderation on a contract owned by `owner_id`.
    /// See [`ContractModerators::protects`].
    pub fn protects(&self, owner_id: &Identifier, identity_id: &Identifier) -> bool {
        self.moderators.protects(owner_id, identity_id)
    }

    /// Whether every document transition of the document type is refused until a moderation
    /// team is seated. See [`ContractModerators::interim_blocks_document_type`].
    pub fn interim_blocks_document_type(&self, document_type_name: &str) -> bool {
        self.moderators
            .interim_blocks_document_type(document_type_name)
    }

    /// The moderation team of a contract owned by `owner_id`: the identities that share its
    /// moderators fee pot. See [`ContractModerators::team`].
    pub fn team(&self, owner_id: &Identifier) -> BTreeSet<Identifier> {
        self.moderators.team(owner_id)
    }

    /// The pure-data rules of the declaration: it gives the moderators something to do, and a
    /// moderator set is non-empty and within `SystemLimits::max_contract_moderators` (a named
    /// owner counts). Something to do is a list to edit or, failing that, a document type whose
    /// documents they may delete, read from the raw `document_schemas` of the contract the
    /// declaration belongs to, which an elected declaration is also checked against: its
    /// moderated types must name document types of the contract
    /// ([`ElectedModerators::validation_error`] has its rules).
    /// Whether the named identities exist is state validation, done by the contract create
    /// and update transitions: a moderator that does not exist can never sign, so naming one
    /// is a mistake, caught where it is cheapest.
    pub fn validate(
        &self,
        document_schemas: &BTreeMap<DocumentName, Value>,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .methods
            .validate_moderation_config
        {
            0 => Ok(self.validate_v0(document_schemas, platform_version)),
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
        document_schemas: &BTreeMap<DocumentName, Value>,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        let has_document_type_deletable_by_moderators = document_schemas
            .values()
            .any(document_schema_lets_moderators_delete);
        if !self.banlist
            && !self.suspensions
            && !self.warnings
            && !has_document_type_deletable_by_moderators
        {
            return SimpleConsensusValidationResult::new_with_error(
                InvalidContractModerationConfigError::new(
                    "moderation declares neither a banlist, a suspension list nor a warning \
                     list, and no document type can be deleted by moderators"
                        .to_string(),
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
        }
        if let Some(reason) = self
            .moderators
            .elected()
            .and_then(|elected| elected.validation_error(self, document_schemas, platform_version))
        {
            return SimpleConsensusValidationResult::new_with_error(
                InvalidContractModerationConfigError::new(format!("elected moderation: {reason}"))
                    .into(),
            );
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

/// A banlist entry.
#[derive(
    Debug, Clone, PartialEq, Eq, Default, Encode, Decode, DecodeUntrusted, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct ContractBan {
    /// Why the moderator banned the identity.
    pub reason: ContractModerationReason,
}

/// A suspension list entry.
#[derive(
    Debug, Clone, PartialEq, Eq, Default, Encode, Decode, DecodeUntrusted, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct ContractSuspension {
    /// The block time, in milliseconds, at which the suspension lapses.
    pub until: TimestampMillis,
    /// Why the moderator suspended the identity.
    pub reason: ContractModerationReason,
}

/// One warning of a warning list entry.
#[derive(
    Debug, Clone, PartialEq, Eq, Default, Encode, Decode, DecodeUntrusted, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct ContractWarning {
    /// The time of the block that issued the warning, in milliseconds.
    pub warned_at: TimestampMillis,
    /// Why the moderator warned the identity.
    pub reason: ContractModerationReason,
}

/// What a contract's moderation lists say about one identity.
#[derive(
    Debug, Clone, PartialEq, Eq, Default, Encode, Decode, DecodeUntrusted, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct ContractModerationStatus {
    /// The identity's banlist entry, `None` when it is not banned.
    pub ban: Option<ContractBan>,
    /// The identity's suspension list entry, `None` when it is not suspended. A lapsed
    /// suspension (at or before the block time) still appears here until it is swept.
    pub suspension: Option<ContractSuspension>,
    /// The identity's warnings, oldest first; empty when it carries none. They stay until a
    /// moderator clears them, and bar nothing.
    #[serde(default)]
    pub warnings: Vec<ContractWarning>,
}

impl ContractModerationStatus {
    /// Whether the identity is on the banlist.
    pub fn banned(&self) -> bool {
        self.ban.is_some()
    }

    /// Whether the identity carries at least one warning.
    pub fn warned(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// The block time, in milliseconds, until which the identity is suspended, lapsed or not.
    pub fn suspended_until(&self) -> Option<TimestampMillis> {
        self.suspension.as_ref().map(|suspension| suspension.until)
    }

    /// Whether the identity is barred from acting on the contract at the document level at
    /// `block_time_ms`: it is banned, or under a suspension that has not lapsed.
    pub fn is_barred_at(&self, block_time_ms: TimestampMillis) -> bool {
        self.banned()
            || self
                .suspended_until()
                .is_some_and(|until| until > block_time_ms)
    }

    /// Whether the identity carries a suspension that has lapsed at `block_time_ms`.
    pub fn has_lapsed_suspension_at(&self, block_time_ms: TimestampMillis) -> bool {
        self.suspended_until()
            .is_some_and(|until| until <= block_time_ms)
    }
}

/// What one of a contract's moderation lists says about one identity, and nothing about the
/// other list. It is what the proof of a moderation transition's execution shows: that proof
/// holds the edited entry only, so the other list stays unknown rather than being reported as
/// empty (an identity unsuspended a moment ago may well be banned).
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
#[serde(tag = "list", rename_all = "camelCase")]
pub enum ContractModerationListStatus {
    /// The banlist entry
    #[serde(rename_all = "camelCase")]
    Banlist {
        /// The identity's banlist entry, `None` when it is not banned.
        ban: Option<ContractBan>,
    },
    /// The suspension list entry
    #[serde(rename_all = "camelCase")]
    Suspensions {
        /// The identity's suspension list entry, `None` when it is not suspended. A lapsed
        /// suspension still appears here until it is swept.
        suspension: Option<ContractSuspension>,
    },
    /// The warning list entry
    #[serde(rename_all = "camelCase")]
    Warnings {
        /// The identity's warnings, oldest first; empty when it carries none.
        warnings: Vec<ContractWarning>,
    },
}

impl ContractModerationListStatus {
    /// The part of a full `status` that `list` holds.
    pub fn from_status(list: ContractModerationList, status: &ContractModerationStatus) -> Self {
        match list {
            ContractModerationList::Banlist => Self::Banlist {
                ban: status.ban.clone(),
            },
            ContractModerationList::Suspensions => Self::Suspensions {
                suspension: status.suspension.clone(),
            },
            ContractModerationList::Warnings => Self::Warnings {
                warnings: status.warnings.clone(),
            },
        }
    }

    /// The list this status was read from.
    pub fn list(&self) -> ContractModerationList {
        match self {
            Self::Banlist { .. } => ContractModerationList::Banlist,
            Self::Suspensions { .. } => ContractModerationList::Suspensions,
            Self::Warnings { .. } => ContractModerationList::Warnings,
        }
    }
}

/// One identity's status on the lists that were read, one entry per list, in the order read:
/// what a status query answers for the lists it names, and what the proof of a moderation
/// transition's execution shows. A list the query did not name is absent, not empty: an identity that is not
/// suspended may still be banned when the banlist was not read. Query every list the contract
/// keeps for the whole picture.
#[derive(Debug, Clone, PartialEq, Eq, Default, Encode, Decode, Serialize, Deserialize)]
pub struct ContractModerationListStatuses(pub Vec<ContractModerationListStatus>);

impl ContractModerationListStatuses {
    /// The part of `status` that `lists` cover.
    pub fn from_status(
        lists: &[ContractModerationList],
        status: &ContractModerationStatus,
    ) -> Self {
        Self(
            lists
                .iter()
                .map(|list| ContractModerationListStatus::from_status(*list, status))
                .collect(),
        )
    }

    /// The identity's banlist entry (`Some(None)`: not banned), `None` when the banlist was
    /// not queried.
    pub fn ban(&self) -> Option<Option<&ContractBan>> {
        self.0.iter().find_map(|status| match status {
            ContractModerationListStatus::Banlist { ban } => Some(ban.as_ref()),
            ContractModerationListStatus::Suspensions { .. }
            | ContractModerationListStatus::Warnings { .. } => None,
        })
    }

    /// The identity's suspension list entry (`Some(None)`: not suspended), `None` when the
    /// suspension list was not queried.
    pub fn suspension(&self) -> Option<Option<&ContractSuspension>> {
        self.0.iter().find_map(|status| match status {
            ContractModerationListStatus::Suspensions { suspension } => Some(suspension.as_ref()),
            ContractModerationListStatus::Banlist { .. }
            | ContractModerationListStatus::Warnings { .. } => None,
        })
    }

    /// The identity's warnings, oldest first (`Some(&[])`: none), `None` when the warning list
    /// was not queried.
    pub fn warnings(&self) -> Option<&[ContractWarning]> {
        self.0.iter().find_map(|status| match status {
            ContractModerationListStatus::Warnings { warnings } => Some(warnings.as_slice()),
            ContractModerationListStatus::Banlist { .. }
            | ContractModerationListStatus::Suspensions { .. } => None,
        })
    }

    /// Whether the identity is banned, `None` when the banlist was not queried.
    pub fn banned(&self) -> Option<bool> {
        self.ban().map(|ban| ban.is_some())
    }

    /// Until when the identity is suspended (`Some(None)`: not suspended), `None` when the
    /// suspension list was not queried.
    pub fn suspended_until(&self) -> Option<Option<TimestampMillis>> {
        self.suspension()
            .map(|suspension| suspension.map(|suspension| suspension.until))
    }

    /// Whether one of the lists queried bars the identity at `block_time_ms`. `false` says
    /// nothing about a list that was not queried.
    pub fn is_barred_on_queried_lists_at(&self, block_time_ms: TimestampMillis) -> bool {
        self.banned() == Some(true)
            || self
                .suspended_until()
                .flatten()
                .is_some_and(|until| until > block_time_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_value::platform_value;

    fn set(ids: &[u8]) -> BTreeSet<Identifier> {
        ids.iter().map(|b| Identifier::from([*b; 32])).collect()
    }

    /// One document type, `post`, whose documents moderators may delete
    fn schemas_with_a_deletable_type() -> BTreeMap<DocumentName, Value> {
        BTreeMap::from([(
            "post".to_string(),
            platform_value!({ "type": "object", "canBeDeletedByModerators": true }),
        )])
    }

    #[test]
    fn should_round_trip_moderators_through_json() {
        for moderators in [
            ContractModerators::ContractOwner,
            ContractModerators::AppointedModerators(set(&[1, 2])),
        ] {
            let json = serde_json::to_value(&moderators).expect("serialize");
            let back: ContractModerators = serde_json::from_value(json).expect("deserialize");
            assert_eq!(moderators, back);
        }
        let json = serde_json::to_value(ContractModerators::AppointedModerators(set(&[1])))
            .expect("serialize");
        assert_eq!(json["$type"], "appointedModerators");
        assert_eq!(json["identities"].as_array().map(|a| a.len()), Some(1));
    }

    #[test]
    fn should_refuse_a_misspelled_key_instead_of_dropping_it() {
        // `suspension` for `suspensions`: dropped, it would leave the contract without the
        // list for good.
        let misspelled_list = serde_json::json!({ "banlist": true, "suspension": true });
        assert!(serde_json::from_value::<ContractModerationConfig>(misspelled_list).is_err());

        let misspelled_moderators = serde_json::json!({
            "banlist": true,
            "moderators": { "$type": "appointedModerators", "identity": [] },
        });
        assert!(serde_json::from_value::<ContractModerationConfig>(misspelled_moderators).is_err());

        let identities_under_the_owner = serde_json::json!({
            "$type": "contractOwner",
            "identities": [],
        });
        assert!(serde_json::from_value::<ContractModerators>(identities_under_the_owner).is_err());

        let well_formed = serde_json::json!({ "banlist": true, "suspensions": true });
        let config: ContractModerationConfig =
            serde_json::from_value(well_formed).expect("deserialize");
        assert!(config.banlist && config.suspensions);
    }

    #[test]
    fn should_reject_a_config_with_no_list() {
        let config = ContractModerationConfig {
            banlist: false,
            suspensions: false,
            warnings: false,
            moderators: ContractModerators::ContractOwner,
        };
        let result = config
            .validate(&BTreeMap::new(), PlatformVersion::latest())
            .expect("validate");
        assert!(!result.is_valid());
    }

    #[test]
    fn should_accept_a_config_with_no_list_when_a_document_type_can_be_deleted_by_moderators() {
        let config = ContractModerationConfig {
            banlist: false,
            suspensions: false,
            warnings: false,
            moderators: ContractModerators::ContractOwner,
        };
        let result = config
            .validate(&schemas_with_a_deletable_type(), PlatformVersion::latest())
            .expect("validate");
        assert!(result.is_valid(), "{:?}", result.errors);
        assert_eq!(config.lists().count(), 0);
    }

    #[test]
    fn should_accept_the_owner_among_the_moderators() {
        let owner = Identifier::from([9; 32]);
        let config = ContractModerationConfig {
            banlist: true,
            suspensions: false,
            warnings: false,
            moderators: ContractModerators::AppointedModerators(set(&[9, 1])),
        };
        let result = config
            .validate(&BTreeMap::new(), PlatformVersion::latest())
            .expect("validate");
        assert!(result.is_valid(), "{:?}", result.errors);
        // Naming the owner changes nothing about who may moderate or who is protected.
        assert!(config.may_moderate(&owner, &owner));
    }

    #[test]
    fn should_count_a_named_owner_toward_the_moderator_limit() {
        let platform_version = PlatformVersion::latest();
        let max = platform_version.system_limits.max_contract_moderators as u8;
        // Identity `[1; 32]`, the first of every set below, stands for the owner: it counts.
        let config = |count: u8| ContractModerationConfig {
            banlist: true,
            suspensions: false,
            warnings: false,
            moderators: ContractModerators::AppointedModerators(set(
                &(1..=count).collect::<Vec<u8>>()
            )),
        };
        assert!(config(max)
            .validate(&BTreeMap::new(), platform_version)
            .expect("validate")
            .is_valid());
        assert!(!config(max + 1)
            .validate(&BTreeMap::new(), platform_version)
            .expect("validate")
            .is_valid());
    }

    #[test]
    fn should_reject_an_empty_or_oversized_moderator_set() {
        let platform_version = PlatformVersion::latest();
        let empty = ContractModerationConfig {
            banlist: true,
            suspensions: true,
            warnings: false,
            moderators: ContractModerators::AppointedModerators(BTreeSet::new()),
        };
        assert!(!empty
            .validate(&BTreeMap::new(), platform_version)
            .expect("validate")
            .is_valid());
        let too_many: Vec<u8> =
            (1..=(platform_version.system_limits.max_contract_moderators as u8 + 1)).collect();
        let oversized = ContractModerationConfig {
            banlist: true,
            suspensions: true,
            warnings: false,
            moderators: ContractModerators::AppointedModerators(set(&too_many)),
        };
        assert!(!oversized
            .validate(&BTreeMap::new(), platform_version)
            .expect("validate")
            .is_valid());
    }

    #[test]
    fn should_accept_a_well_formed_config() {
        let owner = Identifier::from([9; 32]);
        let config = ContractModerationConfig {
            banlist: true,
            suspensions: true,
            warnings: false,
            moderators: ContractModerators::AppointedModerators(set(&[1, 2, 3])),
        };
        assert!(config
            .validate(&BTreeMap::new(), PlatformVersion::latest())
            .expect("validate")
            .is_valid());
        assert!(config.may_moderate(&owner, &owner));
        assert!(config.may_moderate(&owner, &Identifier::from([2; 32])));
        assert!(!config.may_moderate(&owner, &Identifier::from([7; 32])));
        assert_eq!(config.lists().count(), 2);
    }

    #[test]
    fn should_put_the_owner_on_the_team_only_when_appointed_or_alone() {
        let owner = Identifier::from([9; 32]);
        assert_eq!(
            ContractModerators::ContractOwner.team(&owner),
            BTreeSet::from([owner])
        );
        assert_eq!(
            ContractModerators::AppointedModerators(set(&[1, 2])).team(&owner),
            set(&[1, 2])
        );
        assert_eq!(
            ContractModerators::AppointedModerators(set(&[1, 9])).team(&owner),
            set(&[1, 9])
        );
    }

    #[test]
    fn should_keep_a_warning_list_alone_and_bar_nobody_with_it() {
        let config = ContractModerationConfig {
            banlist: false,
            suspensions: false,
            warnings: true,
            moderators: ContractModerators::ContractOwner,
        };
        let result = config
            .validate(&BTreeMap::new(), PlatformVersion::latest())
            .expect("validate");
        assert!(result.is_valid(), "{:?}", result.errors);
        assert_eq!(
            config.lists().collect::<Vec<_>>(),
            vec![ContractModerationList::Warnings]
        );
        // Warnings bar nothing: the document gate reads no list of this contract.
        assert_eq!(config.barring_lists().count(), 0);

        let all = ContractModerationConfig {
            banlist: true,
            suspensions: true,
            warnings: true,
            moderators: ContractModerators::ContractOwner,
        };
        assert_eq!(
            all.lists().collect::<Vec<_>>(),
            vec![
                ContractModerationList::Banlist,
                ContractModerationList::Suspensions,
                ContractModerationList::Warnings,
            ]
        );
        assert_eq!(
            all.barring_lists().collect::<Vec<_>>(),
            vec![
                ContractModerationList::Banlist,
                ContractModerationList::Suspensions,
            ]
        );
    }

    #[test]
    fn should_round_trip_a_config_with_warnings_through_json_and_default_them_off() {
        let config = ContractModerationConfig {
            banlist: true,
            suspensions: false,
            warnings: true,
            moderators: ContractModerators::ContractOwner,
        };
        let json = serde_json::to_value(&config).expect("to json");
        assert_eq!(json["warnings"], true);
        let back: ContractModerationConfig = serde_json::from_value(json).expect("from json");
        assert_eq!(back, config);

        // A declaration written before warning lists existed keeps no warning list.
        let older: ContractModerationConfig = serde_json::from_value(serde_json::json!({
            "banlist": true,
            "moderators": { "$type": "contractOwner" },
        }))
        .expect("from json");
        assert!(!older.warnings);
    }

    #[test]
    fn should_report_the_warnings_of_the_list_queried_only() {
        let warned = ContractModerationStatus {
            ban: None,
            suspension: None,
            warnings: vec![ContractWarning {
                warned_at: 5,
                reason: ContractModerationReason::from_text("first strike"),
            }],
        };
        assert!(warned.warned());
        assert!(!warned.is_barred_at(0));

        let warnings_only = ContractModerationListStatuses::from_status(
            &[ContractModerationList::Warnings],
            &warned,
        );
        assert_eq!(warnings_only.warnings().map(<[_]>::len), Some(1));
        assert_eq!(warnings_only.banned(), None);
        assert_eq!(warnings_only.suspended_until(), None);
        assert!(!warnings_only.is_barred_on_queried_lists_at(0));

        let banlist_only = ContractModerationListStatuses::from_status(
            &[ContractModerationList::Banlist],
            &warned,
        );
        assert_eq!(banlist_only.warnings(), None);
        assert_eq!(
            ContractModerationListStatus::Warnings { warnings: vec![] }.list(),
            ContractModerationList::Warnings
        );
    }

    #[test]
    fn should_tell_barred_from_lapsed() {
        let banned = ContractModerationStatus {
            ban: Some(ContractBan::default()),
            suspension: None,
            warnings: vec![],
        };
        assert!(banned.banned());
        assert!(banned.is_barred_at(0));
        let suspended = ContractModerationStatus {
            ban: None,
            suspension: Some(ContractSuspension {
                until: 100,
                reason: ContractModerationReason::from_text("flooding"),
            }),
            warnings: vec![],
        };
        assert!(!suspended.banned());
        assert_eq!(suspended.suspended_until(), Some(100));
        assert!(suspended.is_barred_at(99));
        assert!(!suspended.is_barred_at(100));
        assert!(suspended.has_lapsed_suspension_at(100));
        assert!(!suspended.has_lapsed_suspension_at(99));
    }

    #[test]
    fn should_report_only_the_lists_read() {
        let status = ContractModerationStatus {
            ban: Some(ContractBan {
                reason: ContractModerationReason::from_text("spam"),
            }),
            suspension: None,
            warnings: vec![],
        };

        let banlist_only = ContractModerationListStatuses::from_status(
            &[ContractModerationList::Banlist],
            &status,
        );
        assert_eq!(banlist_only.banned(), Some(true));
        assert_eq!(
            banlist_only
                .ban()
                .flatten()
                .map(|ban| ban.reason.text.as_str()),
            Some("spam")
        );
        assert_eq!(banlist_only.suspension(), None);
        assert_eq!(banlist_only.suspended_until(), None);

        let both = ContractModerationListStatuses::from_status(
            &[
                ContractModerationList::Banlist,
                ContractModerationList::Suspensions,
            ],
            &status,
        );
        assert_eq!(both.suspension(), Some(None));
        assert_eq!(both.suspended_until(), Some(None));
        assert_eq!(
            serde_json::to_value(&both).expect("to json"),
            serde_json::json!([
                {"list": "banlist", "ban": {"reason": {"code": null, "text": "spam"}}},
                {"list": "suspensions", "suspension": null},
            ])
        );
    }
}
