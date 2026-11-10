//! Parsing of a `refersTo` whose target the referring document names
//! (`contractIdProperty`, `documentTypeProperty`, `documentTypeRequirements`), which
//! version 1 of `apply_property_reference` reads (protocol version 15). See
//! [`DynamicDocumentReference`].

use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::property::{
    DocumentPropertyType, DocumentTypeRequirements, DynamicDocumentReference,
    DynamicDocumentReferenceKind, PropertyReference, ReferenceHolder, RequiredPropertySlot,
    SlotPropertyName, SlotRequirement, SlotRequirementTerm, SlotValue, ANY_PROPERTY_NAME,
};
use crate::data_contract::document_type::{property_names, DocumentTypeRef};
use crate::data_contract::errors::DataContractError;
use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use std::collections::{BTreeMap, BTreeSet};

/// The keys only a dynamic reference carries.
const DYNAMIC_KEYS: [&str; 3] = [
    property_names::CONTRACT_ID_PROPERTY,
    property_names::DOCUMENT_TYPE_PROPERTY,
    property_names::DOCUMENT_TYPE_REQUIREMENTS,
];

/// The keys of a slot of `documentTypeRequirements.properties`.
const SLOT_KEYS: [&str; 5] = ["name", "type", "byteArray", "maxBytes", "distinctFrom"];

fn refused(message: String) -> DataContractError {
    DataContractError::InvalidContractStructure(message)
}

/// Whether a `refersTo` declaration names its target through the referring document: it
/// carries one of the keys only a dynamic reference does.
pub(super) fn declares_dynamic_target(refers_to_value: &Value) -> bool {
    refers_to_value
        .to_btree_ref_string_map()
        .is_ok_and(|declaration| {
            DYNAMIC_KEYS
                .iter()
                .any(|key| declaration.contains_key(*key))
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
            "a refersTo naming its target through contractIdProperty and documentTypeProperty \
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

fn word_name(value: &Value, what: &str) -> Result<String, DataContractError> {
    value
        .as_text()
        .filter(|name| is_word_name(name))
        .map(str::to_string)
        .ok_or_else(|| {
            refused(format!(
                "{what} must be a name of 1 to 64 letters, digits or underscores"
            ))
        })
}

/// Parses a declaration [`declares_dynamic_target`] holds for, on an identifier property.
pub(super) fn parse_dynamic_document_reference(
    refers_to_map: &BTreeMap<String, &Value>,
    platform_version: &PlatformVersion,
) -> Result<DynamicDocumentReference, DataContractError> {
    let reference_type = refers_to_map
        .get_str(property_names::TYPE)
        .map_err(|e| DataContractError::ValueWrongType(e.to_string()))?;
    let kind = match reference_type {
        "permanentDocument" => DynamicDocumentReferenceKind::Permanent,
        "moderatedDocument" => DynamicDocumentReferenceKind::Moderated,
        other => {
            return Err(refused(format!(
                "{other} refersTo does not take contractIdProperty, documentTypeProperty or \
                 documentTypeRequirements: only a permanentDocument or a moderatedDocument \
                 reference names its target through the referring document, since only those \
                 document types promise their documents stay"
            )))
        }
    };

    // The target is known only when a document is written, so nothing that needs the
    // referenced type at registration can sit beside it
    if let Some(key) = refers_to_map
        .keys()
        .find(|key| key.as_str() != property_names::TYPE && !DYNAMIC_KEYS.contains(&key.as_str()))
    {
        return Err(refused(format!(
            "{reference_type} refersTo naming its target through contractIdProperty and \
             documentTypeProperty does not take {key}: the referenced document type is not \
             known until a document is written"
        )));
    }

    let property = |key: &str| -> Result<String, DataContractError> {
        let value = refers_to_map.get(key).ok_or_else(|| {
            refused(format!(
                "{reference_type} refersTo naming its target through the referring document \
                 needs both contractIdProperty and documentTypeProperty: {key} is missing"
            ))
        })?;
        word_name(value, &format!("{reference_type} refersTo {key}"))
    };
    let contract_id_property = property(property_names::CONTRACT_ID_PROPERTY)?;
    let document_type_property = property(property_names::DOCUMENT_TYPE_PROPERTY)?;
    if contract_id_property == document_type_property {
        return Err(refused(format!(
            "{reference_type} refersTo contractIdProperty and documentTypeProperty name the \
             same property, \"{contract_id_property}\""
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
        .find(|key| !matches!(key.as_str(), "properties" | "require"))
    {
        return Err(refused(format!(
            "documentTypeRequirements does not take {key}: it holds properties and require"
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
            Ok((slot_name.clone(), parse_slot(slot_name, slot_value)?))
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

    Ok(DocumentTypeRequirements { slots, require })
}

fn parse_slot(slot_name: &str, value: &Value) -> Result<RequiredPropertySlot, DataContractError> {
    let at = || format!("documentTypeRequirements slot \"{slot_name}\"");
    let slot = value
        .to_btree_ref_string_map()
        .map_err(|_| refused(format!("{} must be an object", at())))?;
    if let Some(key) = slot.keys().find(|key| !SLOT_KEYS.contains(&key.as_str())) {
        return Err(refused(format!("{} does not take {key}", at())));
    }

    let name = match slot.get("name") {
        Some(Value::Text(name)) if name == ANY_PROPERTY_NAME => SlotPropertyName::Any,
        Some(Value::Text(name)) if is_word_name(name) => SlotPropertyName::Named(name.clone()),
        Some(from_property @ Value::Map(_)) => {
            let from_property = from_property
                .to_btree_ref_string_map()
                .map_err(|e| DataContractError::ValueWrongType(e.to_string()))?;
            match (from_property.len(), from_property.get("property")) {
                (1, Some(property)) => SlotPropertyName::FromProperty(word_name(
                    property,
                    &format!("{} name property", at()),
                )?),
                _ => {
                    return Err(refused(format!(
                        "{} name must be \"$any\", a property name, or {{ \"property\": <a \
                         string property of the referring document> }}",
                        at()
                    )))
                }
            }
        }
        _ => {
            return Err(refused(format!(
                "{} needs a name: \"$any\", a property name of 1 to 64 letters, digits or \
                 underscores, or {{ \"property\": <a string property of the referring \
                 document> }}",
                at()
            )))
        }
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

    Ok(RequiredPropertySlot {
        name,
        value: value_kind,
        max_bytes,
        distinct_from,
    })
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

    let requirement = if value.is_text() {
        SlotRequirement::Slot(slot(value)?)
    } else {
        let (combinator, operands) = combinator_operands(value).ok_or_else(shape)?;
        let terms = operands
            .iter()
            .map(|operand| {
                if operand.is_text() {
                    return Ok(SlotRequirementTerm::Slot(slot(operand)?));
                }
                let (inner, inner_operands) = combinator_operands(operand).ok_or_else(shape)?;
                // A combinator directly inside itself says what one flat list says
                if inner == combinator {
                    return Err(shape());
                }
                Ok(SlotRequirementTerm::Group(
                    inner_operands
                        .iter()
                        .map(&slot)
                        .collect::<Result<Vec<_>, _>>()?,
                ))
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
            property_names::CONTRACT_ID_PROPERTY,
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
            property_names::DOCUMENT_TYPE_PROPERTY,
            dynamic.document_type_property.as_str(),
        ))
        .chain(
            dynamic
                .requirements
                .iter()
                .flat_map(|requirements| requirements.name_properties())
                .map(|name| ("documentTypeRequirements slot name property", name)),
        );
        for (key, name) in string_sources {
            source(key, name, "a string", |property_type| {
                matches!(property_type, DocumentPropertyType::String(_))
            })?;
        }
    }
    Ok(())
}
