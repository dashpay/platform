use crate::inner_value_at_path::is_array_path;
use crate::value_map::ValueMapHelper;
use crate::{Error, Value};
use std::collections::BTreeMap;

use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use std::iter::Peekable;
use std::vec::IntoIter;

#[derive(Debug, Clone, Copy)]
pub enum IntegerReplacementType {
    U128,
    I128,
    U64,
    I64,
    U32,
    I32,
    U16,
    I16,
    U8,
    I8,
}

impl IntegerReplacementType {
    pub fn replace_for_value(&self, value: Value) -> Result<Value, Error> {
        Ok(match self {
            IntegerReplacementType::U128 => Value::U128(value.try_into()?),
            IntegerReplacementType::I128 => Value::I128(value.try_into()?),
            IntegerReplacementType::U64 => Value::U64(value.try_into()?),
            IntegerReplacementType::I64 => Value::I64(value.try_into()?),
            IntegerReplacementType::U32 => Value::U32(value.try_into()?),
            IntegerReplacementType::I32 => Value::I32(value.try_into()?),
            IntegerReplacementType::U16 => Value::U16(value.try_into()?),
            IntegerReplacementType::I16 => Value::I16(value.try_into()?),
            IntegerReplacementType::U8 => Value::U8(value.try_into()?),
            IntegerReplacementType::I8 => Value::I8(value.try_into()?),
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ReplacementType {
    Identifier,
    BinaryBytes,
    TextBase58,
    TextBase64,
}

impl ReplacementType {
    pub fn replace_for_bytes(&self, bytes: Vec<u8>) -> Result<Value, Error> {
        match self {
            ReplacementType::Identifier => {
                Ok(Value::Identifier(bytes.try_into().map_err(|_| {
                    Error::ByteLengthNot32BytesError(String::from(
                        "Trying to replace into an identifier, but not 32 bytes long",
                    ))
                })?))
            }
            ReplacementType::BinaryBytes => Ok(Value::Bytes(bytes)),
            ReplacementType::TextBase58 => Ok(Value::Text(bs58::encode(bytes).into_string())),
            ReplacementType::TextBase64 => Ok(Value::Text(BASE64_STANDARD.encode(bytes))),
        }
    }

    pub fn replace_for_bytes_20(&self, bytes: [u8; 20]) -> Result<Value, Error> {
        match self {
            ReplacementType::BinaryBytes => Ok(Value::Bytes20(bytes)),
            ReplacementType::TextBase58 => Ok(Value::Text(bs58::encode(bytes).into_string())),
            ReplacementType::TextBase64 => Ok(Value::Text(BASE64_STANDARD.encode(bytes))),
            _ => Err(Error::ByteLengthNot36BytesError(
                "trying to replace 36 bytes into an identifier".to_string(),
            )),
        }
    }

    pub fn replace_for_bytes_32(&self, bytes: [u8; 32]) -> Result<Value, Error> {
        match self {
            ReplacementType::Identifier => Ok(Value::Identifier(bytes)),
            ReplacementType::BinaryBytes => Ok(Value::Bytes32(bytes)),
            ReplacementType::TextBase58 => Ok(Value::Text(bs58::encode(bytes).into_string())),
            ReplacementType::TextBase64 => Ok(Value::Text(BASE64_STANDARD.encode(bytes))),
        }
    }

    pub fn replace_for_bytes_36(&self, bytes: [u8; 36]) -> Result<Value, Error> {
        match self {
            ReplacementType::BinaryBytes => Ok(Value::Bytes36(bytes)),
            ReplacementType::TextBase58 => Ok(Value::Text(bs58::encode(bytes).into_string())),
            ReplacementType::TextBase64 => Ok(Value::Text(BASE64_STANDARD.encode(bytes))),
            _ => Err(Error::ByteLengthNot36BytesError(
                "trying to replace 36 bytes into an identifier".to_string(),
            )),
        }
    }

    pub fn replace_consume_value(&self, value: Value) -> Result<Value, Error> {
        let bytes = value.into_identifier_bytes()?;
        self.replace_for_bytes(bytes)
    }

    pub fn replace_value_in_place(&self, value: &mut Value) -> Result<(), Error> {
        let bytes = value.take().into_identifier_bytes()?;
        *value = self.replace_for_bytes(bytes)?;
        Ok(())
    }
}

pub trait BTreeValueMapReplacementPathHelper {
    fn replace_at_path(
        &mut self,
        path: &str,
        replacement_type: ReplacementType,
    ) -> Result<(), Error>;
    fn replace_at_paths<'a, I: IntoIterator<Item = &'a String>>(
        &mut self,
        paths: I,
        replacement_type: ReplacementType,
    ) -> Result<(), Error>;
}

/// Replaces one value in place with its `replacement_type` form. The
/// fixed-size byte values keep their width; anything else is read as the
/// bytes it holds in whatever encoding it currently has. Shared by the
/// `Value` and the `BTreeMap<String, Value>` path replacers, so both give
/// the same value kind for the same input.
pub(crate) fn replace_leaf(
    value: &mut Value,
    replacement_type: ReplacementType,
) -> Result<(), Error> {
    match value {
        Value::Bytes20(bytes) => {
            *value = replacement_type.replace_for_bytes_20(*bytes)?;
        }
        Value::Bytes32(bytes) => {
            *value = replacement_type.replace_for_bytes_32(*bytes)?;
        }
        Value::Bytes36(bytes) => {
            *value = replacement_type.replace_for_bytes_36(*bytes)?;
        }
        _ => {
            let bytes = match replacement_type {
                ReplacementType::Identifier | ReplacementType::TextBase58 => {
                    value.to_identifier_bytes()
                }
                ReplacementType::BinaryBytes | ReplacementType::TextBase64 => {
                    value.to_binary_bytes()
                }
            }?;
            *value = replacement_type.replace_for_bytes(bytes)?;
        }
    }
    Ok(())
}

/// The members a `list[]` or `list[3]` component addresses in a list value:
/// every element, or the one at the index.
fn list_members(list: &mut Value, index: Option<usize>) -> Result<Vec<&mut Value>, Error> {
    let list = list.to_array_mut()?;
    match index {
        Some(index) => match list.get_mut(index) {
            Some(member) => Ok(vec![member]),
            None => Err(Error::StructureError(format!(
                "element at position {index} in array does not exist"
            ))),
        },
        None => Ok(list.iter_mut().collect()),
    }
}

fn replace_down(
    mut current_values: Vec<&mut Value>,
    mut split: Peekable<IntoIter<&str>>,
    replacement_type: ReplacementType,
) -> Result<(), Error> {
    if let Some(path_component) = split.next() {
        let is_last_component = split.peek().is_none();
        let next_values = current_values
            .iter_mut()
            .map(|current_value| {
                if current_value.is_map() {
                    let map = current_value.as_map_mut_ref()?;
                    // `list[]` names the members of a list: the values to
                    // replace when it ends the path, the values to descend
                    // into otherwise. An absent list is nothing to replace.
                    if let Some((list_name, index)) = is_array_path(path_component)? {
                        let Some(list) = map.get_optional_key_mut(list_name) else {
                            return Ok(None);
                        };
                        let members = list_members(list, index)?;
                        if is_last_component {
                            for member in members {
                                replace_leaf(member, replacement_type)?;
                            }
                            return Ok(None);
                        }
                        return Ok(Some(members));
                    }
                    let Some(new_value) = map.get_optional_key_mut(path_component) else {
                        return Ok(None);
                    };
                    if is_last_component {
                        replace_leaf(new_value, replacement_type)?;
                        Ok(None)
                    } else {
                        Ok(Some(vec![new_value]))
                    }
                } else if current_value.is_array() {
                    // if it's an array we apply to all members
                    let array = current_value.to_array_mut()?.iter_mut().collect();
                    Ok(Some(array))
                } else {
                    Err(Error::PathError("path was not an array or map".to_string()))
                }
            })
            .collect::<Result<Vec<_>, Error>>()?
            .into_iter()
            .flatten()
            .flatten()
            .collect();
        replace_down(next_values, split, replacement_type)
    } else {
        Ok(())
    }
}

impl BTreeValueMapReplacementPathHelper for BTreeMap<String, Value> {
    fn replace_at_path(
        &mut self,
        path: &str,
        replacement_type: ReplacementType,
    ) -> Result<(), Error> {
        let mut split: Vec<_> = path.split('.').collect();
        let first = split.first();
        let Some(first_path_component) = first else {
            return Err(Error::PathError("path was empty".to_string()));
        };
        // `list[]` first: the members of a top-level list
        if let Some((list_name, index)) = is_array_path(first_path_component)? {
            let Some(list) = self.get_mut(list_name) else {
                return Ok(());
            };
            let members = list_members(list, index)?;
            if split.len() == 1 {
                for member in members {
                    replace_leaf(member, replacement_type)?;
                }
                return Ok(());
            }
            split.remove(0);
            return replace_down(members, split.into_iter().peekable(), replacement_type);
        }
        let Some(current_value) = self.get_mut(first_path_component.to_owned()) else {
            return Ok(());
        };
        if split.len() == 1 {
            replace_leaf(current_value, replacement_type)
        } else {
            split.remove(0);
            let current_values = vec![current_value];
            //todo: make this non recursive
            replace_down(
                current_values,
                split.into_iter().peekable(),
                replacement_type,
            )
        }
    }

    fn replace_at_paths<'a, I: IntoIterator<Item = &'a String>>(
        &mut self,
        paths: I,
        replacement_type: ReplacementType,
    ) -> Result<(), Error> {
        paths
            .into_iter()
            .try_for_each(|path| self.replace_at_path(path.as_str(), replacement_type))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::string_encoding::Encoding;
    use crate::value_map::ValueMapHelper;
    use crate::{Error, Identifier, Value};
    use base64::prelude::BASE64_STANDARD;
    use base64::Engine;
    use std::collections::BTreeMap;

    // -----------------------------------------------------------------------
    // IntegerReplacementType::replace_for_value — each variant
    // -----------------------------------------------------------------------

    #[test]
    fn integer_replacement_u8() {
        let result = IntegerReplacementType::U8
            .replace_for_value(Value::U64(200))
            .unwrap();
        assert_eq!(result, Value::U8(200));
    }

    #[test]
    fn integer_replacement_i8() {
        let result = IntegerReplacementType::I8
            .replace_for_value(Value::I64(-100))
            .unwrap();
        assert_eq!(result, Value::I8(-100));
    }

    #[test]
    fn integer_replacement_u16() {
        let result = IntegerReplacementType::U16
            .replace_for_value(Value::U64(60000))
            .unwrap();
        assert_eq!(result, Value::U16(60000));
    }

    #[test]
    fn integer_replacement_i16() {
        let result = IntegerReplacementType::I16
            .replace_for_value(Value::I64(-30000))
            .unwrap();
        assert_eq!(result, Value::I16(-30000));
    }

    #[test]
    fn integer_replacement_u32() {
        let result = IntegerReplacementType::U32
            .replace_for_value(Value::U64(3_000_000))
            .unwrap();
        assert_eq!(result, Value::U32(3_000_000));
    }

    #[test]
    fn integer_replacement_i32() {
        let result = IntegerReplacementType::I32
            .replace_for_value(Value::I64(-3_000_000))
            .unwrap();
        assert_eq!(result, Value::I32(-3_000_000));
    }

    #[test]
    fn integer_replacement_u64() {
        let result = IntegerReplacementType::U64
            .replace_for_value(Value::U64(u64::MAX))
            .unwrap();
        assert_eq!(result, Value::U64(u64::MAX));
    }

    #[test]
    fn integer_replacement_i64() {
        let result = IntegerReplacementType::I64
            .replace_for_value(Value::I64(i64::MIN))
            .unwrap();
        assert_eq!(result, Value::I64(i64::MIN));
    }

    #[test]
    fn integer_replacement_u128() {
        let result = IntegerReplacementType::U128
            .replace_for_value(Value::U64(42))
            .unwrap();
        assert_eq!(result, Value::U128(42));
    }

    #[test]
    fn integer_replacement_i128() {
        let result = IntegerReplacementType::I128
            .replace_for_value(Value::I64(-42))
            .unwrap();
        assert_eq!(result, Value::I128(-42));
    }

    #[test]
    fn integer_replacement_overflow_error() {
        // Trying to fit a large u64 into u8 should error
        let result = IntegerReplacementType::U8.replace_for_value(Value::U64(300));
        assert!(result.is_err());
    }

    #[test]
    fn integer_replacement_non_integer_error() {
        // Non-integer value should fail
        let result =
            IntegerReplacementType::U64.replace_for_value(Value::Text("not a number".into()));
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // ReplacementType::replace_for_bytes
    // -----------------------------------------------------------------------

    #[test]
    fn replace_for_bytes_identifier_32_bytes_ok() {
        let bytes = vec![0xABu8; 32];
        let result = ReplacementType::Identifier
            .replace_for_bytes(bytes.clone())
            .unwrap();
        let expected: [u8; 32] = bytes.try_into().unwrap();
        assert_eq!(result, Value::Identifier(expected));
    }

    #[test]
    fn replace_for_bytes_identifier_wrong_size() {
        let bytes = vec![0xABu8; 31]; // not 32 bytes
        let result = ReplacementType::Identifier.replace_for_bytes(bytes);
        assert!(matches!(result, Err(Error::ByteLengthNot32BytesError(_))));
    }

    #[test]
    fn replace_for_bytes_identifier_too_long() {
        let bytes = vec![0xABu8; 33];
        let result = ReplacementType::Identifier.replace_for_bytes(bytes);
        assert!(matches!(result, Err(Error::ByteLengthNot32BytesError(_))));
    }

    #[test]
    fn replace_for_bytes_binary_bytes() {
        let bytes = vec![1, 2, 3, 4, 5];
        let result = ReplacementType::BinaryBytes
            .replace_for_bytes(bytes.clone())
            .unwrap();
        assert_eq!(result, Value::Bytes(bytes));
    }

    #[test]
    fn replace_for_bytes_text_base58() {
        let bytes = vec![0x01, 0x02, 0x03];
        let expected = bs58::encode(&bytes).into_string();
        let result = ReplacementType::TextBase58
            .replace_for_bytes(bytes)
            .unwrap();
        assert_eq!(result, Value::Text(expected));
    }

    #[test]
    fn replace_for_bytes_text_base64() {
        let bytes = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let expected = BASE64_STANDARD.encode(&bytes);
        let result = ReplacementType::TextBase64
            .replace_for_bytes(bytes)
            .unwrap();
        assert_eq!(result, Value::Text(expected));
    }

    // -----------------------------------------------------------------------
    // replace_for_bytes_20: correct size and wrong replacement type
    // -----------------------------------------------------------------------

    #[test]
    fn replace_for_bytes_20_binary() {
        let bytes = [0xFFu8; 20];
        let result = ReplacementType::BinaryBytes
            .replace_for_bytes_20(bytes)
            .unwrap();
        assert_eq!(result, Value::Bytes20(bytes));
    }

    #[test]
    fn replace_for_bytes_20_text_base58() {
        let bytes = [0x01u8; 20];
        let expected = bs58::encode(bytes).into_string();
        let result = ReplacementType::TextBase58
            .replace_for_bytes_20(bytes)
            .unwrap();
        assert_eq!(result, Value::Text(expected));
    }

    #[test]
    fn replace_for_bytes_20_text_base64() {
        let bytes = [0x02u8; 20];
        let expected = BASE64_STANDARD.encode(bytes);
        let result = ReplacementType::TextBase64
            .replace_for_bytes_20(bytes)
            .unwrap();
        assert_eq!(result, Value::Text(expected));
    }

    #[test]
    fn replace_for_bytes_20_identifier_error() {
        let bytes = [0xAAu8; 20];
        let result = ReplacementType::Identifier.replace_for_bytes_20(bytes);
        assert!(matches!(result, Err(Error::ByteLengthNot36BytesError(_))));
    }

    // -----------------------------------------------------------------------
    // replace_for_bytes_32: correct size and all replacement types
    // -----------------------------------------------------------------------

    #[test]
    fn replace_for_bytes_32_identifier() {
        let bytes = [0xBBu8; 32];
        let result = ReplacementType::Identifier
            .replace_for_bytes_32(bytes)
            .unwrap();
        assert_eq!(result, Value::Identifier(bytes));
    }

    #[test]
    fn replace_for_bytes_32_binary() {
        let bytes = [0xCCu8; 32];
        let result = ReplacementType::BinaryBytes
            .replace_for_bytes_32(bytes)
            .unwrap();
        assert_eq!(result, Value::Bytes32(bytes));
    }

    #[test]
    fn replace_for_bytes_32_text_base58() {
        let bytes = [0x01u8; 32];
        let expected = bs58::encode(bytes).into_string();
        let result = ReplacementType::TextBase58
            .replace_for_bytes_32(bytes)
            .unwrap();
        assert_eq!(result, Value::Text(expected));
    }

    #[test]
    fn replace_for_bytes_32_text_base64() {
        let bytes = [0x02u8; 32];
        let expected = BASE64_STANDARD.encode(bytes);
        let result = ReplacementType::TextBase64
            .replace_for_bytes_32(bytes)
            .unwrap();
        assert_eq!(result, Value::Text(expected));
    }

    // -----------------------------------------------------------------------
    // replace_for_bytes_36: correct size and wrong replacement type
    // -----------------------------------------------------------------------

    #[test]
    fn replace_for_bytes_36_binary() {
        let bytes = [0xDDu8; 36];
        let result = ReplacementType::BinaryBytes
            .replace_for_bytes_36(bytes)
            .unwrap();
        assert_eq!(result, Value::Bytes36(bytes));
    }

    #[test]
    fn replace_for_bytes_36_text_base58() {
        let bytes = [0x03u8; 36];
        let expected = bs58::encode(bytes).into_string();
        let result = ReplacementType::TextBase58
            .replace_for_bytes_36(bytes)
            .unwrap();
        assert_eq!(result, Value::Text(expected));
    }

    #[test]
    fn replace_for_bytes_36_text_base64() {
        let bytes = [0x04u8; 36];
        let expected = BASE64_STANDARD.encode(bytes);
        let result = ReplacementType::TextBase64
            .replace_for_bytes_36(bytes)
            .unwrap();
        assert_eq!(result, Value::Text(expected));
    }

    #[test]
    fn replace_for_bytes_36_identifier_error() {
        let bytes = [0xEEu8; 36];
        let result = ReplacementType::Identifier.replace_for_bytes_36(bytes);
        assert!(matches!(result, Err(Error::ByteLengthNot36BytesError(_))));
    }

    // -----------------------------------------------------------------------
    // replace_at_path — single segment
    // -----------------------------------------------------------------------

    #[test]
    fn replace_at_path_single_segment_bytes32() {
        let bytes = [0xABu8; 32];
        let mut map = BTreeMap::new();
        map.insert("id".to_string(), Value::Bytes32(bytes));

        map.replace_at_path("id", ReplacementType::Identifier)
            .unwrap();
        assert_eq!(map.get("id"), Some(&Value::Identifier(bytes)));
    }

    #[test]
    fn replace_at_path_single_segment_bytes20() {
        let bytes = [0x11u8; 20];
        let mut map = BTreeMap::new();
        map.insert("addr".to_string(), Value::Bytes20(bytes));

        map.replace_at_path("addr", ReplacementType::BinaryBytes)
            .unwrap();
        assert_eq!(map.get("addr"), Some(&Value::Bytes20(bytes)));
    }

    #[test]
    fn replace_at_path_single_segment_bytes36() {
        let bytes = [0x22u8; 36];
        let mut map = BTreeMap::new();
        map.insert("outpoint".to_string(), Value::Bytes36(bytes));

        map.replace_at_path("outpoint", ReplacementType::BinaryBytes)
            .unwrap();
        assert_eq!(map.get("outpoint"), Some(&Value::Bytes36(bytes)));
    }

    #[test]
    fn replace_at_path_single_segment_identifier_to_base58() {
        let bytes = [0xCCu8; 32];
        let mut map = BTreeMap::new();
        map.insert("id".to_string(), Value::Identifier(bytes));

        map.replace_at_path("id", ReplacementType::TextBase58)
            .unwrap();
        let expected = bs58::encode(bytes).into_string();
        assert_eq!(map.get("id"), Some(&Value::Text(expected)));
    }

    // -----------------------------------------------------------------------
    // replace_at_path — multi-segment nested path
    // -----------------------------------------------------------------------

    #[test]
    fn replace_at_path_nested() {
        let bytes = [0xFFu8; 32];
        let inner_map = vec![(Value::Text("nested_id".into()), Value::Bytes32(bytes))];
        let mut map = BTreeMap::new();
        map.insert("parent".to_string(), Value::Map(inner_map));

        map.replace_at_path("parent.nested_id", ReplacementType::Identifier)
            .unwrap();

        let parent = map.get("parent").unwrap();
        if let Value::Map(inner) = parent {
            let val = inner.get_optional_key("nested_id").unwrap();
            assert_eq!(*val, Value::Identifier(bytes));
        } else {
            panic!("expected Map");
        }
    }

    #[test]
    fn replace_at_path_deep_nested() {
        let bytes = [0xAAu8; 32];
        let level2 = vec![(Value::Text("deep_id".into()), Value::Bytes32(bytes))];
        let level1 = vec![(Value::Text("level2".into()), Value::Map(level2))];
        let mut map = BTreeMap::new();
        map.insert("level1".to_string(), Value::Map(level1));

        map.replace_at_path("level1.level2.deep_id", ReplacementType::Identifier)
            .unwrap();

        let l1 = map.get("level1").unwrap();
        if let Value::Map(l1_map) = l1 {
            let l2 = l1_map.get_optional_key("level2").unwrap();
            if let Value::Map(l2_map) = l2 {
                let val = l2_map.get_optional_key("deep_id").unwrap();
                assert_eq!(*val, Value::Identifier(bytes));
            } else {
                panic!("expected Map at level2");
            }
        } else {
            panic!("expected Map at level1");
        }
    }

    // -----------------------------------------------------------------------
    // replace_at_path — array traversal
    // -----------------------------------------------------------------------

    #[test]
    fn replace_at_path_through_array_applies_to_elements() {
        // When replace_down encounters an array at a non-terminal path component,
        // it expands the array elements into the next recursion level. The path
        // component consumed at the array level is effectively discarded (since
        // arrays don't have named keys). The NEXT component is then applied to
        // each array element.
        //
        // Structure:
        //   top-level BTreeMap: "wrapper" -> Map { "arr" -> Array [ Map{"id": Bytes32}, ... ] }
        // Path: "wrapper.arr.placeholder.id"
        //   - "wrapper" handled by replace_at_path (first component)
        //   - replace_down gets ["arr", "placeholder", "id"]
        //   - "arr" consumed: looks up in wrapper map, finds Array, returns it
        //   - "placeholder" consumed: current is Array, expands to array items (Maps)
        //   - "id" consumed: terminal component, looks up in each item Map, performs replacement
        let bytes1 = [0x11u8; 32];
        let bytes2 = [0x22u8; 32];
        let item1 = Value::Map(vec![(Value::Text("id".into()), Value::Bytes32(bytes1))]);
        let item2 = Value::Map(vec![(Value::Text("id".into()), Value::Bytes32(bytes2))]);
        let wrapper_map = vec![(Value::Text("arr".into()), Value::Array(vec![item1, item2]))];
        let mut map = BTreeMap::new();
        map.insert("wrapper".to_string(), Value::Map(wrapper_map));

        // "placeholder" is consumed by the array level and discarded
        map.replace_at_path("wrapper.arr.placeholder.id", ReplacementType::Identifier)
            .unwrap();

        if let Value::Map(wrapper) = map.get("wrapper").unwrap() {
            let arr_val = wrapper.get_optional_key("arr").unwrap();
            if let Value::Array(arr) = arr_val {
                assert_eq!(arr.len(), 2);
                for (i, item) in arr.iter().enumerate() {
                    if let Value::Map(m) = item {
                        let val = m.get_optional_key("id").unwrap();
                        let expected_bytes = if i == 0 { bytes1 } else { bytes2 };
                        assert_eq!(*val, Value::Identifier(expected_bytes));
                    } else {
                        panic!("expected Map in array");
                    }
                }
            } else {
                panic!("expected Array");
            }
        } else {
            panic!("expected Map at wrapper");
        }
    }

    // -----------------------------------------------------------------------
    // replace_at_path: `list[]` names the members of a list
    // -----------------------------------------------------------------------

    fn base58_id(seed: u8) -> Value {
        Value::Text(Identifier::from([seed; 32]).to_string(Encoding::Base58))
    }

    #[test]
    fn should_replace_the_members_of_a_top_level_list() {
        let mut map = BTreeMap::new();
        map.insert(
            "ids".to_string(),
            Value::Array(vec![base58_id(1), Value::Bytes32([2u8; 32])]),
        );
        map.replace_at_path("ids[]", ReplacementType::Identifier)
            .unwrap();
        assert_eq!(
            map.get("ids").unwrap(),
            &Value::Array(vec![
                Value::Identifier([1u8; 32]),
                Value::Identifier([2u8; 32])
            ])
        );

        // One member by index; an index past the end is an error
        map.insert(
            "more".to_string(),
            Value::Array(vec![base58_id(3), Value::Text("keep".into())]),
        );
        map.replace_at_path("more[0]", ReplacementType::Identifier)
            .unwrap();
        assert_eq!(
            map.get("more").unwrap(),
            &Value::Array(vec![
                Value::Identifier([3u8; 32]),
                Value::Text("keep".into())
            ])
        );
        assert!(map
            .replace_at_path("more[2]", ReplacementType::Identifier)
            .is_err());
    }

    #[test]
    fn should_replace_the_members_of_a_nested_list_and_descend_into_a_list_of_maps() {
        let inner = Value::Map(vec![(
            Value::Text("ids".into()),
            Value::Array(vec![base58_id(4)]),
        )]);
        let mut map = BTreeMap::new();
        map.insert("meta".to_string(), inner);
        map.replace_at_path("meta.ids[]", ReplacementType::Identifier)
            .unwrap();
        let Some(Value::Map(meta)) = map.get("meta") else {
            panic!("expected the map");
        };
        assert_eq!(
            meta.get_optional_key("ids").unwrap(),
            &Value::Array(vec![Value::Identifier([4u8; 32])])
        );

        let item = Value::Map(vec![(Value::Text("id".into()), base58_id(5))]);
        map.insert("items".to_string(), Value::Array(vec![item]));
        map.replace_at_path("items[].id", ReplacementType::Identifier)
            .unwrap();
        let Some(Value::Array(items)) = map.get("items") else {
            panic!("expected the list");
        };
        let Value::Map(item) = &items[0] else {
            panic!("expected a map member");
        };
        assert_eq!(
            item.get_optional_key("id").unwrap(),
            &Value::Identifier([5u8; 32])
        );
    }

    #[test]
    fn should_treat_an_absent_top_level_list_as_nothing_to_replace() {
        let mut map = BTreeMap::new();
        map.insert("a".to_string(), Value::U32(1));
        assert!(map
            .replace_at_path("ids[]", ReplacementType::Identifier)
            .is_ok());
        // Below a scalar there is no list to look in
        assert!(map
            .replace_at_path("a.ids[]", ReplacementType::Identifier)
            .is_err());
    }

    // -----------------------------------------------------------------------
    // Error paths
    // -----------------------------------------------------------------------

    #[test]
    fn replace_at_path_empty_path_error() {
        let mut map = BTreeMap::new();
        map.insert("key".to_string(), Value::U64(1));
        let result = map.replace_at_path("", ReplacementType::Identifier);
        // Empty string splits to [""] which is a single component, not truly empty
        // The path "" will try to look up key "" in the map, which doesn't exist
        // So it returns Ok(()) because missing key is not an error
        assert!(result.is_ok());
    }

    #[test]
    fn replace_at_path_missing_key_returns_ok() {
        let mut map = BTreeMap::new();
        map.insert("key".to_string(), Value::U64(1));
        // Nonexistent key -> returns Ok(())
        let result = map.replace_at_path("nonexistent", ReplacementType::BinaryBytes);
        assert!(result.is_ok());
    }

    #[test]
    fn replace_at_path_non_map_value_in_nested_path_error() {
        let mut map = BTreeMap::new();
        map.insert("key".to_string(), Value::U64(42));
        // Trying to traverse into a non-map/non-array value
        let result = map.replace_at_path("key.sub", ReplacementType::BinaryBytes);
        assert!(matches!(result, Err(Error::PathError(_))));
    }

    // -----------------------------------------------------------------------
    // replace_at_paths — multiple paths
    // -----------------------------------------------------------------------

    #[test]
    fn replace_at_paths_multiple() {
        let bytes1 = [0xAAu8; 32];
        let bytes2 = [0xBBu8; 32];
        let mut map = BTreeMap::new();
        map.insert("id1".to_string(), Value::Bytes32(bytes1));
        map.insert("id2".to_string(), Value::Bytes32(bytes2));

        let paths = vec!["id1".to_string(), "id2".to_string()];
        map.replace_at_paths(&paths, ReplacementType::Identifier)
            .unwrap();

        assert_eq!(map.get("id1"), Some(&Value::Identifier(bytes1)));
        assert_eq!(map.get("id2"), Some(&Value::Identifier(bytes2)));
    }

    // -----------------------------------------------------------------------
    // replace_consume_value and replace_value_in_place
    // -----------------------------------------------------------------------

    #[test]
    fn replace_consume_value_identifier_to_base58() {
        let bytes = [0xCCu8; 32];
        let val = Value::Identifier(bytes);
        let result = ReplacementType::TextBase58
            .replace_consume_value(val)
            .unwrap();
        let expected = bs58::encode(bytes).into_string();
        assert_eq!(result, Value::Text(expected));
    }

    #[test]
    fn replace_value_in_place_identifier_to_binary() {
        let bytes = [0xDDu8; 32];
        let mut val = Value::Identifier(bytes);
        ReplacementType::BinaryBytes
            .replace_value_in_place(&mut val)
            .unwrap();
        assert_eq!(val, Value::Bytes(bytes.to_vec()));
    }
}
