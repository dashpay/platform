use super::contested_index_identifier;
use crate::data_contract::document_type::property::{DocumentProperty, DocumentPropertyType};
use crate::data_contract::document_type::Index;
use indexmap::IndexMap;
use platform_value::Value;
use std::collections::BTreeMap;

impl Index {
    /// The index values of `data`, each written the one way its property's type writes every
    /// value that shares its key in the paths of a vote poll.
    #[inline(always)]
    pub(super) fn extract_contested_values_v1(
        &self,
        data: &BTreeMap<String, Value>,
        document_properties: &IndexMap<String, DocumentProperty>,
    ) -> Vec<Value> {
        self.properties
            .iter()
            .zip(self.extract_values(data))
            .map(
                |(index_property, value)| match document_properties.get(&index_property.name) {
                    Some(property) => {
                        canonical_contested_index_value(&property.property_type, value)
                    }
                    // Not a top-level property of the type: a contested create reads its
                    // index values from the document's top-level properties, so it finds
                    // no value here
                    None => value,
                },
            )
            .collect()
    }
}

/// `value` as a property of `property_type` writes every value with the same key in the paths
/// of a vote poll (`DocumentPropertyType::encode_value_for_tree_keys`), which is also the
/// value a query of the contest decodes from that key. A value the type does not key (one of
/// another kind, or an integer out of the type's range) is returned as it is: no contest holds
/// it.
///
/// Every type is listed, so that a new type states the form it takes.
fn canonical_contested_index_value(property_type: &DocumentPropertyType, value: Value) -> Value {
    if value.is_null() {
        return value;
    }
    let canonical = match property_type {
        // An integer in any integer form is keyed by its number alone
        DocumentPropertyType::U128 => value.to_integer().ok().map(Value::U128),
        DocumentPropertyType::I128 => value.to_integer().ok().map(Value::I128),
        DocumentPropertyType::U64 => value.to_integer().ok().map(Value::U64),
        DocumentPropertyType::I64 => value.to_integer().ok().map(Value::I64),
        DocumentPropertyType::U32 | DocumentPropertyType::KeyIdWithReference(_) => {
            value.to_integer().ok().map(Value::U32)
        }
        DocumentPropertyType::I32 => value.to_integer().ok().map(Value::I32),
        DocumentPropertyType::U16 => value.to_integer().ok().map(Value::U16),
        DocumentPropertyType::I16 => value.to_integer().ok().map(Value::I16),
        DocumentPropertyType::U8 => value.to_integer().ok().map(Value::U8),
        DocumentPropertyType::I8 => value.to_integer().ok().map(Value::I8),
        // A date is keyed as its timestamp
        DocumentPropertyType::Date => value.to_integer().ok().map(Value::U64),
        // A float is keyed by its bits, and an integer as the float it converts to
        DocumentPropertyType::F64 => value.to_float().ok().map(Value::Float),
        // Only the empty string and `"\0"` share a key, `[0]`
        DocumentPropertyType::String(_) => match &value {
            Value::Text(text) if text == "\0" => Some(Value::Text(String::new())),
            _ => None,
        },
        // Bytes in any form are keyed by the bytes alone, and no bytes by the empty key, as
        // a missing value is
        DocumentPropertyType::ByteArray(_) => value.to_binary_bytes().ok().map(|bytes| {
            if bytes.is_empty() {
                Value::Null
            } else {
                Value::Bytes(bytes)
            }
        }),
        // The forms protocol version 14 already wrote as an identifier, then every other
        // form keyed by the same 32 bytes
        DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
            contested_index_identifier(&value)
                .or_else(|| {
                    value
                        .to_identifier_bytes()
                        .ok()
                        .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
                })
                .map(Value::Identifier)
        }
        // Keyed only as a boolean
        DocumentPropertyType::Boolean => None,
        // Never keyed whole: no index holds an object, and a contested index holds no array
        DocumentPropertyType::Object(_)
        | DocumentPropertyType::Array(_)
        | DocumentPropertyType::VariableTypeArray(_)
        | DocumentPropertyType::TypedArray(_) => None,
    };
    canonical.unwrap_or(value)
}
