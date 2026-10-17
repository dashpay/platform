//! A document of chosen sizes, for pricing a document type without real
//! values: what a document costs depends on the length of each value and on
//! which optional values it carries, not on the values themselves. Each
//! variable-size value defaults to its middle size, the size Drive's own fee
//! estimates assume (`DocumentTypeV0Methods::estimated_size`), and each
//! optional value to present.

use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeBasicMethods;
use dpp::data_contract::document_type::{DocumentProperty, DocumentPropertyType, DocumentTypeRef};
use dpp::data_contract::DataContract;
use dpp::document::{Document, DocumentV0};
use dpp::platform_value::btreemap_extensions::BTreeValueMapInsertionPathHelper;
use dpp::platform_value::{Identifier, Value};
use dpp::version::PlatformVersion;
use indexmap::IndexMap;
use std::collections::{BTreeMap, BTreeSet};

/// The block time the document is created at: its timestamps are 8 bytes
/// whatever the time.
const CREATED_AT_MS: u64 = 1_750_000_000_000;

/// What to put in one field.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FieldChoice {
    /// Whether an optional field is present; required fields always are.
    pub present: Option<bool>,
    /// The length of a variable-size value: characters of a string, bytes
    /// of a byte array, elements of an array.
    pub length: Option<u32>,
}

/// How a field of the priced document was filled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldSize {
    /// The field's path (`a.b` for a property of an object).
    pub path: String,
    /// The field's type, as a word: `string`, `byteArray`, `array`,
    /// `identifier`, `integer`, `number`, `boolean`, `date`, `object`.
    pub kind: &'static str,
    /// Whether the field is optional.
    pub optional: bool,
    /// Whether the document carries it.
    pub present: bool,
    /// The length used, for a variable-size field (not one of a single size).
    pub length: Option<u32>,
    /// The least length the schema allows, for a variable-size field.
    pub min_length: Option<u32>,
    /// The most length the schema allows, for a variable-size field.
    pub max_length: Option<u32>,
}

fn kind(property_type: &DocumentPropertyType) -> &'static str {
    match property_type {
        DocumentPropertyType::String(_) => "string",
        DocumentPropertyType::ByteArray(_) => "byteArray",
        DocumentPropertyType::Array(_)
        | DocumentPropertyType::VariableTypeArray(_)
        | DocumentPropertyType::TypedArray(_) => "array",
        DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
            "identifier"
        }
        DocumentPropertyType::F64 => "number",
        DocumentPropertyType::Boolean => "boolean",
        DocumentPropertyType::Date => "date",
        DocumentPropertyType::Object(_) => "object",
        _ => "integer",
    }
}

/// The length bounds of a variable-size type, and the middle between them;
/// `None` for a type of one size.
fn bounds(
    property_type: &DocumentPropertyType,
    platform_version: &PlatformVersion,
) -> Option<(u32, Option<u32>, u32)> {
    let variable = |(min, max, middle): (u32, Option<u32>, u32)| {
        (max != Some(min)).then_some((min, max, middle))
    };
    match property_type {
        DocumentPropertyType::String(_) | DocumentPropertyType::ByteArray(_) => {
            let min = u32::from(property_type.min_size().unwrap_or(0));
            let max = property_type.max_size().map(u32::from);
            let middle = property_type
                .middle_size(platform_version)
                .map(u32::from)
                .unwrap_or_else(|| min.max(16));
            variable((min, max, middle))
        }
        DocumentPropertyType::TypedArray(array) => {
            let min = u32::from(array.min_items.unwrap_or(0));
            let max = u32::from(array.max_items);
            variable((min, Some(max), (min + max) / 2))
        }
        _ => None,
    }
}

/// A value of `property_type` of `length` (for a variable-size type).
pub(crate) fn value_of(
    property_type: &DocumentPropertyType,
    length: u32,
    platform_version: &PlatformVersion,
) -> Value {
    match property_type {
        DocumentPropertyType::U128 => Value::U128(1),
        DocumentPropertyType::I128 => Value::I128(1),
        DocumentPropertyType::U64 => Value::U64(1),
        DocumentPropertyType::I64 => Value::I64(1),
        DocumentPropertyType::U32 | DocumentPropertyType::KeyIdWithReference(_) => Value::U32(1),
        DocumentPropertyType::I32 => Value::I32(1),
        DocumentPropertyType::U16 => Value::U16(1),
        DocumentPropertyType::I16 => Value::I16(1),
        DocumentPropertyType::U8 => Value::U8(1),
        DocumentPropertyType::I8 => Value::I8(1),
        DocumentPropertyType::F64 => Value::Float(1.0),
        DocumentPropertyType::Boolean => Value::Bool(true),
        DocumentPropertyType::Date => Value::Float((CREATED_AT_MS / 1000) as f64),
        DocumentPropertyType::String(_) => Value::Text("a".repeat(length as usize)),
        DocumentPropertyType::ByteArray(_) => {
            let bytes = vec![1u8; length as usize];
            if property_type.min_size() == property_type.max_size() {
                match bytes.len() {
                    20 => Value::Bytes20([1; 20]),
                    32 => Value::Bytes32([1; 32]),
                    36 => Value::Bytes36([1; 36]),
                    _ => Value::Bytes(bytes),
                }
            } else {
                Value::Bytes(bytes)
            }
        }
        DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
            Value::Identifier([1; 32])
        }
        DocumentPropertyType::TypedArray(array) => {
            let item_length = match bounds(&array.item_type, platform_version) {
                // At least one unit when the element can hold one: the ordinal is
                // stamped into the element's last bytes, and empty elements are all
                // one key, where a document of `maxItems` one-unit elements writes as
                // many (a string of `maxLength` 1 has a middle length of 0).
                Some((_, max, middle)) => middle.max(u32::from(max.is_none_or(|max| max > 0))),
                None => u32::from(array.item_type.min_size().unwrap_or_default()),
            };
            // Distinct elements of one size: an index naming the array keys
            // the document once per distinct element, so repeats would price
            // one entry where a document of distinct elements writes many.
            Value::Array(
                (0..length)
                    .map(|ordinal| {
                        distinct_element(
                            value_of(&array.item_type, item_length, platform_version),
                            ordinal,
                        )
                    })
                    .collect(),
            )
        }
        DocumentPropertyType::Array(_) | DocumentPropertyType::VariableTypeArray(_) => {
            Value::Array(vec![])
        }
        DocumentPropertyType::Object(_) => Value::Map(vec![]),
    }
}

/// The 94 printable ASCII characters other than the space: more symbols than
/// an indexed typed array holds elements, so a one-character string element
/// keeps every ordinal apart, and each is one UTF-8 byte.
const PRINTABLE_ASCII: &[u8] =
    b"!\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~";

/// `element`, a value `value_of` built, made the `ordinal`th of a list of
/// distinct values of the same size: an integer or a number counts up from it
/// (wrapping within its width), a string or byte array takes the ordinal in its
/// last bytes, and a boolean alternates. Its stored size does not change.
fn distinct_element(element: Value, ordinal: u32) -> Value {
    fn stamp(bytes: &mut [u8], ordinal: u32, alphabet: Option<&[u8]>) {
        let mut rest = ordinal;
        for byte in bytes.iter_mut().rev() {
            match alphabet {
                Some(alphabet) => {
                    let radix = alphabet.len() as u32;
                    *byte = alphabet[(rest % radix) as usize];
                    rest /= radix;
                }
                None => {
                    *byte = (rest & 0xff) as u8;
                    rest >>= 8;
                }
            }
            if rest == 0 {
                break;
            }
        }
    }
    match element {
        Value::U128(value) => Value::U128(value.wrapping_add(u128::from(ordinal))),
        Value::I128(value) => Value::I128(value.wrapping_add(i128::from(ordinal))),
        Value::U64(value) => Value::U64(value.wrapping_add(u64::from(ordinal))),
        Value::I64(value) => Value::I64(value.wrapping_add(i64::from(ordinal))),
        Value::U32(value) => Value::U32(value.wrapping_add(ordinal)),
        Value::I32(value) => Value::I32(value.wrapping_add(ordinal as i32)),
        Value::U16(value) => Value::U16(value.wrapping_add(ordinal as u16)),
        Value::I16(value) => Value::I16(value.wrapping_add(ordinal as i16)),
        Value::U8(value) => Value::U8(value.wrapping_add(ordinal as u8)),
        Value::I8(value) => Value::I8(value.wrapping_add(ordinal as i8)),
        Value::Float(value) => Value::Float(value + f64::from(ordinal)),
        Value::Bool(_) => Value::Bool(ordinal.is_multiple_of(2)),
        Value::Text(text) => {
            let mut bytes = text.into_bytes();
            stamp(&mut bytes, ordinal, Some(PRINTABLE_ASCII));
            Value::Text(String::from_utf8(bytes).unwrap_or_default())
        }
        Value::Bytes(mut bytes) => {
            stamp(&mut bytes, ordinal, None);
            Value::Bytes(bytes)
        }
        Value::Bytes20(mut bytes) => {
            stamp(&mut bytes, ordinal, None);
            Value::Bytes20(bytes)
        }
        Value::Bytes32(mut bytes) => {
            stamp(&mut bytes, ordinal, None);
            Value::Bytes32(bytes)
        }
        Value::Bytes36(mut bytes) => {
            stamp(&mut bytes, ordinal, None);
            Value::Bytes36(bytes)
        }
        Value::Identifier(mut bytes) => {
            stamp(&mut bytes, ordinal, None);
            Value::Identifier(bytes)
        }
        other => other,
    }
}

/// Fills `properties` into `data`, recording each field in `fields`. A
/// transient property, by top-level name, is judged on the transition and
/// never stored (`drop_transient_values`): it is neither filled nor listed.
fn fill(
    properties: &IndexMap<String, DocumentProperty>,
    prefix: &str,
    choices: &BTreeMap<String, FieldChoice>,
    transient: &BTreeSet<String>,
    data: &mut BTreeMap<String, Value>,
    fields: &mut Vec<FieldSize>,
    platform_version: &PlatformVersion,
) {
    for (name, property) in properties {
        if prefix.is_empty() && transient.contains(name) {
            continue;
        }
        let path = format!("{prefix}{name}");
        let choice = choices.get(&path).copied().unwrap_or_default();
        let optional = !property.required;
        let present = !optional || choice.present.unwrap_or(true);
        let bounds = bounds(&property.property_type, platform_version);
        let length = bounds.map(|(min, max, middle)| {
            let chosen = choice.length.unwrap_or(middle).max(min);
            max.map_or(chosen, |max| chosen.min(max))
        });
        fields.push(FieldSize {
            path: path.clone(),
            kind: kind(&property.property_type),
            optional,
            present,
            length,
            min_length: bounds.map(|(min, _, _)| min),
            max_length: bounds.and_then(|(_, max, _)| max),
        });
        if !present {
            continue;
        }
        let value = match &property.property_type {
            DocumentPropertyType::Object(sub_properties) => {
                let mut sub_data = BTreeMap::new();
                fill(
                    sub_properties,
                    &format!("{path}."),
                    choices,
                    transient,
                    &mut sub_data,
                    fields,
                    platform_version,
                );
                Value::Map(
                    sub_data
                        .into_iter()
                        .map(|(key, value)| (Value::Text(key), value))
                        .collect(),
                )
            }
            property_type => value_of(
                property_type,
                length.unwrap_or_else(|| u32::from(property_type.min_size().unwrap_or_default())),
                platform_version,
            ),
        };
        data.insert(name.clone(), value);
    }
}

/// A document of `document_type` of the sizes `choices` name, owned by a
/// placeholder identity, as a create stores it, with the fields it was
/// filled with.
pub fn sized_document(
    contract: &DataContract,
    document_type: DocumentTypeRef,
    choices: &BTreeMap<String, FieldChoice>,
    platform_version: &PlatformVersion,
) -> Result<(Document, Vec<FieldSize>), Error> {
    if let Some(unknown) = choices
        .keys()
        .find(|path| !document_type.flattened_properties().contains_key(*path))
    {
        return Err(Error::Drive(DriveError::InvalidInput(format!(
            "document type {} has no field {unknown}",
            document_type.name()
        ))));
    }
    let mut data = BTreeMap::new();
    let mut fields = Vec::new();
    fill(
        document_type.properties(),
        "",
        choices,
        document_type.transient_fields(),
        &mut data,
        &mut fields,
        platform_version,
    );
    // Integers are sampled as 1, which for an integer-range source can sit in
    // the clamped bottom window and so price fewer windows than a typical
    // document writes. Such a source is sampled clear of the bottom instead,
    // so the estimate prices the grid's full fan-out.
    for index in document_type.indexes().values() {
        if let Some(transform) = &index.integer_range {
            if let Some(value) = transform.key_type.value_of(transform.full_fan_out_value()) {
                data.insert_at_path(&transform.source, value)?;
            }
        }
    }

    let owner_id = Identifier::from([1; 32]);
    let required = document_type.required_fields();
    let time = |field: &str| required.contains(field).then_some(CREATED_AT_MS);
    let height = |field: &str| required.contains(field).then_some(1u64);
    let core_height = |field: &str| required.contains(field).then_some(1u32);
    let creator_id = document_type
        .should_use_creator_id(
            contract.system_version_type(),
            contract.config().version(),
            platform_version,
        )?
        .then_some(owner_id);
    let document = DocumentV0 {
        contract_version: None,
        id: Identifier::from([2; 32]),
        owner_id,
        properties: data,
        revision: document_type.initial_revision(),
        created_at: time("$createdAt"),
        updated_at: time("$updatedAt"),
        transferred_at: time("$transferredAt"),
        created_at_block_height: height("$createdAtBlockHeight"),
        updated_at_block_height: height("$updatedAtBlockHeight"),
        transferred_at_block_height: height("$transferredAtBlockHeight"),
        created_at_core_block_height: core_height("$createdAtCoreBlockHeight"),
        updated_at_core_block_height: core_height("$updatedAtCoreBlockHeight"),
        transferred_at_core_block_height: core_height("$transferredAtCoreBlockHeight"),
        creator_id,
        moderated_at: None,
        moderated_by: None,
    };
    Ok((document.into(), fields))
}
