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

/// How property schemas sit below one key of an object holding them.
enum Children {
    /// The values of a map (`properties`, `$defs`): a property schema each,
    /// named by its key.
    Members,
    /// The value itself (`items`): the element schema of a typed array.
    Element,
}

/// The property schemas below `key` of a property schema: the members its
/// `properties` declares and the element schema of its `items`. The one rule
/// both walks follow. A `refersTo`, an `enum` or any other keyword's value is
/// never read, so a key named `type` inside one is never taken for a
/// property's type.
fn children_below(key: &Value, value: &Value) -> Option<Children> {
    match key.as_text()? {
        property_names::PROPERTIES if value.is_map() => Some(Children::Members),
        property_names::ITEMS if value.is_map() => Some(Children::Element),
        _ => None,
    }
}

/// The property schemas below `key` at the top of a document type schema, or
/// of an object holding them as one does (the root schema built from one, a
/// `$defs` wrapper): the members of its `properties`, named as they are, and
/// of its `$defs`, named `$defs.<name>`. Returns the prefix of their names.
fn top_level_prefix(key: &Value, value: &Value) -> Option<&'static str> {
    match key.as_text()? {
        property_names::PROPERTIES if value.is_map() => Some(""),
        contract_property_names::DEFINITIONS if value.is_map() => Some("$defs."),
        _ => None,
    }
}

/// The values of the map `value`, none when it is no map.
fn members(value: &Value) -> impl Iterator<Item = &Value> {
    value
        .as_map()
        .into_iter()
        .flatten()
        .map(|(_, member)| member)
}

/// Whether a property schema among `to_visit`, or below one of them, declares
/// a shorthand. The walk keeps its own stack and adds no recursion; it does
/// not bound the nesting either, which is the depth check's job, later.
fn holds_shorthand(mut to_visit: Vec<&Value>) -> bool {
    while let Some(schema) = to_visit.pop() {
        let Some(map) = schema.as_map() else {
            continue;
        };
        if shorthand_of(map).is_some() {
            return true;
        }
        for (key, value) in map {
            match children_below(key, value) {
                Some(Children::Members) => to_visit.extend(members(value)),
                Some(Children::Element) => to_visit.push(value),
                None => {}
            }
        }
    }
    false
}

/// Rewrites every shorthand among the property schemas `to_visit` holds, and
/// below them, in place; each is paired with where it is, for the errors.
/// Walks as [`holds_shorthand`] does, below the keys [`children_below`] names.
///
/// Only full validation refuses. Without it the schema is one being read, not
/// registered: a contract read back from state, which was judged when it was
/// written, or one only parsed (check_tx, a client). A shorthand that can not
/// be rewritten there is left as sent. In a contract stored before protocol
/// version 14 it can only sit where no reader looks (the definitions of a
/// contract without document types, an entry a repeated key shadows), since
/// every earlier meta-schema and parser refuses both type names wherever they
/// read one; anywhere else the parser still refuses it as an unknown type.
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
            // `expand_shorthand` refuses before it changes anything, so a
            // shorthand it refuses stays as sent
            match expand_shorthand(map, shorthand, &location, full_validation, platform_version) {
                Ok(()) => {}
                Err(_) if !full_validation => {}
                Err(error) => return Err(error),
            }
        }
        for (key, value) in map {
            match children_below(key, value) {
                Some(Children::Members) => {
                    if let Value::Map(members) = value {
                        for (member_key, member) in members {
                            let name = member_key.as_text().unwrap_or_default();
                            to_visit.push((format!("{location}.{name}"), member));
                        }
                    }
                }
                Some(Children::Element) => to_visit.push((format!("{location}[]"), value)),
                None => {}
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
/// contract already stored keeps parsing whatever a later table says; without
/// full validation the size only has to fit a byte array's length (65535), and
/// a refusal there is not reported ([`expand_in_place`]).
fn bytes_size(
    map: &[(Value, Value)],
    location: &str,
    full_validation: bool,
    platform_version: &PlatformVersion,
) -> Result<u16, DataContractError> {
    let max_size = if full_validation {
        u16::try_from(platform_version.system_limits.max_field_value_size).unwrap_or(u16::MAX)
    } else {
        u16::MAX
    };
    let Some(size) = last_value(map, SIZE) else {
        return Err(DataContractError::InvalidContractStructure(format!(
            "property \"{location}\" declares type \"bytes\" without a size: a byte array of \
             exactly that many bytes, from 1 to {max_size}"
        )));
    };
    size.to_integer::<u16>()
        .ok()
        .filter(|size| (1..=max_size).contains(size))
        .ok_or_else(|| {
            DataContractError::InvalidContractStructure(format!(
                "property \"{location}\" declares type \"bytes\" with a size that is not an \
                 integer from 1 to {max_size}, the most bytes a document field may hold"
            ))
        })
}

/// The property schemas at the top of `schema`, as [`top_level_prefix`]
/// names them, each with where it is.
fn top_level_schemas(schema: &mut [(Value, Value)]) -> Vec<(String, &mut Value)> {
    let mut schemas = Vec::new();
    for (key, value) in schema {
        let Some(prefix) = top_level_prefix(key, value) else {
            continue;
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
    let roots = map
        .iter()
        .filter(|(key, value)| top_level_prefix(key, value).is_some())
        .flat_map(|(_, value)| members(value))
        .collect();
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
