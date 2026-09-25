use crate::data_contract::document_type::property::DocumentPropertyType;
use crate::data_contract::errors::DataContractError;
use crate::ProtocolError;
use byteorder::{BigEndian, ReadBytesExt};
use integer_encoding::{VarInt, VarIntReader};
use platform_value::Value;
use platform_version::version::PlatformVersion;
use rand::rngs::StdRng;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::io::BufReader;

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

/// A typed array property: `type: "array"` with an `items` schema naming the
/// type of every element, parsed from protocol version 14
/// (`parse_typed_array` 0).
///
/// It is stored inline in the document like any other property: a varint
/// element count followed by the elements, each encoded exactly as a required
/// scalar property of `item_type` is. So an identifier element is 32 raw
/// bytes, a byte array element whose bounds pin one size is raw, an integer
/// element takes the width its schema's bounds give it, and a string or a
/// variable-size byte array element carries a varint length. Nothing is
/// indexed per element, so a typed array cannot be an index property.
#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct TypedArrayProperty {
    /// The scalar type of every element, parsed from `items` exactly as a
    /// scalar property schema is: an integer, a number, a string, a boolean,
    /// a byte array or an identifier, never an object or an array.
    pub item_type: Box<DocumentPropertyType>,
    /// What the `items` schema bounds beyond the type: read by random
    /// document generation, enforced on every document by the JSON schema
    /// validator.
    pub item_constraints: ArrayItemConstraints,
    /// `minItems`: the fewest elements a document may hold, never above
    /// `max_items`.
    pub min_items: Option<u16>,
    /// `maxItems`: the most elements a document may hold. Every parse
    /// requires it; full validation also caps it at
    /// `SystemLimits::max_typed_array_items`.
    pub max_items: u16,
    /// `uniqueItems`: whether a document is refused for repeating an element.
    pub unique_items: bool,
}

/// The bounds an `items` schema declares beyond its element type. The JSON
/// schema validator enforces them on every document; they are parsed so
/// random document generation stays inside them (an `exclusiveMinimum`,
/// `exclusiveMaximum`, `multipleOf`, `pattern` or `format` on an element is
/// not read here, exactly as it is not for a scalar property).
#[derive(Debug, PartialEq, Clone, Default, Serialize)]
pub struct ArrayItemConstraints {
    /// `enum`: the values every element must be one of, in declared order.
    /// Every member is of the element type; a byte array or identifier
    /// element takes none.
    pub allowed_values: Option<Vec<Value>>,
    /// `minimum` of an integer or number element, inclusive.
    pub minimum: Option<Value>,
    /// `maximum` of an integer or number element, inclusive, never below
    /// `minimum`.
    pub maximum: Option<Value>,
}

/// Which size a random element is drawn at.
#[derive(Clone, Copy)]
enum RandomFill {
    /// Any size the bounds allow.
    Any,
    /// The smallest value the bounds allow.
    Smallest,
    /// The largest value the bounds allow.
    Largest,
}

/// The range an integer element type can hold, for drawing a random element
/// when its schema bounds only one side.
fn integer_kind_range(kind: &DocumentPropertyType) -> Option<(i128, i128)> {
    Some(match kind {
        DocumentPropertyType::U8 => (0, u8::MAX as i128),
        DocumentPropertyType::I8 => (i8::MIN as i128, i8::MAX as i128),
        DocumentPropertyType::U16 => (0, u16::MAX as i128),
        DocumentPropertyType::I16 => (i16::MIN as i128, i16::MAX as i128),
        DocumentPropertyType::U32 => (0, u32::MAX as i128),
        DocumentPropertyType::I32 => (i32::MIN as i128, i32::MAX as i128),
        DocumentPropertyType::U64 => (0, u64::MAX as i128),
        DocumentPropertyType::I64 => (i64::MIN as i128, i64::MAX as i128),
        DocumentPropertyType::U128 => (0, i128::MAX),
        DocumentPropertyType::I128 => (i128::MIN, i128::MAX),
        _ => return None,
    })
}

/// An integer in the value kind an element type reads back as, so a random
/// document round-trips through the codec unchanged. `n` is within the
/// kind's range, which the caller clamps it to.
fn integer_value_of_kind(kind: &DocumentPropertyType, n: i128) -> Value {
    match kind {
        DocumentPropertyType::U8 => Value::U8(n as u8),
        DocumentPropertyType::I8 => Value::I8(n as i8),
        DocumentPropertyType::U16 => Value::U16(n as u16),
        DocumentPropertyType::I16 => Value::I16(n as i16),
        DocumentPropertyType::U32 => Value::U32(n as u32),
        DocumentPropertyType::I32 => Value::I32(n as i32),
        DocumentPropertyType::U64 => Value::U64(n as u64),
        DocumentPropertyType::U128 => Value::U128(n as u128),
        DocumentPropertyType::I128 => Value::I128(n),
        _ => Value::I64(n as i64),
    }
}

impl TypedArrayProperty {
    /// Whether an element's encoding is a varint length followed by the bytes:
    /// a string, and a byte array whose bounds do not pin one size. Every
    /// other scalar has a fixed width.
    fn element_is_length_prefixed(&self) -> bool {
        match self.item_type.as_ref() {
            DocumentPropertyType::String(_) => true,
            DocumentPropertyType::ByteArray(sizes) => {
                !(sizes.min_size.is_some() && sizes.min_size == sizes.max_size)
            }
            _ => false,
        }
    }

    /// The width of an element written raw from bytes: 32 for an identifier,
    /// the size of a byte array whose bounds pin one. The scalar encoder
    /// writes those bytes as given, so the list checks their length: a short
    /// element would shift every element after it.
    fn element_raw_width(&self) -> Option<usize> {
        match self.item_type.as_ref() {
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                Some(32)
            }
            DocumentPropertyType::ByteArray(sizes)
                if sizes.min_size.is_some() && sizes.min_size == sizes.max_size =>
            {
                sizes.min_size.map(usize::from)
            }
            _ => None,
        }
    }

    /// An element's encoded size for its scalar byte bound: the bound, plus
    /// the varint length in front of a length-prefixed element.
    fn element_encoded_size(&self, element_bytes: u16) -> u64 {
        let prefix = if self.element_is_length_prefixed() {
            usize::from(element_bytes).required_space() as u64
        } else {
            0
        };
        u64::from(element_bytes).saturating_add(prefix)
    }

    /// The fewest bytes the array encodes to: the varint count of `minItems`
    /// elements and that many of the smallest element, sized as its scalar
    /// type is sized, saturating at `u16::MAX`.
    pub fn min_encoded_size(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<u16, ProtocolError> {
        let min_items = self.min_items.unwrap_or(0);
        let element_bytes = self.item_type.min_byte_size(platform_version)?.unwrap_or(0);
        let size = (min_items.required_space() as u64).saturating_add(
            u64::from(min_items).saturating_mul(self.element_encoded_size(element_bytes)),
        );
        Ok(u16::try_from(size).unwrap_or(u16::MAX))
    }

    /// The most bytes the array encodes to: the varint count of `maxItems`
    /// elements and that many of the largest element, sized as its scalar
    /// type is sized, saturating at `u16::MAX`, the size an unbounded string
    /// or byte array reports. Also `u16::MAX` when the element is unbounded.
    pub fn max_encoded_size(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<u16, ProtocolError> {
        let element_bytes = match self.item_type.max_byte_size(platform_version)? {
            Some(element_bytes) if element_bytes < u16::MAX => element_bytes,
            _ => return Ok(u16::MAX),
        };
        let size = (self.max_items.required_space() as u64).saturating_add(
            u64::from(self.max_items).saturating_mul(self.element_encoded_size(element_bytes)),
        );
        Ok(u16::try_from(size).unwrap_or(u16::MAX))
    }

    /// Encodes a list: the varint element count, then each element exactly as
    /// a required scalar property of `item_type` is encoded.
    pub(super) fn encode_value_ref(&self, value: &Value) -> Result<Vec<u8>, ProtocolError> {
        let Value::Array(elements) = value else {
            return Err(DataContractError::ValueWrongType(format!(
                "a typed array value must be a list, got {value}"
            ))
            .into());
        };
        let mut bytes = elements.len().encode_var_vec();
        for element in elements {
            // A null encodes to no bytes at all, which would drop the element
            if element.is_null() {
                return Err(DataContractError::ValueWrongType(
                    "a typed array element can not be null".to_string(),
                )
                .into());
            }
            let element_bytes = self.item_type.encode_value_ref_with_size(element, true)?;
            if let Some(width) = self.element_raw_width() {
                if element_bytes.len() != width {
                    return Err(DataContractError::ValueWrongType(format!(
                        "a typed array element must be {width} bytes, got {}",
                        element_bytes.len()
                    ))
                    .into());
                }
            }
            bytes.extend(element_bytes);
        }
        Ok(bytes)
    }

    /// Reads a list, the mirror of [`Self::encode_value_ref`]: a varint
    /// element count, then each element as a required scalar property of
    /// `item_type` is read. The count comes from the serialized document, so
    /// a count above `maxItems` is refused before anything is read: that
    /// bounds the loop even for elements of zero width (a byte array pinned
    /// to zero bytes).
    pub(super) fn read_from(&self, buf: &mut BufReader<&[u8]>) -> Result<Value, DataContractError> {
        let count: usize = buf.read_varint().map_err(|_| {
            DataContractError::CorruptedSerialization(
                "error reading varint of typed array element count".to_string(),
            )
        })?;
        if count > usize::from(self.max_items) {
            return Err(DataContractError::CorruptedSerialization(format!(
                "a serialized typed array claims {count} elements, more than its maxItems of {}",
                self.max_items
            )));
        }
        let mut elements = Vec::new();
        for _ in 0..count {
            let (element, _) = self.item_type.read_optionally_from(buf, true)?;
            let Some(element) = element else {
                return Err(DataContractError::CorruptedSerialization(
                    "a typed array element read back as absent".to_string(),
                ));
            };
            elements.push(element);
        }
        Ok(Value::Array(elements))
    }

    /// How many elements a random value holds: between `minItems` and
    /// `maxItems`.
    fn random_items_range(&self) -> (usize, usize) {
        let max_items = usize::from(self.max_items);
        let min_items = usize::from(self.min_items.unwrap_or(0)).min(max_items);
        (min_items, max_items)
    }

    /// A random value holding between `minItems` and `maxItems` random
    /// elements.
    pub(super) fn random_value(&self, rng: &mut StdRng) -> Value {
        let (min_items, max_items) = self.random_items_range();
        let count = rng.gen_range(min_items..=max_items);
        self.random_items(count, rng, RandomFill::Any)
    }

    /// A random value holding `minItems` elements, each of its smallest size.
    pub(super) fn random_sub_filled_value(&self, rng: &mut StdRng) -> Value {
        let (min_items, _) = self.random_items_range();
        self.random_items(min_items, rng, RandomFill::Smallest)
    }

    /// A random value holding `maxItems` elements, each of its largest size.
    pub(super) fn random_filled_value(&self, rng: &mut StdRng) -> Value {
        let (_, max_items) = self.random_items_range();
        self.random_items(max_items, rng, RandomFill::Largest)
    }

    /// `count` elements drawn at `fill`, each in the value kind it reads
    /// back as. Under `uniqueItems` a repeat is drawn again, a bounded number
    /// of times, so an element type with fewer distinct values than `count`
    /// (a boolean, a short `enum`) yields fewer elements rather than looping
    /// forever.
    fn random_items(&self, count: usize, rng: &mut StdRng, fill: RandomFill) -> Value {
        let fixed_size_bytes = matches!(
            self.item_type.as_ref(),
            DocumentPropertyType::ByteArray(sizes)
                if sizes.min_size.is_some() && sizes.min_size == sizes.max_size
        );
        let mut items: Vec<Value> = Vec::with_capacity(count);
        let mut draws_left = count.saturating_mul(8).saturating_add(16);
        while items.len() < count && draws_left > 0 {
            draws_left -= 1;
            let item = match self.random_item(rng, fill) {
                Value::Bytes(bytes) if fixed_size_bytes => fixed_size_bytes_value(bytes),
                item => item,
            };
            if self.unique_items && items.contains(&item) {
                continue;
            }
            items.push(item);
        }
        Value::Array(items)
    }

    /// One element within the item constraints: a member of the `enum` when
    /// there is one (the shortest, the longest or any), a number within
    /// `minimum` / `maximum`, and otherwise the element type's own random
    /// value at `fill`. Integer and number members come back in the element
    /// type's own value kind, which is what the codec reads back.
    fn random_item(&self, rng: &mut StdRng, fill: RandomFill) -> Value {
        if let Some(allowed_values) = &self.item_constraints.allowed_values {
            let encoded_len = |value: &Value| {
                self.item_type
                    .encode_value_ref_with_size(value, true)
                    .map(|bytes| bytes.len())
                    .unwrap_or(0)
            };
            let member = match fill {
                RandomFill::Any => {
                    allowed_values.get(rng.gen_range(0..allowed_values.len().max(1)))
                }
                RandomFill::Smallest => {
                    allowed_values.iter().min_by_key(|value| encoded_len(value))
                }
                RandomFill::Largest => allowed_values.iter().max_by_key(|value| encoded_len(value)),
            };
            if let Some(member) = member {
                return self.in_element_kind(member);
            }
        }
        if let Some(bounded) = self.random_bounded_number(rng, fill) {
            return bounded;
        }
        match fill {
            RandomFill::Any => self.item_type.random_value(rng),
            RandomFill::Smallest => self.item_type.random_sub_filled_value(rng),
            RandomFill::Largest => self.item_type.random_filled_value(rng),
        }
    }

    /// A constraint value in the element type's own value kind.
    fn in_element_kind(&self, value: &Value) -> Value {
        if let Some((min, max)) = integer_kind_range(&self.item_type) {
            if let Ok(n) = value.to_integer::<i128>() {
                return integer_value_of_kind(&self.item_type, n.clamp(min, max));
            }
        }
        if matches!(self.item_type.as_ref(), DocumentPropertyType::F64) {
            if let Ok(f) = value.to_float() {
                return Value::Float(f);
            }
        }
        value.clone()
    }

    /// A random integer or number element within the declared `minimum` /
    /// `maximum`, or `None` when the element declares neither or is not a
    /// number. A bound the parser could not read as the element's type is
    /// ignored, so generation never panics on a stored contract.
    fn random_bounded_number(&self, rng: &mut StdRng, fill: RandomFill) -> Option<Value> {
        let constraints = &self.item_constraints;
        if constraints.minimum.is_none() && constraints.maximum.is_none() {
            return None;
        }
        if let Some((kind_min, kind_max)) = integer_kind_range(&self.item_type) {
            let min = constraints
                .minimum
                .as_ref()
                .and_then(|value| value.to_integer::<i128>().ok())
                .unwrap_or(kind_min)
                .clamp(kind_min, kind_max);
            let max = constraints
                .maximum
                .as_ref()
                .and_then(|value| value.to_integer::<i128>().ok())
                .unwrap_or(kind_max)
                .clamp(kind_min, kind_max)
                .max(min);
            let n = match fill {
                RandomFill::Any => rng.gen_range(min..=max),
                RandomFill::Smallest => min,
                RandomFill::Largest => max,
            };
            return Some(integer_value_of_kind(&self.item_type, n));
        }
        if matches!(self.item_type.as_ref(), DocumentPropertyType::F64) {
            let min = constraints
                .minimum
                .as_ref()
                .and_then(|value| value.to_float().ok())
                .filter(|value| value.is_finite())
                .unwrap_or(-1.0e9);
            let max = constraints
                .maximum
                .as_ref()
                .and_then(|value| value.to_float().ok())
                .filter(|value| value.is_finite())
                .unwrap_or(1.0e9)
                .max(min);
            return Some(Value::Float(match fill {
                RandomFill::Any => rng.gen_range(min..=max),
                RandomFill::Smallest => min,
                RandomFill::Largest => max,
            }));
        }
        None
    }
}

/// The value a fixed-size byte array reads back as: 20, 32 or 36 bytes as
/// `Bytes20`, `Bytes32` or `Bytes36`, any other size as `Bytes`.
fn fixed_size_bytes_value(bytes: Vec<u8>) -> Value {
    let bytes = match <[u8; 20]>::try_from(bytes) {
        Ok(bytes) => return Value::Bytes20(bytes),
        Err(bytes) => bytes,
    };
    let bytes = match <[u8; 32]>::try_from(bytes) {
        Ok(bytes) => return Value::Bytes32(bytes),
        Err(bytes) => bytes,
    };
    match <[u8; 36]>::try_from(bytes) {
        Ok(bytes) => Value::Bytes36(bytes),
        Err(bytes) => Value::Bytes(bytes),
    }
}

// Internal-`$type` serde shape. Mixed unit + 2-tuple variants, so a
// struct-variant Repr (serde can't auto-internal-tag tuple variants). Unit
// variants -> `{"$type":"integer"}`; the tuple variants get named size bounds
// (`#[serde(default)]` so an omitted bound deserializes as `None`). Serde-only
// type (no bincode); its on-wire form is exercised solely by these tests. No
// document schema parses into an `ArrayItemType`: a typed array's elements are
// `DocumentPropertyType`s.
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

    /// Reads one element of the never-produced [`DocumentPropertyType::Array`],
    /// mirroring [`Self::encode_value_ref_with_size`]. Every element takes at
    /// least one byte, so a reader looping over a claimed element count stops
    /// when the serialized document runs out.
    pub(super) fn read_from(&self, buf: &mut BufReader<&[u8]>) -> Result<Value, DataContractError> {
        match self {
            ArrayItemType::String(_, _) => {
                let bytes = DocumentPropertyType::read_varint_value(buf)?;
                String::from_utf8(bytes).map(Value::Text).map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading string array element from serialized document".to_string(),
                    )
                })
            }
            ArrayItemType::Integer => buf.read_i64::<BigEndian>().map(Value::I64).map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading integer array element from serialized document".to_string(),
                )
            }),
            ArrayItemType::Number | ArrayItemType::Date => {
                buf.read_f64::<BigEndian>().map(Value::Float).map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading number array element from serialized document".to_string(),
                    )
                })
            }
            ArrayItemType::ByteArray(_, _) => {
                let bytes = DocumentPropertyType::read_varint_value(buf)?;
                Ok(self.byte_array_value(bytes))
            }
            ArrayItemType::Identifier => {
                let bytes = DocumentPropertyType::read_varint_value(buf)?;
                <[u8; 32]>::try_from(bytes)
                    .map(Value::Identifier)
                    .map_err(|bytes| {
                        DataContractError::CorruptedSerialization(format!(
                            "identifier array element must be 32 bytes, found {}",
                            bytes.len()
                        ))
                    })
            }
            ArrayItemType::Boolean => match buf.read_u8() {
                Ok(0) => Ok(Value::Bool(false)),
                Ok(1) => Ok(Value::Bool(true)),
                _ => Err(DataContractError::CorruptedSerialization(
                    "error reading boolean array element from serialized document".to_string(),
                )),
            },
        }
    }

    /// The value a byte array element reads back as: a fixed-size element
    /// of 20, 32 or 36 bytes as `Bytes20`, `Bytes32` or `Bytes36`, the kinds a
    /// fixed-size scalar byte array reads back as, and `Bytes` otherwise.
    fn byte_array_value(&self, bytes: Vec<u8>) -> Value {
        match self {
            ArrayItemType::ByteArray(min_size, max_size)
                if min_size.is_some() && min_size == max_size =>
            {
                fixed_size_bytes_value(bytes)
            }
            _ => Value::Bytes(bytes),
        }
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
