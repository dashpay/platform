//! Parsing of a `refersTo` whose target the referring document names (`contractId` and
//! `documentType` written `"$self.<property>"`, with `documentTypeRequirements`), which
//! version 1 of `apply_property_reference` reads (protocol version 15). See
//! [`DynamicDocumentReference`].

use crate::data_contract::config::moderation::ModerationAbility;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::property::{
    DocumentPropertyType, DocumentTypeRequirements, DynamicDocumentReference,
    DynamicDocumentReferenceKind, PropertyReference, ReferenceHolder, RequiredPropertySlot,
    SlotCondition, SlotConditionList, SlotConditionSetting, SlotConditionValue, SlotConditions,
    SlotPropertyName, SlotRequirement, SlotRequirementTerm, SlotValue, ANY_PROPERTY_NAME,
    REFERRING_DOCUMENT_PREFIX, SLOT_CONDITION_CHANGE_FIELDS, SLOT_CONDITION_DELETE_KEEPS_FIELDS,
    SLOT_CONDITION_DELETE_SETTLED, SLOT_CONDITION_DELETE_WITHIN, SLOT_CONDITION_ELECTED_ABILITIES,
    SLOT_CONDITION_KIND, SLOT_CONDITION_PROPERTY,
};
use crate::data_contract::document_type::{property_names, DocumentTypeRef};
use crate::data_contract::errors::DataContractError;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::RangeInclusive;

/// The keys of a dynamic reference beside its `type`.
const DYNAMIC_KEYS: [&str; 3] = [
    property_names::CONTRACT_ID,
    property_names::DOCUMENT_TYPE,
    property_names::DOCUMENT_TYPE_REQUIREMENTS,
];

/// The two keys a dynamic reference reads from the referring document.
const TARGET_KEYS: [&str; 2] = [property_names::CONTRACT_ID, property_names::DOCUMENT_TYPE];

/// What `"$self.<rest>"` reads from the referring document: `<rest>`. `None` for a value
/// that is not written with the prefix, a literal contract id or document type among them.
fn read_from_referring_document(value: &Value) -> Option<&str> {
    value.as_text()?.strip_prefix(REFERRING_DOCUMENT_PREFIX)
}

/// The keys of a slot of `documentTypeRequirements.properties`.
const SLOT_KEYS: [&str; 6] = [
    "name",
    "type",
    "byteArray",
    "maxBytes",
    "distinctFrom",
    "when",
];

/// The kinds of document type a reference naming its target through the referring document
/// points into, by the `refersTo` `type` each is written with: what `$kind` reads in a
/// slot's `when` for a type the reference admits.
const DYNAMIC_KIND_NAMES: [&str; 2] = ["permanentDocument", "moderatedDocument"];

/// The limits a slot's `when` is parsed under, of `SystemLimits`.
#[derive(Clone, Copy)]
struct SlotConditionLimits {
    /// `max_document_type_requirement_condition_nodes`.
    nodes: usize,
    /// `max_document_type_requirement_condition_text_length`.
    text_length: usize,
}

impl SlotConditionLimits {
    fn of(platform_version: &PlatformVersion) -> Self {
        let limits = &platform_version.system_limits;
        Self {
            nodes: usize::from(limits.max_document_type_requirement_condition_nodes),
            text_length: usize::from(limits.max_document_type_requirement_condition_text_length),
        }
    }
}

/// The names of the abilities a contract may give its elected moderation team on a document
/// type, what `$electedAbilities` lists.
const ELECTED_ABILITY_NAMES: [ModerationAbility; 5] = [
    ModerationAbility::DeleteDocuments,
    ModerationAbility::Ban,
    ModerationAbility::Suspend,
    ModerationAbility::Warn,
    ModerationAbility::ChangeDocumentFields,
];

fn refused(message: String) -> DataContractError {
    DataContractError::InvalidContractStructure(message)
}

/// Whether a `refersTo` declaration names its target through the referring document: its
/// `contractId` or its `documentType` is read from it (`"$self.<property>"`).
///
/// Nothing else tells one. This runs on every parse, the reading of a contract stored
/// under an earlier protocol version included, and must read such a contract as that
/// version did. No stored declaration is written this way: before protocol version 15 a
/// `contractId` that is no identifier does not parse, and a `documentType` names a document
/// type that exists, whose name holds no `$` and no dot. `documentTypeRequirements` beside
/// a target the schema names is another matter: earlier versions did not read the key, so
/// a stored declaration may carry it, and it stays unread here. Registration refuses it
/// (the native schema check), like every key a declaration does not take.
pub(super) fn declares_dynamic_target(refers_to_value: &Value) -> bool {
    refers_to_value
        .to_btree_ref_string_map()
        .is_ok_and(|declaration| {
            TARGET_KEYS.iter().any(|key| {
                declaration
                    .get(*key)
                    .is_some_and(|value| read_from_referring_document(value).is_some())
            })
        })
}

/// Whether a `refersTo` declaration, or an operand anywhere inside a reference expression
/// it holds, names its target through the referring document.
fn declares_dynamic_target_anywhere(refers_to_value: &Value) -> bool {
    if declares_dynamic_target(refers_to_value) {
        return true;
    }
    let Ok(declaration) = refers_to_value.to_btree_ref_string_map() else {
        return false;
    };
    ["anyOf", "allOf"].iter().any(|combinator| {
        declaration
            .get(*combinator)
            .and_then(|operands| operands.as_array())
            .is_some_and(|operands| operands.iter().any(declares_dynamic_target_anywhere))
    })
}

/// Refuses a dynamic target where a reference cannot be one: `place` says where, as the end
/// of a sentence ("on the elements of a typed array"). `refers_to_value` is the declaration,
/// if there is one.
pub(super) fn refuse_dynamic_target(
    refers_to_value: Option<&Value>,
    place: &str,
) -> Result<(), DataContractError> {
    if refers_to_value.is_some_and(declares_dynamic_target_anywhere) {
        return Err(refused(format!(
            "a refersTo reading its contractId and documentType from the referring document \
             is not allowed {place}: it sits on an identifier property, alone"
        )));
    }
    Ok(())
}

/// A name of 1 to 64 letters, digits or underscores: a top-level property, a document
/// type, or a slot.
fn is_word_name(name: &str) -> bool {
    (1..=64).contains(&name.len())
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
}

/// Parses a declaration [`declares_dynamic_target`] holds for, on an identifier property.
pub(super) fn parse_dynamic_document_reference(
    refers_to_map: &BTreeMap<String, &Value>,
    platform_version: &PlatformVersion,
) -> Result<DynamicDocumentReference, DataContractError> {
    // One kind, or the list of both: a type of either is then admitted
    let (kind, reference_type) = match refers_to_map.get(property_names::TYPE) {
        Some(Value::Text(reference_type)) => match reference_type.as_str() {
            "permanentDocument" => (DynamicDocumentReferenceKind::Permanent, "permanentDocument"),
            "moderatedDocument" => (DynamicDocumentReferenceKind::Moderated, "moderatedDocument"),
            other => {
                return Err(refused(format!(
                    "{other} refersTo reads no contractId or documentType from the referring \
                     document and takes no documentTypeRequirements: only a \
                     permanentDocument or a moderatedDocument reference names its target \
                     through the referring document, since only those document types \
                     promise their documents stay"
                )))
            }
        },
        Some(Value::Array(kinds)) => {
            let listed: BTreeSet<&str> = kinds.iter().filter_map(|kind| kind.as_text()).collect();
            if kinds.len() != DYNAMIC_KIND_NAMES.len()
                || listed != BTreeSet::from(DYNAMIC_KIND_NAMES)
            {
                return Err(refused(
                    "a refersTo type list holds permanentDocument and moderatedDocument, once \
                     each: a reference naming its target through the referring document \
                     points into a type of either kind, or names one kind alone"
                        .to_string(),
                ));
            }
            (
                DynamicDocumentReferenceKind::PermanentOrModerated,
                "permanentDocument or moderatedDocument",
            )
        }
        _ => {
            return Err(DataContractError::ValueWrongType(
                "refersTo type must be a string, or the list of permanentDocument and \
                 moderatedDocument"
                    .to_string(),
            ))
        }
    };

    // The target is known only when a document is written, so nothing that needs the
    // referenced type at registration can sit beside it
    if let Some(key) = refers_to_map
        .keys()
        .find(|key| key.as_str() != property_names::TYPE && !DYNAMIC_KEYS.contains(&key.as_str()))
    {
        return Err(refused(format!(
            "{reference_type} refersTo naming its target through the referring document does \
             not take {key}: the referenced document type is not known until a document is \
             written"
        )));
    }

    // Both come from the document: a declaration names its target in the schema or
    // through the referring document, not half of each
    let property = |key: &str| -> Result<String, DataContractError> {
        let read = refers_to_map
            .get(key)
            .and_then(|value| read_from_referring_document(value))
            .ok_or_else(|| {
                refused(format!(
                    "{reference_type} refersTo naming its target through the referring \
                     document reads both contractId and documentType from it: {key} must be \
                     \"{REFERRING_DOCUMENT_PREFIX}<a top-level property>\""
                ))
            })?;
        if !is_word_name(read) {
            return Err(refused(format!(
                "{reference_type} refersTo {key} must read a top-level property, \
                 \"{REFERRING_DOCUMENT_PREFIX}\" followed by a name of 1 to 64 letters, digits \
                 or underscores"
            )));
        }
        Ok(read.to_string())
    };
    let contract_id_property = property(property_names::CONTRACT_ID)?;
    let document_type_property = property(property_names::DOCUMENT_TYPE)?;
    if contract_id_property == document_type_property {
        return Err(refused(format!(
            "{reference_type} refersTo contractId and documentType read the same property, \
             \"{contract_id_property}\""
        )));
    }

    let requirements = refers_to_map
        .get(property_names::DOCUMENT_TYPE_REQUIREMENTS)
        .map(|value| parse_document_type_requirements(value, platform_version))
        .transpose()?;

    Ok(DynamicDocumentReference {
        kind,
        contract_id_property,
        document_type_property,
        requirements,
    })
}

fn parse_document_type_requirements(
    value: &Value,
    platform_version: &PlatformVersion,
) -> Result<DocumentTypeRequirements, DataContractError> {
    let requirements = value
        .to_btree_ref_string_map()
        .map_err(|_| refused("documentTypeRequirements must be an object".to_string()))?;
    if let Some(key) = requirements
        .keys()
        .find(|key| !matches!(key.as_str(), "properties" | "require" | "when"))
    {
        return Err(refused(format!(
            "documentTypeRequirements does not take {key}: it holds properties, require and \
             when"
        )));
    }
    let slot_values = requirements
        .get("properties")
        .and_then(|properties| properties.to_btree_ref_string_map().ok())
        .ok_or_else(|| {
            refused(
                "documentTypeRequirements needs properties, an object of named slots".to_string(),
            )
        })?;
    let max_slots = platform_version
        .system_limits
        .max_document_type_requirement_slots as usize;
    if slot_values.is_empty() || slot_values.len() > max_slots {
        return Err(refused(format!(
            "documentTypeRequirements properties must hold 1 to {max_slots} slots, got {}",
            slot_values.len()
        )));
    }

    let slots = slot_values
        .iter()
        .map(|(slot_name, slot_value)| {
            if !is_word_name(slot_name) {
                return Err(refused(format!(
                    "documentTypeRequirements slot \"{slot_name}\" must be named with 1 to 64 \
                     letters, digits or underscores"
                )));
            }
            Ok((
                slot_name.clone(),
                parse_slot(slot_name, slot_value, platform_version)?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;

    for (slot_name, slot) in &slots {
        if let Some(other) = slot
            .distinct_from
            .iter()
            .find(|other| *other == slot_name || !slots.contains_key(*other))
        {
            return Err(refused(format!(
                "documentTypeRequirements slot \"{slot_name}\" distinctFrom \"{other}\" must \
                 name another slot of the same properties"
            )));
        }
    }

    let require = requirements
        .get("require")
        .map(|value| parse_require(value, &slots))
        .transpose()?;

    // What the type must say of itself, whatever fills the slots: the grammar of a slot's
    // `when`, with no property in hand to read
    let when = requirements
        .get("when")
        .map(|when| {
            let mut nodes = 0;
            let condition = parse_slot_condition(
                when,
                "documentTypeRequirements",
                &mut nodes,
                SlotConditionLimits::of(platform_version),
            )?;
            if condition.reads_the_property() {
                return Err(refused(format!(
                    "documentTypeRequirements when is judged for the document type as a \
                     whole and reads no \"{SLOT_CONDITION_PROPERTY}\": a condition on a property \
                     belongs on the slot the property fills"
                )));
            }
            Ok(condition)
        })
        .transpose()?;

    Ok(DocumentTypeRequirements {
        slots,
        require,
        when,
    })
}

fn parse_slot(
    slot_name: &str,
    value: &Value,
    platform_version: &PlatformVersion,
) -> Result<RequiredPropertySlot, DataContractError> {
    let at = || format!("documentTypeRequirements slot \"{slot_name}\"");
    let slot = value
        .to_btree_ref_string_map()
        .map_err(|_| refused(format!("{} must be an object", at())))?;
    if let Some(key) = slot.keys().find(|key| !SLOT_KEYS.contains(&key.as_str())) {
        return Err(refused(format!("{} does not take {key}", at())));
    }

    let named_wrongly = || {
        refused(format!(
            "{} needs a name: \"$any\", a property name of 1 to 64 letters, digits or \
             underscores, or \"{REFERRING_DOCUMENT_PREFIX}<a string property of the referring \
             document>\"",
            at()
        ))
    };
    let name = match slot.get("name").and_then(|name| name.as_text()) {
        Some(ANY_PROPERTY_NAME) => SlotPropertyName::Any,
        Some(name) if is_word_name(name) => SlotPropertyName::Named(name.to_string()),
        // The name the referring document gives in a string property of its own
        Some(name) => SlotPropertyName::FromProperty(
            name.strip_prefix(REFERRING_DOCUMENT_PREFIX)
                .filter(|property| is_word_name(property))
                .ok_or_else(named_wrongly)?
                .to_string(),
        ),
        None => return Err(named_wrongly()),
    };

    let byte_array = match slot.get("byteArray") {
        None => false,
        Some(Value::Bool(true)) => true,
        Some(_) => {
            return Err(refused(format!(
                "{} byteArray, when present, must be true",
                at()
            )))
        }
    };
    let value_kind = match (
        slot.get("type").and_then(|value_type| value_type.as_text()),
        byte_array,
    ) {
        (Some("string"), false) => SlotValue::String,
        (Some("identifier"), false) => SlotValue::Identifier,
        (Some("integer"), false) => SlotValue::Integer,
        (Some("boolean"), false) => SlotValue::Boolean,
        (Some("array"), true) => SlotValue::ByteArray,
        _ => {
            return Err(refused(format!(
                "{} needs a type: \"string\", \"identifier\", \"integer\", \"boolean\", or \
                 \"array\" with \"byteArray\": true",
                at()
            )))
        }
    };

    let max_bytes = slot
        .get("maxBytes")
        .map(|max_bytes| {
            if !matches!(value_kind, SlotValue::String | SlotValue::ByteArray) {
                return Err(refused(format!(
                    "{} maxBytes bounds a string or a byte array, not a {}",
                    at(),
                    value_kind.wire_name()
                )));
            }
            max_bytes
                .to_integer::<u32>()
                .ok()
                .filter(|max_bytes| *max_bytes >= 1)
                .ok_or_else(|| {
                    refused(format!(
                        "{} maxBytes must be an integer from 1 to 4294967295",
                        at()
                    ))
                })
        })
        .transpose()?;

    let distinct_from = match slot.get("distinctFrom") {
        None => BTreeSet::new(),
        Some(Value::Text(other)) => BTreeSet::from([other.clone()]),
        Some(Value::Array(others)) if !others.is_empty() => {
            let names = others
                .iter()
                .map(|other| other.as_text().map(str::to_string))
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| {
                    refused(format!(
                        "{} distinctFrom must be a slot name or a list of slot names",
                        at()
                    ))
                })?;
            let distinct: BTreeSet<String> = names.iter().cloned().collect();
            if distinct.len() != names.len() {
                return Err(refused(format!("{} distinctFrom names a slot twice", at())));
            }
            distinct
        }
        Some(_) => {
            return Err(refused(format!(
                "{} distinctFrom must be a slot name or a list of at least one slot name",
                at()
            )))
        }
    };

    let when = slot
        .get("when")
        .map(|when| {
            let mut nodes = 0;
            parse_slot_condition(
                when,
                &at(),
                &mut nodes,
                SlotConditionLimits::of(platform_version),
            )
        })
        .transpose()?;

    Ok(RequiredPropertySlot {
        name,
        value: value_kind,
        max_bytes,
        distinct_from,
        when,
    })
}

/// A string a slot's `when` compares: `$kind`, `$property` or `{ "const": <string> }`.
fn slot_condition_value(
    value: &Value,
    at: &str,
    max_length: usize,
) -> Result<SlotConditionValue, DataContractError> {
    let shape = || {
        refused(format!(
            "{at} when compares \"{SLOT_CONDITION_KIND}\", \"{SLOT_CONDITION_PROPERTY}\" or {{ \
             \"const\": <a string of 1 to {max_length} characters> }}, \
             got {value}"
        ))
    };
    match value {
        Value::Text(operand) if operand == SLOT_CONDITION_KIND => Ok(SlotConditionValue::Kind),
        Value::Text(operand) if operand == SLOT_CONDITION_PROPERTY => {
            Ok(SlotConditionValue::PropertyName)
        }
        Value::Map(_) => {
            let constant = value.to_btree_ref_string_map().map_err(|_| shape())?;
            match (constant.len(), constant.get("const")) {
                (1, Some(Value::Text(constant)))
                    if (1..=max_length).contains(&constant.chars().count()) =>
                {
                    Ok(SlotConditionValue::Constant(constant.clone()))
                }
                _ => Err(shape()),
            }
        }
        _ => Err(shape()),
    }
}

/// Refuses a constant `$kind` is compared with that names no kind the reference admits: the
/// comparison could never hold, or always would.
fn kind_name(constant: &str, at: &str) -> Result<(), DataContractError> {
    if DYNAMIC_KIND_NAMES.contains(&constant) {
        return Ok(());
    }
    Err(refused(format!(
        "{at} when compares \"{SLOT_CONDITION_KIND}\" with \"{constant}\": the kind of the \
         referenced document type is permanentDocument or moderatedDocument"
    )))
}

/// The conditions of an `anyOf`, `allOf`, `ifThen` or `ifThenElse`, or the one of a `not`.
fn parse_slot_conditions(
    values: &[Value],
    at: &str,
    nodes: &mut usize,
    limits: SlotConditionLimits,
) -> Result<SlotConditions, DataContractError> {
    values
        .iter()
        .map(|value| parse_slot_condition(value, at, nodes, limits))
        .collect::<Result<Vec<_>, _>>()
        .map(SlotConditions::new)
}

/// Parses a slot's `when`, or a condition inside one. `nodes` counts the conditions read so
/// far, and each string an `in` or a `notIn` lists, refused past `limits.nodes`
/// (`SystemLimits::max_document_type_requirement_condition_nodes`) before anything is read
/// of the one that passes it, so the count also bounds how deep this recurses.
fn parse_slot_condition(
    value: &Value,
    at: &str,
    nodes: &mut usize,
    limits: SlotConditionLimits,
) -> Result<SlotCondition, DataContractError> {
    *nodes += 1;
    let too_large = || {
        refused(format!(
            "{at} when is made of more than {} conditions, counting each string an in or a \
             notIn lists",
            limits.nodes
        ))
    };
    if *nodes > limits.nodes {
        return Err(too_large());
    }
    let condition = value
        .to_btree_ref_string_map()
        .ok()
        .filter(|condition| condition.len() == 1)
        .ok_or_else(|| {
            refused(format!(
                "{at} when, and every condition inside it, is an object of one key: equal, \
                 notEqual, in, notIn, contains, present, absent, not, anyOf, allOf, ifThen or \
                 ifThenElse"
            ))
        })?;
    let Some((key, operand)) = condition.iter().next() else {
        return Err(refused(format!("{at} when holds an empty condition")));
    };
    let key = key.as_str();
    let items =
        |count: RangeInclusive<usize>, what: &str| -> Result<&Vec<Value>, DataContractError> {
            operand
                .as_array()
                .filter(|items| count.contains(&items.len()))
                .ok_or_else(|| refused(format!("{at} when {key} takes {what}")))
        };

    match key {
        "equal" | "notEqual" => {
            let pair = items(2..=2, "two strings")?;
            let left = slot_condition_value(&pair[0], at, limits.text_length)?;
            let right = slot_condition_value(&pair[1], at, limits.text_length)?;
            match (&left, &right) {
                (SlotConditionValue::Kind, SlotConditionValue::Constant(constant))
                | (SlotConditionValue::Constant(constant), SlotConditionValue::Kind) => {
                    kind_name(constant, at)?
                }
                (SlotConditionValue::PropertyName, SlotConditionValue::Constant(_))
                | (SlotConditionValue::Constant(_), SlotConditionValue::PropertyName) => {}
                _ => {
                    return Err(refused(format!(
                        "{at} when {key} compares \"{SLOT_CONDITION_KIND}\" or \
                         \"{SLOT_CONDITION_PROPERTY}\" with a {{ \"const\": ... }}"
                    )))
                }
            }
            Ok(if key == "equal" {
                SlotCondition::Equal(left, right)
            } else {
                SlotCondition::NotEqual(left, right)
            })
        }
        "in" | "notIn" => {
            let pair = items(2..=2, "a string and a list of two or more distinct strings")?;
            let value = slot_condition_value(&pair[0], at, limits.text_length)?;
            // Each string listed counts toward the limit, before any of them is read: the
            // list is what one condition could otherwise grow without bound by
            *nodes = nodes.saturating_add(pair[1].as_array().map_or(0, Vec::len));
            if *nodes > limits.nodes {
                return Err(too_large());
            }
            let among = pair[1]
                .as_array()
                .filter(|among| among.len() >= 2)
                .and_then(|among| {
                    let texts = among
                        .iter()
                        .map(|text| {
                            text.as_text()
                                .filter(|text| {
                                    (1..=limits.text_length).contains(&text.chars().count())
                                })
                                .map(str::to_string)
                        })
                        .collect::<Option<BTreeSet<String>>>()?;
                    (texts.len() == among.len()).then_some(texts)
                })
                .ok_or_else(|| {
                    refused(format!(
                        "{at} when {key} takes a string and a list of two or more distinct \
                         strings of 1 to {} characters",
                        limits.text_length
                    ))
                })?;
            match value {
                SlotConditionValue::Kind => {
                    for constant in &among {
                        kind_name(constant, at)?;
                    }
                }
                SlotConditionValue::PropertyName => {}
                SlotConditionValue::Constant(_) => {
                    return Err(refused(format!(
                        "{at} when {key} looks for \"{SLOT_CONDITION_KIND}\" or \
                         \"{SLOT_CONDITION_PROPERTY}\" in the list, not for a constant"
                    )))
                }
            }
            Ok(if key == "in" {
                SlotCondition::In(value, among)
            } else {
                SlotCondition::NotIn(value, among)
            })
        }
        "contains" => {
            let pair = items(2..=2, "a list of the referenced type and a string")?;
            let list = match pair[0].as_text() {
                Some(SLOT_CONDITION_DELETE_KEEPS_FIELDS) => SlotConditionList::DeletionKeptFields,
                Some(SLOT_CONDITION_CHANGE_FIELDS) => SlotConditionList::ModeratorChangeableFields,
                Some(SLOT_CONDITION_ELECTED_ABILITIES) => SlotConditionList::ElectedAbilities,
                _ => {
                    return Err(refused(format!(
                        "{at} when contains looks into \"{SLOT_CONDITION_DELETE_KEEPS_FIELDS}\", \
                         \"{SLOT_CONDITION_CHANGE_FIELDS}\" or \
                         \"{SLOT_CONDITION_ELECTED_ABILITIES}\", got {}",
                        pair[0]
                    )))
                }
            };
            let needle = slot_condition_value(&pair[1], at, limits.text_length)?;
            // The team's abilities are named, not properties: a constant that names one
            if list == SlotConditionList::ElectedAbilities {
                let names_an_ability = matches!(
                    &needle,
                    SlotConditionValue::Constant(name)
                        if ELECTED_ABILITY_NAMES.iter().any(|ability| ability.as_str() == name)
                );
                if !names_an_ability {
                    return Err(refused(format!(
                        "{at} when contains looks in \"{SLOT_CONDITION_ELECTED_ABILITIES}\" for \
                         a {{ \"const\": ... }} naming an ability: {}",
                        ELECTED_ABILITY_NAMES
                            .iter()
                            .map(ModerationAbility::as_str)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )));
                }
                return Ok(SlotCondition::Contains(list, needle));
            }
            if needle == SlotConditionValue::Kind {
                return Err(refused(format!(
                    "{at} when contains looks for \"{SLOT_CONDITION_PROPERTY}\" or a {{ \"const\": \
                     ... }} in {}, which lists properties, not for the kind of the type",
                    list.wire_name()
                )));
            }
            Ok(SlotCondition::Contains(list, needle))
        }
        "not" => {
            // As in `propertyConstraints`: `not` of `not` says what the inner condition
            // says, and `not` of `notIn` what `in` says
            if operand
                .to_btree_ref_string_map()
                .is_ok_and(|negated| negated.contains_key("not") || negated.contains_key("notIn"))
            {
                return Err(refused(format!(
                    "{at} when not holds a not or a notIn: write the condition it negates, or in"
                )));
            }
            parse_slot_conditions(std::slice::from_ref(operand), at, nodes, limits)
                .map(SlotCondition::Not)
        }
        "anyOf" | "allOf" => {
            let operands = items(2..=usize::MAX, "two or more conditions")?;
            // One directly inside itself says what one flat list says
            if operands.iter().any(|operand| {
                operand
                    .to_btree_ref_string_map()
                    .is_ok_and(|operand| operand.contains_key(key))
            }) {
                return Err(refused(format!(
                    "{at} when {key} holds another {key}: list its conditions in the outer one"
                )));
            }
            let conditions = parse_slot_conditions(operands, at, nodes, limits)?;
            let listed = conditions.conditions();
            if listed
                .iter()
                .enumerate()
                .any(|(index, condition)| listed[..index].contains(condition))
            {
                return Err(refused(format!(
                    "{at} when {key} lists the same condition twice"
                )));
            }
            Ok(if key == "anyOf" {
                SlotCondition::AnyOf(conditions)
            } else {
                SlotCondition::AllOf(conditions)
            })
        }
        "ifThen" => parse_slot_conditions(items(2..=2, "two conditions")?, at, nodes, limits)
            .map(SlotCondition::IfThen),
        "ifThenElse" => parse_slot_conditions(items(3..=3, "three conditions")?, at, nodes, limits)
            .map(SlotCondition::IfThenElse),
        "present" | "absent" => {
            let setting = match operand.as_text() {
                Some(SLOT_CONDITION_DELETE_WITHIN) => SlotConditionSetting::DeleteWithin,
                Some(SLOT_CONDITION_DELETE_SETTLED) => SlotConditionSetting::DeleteSettled,
                _ => {
                    return Err(refused(format!(
                        "{at} when {key} tests \"{SLOT_CONDITION_DELETE_WITHIN}\" or \
                         \"{SLOT_CONDITION_DELETE_SETTLED}\", got {operand}"
                    )))
                }
            };
            Ok(if key == "present" {
                SlotCondition::Present(setting)
            } else {
                SlotCondition::Absent(setting)
            })
        }
        other => Err(refused(format!(
            "{at} when does not take {other}: a condition is equal, notEqual, in, notIn, \
             contains, present, absent, not, anyOf, allOf, ifThen or ifThenElse"
        ))),
    }
}

/// The one key of a combinator object and its operands, at least two of them.
fn combinator_operands(value: &Value) -> Option<(&'static str, &Vec<Value>)> {
    let map = value.to_btree_ref_string_map().ok()?;
    if map.len() != 1 {
        return None;
    }
    ["anyOf", "allOf"].into_iter().find_map(|combinator| {
        map.get(combinator)
            .and_then(|operands| operands.as_array())
            .filter(|operands| operands.len() >= 2)
            .map(|operands| (combinator, operands))
    })
}

fn parse_require(
    value: &Value,
    slots: &BTreeMap<String, RequiredPropertySlot>,
) -> Result<SlotRequirement, DataContractError> {
    let shape = || {
        refused(
            "documentTypeRequirements require must be a slot name, or anyOf or allOf over two \
             or more terms, each a slot name or the other combinator over two or more slot \
             names"
                .to_string(),
        )
    };
    let slot = |value: &Value| -> Result<String, DataContractError> {
        let name = value.as_text().ok_or_else(shape)?;
        if !slots.contains_key(name) {
            return Err(refused(format!(
                "documentTypeRequirements require names \"{name}\", which is not a slot of \
                 properties"
            )));
        }
        Ok(name.to_string())
    };

    // The condition is judged on every write that checks the reference, so it stays as
    // small as the slots are few: a slot named twice in a group, or a term given twice
    // (a group whatever the order of its slots), says nothing the first did not
    let repeated = |what: &str| {
        refused(format!(
            "documentTypeRequirements require names {what} twice: each slot is named once in \
             a group, and each term once"
        ))
    };
    let requirement = if value.is_text() {
        SlotRequirement::Slot(slot(value)?)
    } else {
        let (combinator, operands) = combinator_operands(value).ok_or_else(shape)?;
        let mut seen: BTreeSet<BTreeSet<String>> = BTreeSet::new();
        let terms = operands
            .iter()
            .map(|operand| {
                if operand.is_text() {
                    let name = slot(operand)?;
                    if !seen.insert(BTreeSet::from([name.clone()])) {
                        return Err(repeated(&format!("the slot \"{name}\"")));
                    }
                    return Ok(SlotRequirementTerm::Slot(name));
                }
                let (inner, inner_operands) = combinator_operands(operand).ok_or_else(shape)?;
                // A combinator directly inside itself says what one flat list says
                if inner == combinator {
                    return Err(shape());
                }
                let group = inner_operands
                    .iter()
                    .map(&slot)
                    .collect::<Result<Vec<_>, _>>()?;
                let members: BTreeSet<String> = group.iter().cloned().collect();
                if members.len() != group.len() {
                    return Err(repeated("a slot in one group"));
                }
                if !seen.insert(members) {
                    return Err(repeated("a group of slots"));
                }
                Ok(SlotRequirementTerm::Group(group))
            })
            .collect::<Result<Vec<_>, DataContractError>>()?;
        match combinator {
            "anyOf" => SlotRequirement::AnyOf(terms),
            _ => SlotRequirement::AllOf(terms),
        }
    };

    // A slot `require` never names could never matter: every slot is part of the condition
    let named: BTreeSet<&str> = requirement.slots().into_iter().collect();
    if let Some(unnamed) = slots.keys().find(|slot| !named.contains(slot.as_str())) {
        return Err(refused(format!(
            "documentTypeRequirements require does not name the slot \"{unnamed}\": every slot \
             of properties is part of the condition, or is left out of properties"
        )));
    }
    Ok(requirement)
}

/// Checks the referring side of every dynamic reference of a document type, once all its
/// properties are parsed. The contract id comes from a top-level identifier property and
/// the document type name, like every property name a slot reads, from a top-level string
/// property; none of them is transient or the reference property itself; and each is
/// required wherever the reference property is, so a document that must hold the reference
/// always says where it points.
///
/// Runs on every parse, validating or not, like the `findBy` source check: the write-time
/// check reads the target through these properties.
pub(super) fn validate_dynamic_reference_sources(
    document_type: DocumentTypeRef,
    document_type_name: &str,
) -> Result<(), DataContractError> {
    for (holder, reference) in document_type.reference_declarations() {
        let Some(dynamic) = reference
            .target()
            .and_then(|target| target.as_dynamic_document_reference())
        else {
            continue;
        };
        // `describe` ends with the keyword: `property "descriptionId" refersTo`
        let at = format!(
            "document type \"{document_type_name}\" {}",
            holder.describe()
        );
        // The parser admits one on an identifier property alone
        let (ReferenceHolder::Property(reference_path), PropertyReference::Value(_)) =
            (&holder, &reference)
        else {
            return Err(refused(format!(
                "{at} names its target through the referring document, which only a single \
                 identifier property's reference may"
            )));
        };
        let reference_path: &str = reference_path;
        let reference_is_required = document_type
            .flattened_properties()
            .get(reference_path)
            .is_some_and(|property| property.required);

        // `holds` says whether the property is of the kind `kind` names
        let source = |key: &str,
                      name: &str,
                      kind: &str,
                      holds: fn(&DocumentPropertyType) -> bool|
         -> Result<(), DataContractError> {
            if name == reference_path {
                return Err(refused(format!(
                    "{at} {key} names \"{name}\", the property carrying the reference"
                )));
            }
            let property = document_type.properties().get(name).ok_or_else(|| {
                refused(format!(
                    "{at} {key} names \"{name}\", which is not a top-level property of the \
                     document type"
                ))
            })?;
            if !holds(&property.property_type) {
                return Err(refused(format!(
                    "{at} {key} names \"{name}\", which is not {kind} property"
                )));
            }
            if property.transient {
                return Err(refused(format!(
                    "{at} {key} names \"{name}\", a transient property: the reference is \
                     checked again from the stored document on a replace"
                )));
            }
            if reference_is_required && !property.required {
                return Err(refused(format!(
                    "{at} {key} names \"{name}\", which is not required while the reference \
                     is: a document holding the reference must say where it points"
                )));
            }
            Ok(())
        };

        source(
            property_names::CONTRACT_ID,
            &dynamic.contract_id_property,
            "an identifier",
            |property_type| {
                matches!(
                    property_type,
                    DocumentPropertyType::Identifier
                        | DocumentPropertyType::IdentifierWithReference(_)
                )
            },
        )?;
        let string_sources = std::iter::once((
            property_names::DOCUMENT_TYPE,
            dynamic.document_type_property.as_str(),
        ))
        .chain(
            dynamic
                .requirements
                .iter()
                .flat_map(|requirements| requirements.name_properties())
                .map(|name| ("documentTypeRequirements slot name", name)),
        );
        for (key, name) in string_sources {
            source(key, name, "a string", |property_type| {
                matches!(property_type, DocumentPropertyType::String(_))
            })?;
        }
    }
    Ok(())
}
