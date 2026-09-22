use crate::data_contract::document_type::property::DocumentPropertyType;
use crate::data_contract::document_type::property_names;
use crate::data_contract::errors::DataContractError;
use crate::ProtocolError;
use byteorder::{BigEndian, ReadBytesExt};
use integer_encoding::VarInt;
use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::Value;
use rand::distributions::{Alphanumeric, Standard};
use rand::rngs::StdRng;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::BufReader;

/// The element type of an array property: what one element encodes to inside
/// the array's inline value (a count followed by the elements). Produced by the
/// parser of a typed scalar array (`type: array` with an `items` schema,
/// meta-schema v3) through [`ArrayItemType::try_from_item_schema`]; only
/// scalars are elements, an object or a typed array as an item is refused
/// there. `Date` has no schema spelling (a user property cannot be declared a
/// date) and is kept for the codec's completeness.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(into = "ArrayItemTypeRepr", from = "ArrayItemTypeRepr")]
pub enum ArrayItemType {
    Integer,
    Number,
    String(Option<usize>, Option<usize>),
    ByteArray(Option<usize>, Option<usize>),
    Identifier,
    Boolean,
    Date,
}

// Internal-`$type` serde shape. Mixed unit + 2-tuple variants, so a
// struct-variant Repr (serde can't auto-internal-tag tuple variants). Unit
// variants -> `{"$type":"integer"}`; the tuple variants get named size bounds
// (`#[serde(default)]` so an omitted bound deserializes as `None`). Serde-only
// type (no bincode); its on-wire form is exercised solely by these tests —
// document-schema parsing goes through `TryFrom<&Value>`, not serde.
#[derive(Serialize, Deserialize)]
#[serde(tag = "$type", rename_all = "camelCase")]
enum ArrayItemTypeRepr {
    Integer,
    Number,
    String {
        #[serde(default, rename = "minLength")]
        min_length: Option<usize>,
        #[serde(default, rename = "maxLength")]
        max_length: Option<usize>,
    },
    ByteArray {
        #[serde(default, rename = "minSize")]
        min_size: Option<usize>,
        #[serde(default, rename = "maxSize")]
        max_size: Option<usize>,
    },
    Identifier,
    Boolean,
    Date,
}

impl From<ArrayItemType> for ArrayItemTypeRepr {
    fn from(t: ArrayItemType) -> Self {
        match t {
            ArrayItemType::Integer => Self::Integer,
            ArrayItemType::Number => Self::Number,
            ArrayItemType::String(min_length, max_length) => Self::String {
                min_length,
                max_length,
            },
            ArrayItemType::ByteArray(min_size, max_size) => Self::ByteArray { min_size, max_size },
            ArrayItemType::Identifier => Self::Identifier,
            ArrayItemType::Boolean => Self::Boolean,
            ArrayItemType::Date => Self::Date,
        }
    }
}

impl From<ArrayItemTypeRepr> for ArrayItemType {
    fn from(r: ArrayItemTypeRepr) -> Self {
        match r {
            ArrayItemTypeRepr::Integer => Self::Integer,
            ArrayItemTypeRepr::Number => Self::Number,
            ArrayItemTypeRepr::String {
                min_length,
                max_length,
            } => Self::String(min_length, max_length),
            ArrayItemTypeRepr::ByteArray { min_size, max_size } => {
                Self::ByteArray(min_size, max_size)
            }
            ArrayItemTypeRepr::Identifier => Self::Identifier,
            ArrayItemTypeRepr::Boolean => Self::Boolean,
            ArrayItemTypeRepr::Date => Self::Date,
        }
    }
}

impl ArrayItemType {
    /// Sanitize a value to match the expected array item type
    pub fn sanitize_value_mut(&self, value: &mut Value) {
        match (self, value.clone()) {
            // Convert hex or base64 strings to byte arrays for ByteArray items
            (ArrayItemType::ByteArray(min_size, max_size), Value::Text(str_value)) => {
                // Try to decode the string
                let decoded_bytes = if let Ok(bytes) = hex::decode(str_value.as_str()) {
                    Some(bytes)
                } else {
                    // If hex fails, try base64 decoding
                    use base64::{engine::general_purpose, Engine as _};
                    general_purpose::STANDARD.decode(str_value.as_str()).ok()
                };

                if let Some(bytes) = decoded_bytes {
                    let byte_len = bytes.len();

                    // Check if the decoded bytes meet the size constraints
                    let size_ok = match (*min_size, *max_size) {
                        (Some(min), Some(max)) => byte_len >= min && byte_len <= max,
                        (Some(min), None) => byte_len >= min,
                        (None, Some(max)) => byte_len <= max,
                        (None, None) => true,
                    };

                    if size_ok {
                        // Use specific byte array types for exact sizes
                        match bytes.len() {
                            20 => {
                                if let Ok(arr) = bytes.try_into() {
                                    *value = Value::Bytes20(arr);
                                }
                            }
                            32 => {
                                if let Ok(arr) = bytes.try_into() {
                                    *value = Value::Bytes32(arr);
                                }
                            }
                            36 => {
                                if let Ok(arr) = bytes.try_into() {
                                    *value = Value::Bytes36(arr);
                                }
                            }
                            _ => {
                                *value = Value::Bytes(bytes);
                            }
                        }
                    }
                    // If size constraints are not met, leave the value as is
                }
                // If decoding fails, leave the value as is (validation will catch it later)
            }

            // Convert hex or base58 strings to identifiers for Identifier items
            (ArrayItemType::Identifier, Value::Text(str_value)) => {
                use platform_value::Identifier;
                // First try base58 decoding (most common for identifiers)
                if let Ok(id) = Identifier::from_string(
                    &str_value,
                    platform_value::string_encoding::Encoding::Base58,
                ) {
                    *value = Value::Identifier(id.into_buffer());
                } else {
                    // If base58 fails, try hex decoding
                    // Remove any spaces or non-hex characters
                    let clean_hex: String = str_value
                        .chars()
                        .filter(|c| c.is_ascii_hexdigit())
                        .collect();

                    // Try to decode hex string to identifier
                    if clean_hex.len() == 64 {
                        // 32 bytes = 64 hex chars
                        if let Ok(bytes) = hex::decode(&clean_hex) {
                            if let Ok(id) = Identifier::try_from(bytes.as_slice()) {
                                *value = Value::Identifier(id.into_buffer());
                            }
                        }
                    }
                }
                // If both conversions fail, leave the value as is (validation will catch it later)
            }

            // Convert positive I64 to U64 for Date items
            (ArrayItemType::Date, Value::I64(timestamp)) if timestamp >= 0 => {
                *value = Value::U64(timestamp as u64);
            }

            // Ensure integers are converted properly
            (ArrayItemType::Integer, Value::U64(n)) if n <= i64::MAX as u64 => {
                *value = Value::I64(n as i64);
            }
            (ArrayItemType::Integer, Value::U32(n)) => {
                *value = Value::I64(n as i64);
            }
            (ArrayItemType::Integer, Value::U16(n)) => {
                *value = Value::I64(n as i64);
            }
            (ArrayItemType::Integer, Value::U8(n)) => {
                *value = Value::I64(n as i64);
            }

            // Ensure numbers are converted to F64
            (ArrayItemType::Number, Value::I64(n)) => {
                *value = Value::Float(n as f64);
            }
            (ArrayItemType::Number, Value::U64(n)) => {
                *value = Value::Float(n as f64);
            }
            (ArrayItemType::Number, Value::I32(n)) => {
                *value = Value::Float(n as f64);
            }
            (ArrayItemType::Number, Value::U32(n)) => {
                *value = Value::Float(n as f64);
            }
            (ArrayItemType::Number, Value::I16(n)) => {
                *value = Value::Float(n as f64);
            }
            (ArrayItemType::Number, Value::U16(n)) => {
                *value = Value::Float(n as f64);
            }
            (ArrayItemType::Number, Value::I8(n)) => {
                *value = Value::Float(n as f64);
            }
            (ArrayItemType::Number, Value::U8(n)) => {
                *value = Value::Float(n as f64);
            }

            // For all other cases, leave the value as is
            _ => {}
        }
    }

    pub fn encode_value_with_size(&self, value: Value) -> Result<Vec<u8>, ProtocolError> {
        match self {
            ArrayItemType::String(_, _) => {
                if let Value::Text(value) = value {
                    let vec = value.into_bytes();
                    let mut r_vec = vec.len().encode_var_vec();
                    r_vec.extend(vec);
                    Ok(r_vec)
                } else {
                    Err(get_field_type_matching_error())
                }
            }
            ArrayItemType::Date => {
                let value_as_f64 = value.into_float().map_err(ProtocolError::ValueError)?;
                let value_bytes = value_as_f64.to_be_bytes().to_vec();
                Ok(value_bytes)
            }
            ArrayItemType::Integer => {
                let value_as_i64: i64 = value.into_integer().map_err(ProtocolError::ValueError)?;

                let value_bytes = value_as_i64.to_be_bytes().to_vec();
                Ok(value_bytes)
            }
            ArrayItemType::Number => {
                let value_as_f64 = value.into_float().map_err(ProtocolError::ValueError)?;
                let value_bytes = value_as_f64.to_be_bytes().to_vec();
                Ok(value_bytes)
            }
            ArrayItemType::ByteArray(_, _) => {
                let mut bytes = value.into_binary_bytes()?;

                let mut r_vec = bytes.len().encode_var_vec();
                r_vec.append(&mut bytes);
                Ok(r_vec)
            }
            ArrayItemType::Identifier => {
                let mut bytes = value.into_identifier_bytes()?;

                let mut r_vec = bytes.len().encode_var_vec();
                r_vec.append(&mut bytes);
                Ok(r_vec)
            }
            ArrayItemType::Boolean => {
                let value_as_boolean = value.as_bool().ok_or_else(get_field_type_matching_error)?;
                if value_as_boolean {
                    Ok(vec![1]) // 1 is true
                } else {
                    Ok(vec![0]) // 2 is false
                }
            }
        }
    }

    pub fn encode_value_ref_with_size(&self, value: &Value) -> Result<Vec<u8>, ProtocolError> {
        match self {
            ArrayItemType::String(_, _) => {
                let value_as_text = value.as_text().ok_or_else(get_field_type_matching_error)?;
                let vec = value_as_text.as_bytes().to_vec();
                let mut r_vec = vec.len().encode_var_vec();
                r_vec.extend(vec);
                Ok(r_vec)
            }
            ArrayItemType::Date => {
                let value_as_f64 = value.to_float().map_err(ProtocolError::ValueError)?;
                let value_bytes = value_as_f64.to_be_bytes().to_vec();
                Ok(value_bytes)
            }
            ArrayItemType::Integer => {
                let value_as_i64: i64 = value.to_integer().map_err(ProtocolError::ValueError)?;
                let value_bytes = value_as_i64.to_be_bytes().to_vec();
                Ok(value_bytes)
            }
            ArrayItemType::Number => {
                let value_as_f64 = value.to_float().map_err(ProtocolError::ValueError)?;
                let value_bytes = value_as_f64.to_be_bytes().to_vec();
                Ok(value_bytes)
            }
            ArrayItemType::ByteArray(_, _) => {
                let mut bytes = value.to_binary_bytes()?;

                let mut r_vec = bytes.len().encode_var_vec();
                r_vec.append(&mut bytes);
                Ok(r_vec)
            }
            ArrayItemType::Identifier => {
                let mut bytes = value.to_identifier_bytes()?;

                let mut r_vec = bytes.len().encode_var_vec();
                r_vec.append(&mut bytes);
                Ok(r_vec)
            }
            ArrayItemType::Boolean => {
                let value_as_boolean = value.as_bool().ok_or_else(get_field_type_matching_error)?;
                // 0 means does not exist
                if value_as_boolean {
                    Ok(vec![1]) // 1 is true
                } else {
                    Ok(vec![0]) // 2 is false
                }
            }
        }
    }
}

/// The `contentMediaType` value that makes a byte array an identifier.
const IDENTIFIER_CONTENT_MEDIA_TYPE: &str = "application/x.dash.dpp.identifier";

/// The longest string an item without `maxLength` is sized and generated at,
/// the same default the parent string property type uses.
const DEFAULT_MAX_STRING_LENGTH: u16 = 16383;

/// Bytes of the varint that prefixes a length or a count.
fn varint_len(value: u16) -> u16 {
    value.required_space() as u16
}

impl ArrayItemType {
    /// Parses the `items` schema of a typed scalar array: one scalar property
    /// schema, read with the same keywords the parent parser reads for a
    /// top-level property of that type (`minLength` / `maxLength` on a string,
    /// `byteArray` with `minItems` / `maxItems` and the identifier
    /// `contentMediaType` on a byte array). The bounds a keyword declares on
    /// the elements (`minimum`, `pattern`, and so on) stay in the schema and
    /// are enforced on the document by the JSON schema validator.
    ///
    /// Refused as items, with a clear error: an object, an array that is not
    /// a byte array (arrays of arrays), a `$ref`, and `refersTo`. A reference
    /// on the items of an identifier array is a separate follow-up; this
    /// parser reads the whole item schema so it can carry one later.
    pub fn try_from_item_schema(
        item_schema: &BTreeMap<String, &Value>,
    ) -> Result<Self, DataContractError> {
        if item_schema.contains_key(property_names::REFERS_TO) {
            return Err(DataContractError::InvalidContractStructure(
                "array items may not carry refersTo: references on array items are not \
                 supported yet"
                    .to_string(),
            ));
        }
        if item_schema.contains_key(property_names::REF) {
            return Err(DataContractError::InvalidContractStructure(
                "array items must be an inline scalar schema, not a $ref".to_string(),
            ));
        }
        let type_name = item_schema.get_str(property_names::TYPE)?;
        let item_type = match type_name {
            "integer" => ArrayItemType::Integer,
            "number" => ArrayItemType::Number,
            "boolean" => ArrayItemType::Boolean,
            "string" => ArrayItemType::String(
                item_schema.get_optional_integer(property_names::MIN_LENGTH)?,
                item_schema.get_optional_integer(property_names::MAX_LENGTH)?,
            ),
            "array" => {
                match item_schema.get_optional_bool(property_names::BYTE_ARRAY)? {
                    Some(true) => {}
                    Some(false) => {
                        return Err(DataContractError::InvalidContractStructure(
                            "byteArray should always be true if defined".to_string(),
                        ));
                    }
                    None => {
                        return Err(DataContractError::InvalidContractStructure(
                            "arrays of arrays are not supported: an array item that is an \
                             array must be a byte array (byteArray: true)"
                                .to_string(),
                        ));
                    }
                }
                match item_schema.get_optional_str(property_names::CONTENT_MEDIA_TYPE)? {
                    Some(IDENTIFIER_CONTENT_MEDIA_TYPE) => ArrayItemType::Identifier,
                    Some(_) | None => ArrayItemType::ByteArray(
                        item_schema.get_optional_integer(property_names::MIN_ITEMS)?,
                        item_schema.get_optional_integer(property_names::MAX_ITEMS)?,
                    ),
                }
            }
            "object" => {
                return Err(DataContractError::InvalidContractStructure(
                    "arrays of objects are not supported: array items must be a scalar (an \
                     integer, a number, a string, a boolean, a byte array or an identifier)"
                        .to_string(),
                ));
            }
            other => {
                return Err(DataContractError::InvalidContractStructure(format!(
                    "unsupported array item type: {other}"
                )));
            }
        };
        Ok(item_type)
    }
}

impl TryFrom<&Value> for ArrayItemType {
    type Error = DataContractError;

    /// The `items` schema as a value map, see [`ArrayItemType::try_from_item_schema`].
    fn try_from(item_schema: &Value) -> Result<Self, Self::Error> {
        let item_schema = item_schema.to_btree_ref_string_map()?;
        Self::try_from_item_schema(&item_schema)
    }
}

impl ArrayItemType {
    /// The schema type name of the item, as the parent property types name
    /// themselves.
    pub fn name(&self) -> &'static str {
        match self {
            ArrayItemType::Integer => "integer",
            ArrayItemType::Number => "number",
            ArrayItemType::String(_, _) => "string",
            ArrayItemType::ByteArray(_, _) => "byteArray",
            ArrayItemType::Identifier => "identifier",
            ArrayItemType::Boolean => "boolean",
            ArrayItemType::Date => "date",
        }
    }

    /// Reads one element, the mirror of [`Self::encode_value_with_size`]. A
    /// length the element claims for itself is never trusted to size an
    /// allocation: the bytes are read as they arrive and a short input is an
    /// error.
    pub fn read_value_from(&self, buf: &mut BufReader<&[u8]>) -> Result<Value, DataContractError> {
        match self {
            ArrayItemType::String(_, _) => {
                let bytes = DocumentPropertyType::read_varint_value(buf)?;
                let string = String::from_utf8(bytes).map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading string array item from serialized document".to_string(),
                    )
                })?;
                Ok(Value::Text(string))
            }
            ArrayItemType::Date | ArrayItemType::Number => {
                let value = buf.read_f64::<BigEndian>().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading number array item from serialized document".to_string(),
                    )
                })?;
                Ok(Value::Float(value))
            }
            ArrayItemType::Integer => {
                let value = buf.read_i64::<BigEndian>().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading integer array item from serialized document".to_string(),
                    )
                })?;
                Ok(Value::I64(value))
            }
            ArrayItemType::ByteArray(_, _) => {
                let bytes = DocumentPropertyType::read_varint_value(buf)?;
                Ok(Value::Bytes(bytes))
            }
            ArrayItemType::Identifier => {
                let bytes = DocumentPropertyType::read_varint_value(buf)?;
                let id: [u8; 32] = bytes.try_into().map_err(|bytes: Vec<u8>| {
                    DataContractError::CorruptedSerialization(format!(
                        "identifier array item is {} bytes long, expected 32",
                        bytes.len()
                    ))
                })?;
                Ok(Value::Identifier(id))
            }
            ArrayItemType::Boolean => {
                let value = buf.read_u8().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading boolean array item from serialized document".to_string(),
                    )
                })?;
                Ok(Value::Bool(value != 0))
            }
        }
    }

    /// The fewest bytes one element encodes to, length prefix included: a
    /// string or a byte array at its lower bound (one byte a character),
    /// an identifier at its fixed 33.
    pub fn min_byte_size(&self) -> u16 {
        match self {
            ArrayItemType::Integer | ArrayItemType::Number | ArrayItemType::Date => 8,
            ArrayItemType::Boolean => 1,
            ArrayItemType::String(min_length, _) | ArrayItemType::ByteArray(min_length, _) => {
                let min = min_length.map_or(0, |min| u16::try_from(min).unwrap_or(u16::MAX));
                min.saturating_add(varint_len(min))
            }
            ArrayItemType::Identifier => 32 + varint_len(32),
        }
    }

    /// The most bytes one element encodes to, length prefix included: a
    /// string at four bytes a character (the factor the parent string type
    /// sizes with), a byte array at its upper bound, `u16::MAX` when the
    /// element is unbounded.
    pub fn max_byte_size(&self) -> u16 {
        match self {
            ArrayItemType::Integer | ArrayItemType::Number | ArrayItemType::Date => 8,
            ArrayItemType::Boolean => 1,
            ArrayItemType::String(_, max_length) => match max_length {
                None => u16::MAX,
                Some(max) => {
                    let max = u16::try_from(*max).unwrap_or(u16::MAX).saturating_mul(4);
                    max.saturating_add(varint_len(max))
                }
            },
            ArrayItemType::ByteArray(_, max_size) => match max_size {
                None => u16::MAX,
                Some(max) => {
                    let max = u16::try_from(*max).unwrap_or(u16::MAX);
                    max.saturating_add(varint_len(max))
                }
            },
            ArrayItemType::Identifier => 32 + varint_len(32),
        }
    }

    /// The length a random string or byte array element is generated at:
    /// anywhere within the bounds.
    fn random_length(
        rng: &mut StdRng,
        min: Option<usize>,
        max: Option<usize>,
        default_max: u16,
    ) -> usize {
        let min = min.unwrap_or(0);
        let max = max.unwrap_or(default_max as usize).max(min);
        rng.gen_range(min..=max)
    }

    /// A random element of any size within the item's bounds.
    pub fn random_value(&self, rng: &mut StdRng) -> Value {
        match self {
            ArrayItemType::String(min, max) => {
                let length = Self::random_length(rng, *min, *max, DEFAULT_MAX_STRING_LENGTH);
                Self::random_string(rng, length)
            }
            ArrayItemType::ByteArray(min, max) => {
                let length = Self::random_length(rng, *min, *max, u16::MAX);
                Self::random_bytes(rng, length)
            }
            other => other.random_fixed_size_value(rng),
        }
    }

    /// A random element at the smallest size the item's bounds allow.
    pub fn random_min_value(&self, rng: &mut StdRng) -> Value {
        match self {
            ArrayItemType::String(min, _) => Self::random_string(rng, min.unwrap_or(0)),
            ArrayItemType::ByteArray(min, _) => Self::random_bytes(rng, min.unwrap_or(0)),
            other => other.random_fixed_size_value(rng),
        }
    }

    /// A random element at the largest size the item's bounds allow.
    pub fn random_max_value(&self, rng: &mut StdRng) -> Value {
        match self {
            ArrayItemType::String(_, max) => {
                Self::random_string(rng, max.unwrap_or(DEFAULT_MAX_STRING_LENGTH as usize))
            }
            ArrayItemType::ByteArray(_, max) => {
                Self::random_bytes(rng, max.unwrap_or(u16::MAX as usize))
            }
            other => other.random_fixed_size_value(rng),
        }
    }

    fn random_fixed_size_value(&self, rng: &mut StdRng) -> Value {
        match self {
            ArrayItemType::Integer => Value::I64(rng.gen::<i64>()),
            ArrayItemType::Number => Value::Float(rng.gen::<f64>()),
            ArrayItemType::Identifier => Value::Identifier(rng.gen()),
            ArrayItemType::Boolean => Value::Bool(rng.gen::<bool>()),
            ArrayItemType::Date => {
                let f: f64 = rng.gen_range(1548910575000.0..1648910575000.0);
                Value::Float(f.round() / 1000.0)
            }
            ArrayItemType::String(_, _) | ArrayItemType::ByteArray(_, _) => self.random_value(rng),
        }
    }

    fn random_string(rng: &mut StdRng, length: usize) -> Value {
        Value::Text(
            rng.sample_iter(Alphanumeric)
                .take(length)
                .map(char::from)
                .collect(),
        )
    }

    fn random_bytes(rng: &mut StdRng, length: usize) -> Value {
        Value::Bytes(rng.sample_iter(Standard).take(length).collect())
    }
}

fn get_field_type_matching_error() -> ProtocolError {
    ProtocolError::DataContractError(DataContractError::ValueWrongType(
        "document field type doesn't match document value for array".to_string(),
    ))
}

#[cfg(test)]
#[allow(clippy::approx_constant)]
mod tests {
    use super::*;
    use platform_value::platform_value;
    use rand::SeedableRng;

    // -----------------------------------------------------------------------
    // try_from_item_schema() tests
    // -----------------------------------------------------------------------

    fn item(schema: Value) -> Result<ArrayItemType, DataContractError> {
        ArrayItemType::try_from(&schema)
    }

    #[test]
    fn should_parse_every_scalar_item_schema() {
        assert_eq!(
            item(platform_value!({ "type": "integer", "minimum": 0 })).unwrap(),
            ArrayItemType::Integer
        );
        assert_eq!(
            item(platform_value!({ "type": "number" })).unwrap(),
            ArrayItemType::Number
        );
        assert_eq!(
            item(platform_value!({ "type": "boolean" })).unwrap(),
            ArrayItemType::Boolean
        );
        assert_eq!(
            item(platform_value!({ "type": "string", "minLength": 2, "maxLength": 9 })).unwrap(),
            ArrayItemType::String(Some(2), Some(9))
        );
        assert_eq!(
            item(platform_value!({ "type": "string" })).unwrap(),
            ArrayItemType::String(None, None)
        );
        assert_eq!(
            item(platform_value!({ "type": "array", "byteArray": true, "maxItems": 20 })).unwrap(),
            ArrayItemType::ByteArray(None, Some(20))
        );
        assert_eq!(
            item(platform_value!({
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier"
            }))
            .unwrap(),
            ArrayItemType::Identifier
        );
    }

    #[test]
    fn should_refuse_object_array_ref_and_reference_items() {
        for (schema, fragment) in [
            (platform_value!({ "type": "object" }), "arrays of objects"),
            (
                platform_value!({ "type": "array", "items": { "type": "integer" } }),
                "arrays of arrays",
            ),
            (
                platform_value!({ "type": "array", "byteArray": false }),
                "byteArray should always be true",
            ),
            (platform_value!({ "$ref": "#/$defs/x" }), "$ref"),
            (
                platform_value!({ "type": "integer", "refersTo": { "type": "identity" } }),
                "refersTo",
            ),
            (
                platform_value!({ "type": "date" }),
                "unsupported array item type",
            ),
            (platform_value!({ "minimum": 1 }), "type"),
        ] {
            let error = item(schema.clone())
                .expect_err("should be refused")
                .to_string();
            assert!(
                error.contains(fragment),
                "{schema:?}: expected {fragment:?}, got {error}"
            );
        }
    }

    // -----------------------------------------------------------------------
    // read_value_from() mirrors the encoding
    // -----------------------------------------------------------------------

    #[test]
    fn should_read_back_every_item_type_it_encodes() {
        for (item_type, value) in [
            (ArrayItemType::Integer, Value::I64(-42)),
            (ArrayItemType::Number, Value::Float(2.5)),
            (ArrayItemType::Date, Value::Float(1648910575.0)),
            (ArrayItemType::Boolean, Value::Bool(true)),
            (ArrayItemType::Boolean, Value::Bool(false)),
            (
                ArrayItemType::String(None, None),
                Value::Text("héllo".to_string()),
            ),
            (
                ArrayItemType::String(None, None),
                Value::Text(String::new()),
            ),
            (
                ArrayItemType::ByteArray(None, None),
                Value::Bytes(vec![1, 2, 3]),
            ),
            (ArrayItemType::ByteArray(None, None), Value::Bytes(vec![])),
            (ArrayItemType::Identifier, Value::Identifier([7u8; 32])),
        ] {
            let bytes = item_type
                .encode_value_ref_with_size(&value)
                .expect("should encode");
            let mut reader = BufReader::new(bytes.as_slice());
            let read = item_type
                .read_value_from(&mut reader)
                .expect("should decode");
            assert_eq!(read, value, "{item_type:?}");
        }
    }

    #[test]
    fn should_refuse_a_short_or_wrong_sized_item() {
        let mut short = BufReader::new(&[0u8, 1, 2][..]);
        assert!(ArrayItemType::Integer.read_value_from(&mut short).is_err());

        // a string declaring more bytes than remain
        let mut claims_more = BufReader::new(&[5u8, b'a'][..]);
        assert!(ArrayItemType::String(None, None)
            .read_value_from(&mut claims_more)
            .is_err());

        // an identifier item that is not 32 bytes
        let not_an_id = ArrayItemType::ByteArray(None, None)
            .encode_value_ref_with_size(&Value::Bytes(vec![1u8; 31]))
            .unwrap();
        let mut reader = BufReader::new(not_an_id.as_slice());
        assert!(ArrayItemType::Identifier
            .read_value_from(&mut reader)
            .is_err());
    }

    // -----------------------------------------------------------------------
    // byte sizes and random values
    // -----------------------------------------------------------------------

    #[test]
    fn should_size_items_by_their_encoding() {
        assert_eq!(ArrayItemType::Integer.min_byte_size(), 8);
        assert_eq!(ArrayItemType::Integer.max_byte_size(), 8);
        assert_eq!(ArrayItemType::Boolean.max_byte_size(), 1);
        assert_eq!(ArrayItemType::Identifier.min_byte_size(), 33);
        assert_eq!(ArrayItemType::Identifier.max_byte_size(), 33);
        assert_eq!(ArrayItemType::String(Some(2), Some(9)).min_byte_size(), 3);
        assert_eq!(ArrayItemType::String(Some(2), Some(9)).max_byte_size(), 37);
        assert_eq!(ArrayItemType::String(None, None).max_byte_size(), u16::MAX);
        assert_eq!(
            ArrayItemType::ByteArray(None, Some(200)).max_byte_size(),
            202
        );
        assert_eq!(
            ArrayItemType::ByteArray(None, None).max_byte_size(),
            u16::MAX
        );
    }

    #[test]
    fn should_generate_random_items_within_their_bounds() {
        let mut rng = StdRng::seed_from_u64(1);
        for _ in 0..32 {
            match ArrayItemType::String(Some(2), Some(5)).random_value(&mut rng) {
                Value::Text(text) => assert!((2..=5).contains(&text.chars().count())),
                other => panic!("expected text, got {other:?}"),
            }
            match ArrayItemType::ByteArray(Some(3), Some(3)).random_value(&mut rng) {
                Value::Bytes(bytes) => assert_eq!(bytes.len(), 3),
                other => panic!("expected bytes, got {other:?}"),
            }
        }
        assert!(matches!(
            ArrayItemType::String(Some(2), Some(5)).random_min_value(&mut rng),
            Value::Text(text) if text.len() == 2
        ));
        assert!(matches!(
            ArrayItemType::String(Some(2), Some(5)).random_max_value(&mut rng),
            Value::Text(text) if text.len() == 5
        ));
        assert!(matches!(
            ArrayItemType::Identifier.random_value(&mut rng),
            Value::Identifier(_)
        ));
    }

    // -----------------------------------------------------------------------
    // encode_value_with_size() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_encode_value_with_size_string() {
        let item = ArrayItemType::String(None, None);
        let result = item
            .encode_value_with_size(Value::Text("abc".to_string()))
            .unwrap();
        // varint(3) + b"abc"
        assert_eq!(result.len(), 4);
        assert_eq!(&result[1..], b"abc");
    }

    #[test]
    fn test_encode_value_with_size_string_type_mismatch() {
        let item = ArrayItemType::String(None, None);
        let result = item.encode_value_with_size(Value::U64(42));
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_value_with_size_integer() {
        let item = ArrayItemType::Integer;
        let result = item.encode_value_with_size(Value::I64(42)).unwrap();
        assert_eq!(result.len(), 8);
        assert_eq!(result, 42i64.to_be_bytes().to_vec());
    }

    #[test]
    fn test_encode_value_with_size_number() {
        let item = ArrayItemType::Number;
        let result = item.encode_value_with_size(Value::Float(3.14)).unwrap();
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_encode_value_with_size_date() {
        let item = ArrayItemType::Date;
        let result = item
            .encode_value_with_size(Value::Float(1648910575.0))
            .unwrap();
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_encode_value_with_size_byte_array() {
        let item = ArrayItemType::ByteArray(None, None);
        let bytes = vec![1u8, 2, 3, 4];
        let result = item.encode_value_with_size(Value::Bytes(bytes)).unwrap();
        // varint(4) + [1,2,3,4]
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_encode_value_with_size_identifier() {
        let item = ArrayItemType::Identifier;
        let id_bytes = [5u8; 32];
        let result = item
            .encode_value_with_size(Value::Identifier(id_bytes))
            .unwrap();
        // varint(32) + 32 bytes
        assert_eq!(result.len(), 33);
    }

    #[test]
    fn test_encode_value_with_size_boolean_true() {
        let item = ArrayItemType::Boolean;
        let result = item.encode_value_with_size(Value::Bool(true)).unwrap();
        assert_eq!(result, vec![1]);
    }

    #[test]
    fn test_encode_value_with_size_boolean_false() {
        let item = ArrayItemType::Boolean;
        let result = item.encode_value_with_size(Value::Bool(false)).unwrap();
        assert_eq!(result, vec![0]);
    }

    #[test]
    fn test_encode_value_with_size_boolean_type_mismatch() {
        let item = ArrayItemType::Boolean;
        let result = item.encode_value_with_size(Value::U64(42));
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // encode_value_ref_with_size() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_encode_value_ref_with_size_string() {
        let item = ArrayItemType::String(None, None);
        let val = Value::Text("test".to_string());
        let result = item.encode_value_ref_with_size(&val).unwrap();
        assert_eq!(result.len(), 5); // varint(4) + "test"
    }

    #[test]
    fn test_encode_value_ref_with_size_string_type_mismatch() {
        let item = ArrayItemType::String(None, None);
        let val = Value::U64(42);
        let result = item.encode_value_ref_with_size(&val);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_value_ref_with_size_integer() {
        let item = ArrayItemType::Integer;
        let val = Value::I64(-100);
        let result = item.encode_value_ref_with_size(&val).unwrap();
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_encode_value_ref_with_size_number() {
        let item = ArrayItemType::Number;
        let val = Value::Float(2.718);
        let result = item.encode_value_ref_with_size(&val).unwrap();
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_encode_value_ref_with_size_date() {
        let item = ArrayItemType::Date;
        let val = Value::Float(1648910575.0);
        let result = item.encode_value_ref_with_size(&val).unwrap();
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_encode_value_ref_with_size_byte_array() {
        let item = ArrayItemType::ByteArray(None, None);
        let val = Value::Bytes(vec![10, 20, 30]);
        let result = item.encode_value_ref_with_size(&val).unwrap();
        // varint(3) + [10,20,30]
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_encode_value_ref_with_size_identifier() {
        let item = ArrayItemType::Identifier;
        let val = Value::Identifier([7u8; 32]);
        let result = item.encode_value_ref_with_size(&val).unwrap();
        // varint(32) + 32 bytes
        assert_eq!(result.len(), 33);
    }

    #[test]
    fn test_encode_value_ref_with_size_boolean_true() {
        let item = ArrayItemType::Boolean;
        let result = item.encode_value_ref_with_size(&Value::Bool(true)).unwrap();
        assert_eq!(result, vec![1]);
    }

    #[test]
    fn test_encode_value_ref_with_size_boolean_false() {
        let item = ArrayItemType::Boolean;
        let result = item
            .encode_value_ref_with_size(&Value::Bool(false))
            .unwrap();
        assert_eq!(result, vec![0]);
    }

    #[test]
    fn test_encode_value_ref_with_size_boolean_type_mismatch() {
        let item = ArrayItemType::Boolean;
        let result = item.encode_value_ref_with_size(&Value::U64(42));
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // sanitize_value_mut() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_sanitize_byte_array_from_hex_string() {
        let item = ArrayItemType::ByteArray(None, None);
        let mut val = Value::Text("deadbeef".to_string());
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::Bytes(vec![0xDE, 0xAD, 0xBE, 0xEF]));
    }

    #[test]
    fn test_sanitize_byte_array_from_hex_string_exact_20() {
        let item = ArrayItemType::ByteArray(Some(20), Some(20));
        let hex_str = "aa".repeat(20); // 40 hex chars = 20 bytes
        let mut val = Value::Text(hex_str);
        item.sanitize_value_mut(&mut val);
        assert!(matches!(val, Value::Bytes20(_)));
    }

    #[test]
    fn test_sanitize_byte_array_from_hex_string_exact_32() {
        let item = ArrayItemType::ByteArray(Some(32), Some(32));
        let hex_str = "bb".repeat(32);
        let mut val = Value::Text(hex_str);
        item.sanitize_value_mut(&mut val);
        assert!(matches!(val, Value::Bytes32(_)));
    }

    #[test]
    fn test_sanitize_byte_array_from_hex_string_exact_36() {
        let item = ArrayItemType::ByteArray(Some(36), Some(36));
        let hex_str = "cc".repeat(36);
        let mut val = Value::Text(hex_str);
        item.sanitize_value_mut(&mut val);
        assert!(matches!(val, Value::Bytes36(_)));
    }

    #[test]
    fn test_sanitize_byte_array_size_constraint_too_small() {
        let item = ArrayItemType::ByteArray(Some(10), None);
        let mut val = Value::Text("aabb".to_string()); // 2 bytes, min is 10
        item.sanitize_value_mut(&mut val);
        // Should remain unchanged because size constraint is violated
        assert!(matches!(val, Value::Text(_)));
    }

    #[test]
    fn test_sanitize_byte_array_size_constraint_too_big() {
        let item = ArrayItemType::ByteArray(None, Some(2));
        let mut val = Value::Text("aabbccddee".to_string()); // 5 bytes, max is 2
        item.sanitize_value_mut(&mut val);
        assert!(matches!(val, Value::Text(_)));
    }

    #[test]
    fn test_sanitize_identifier_from_hex_string() {
        let item = ArrayItemType::Identifier;
        let hex_str = "aa".repeat(32);
        let mut val = Value::Text(hex_str);
        item.sanitize_value_mut(&mut val);
        assert!(matches!(val, Value::Identifier(_)));
    }

    #[test]
    fn test_sanitize_date_from_positive_i64() {
        let item = ArrayItemType::Date;
        let mut val = Value::I64(1648910575000);
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::U64(1648910575000));
    }

    #[test]
    fn test_sanitize_date_from_negative_i64_unchanged() {
        let item = ArrayItemType::Date;
        let mut val = Value::I64(-1);
        item.sanitize_value_mut(&mut val);
        // Negative timestamps should not be converted
        assert_eq!(val, Value::I64(-1));
    }

    #[test]
    fn test_sanitize_integer_from_u64() {
        let item = ArrayItemType::Integer;
        let mut val = Value::U64(42);
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::I64(42));
    }

    #[test]
    fn test_sanitize_integer_from_u32() {
        let item = ArrayItemType::Integer;
        let mut val = Value::U32(100);
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::I64(100));
    }

    #[test]
    fn test_sanitize_integer_from_u16() {
        let item = ArrayItemType::Integer;
        let mut val = Value::U16(300);
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::I64(300));
    }

    #[test]
    fn test_sanitize_integer_from_u8() {
        let item = ArrayItemType::Integer;
        let mut val = Value::U8(255);
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::I64(255));
    }

    #[test]
    fn test_sanitize_number_from_i64() {
        let item = ArrayItemType::Number;
        let mut val = Value::I64(42);
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::Float(42.0));
    }

    #[test]
    fn test_sanitize_number_from_u64() {
        let item = ArrayItemType::Number;
        let mut val = Value::U64(100);
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::Float(100.0));
    }

    #[test]
    fn test_sanitize_number_from_i32() {
        let item = ArrayItemType::Number;
        let mut val = Value::I32(-50);
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::Float(-50.0));
    }

    #[test]
    fn test_sanitize_number_from_u32() {
        let item = ArrayItemType::Number;
        let mut val = Value::U32(200);
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::Float(200.0));
    }

    #[test]
    fn test_sanitize_number_from_i16() {
        let item = ArrayItemType::Number;
        let mut val = Value::I16(-10);
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::Float(-10.0));
    }

    #[test]
    fn test_sanitize_number_from_u16() {
        let item = ArrayItemType::Number;
        let mut val = Value::U16(500);
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::Float(500.0));
    }

    #[test]
    fn test_sanitize_number_from_i8() {
        let item = ArrayItemType::Number;
        let mut val = Value::I8(-5);
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::Float(-5.0));
    }

    #[test]
    fn test_sanitize_number_from_u8() {
        let item = ArrayItemType::Number;
        let mut val = Value::U8(7);
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::Float(7.0));
    }

    #[test]
    fn test_sanitize_leaves_matching_type_unchanged() {
        let item = ArrayItemType::Boolean;
        let mut val = Value::Bool(true);
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::Bool(true));
    }

    #[test]
    fn test_sanitize_leaves_unrelated_type_unchanged() {
        let item = ArrayItemType::Integer;
        let mut val = Value::Text("not a number".to_string());
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::Text("not a number".to_string()));
    }
}

// --- canonical conversion trait impls (unification pass 1) ---
#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for ArrayItemType {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for ArrayItemType {}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;

    #[test]
    fn json_round_trip_integer_variant() {
        use crate::serialization::JsonConvertible;
        use serde_json::json;
        // `Integer` is a unit variant — internally tagged it serializes as
        // `{"$type":"integer"}` (camelCase variant name).
        let original = ArrayItemType::Integer;
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!({"$type": "integer"}));
        let recovered = ArrayItemType::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_number_variant() {
        use crate::serialization::JsonConvertible;
        use serde_json::json;
        let original = ArrayItemType::Number;
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!({"$type": "number"}));
        let recovered = ArrayItemType::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_string_variant() {
        use crate::serialization::JsonConvertible;
        use serde_json::json;
        // `String(Option<usize>, Option<usize>)` — tuple variant, internally
        // tagged with named size bounds: `{"$type":"string","minLength":min,
        // "maxLength":max}`. JSON erases the `usize` size.
        let original = ArrayItemType::String(Some(3), Some(50));
        let json = original.to_json().expect("to_json");
        assert_eq!(
            json,
            json!({"$type": "string", "minLength": 3, "maxLength": 50})
        );
        let recovered = ArrayItemType::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_byte_array_variant() {
        use crate::serialization::JsonConvertible;
        use serde_json::json;
        let original = ArrayItemType::ByteArray(Some(0), Some(64));
        let json = original.to_json().expect("to_json");
        assert_eq!(
            json,
            json!({"$type": "byteArray", "minSize": 0, "maxSize": 64})
        );
        let recovered = ArrayItemType::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_identifier_variant() {
        use crate::serialization::JsonConvertible;
        use serde_json::json;
        let original = ArrayItemType::Identifier;
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!({"$type": "identifier"}));
        let recovered = ArrayItemType::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_boolean_variant() {
        use crate::serialization::JsonConvertible;
        use serde_json::json;
        // `Boolean` is a unit variant → `{"$type":"boolean"}` (camelCase name).
        let original = ArrayItemType::Boolean;
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!({"$type": "boolean"}));
        let recovered = ArrayItemType::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_date_variant() {
        use crate::serialization::JsonConvertible;
        use serde_json::json;
        // `Date` is a unit variant → `{"$type":"date"}` (camelCase name).
        let original = ArrayItemType::Date;
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!({"$type": "date"}));
        let recovered = ArrayItemType::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_integer_variant() {
        use crate::serialization::ValueConvertible;
        use platform_value::platform_value;
        let original = ArrayItemType::Integer;
        let value = original.to_object().expect("to_object");
        assert_eq!(value, platform_value!({"$type": "integer"}));
        let recovered = ArrayItemType::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_number_variant() {
        use crate::serialization::ValueConvertible;
        use platform_value::platform_value;
        let original = ArrayItemType::Number;
        let value = original.to_object().expect("to_object");
        assert_eq!(value, platform_value!({"$type": "number"}));
        let recovered = ArrayItemType::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_string_variant() {
        use crate::serialization::ValueConvertible;
        use platform_value::platform_value;
        // `usize` serializes through serde as `u64`-like → `Value::U64` in non-HR.
        let original = ArrayItemType::String(Some(3), Some(50));
        let value = original.to_object().expect("to_object");
        assert_eq!(
            value,
            platform_value!({"$type": "string", "minLength": 3u64, "maxLength": 50u64})
        );
        let recovered = ArrayItemType::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_byte_array_variant() {
        use crate::serialization::ValueConvertible;
        use platform_value::platform_value;
        let original = ArrayItemType::ByteArray(Some(0), Some(64));
        let value = original.to_object().expect("to_object");
        assert_eq!(
            value,
            platform_value!({"$type": "byteArray", "minSize": 0u64, "maxSize": 64u64})
        );
        let recovered = ArrayItemType::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_identifier_variant() {
        use crate::serialization::ValueConvertible;
        use platform_value::platform_value;
        let original = ArrayItemType::Identifier;
        let value = original.to_object().expect("to_object");
        assert_eq!(value, platform_value!({"$type": "identifier"}));
        let recovered = ArrayItemType::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
