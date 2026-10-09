use crate::data_contract::document_type::property_names;
use crate::data_contract::errors::DataContractError;
use crate::data_contract::serialized_version::property_names as contract_property_names;
use platform_value::{Value, IDENTIFIER_MEDIA_TYPE};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

/// The `type` of the identifier shorthand.
const IDENTIFIER_TYPE: &str = "identifier";
/// The `type` of the fixed-size byte array shorthand.
const BYTES_TYPE: &str = "bytes";
/// How many bytes a `bytes` shorthand holds.
const SIZE: &str = "size";

/// The bytes of an identifier.
const IDENTIFIER_BYTES: u16 = 32;

/// The keywords the long form of a shorthand writes. A property schema may not
/// write any of them beside a shorthand: the shorthand says the value, and a
/// second spelling could only agree with it or contradict it.
const LONG_FORM_KEYWORDS: [&str; 4] = [
    property_names::BYTE_ARRAY,
    property_names::MIN_ITEMS,
    property_names::MAX_ITEMS,
    property_names::CONTENT_MEDIA_TYPE,
];

/// The shorthand a property schema declares.
#[derive(Clone, Copy)]
enum Shorthand {
    Identifier,
    Bytes,
}

/// The value of the last `key` entry of `map`, the entry every other reader of
/// a property schema takes (the parser's string map and the JSON the
/// meta-schema and the validator read both keep the last of a repeated key).
fn last_value<'a>(map: &'a [(Value, Value)], key: &str) -> Option<&'a Value> {
    map.iter()
        .rev()
        .find(|(entry_key, _)| entry_key.as_text() == Some(key))
        .map(|(_, value)| value)
}

fn shorthand_of(map: &[(Value, Value)]) -> Option<Shorthand> {
    match last_value(map, property_names::TYPE)?.as_text()? {
        IDENTIFIER_TYPE => Some(Shorthand::Identifier),
        BYTES_TYPE => Some(Shorthand::Bytes),
        _ => None,
    }
}

/// Queues the property schemas directly below the property schema `map`: the
/// members its `properties` declares and the element schema of its `items`.
/// A `refersTo`, an `enum` or any other keyword's value is never read, so a key
/// named `type` inside one is never taken for a property's type.
fn queue_child_schemas<'a>(map: &'a [(Value, Value)], to_visit: &mut Vec<&'a Value>) {
    for (key, value) in map {
        match key.as_text() {
            Some(property_names::PROPERTIES) => {
                if let Some(members) = value.as_map() {
                    to_visit.extend(members.iter().map(|(_, member)| member));
                }
            }
            Some(property_names::ITEMS) if value.is_map() => to_visit.push(value),
            _ => {}
        }
    }
}

/// Whether a property schema among `roots`, or below one of them, declares a
/// shorthand. Walked without recursion: a schema reaches this before the depth
/// check bounds its nesting.
fn holds_shorthand(mut to_visit: Vec<&Value>) -> bool {
    while let Some(schema) = to_visit.pop() {
        let Some(map) = schema.as_map() else {
            continue;
        };
        if shorthand_of(map).is_some() {
            return true;
        }
        queue_child_schemas(map, &mut to_visit);
    }
    false
}

/// Rewrites every shorthand among the property schemas `to_visit` holds, and
/// below them, in place; each is paired with where it is, for the errors.
/// Walked without recursion, as [`holds_shorthand`], and below the same
/// keywords ([`queue_child_schemas`]).
fn expand_in_place(
    mut to_visit: Vec<(String, &mut Value)>,
    full_validation: bool,
    platform_version: &PlatformVersion,
) -> Result<(), DataContractError> {
    while let Some((location, schema)) = to_visit.pop() {
        let Value::Map(map) = schema else {
            continue;
        };
        if let Some(shorthand) = shorthand_of(map) {
            expand_shorthand(map, shorthand, &location, full_validation, platform_version)?;
        }
        for (key, value) in map {
            match key.as_text() {
                Some(property_names::PROPERTIES) => {
                    if let Value::Map(members) = value {
                        for (member_key, member) in members {
                            let name = member_key.as_text().unwrap_or_default();
                            to_visit.push((format!("{location}.{name}"), member));
                        }
                    }
                }
                Some(property_names::ITEMS) if value.is_map() => {
                    to_visit.push((format!("{location}[]"), value));
                }
                _ => {}
            }
        }
    }
    Ok(())
}

/// Rewrites the property schema `map`, which declares `shorthand`, into its
/// long form, refusing what may not sit beside the shorthand.
fn expand_shorthand(
    map: &mut Vec<(Value, Value)>,
    shorthand: Shorthand,
    location: &str,
    full_validation: bool,
    platform_version: &PlatformVersion,
) -> Result<(), DataContractError> {
    let declared = match shorthand {
        Shorthand::Identifier => IDENTIFIER_TYPE,
        Shorthand::Bytes => BYTES_TYPE,
    };
    if let Some(keyword) = LONG_FORM_KEYWORDS
        .into_iter()
        .find(|keyword| last_value(map, keyword).is_some())
    {
        return Err(DataContractError::InvalidContractStructure(format!(
            "property \"{location}\" declares type \"{declared}\", which writes its own \
             {keyword}: leave out {keyword}, or write the property in full with type \"array\""
        )));
    }

    let size = match shorthand {
        Shorthand::Identifier => {
            if last_value(map, SIZE).is_some() {
                return Err(DataContractError::InvalidContractStructure(format!(
                    "property \"{location}\" declares type \"identifier\", which takes no size: \
                     an identifier is always {IDENTIFIER_BYTES} bytes; a byte array of another \
                     size is type \"bytes\""
                )));
            }
            IDENTIFIER_BYTES
        }
        Shorthand::Bytes => bytes_size(map, location, full_validation, platform_version)?,
    };

    map.retain(|(key, _)| !matches!(key.as_text(), Some(property_names::TYPE | SIZE)));
    map.push((
        Value::Text(property_names::TYPE.to_string()),
        Value::Text("array".to_string()),
    ));
    map.push((
        Value::Text(property_names::BYTE_ARRAY.to_string()),
        Value::Bool(true),
    ));
    map.push((
        Value::Text(property_names::MIN_ITEMS.to_string()),
        Value::U16(size),
    ));
    map.push((
        Value::Text(property_names::MAX_ITEMS.to_string()),
        Value::U16(size),
    ));
    if let Shorthand::Identifier = shorthand {
        map.push((
            Value::Text(property_names::CONTENT_MEDIA_TYPE.to_string()),
            Value::Text(IDENTIFIER_MEDIA_TYPE.to_string()),
        ));
    }
    Ok(())
}

/// The `size` of a `bytes` shorthand: an integer from 1 to
/// `max_field_value_size`, the most bytes any document field may hold, so a
/// larger size could never be filled. The upper bound is read from the tables
/// and, like every such bound, checked under full validation only, so a
/// contract already stored keeps parsing whatever a later table says; a size
/// that is no integer from 1 to 65535 (the range of a byte array's length)
/// can not be expanded at all and is refused on every parse.
fn bytes_size(
    map: &[(Value, Value)],
    location: &str,
    full_validation: bool,
    platform_version: &PlatformVersion,
) -> Result<u16, DataContractError> {
    let max_field_value_size = platform_version.system_limits.max_field_value_size;
    let out_of_range = || {
        DataContractError::InvalidContractStructure(format!(
            "property \"{location}\" declares type \"bytes\" with a size that is not an integer \
             from 1 to {max_field_value_size}, the most bytes a document field may hold"
        ))
    };
    let Some(size) = last_value(map, SIZE) else {
        return Err(DataContractError::InvalidContractStructure(format!(
            "property \"{location}\" declares type \"bytes\" without a size: a byte array of \
             exactly that many bytes, from 1 to {max_field_value_size}"
        )));
    };
    let size = size
        .to_integer::<u16>()
        .ok()
        .filter(|size| *size > 0)
        .ok_or_else(out_of_range)?;
    if full_validation && u32::from(size) > max_field_value_size {
        return Err(out_of_range());
    }
    Ok(size)
}

/// The property schemas of a document type schema, or of an object holding
/// them as one does, each with where it is: the values of its `properties`, by
/// name, and of its `$defs`, as `$defs.<name>`.
fn top_level_schemas(schema: &mut [(Value, Value)]) -> Vec<(String, &mut Value)> {
    let mut schemas = Vec::new();
    for (key, value) in schema {
        let prefix = match key.as_text() {
            Some(property_names::PROPERTIES) => "",
            Some(contract_property_names::DEFINITIONS) => "$defs.",
            _ => continue,
        };
        if let Value::Map(members) = value {
            for (member_key, member) in members {
                let name = member_key.as_text().unwrap_or_default();
                schemas.push((format!("{prefix}{name}"), member));
            }
        }
    }
    schemas
}

#[inline(always)]
pub(super) fn expand_property_type_shorthands_v0(
    schema: &Value,
    full_validation: bool,
    platform_version: &PlatformVersion,
) -> Result<Option<Value>, DataContractError> {
    let Some(map) = schema.as_map() else {
        return Ok(None);
    };
    let mut roots = Vec::new();
    for (key, value) in map {
        if matches!(
            key.as_text(),
            Some(property_names::PROPERTIES | contract_property_names::DEFINITIONS)
        ) {
            if let Some(members) = value.as_map() {
                roots.extend(members.iter().map(|(_, member)| member));
            }
        }
    }
    if !holds_shorthand(roots) {
        return Ok(None);
    }

    let mut expanded = schema.clone();
    if let Value::Map(map) = &mut expanded {
        expand_in_place(top_level_schemas(map), full_validation, platform_version)?;
    }
    Ok(Some(expanded))
}

#[inline(always)]
pub(super) fn expand_schema_defs_property_type_shorthands_v0(
    schema_defs: &BTreeMap<String, Value>,
    full_validation: bool,
    platform_version: &PlatformVersion,
) -> Result<Option<BTreeMap<String, Value>>, DataContractError> {
    if !holds_shorthand(schema_defs.values().collect()) {
        return Ok(None);
    }
    let mut expanded = schema_defs.clone();
    let roots = expanded
        .iter_mut()
        .map(|(name, definition)| (format!("$defs.{name}"), definition))
        .collect();
    expand_in_place(roots, full_validation, platform_version)?;
    Ok(Some(expanded))
}
