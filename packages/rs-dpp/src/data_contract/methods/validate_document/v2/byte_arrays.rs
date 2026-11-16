//! The byte arrays of a document as the document serializer reads them.
//!
//! The schema check reads a document's JSON form, where a byte array is a list of integers
//! whatever integer type each member was written as, so `[1, 2]` passes whether its members
//! are bytes (`Value::U8`) or 64-bit integers (`Value::U64`, as a client converting a
//! JavaScript `Uint8Array` writes them). The serializer, which writes the document to state
//! and its index keys, reads a byte array with [`Value::to_binary_bytes`] and an identifier
//! with [`Value::to_identifier_bytes`], which read a list only when every member is a byte.
//! A document the schema check passed could then fail to serialize, an internal error where
//! the document is the client's mistake. Generation 2 refuses it here instead.

use crate::consensus::basic::json_schema_error::JsonSchemaError;
use crate::consensus::ConsensusError;
use crate::data_contract::document_type::array::ArrayItemType;
use crate::data_contract::document_type::{DocumentProperty, DocumentPropertyType};
use indexmap::IndexMap;
use platform_value::{platform_value, Value};

/// How a byte array or identifier is read where the serializer reads one.
#[derive(Clone, Copy)]
enum BytesReading {
    Binary,
    Identifier,
}

impl BytesReading {
    /// Whether the serializer reads `value` as bytes, as it reads this kind.
    fn reads(self, value: &Value) -> bool {
        match self {
            BytesReading::Binary => value.to_binary_bytes().is_ok(),
            BytesReading::Identifier => value.to_identifier_bytes().is_ok(),
        }
    }
}

/// The refusal of the first byte array or identifier value in a document's `properties`,
/// declared by `declared`, that the document serializer cannot read as bytes, in the order
/// the serializer writes them: declared properties in declared order, the members of an
/// object, the elements of a list. `None` when it reads every one. A null value is written
/// as absent and read as nothing.
pub(super) fn unreadable_byte_array(
    declared: &IndexMap<String, DocumentProperty>,
    properties: &Value,
) -> Option<ConsensusError> {
    let mut path = Vec::new();
    in_object(declared, properties, &mut path)
}

fn in_object(
    declared: &IndexMap<String, DocumentProperty>,
    value: &Value,
    path: &mut Vec<String>,
) -> Option<ConsensusError> {
    let Value::Map(entries) = value else {
        return None;
    };
    declared.iter().find_map(|(name, property)| {
        let member = entries.iter().find_map(|(key, member)| {
            matches!(key, Value::Text(key) if key == name).then_some(member)
        })?;
        path.push(name.clone());
        let refusal = in_property(&property.property_type, member, path);
        path.pop();
        refusal
    })
}

fn in_property(
    property_type: &DocumentPropertyType,
    value: &Value,
    path: &mut Vec<String>,
) -> Option<ConsensusError> {
    match property_type {
        DocumentPropertyType::ByteArray(_) => in_bytes(BytesReading::Binary, value, path),
        DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
            in_bytes(BytesReading::Identifier, value, path)
        }
        DocumentPropertyType::Object(declared) => in_object(declared, value, path),
        DocumentPropertyType::TypedArray(typed_array) => {
            in_elements(value, path, |element, path| {
                in_property(&typed_array.item_type, element, path)
            })
        }
        DocumentPropertyType::Array(ArrayItemType::ByteArray(..)) => {
            in_elements(value, path, |element, path| {
                in_bytes(BytesReading::Binary, element, path)
            })
        }
        DocumentPropertyType::Array(ArrayItemType::Identifier) => {
            in_elements(value, path, |element, path| {
                in_bytes(BytesReading::Identifier, element, path)
            })
        }
        _ => None,
    }
}

fn in_elements(
    value: &Value,
    path: &mut Vec<String>,
    mut in_element: impl FnMut(&Value, &mut Vec<String>) -> Option<ConsensusError>,
) -> Option<ConsensusError> {
    let Value::Array(elements) = value else {
        return None;
    };
    elements.iter().enumerate().find_map(|(index, element)| {
        path.push(index.to_string());
        let refusal = in_element(element, path);
        path.pop();
        refusal
    })
}

fn in_bytes(
    reading: BytesReading,
    value: &Value,
    path: &mut Vec<String>,
) -> Option<ConsensusError> {
    if value.is_null() || reading.reads(value) {
        return None;
    }
    // A list the serializer does not read holds a member that is not a byte: name it
    let non_byte = match value {
        Value::Array(members) => members
            .iter()
            .enumerate()
            .find(|(_, member)| !matches!(member, Value::U8(_))),
        _ => None,
    };
    let (instance_path, summary) = match non_byte {
        Some((index, member)) => {
            path.push(index.to_string());
            let instance_path = pointer(path);
            path.pop();
            let summary = format!(
                "{instance_path} is {member:?}, not a byte: each member of a byte array must \
                 be written as an 8-bit unsigned integer"
            );
            (instance_path, summary)
        }
        None => {
            let instance_path = pointer(path);
            let summary = format!("{instance_path} is {value:?}, which is not read as bytes");
            (instance_path, summary)
        }
    };
    Some(
        JsonSchemaError::new(
            summary,
            "byteArray".to_string(),
            instance_path,
            String::new(),
            platform_value!({ "byteArray": true }),
            String::new(),
        )
        .into(),
    )
}

/// The JSON pointer (RFC 6901) of `path`.
fn pointer(path: &[String]) -> String {
    path.iter()
        .map(|segment| format!("/{}", segment.replace('~', "~0").replace('/', "~1")))
        .collect()
}
