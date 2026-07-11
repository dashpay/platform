use crate::btreemap_extensions::btreemap_field_replacement::IntegerReplacementType;
use crate::inner_value_at_path::is_array_path;
use crate::{Error, ReplacementType, Value, ValueMapHelper};
use std::collections::HashSet;

impl Value {
    /// If the `Value` is a `Map`, replaces the value at the path inside the map.
    /// This is used to set inner values as Identifiers or BinaryData, or from Identifiers or
    /// BinaryData to base58 or base64 strings.
    /// Either returns `Err(Error::Structure("reason"))` or `Err(Error::ByteLengthNot32BytesError))`
    /// if the replacement can not happen.
    ///
    /// ```
    /// # use platform_value::{Error, Identifier, ReplacementType, Value};
    /// #
    /// let mut inner_value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("food_id")), Value::Text("6oFRdsUNiAtXscRn52atKYCiF8RBnH9vbUzhtzY3d83e".to_string())),
    ///     ]
    /// );
    /// let mut value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("foods")), inner_value),
    ///     ]
    /// );
    ///
    /// value.replace_at_path("foods.food_id", ReplacementType::Identifier).expect("expected to replace at path with identifier");
    ///
    /// assert_eq!(value.get_value_at_path("foods.food_id"), Ok(&Value::Identifier([86, 35, 118, 67, 167, 43, 101, 109, 72, 97, 35, 99, 0, 254, 108, 154, 254, 154, 190, 40, 237, 25, 58, 246, 111, 19, 44, 215, 141, 140, 156, 117])));
    ///
    /// let mut tangerine_value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("food_id")), Value::Text("6oFRdsUNiAtXscRn52atKYCiF8RBnH9vbUzhtzY3d83e".to_string())),
    ///     ]
    /// );
    /// let mut mandarin_value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("food_id")), Value::Text("6oFRdsUNiAtXscRn52atKYCiF8RBnH9vbUzhtzY3d83e".to_string())),
    ///     ]
    /// );
    /// let mut oranges_value = Value::Array(
    ///     vec![
    ///         tangerine_value,
    ///         mandarin_value
    ///     ]
    /// );
    /// let mut value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("foods")), oranges_value),
    ///     ]
    /// );
    ///
    /// value.replace_at_path("foods[].food_id", ReplacementType::Identifier).expect("expected to replace at path with identifier");
    ///
    /// assert_eq!(value.get_value_at_path("foods[0].food_id"), Ok(&Value::Identifier([86, 35, 118, 67, 167, 43, 101, 109, 72, 97, 35, 99, 0, 254, 108, 154, 254, 154, 190, 40, 237, 25, 58, 246, 111, 19, 44, 215, 141, 140, 156, 117])));
    ///
    /// ```
    pub fn replace_at_path(
        &mut self,
        path: &str,
        replacement_type: ReplacementType,
    ) -> Result<(), Error> {
        let mut split = path.split('.').peekable();
        let mut current_values = vec![self];
        while let Some(path_component) = split.next() {
            if let Some((string_part, number_part)) = is_array_path(path_component)? {
                current_values = current_values
                    .into_iter()
                    .map(|current_value| {
                        let map = current_value.to_map_mut()?;
                        let array_value = map.get_key_mut(string_part)?;
                        let array = array_value.to_array_mut()?;
                        if let Some(number_part) = number_part {
                            if array.len() < number_part {
                                //this already exists
                                Ok(vec![array.get_mut(number_part).unwrap()])
                            } else {
                                Err(Error::StructureError(format!(
                                    "element at position {number_part} in array does not exist"
                                )))
                            }
                        } else {
                            // we are replacing all members in array
                            Ok(array.iter_mut().collect())
                        }
                    })
                    .collect::<Result<Vec<Vec<&mut Value>>, Error>>()?
                    .into_iter()
                    .flatten()
                    .collect()
            } else {
                current_values = current_values
                    .into_iter()
                    .filter_map(|current_value| {
                        let map = match current_value.as_map_mut_ref() {
                            Ok(map) => map,
                            Err(err) => return Some(Err(err)),
                        };

                        let new_value = map.get_optional_key_mut(path_component)?;

                        if split.peek().is_none() {
                            let bytes_result = match replacement_type {
                                ReplacementType::Identifier | ReplacementType::TextBase58 => {
                                    new_value.to_identifier_bytes()
                                }
                                ReplacementType::BinaryBytes | ReplacementType::TextBase64 => {
                                    new_value.to_binary_bytes()
                                }
                            };
                            let bytes = match bytes_result {
                                Ok(bytes) => bytes,
                                Err(err) => return Some(Err(err)),
                            };
                            *new_value = match replacement_type.replace_for_bytes(bytes) {
                                Ok(value) => value,
                                Err(err) => return Some(Err(err)),
                            };
                            return None;
                        }
                        Some(Ok(new_value))
                    })
                    .collect::<Result<Vec<&mut Value>, Error>>()?;
            }
        }
        Ok(())
    }

    /// Calls replace_at_path for every path in a given array.
    /// Either returns `Err(Error::Structure("reason"))` or `Err(Error::ByteLengthNot32BytesError))`
    /// if the replacement can not happen.
    ///
    /// ```
    /// # use platform_value::{Error, Identifier, ReplacementType, Value};
    /// #
    /// let mut inner_value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("grapes")), Value::Text("6oFRdsUNiAtXscRn52atKYCiF8RBnH9vbUzhtzY3d83e".to_string())),
    ///         (Value::Text(String::from("oranges")), Value::Array(vec![Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101)])),
    ///     ]
    /// );
    /// let mut value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("foods")), inner_value),
    ///     ]
    /// );
    ///
    /// let paths = vec!["foods.grapes", "foods.oranges"];
    ///
    /// value.replace_at_paths(paths, ReplacementType::Identifier).expect("expected to replace at paths with identifier");
    ///
    /// assert_eq!(value.get_value_at_path("foods.grapes"), Ok(&Value::Identifier([86, 35, 118, 67, 167, 43, 101, 109, 72, 97, 35, 99, 0, 254, 108, 154, 254, 154, 190, 40, 237, 25, 58, 246, 111, 19, 44, 215, 141, 140, 156, 117])));
    /// assert_eq!(value.get_value_at_path("foods.oranges"), Ok(&Value::Identifier([104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101])));
    ///
    /// ```
    pub fn replace_at_paths<'a, I: IntoIterator<Item = &'a str>>(
        &mut self,
        paths: I,
        replacement_type: ReplacementType,
    ) -> Result<(), Error> {
        paths
            .into_iter()
            .try_for_each(|path| self.replace_at_path(path, replacement_type))
    }

    /// If the `Value` is a `Map`, replaces the value at the path inside the map.
    /// This is used to set inner values as Identifiers or BinaryData, or from Identifiers or
    /// BinaryData to base58 or base64 strings.
    /// Either returns `Err(Error::Structure("reason"))` or `Err(Error::ByteLengthNot32BytesError))`
    /// if the replacement can not happen.
    ///
    /// ```
    /// # use platform_value::{Error, Identifier, IntegerReplacementType, Value};
    /// #
    /// let mut inner_value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("food_id")), Value::U8(5)),
    ///     ]
    /// );
    /// let mut value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("foods")), inner_value),
    ///     ]
    /// );
    ///
    /// value.replace_integer_type_at_path("foods.food_id", IntegerReplacementType::U32).expect("expected to replace at path with identifier");
    ///
    /// assert_eq!(value.get_value_at_path("foods.food_id"), Ok(&Value::U32(5)));
    ///
    /// let mut tangerine_value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("food_id")), Value::U128(8)),
    ///     ]
    /// );
    /// let mut mandarin_value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("food_id")), Value::U32(2)),
    ///     ]
    /// );
    /// let mut oranges_value = Value::Array(
    ///     vec![
    ///         tangerine_value,
    ///         mandarin_value
    ///     ]
    /// );
    /// let mut value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("foods")), oranges_value),
    ///     ]
    /// );
    ///
    /// value.replace_integer_type_at_path("foods[].food_id", IntegerReplacementType::U16).expect("expected to replace at path with identifier");
    ///
    /// assert_eq!(value.get_value_at_path("foods[0].food_id"), Ok(&Value::U16(8)));
    ///
    /// ```
    pub fn replace_integer_type_at_path(
        &mut self,
        path: &str,
        replacement_type: IntegerReplacementType,
    ) -> Result<(), Error> {
        let mut split = path.split('.').peekable();
        let mut current_values = vec![self];
        while let Some(path_component) = split.next() {
            if let Some((string_part, number_part)) = is_array_path(path_component)? {
                current_values = current_values
                    .into_iter()
                    .map(|current_value| {
                        let map = current_value.to_map_mut()?;
                        let array_value = map.get_key_mut(string_part)?;
                        let array = array_value.to_array_mut()?;
                        if let Some(number_part) = number_part {
                            if array.len() < number_part {
                                //this already exists
                                Ok(vec![array.get_mut(number_part).unwrap()])
                            } else {
                                Err(Error::StructureError(format!(
                                    "element at position {number_part} in array does not exist"
                                )))
                            }
                        } else {
                            // we are replacing all members in array
                            Ok(array.iter_mut().collect())
                        }
                    })
                    .collect::<Result<Vec<Vec<&mut Value>>, Error>>()?
                    .into_iter()
                    .flatten()
                    .collect()
            } else {
                current_values = current_values
                    .into_iter()
                    .filter_map(|current_value| {
                        let map = match current_value.as_map_mut_ref() {
                            Ok(map) => map,
                            Err(err) => return Some(Err(err)),
                        };

                        let new_value = map.get_optional_key_mut(path_component)?;

                        if split.peek().is_none() {
                            *new_value = match replacement_type.replace_for_value(new_value.clone())
                            {
                                Ok(value) => value,
                                Err(err) => return Some(Err(err)),
                            };
                            return None;
                        }
                        Some(Ok(new_value))
                    })
                    .collect::<Result<Vec<&mut Value>, Error>>()?;
            }
        }
        Ok(())
    }

    /// Calls replace_at_path for every path in a given array.
    /// Either returns `Err(Error::Structure("reason"))` or `Err(Error::ByteLengthNot32BytesError))`
    /// if the replacement can not happen.
    ///
    /// ```
    /// # use platform_value::{Error, Identifier, IntegerReplacementType, ReplacementType, Value};
    /// #
    /// let mut inner_value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("grapes")), Value::U16(5)),
    ///         (Value::Text(String::from("oranges")), Value::I32(6)),
    ///     ]
    /// );
    /// let mut value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("foods")), inner_value),
    ///     ]
    /// );
    ///
    /// let paths = vec!["foods.grapes", "foods.oranges"];
    ///
    /// value.replace_integer_type_at_paths(paths, IntegerReplacementType::U32).expect("expected to replace at paths with identifier");
    ///
    /// assert_eq!(value.get_value_at_path("foods.grapes"), Ok(&Value::U32(5)));
    /// assert_eq!(value.get_value_at_path("foods.oranges"), Ok(&Value::U32(6)));
    ///
    /// ```
    pub fn replace_integer_type_at_paths<'a, I: IntoIterator<Item = &'a str>>(
        &mut self,
        paths: I,
        replacement_type: IntegerReplacementType,
    ) -> Result<(), Error> {
        paths
            .into_iter()
            .try_for_each(|path| self.replace_integer_type_at_path(path, replacement_type))
    }

    /// `replace_to_binary_types_when_setting_with_path` will replace a value with a corresponding
    /// binary type (Identifier or Binary Data) if that data is in one of the given paths.
    /// Paths can either be terminal, or can represent an object or an array (with values) where
    /// all subvalues must be set to the binary type.
    /// Either returns `Err(Error::Structure("reason"))` or `Err(Error::ByteLengthNot32BytesError))`
    /// if the replacement can not happen.
    ///
    /// ```
    /// # use std::collections::HashSet;
    /// use platform_value::{Error, Identifier, ReplacementType, Value};
    /// #
    /// let mut inner_inner_value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("mandarins")), Value::Text("6oFRdsUNiAtXscRn52atKYCiF8RBnH9vbUzhtzY3d83e".to_string())),
    ///         (Value::Text(String::from("tangerines")), Value::Array(vec![Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101)])),
    ///     ]
    /// );
    /// let mut inner_value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("grapes")), Value::Text("6oFRdsUNiAtXscRn52atKYCiF8RBnH9vbUzhtzY3d83e".to_string())),
    ///         (Value::Text(String::from("oranges")), inner_inner_value),
    ///     ]
    /// );
    /// let mut value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("foods")), inner_value),
    ///     ]
    /// );
    ///
    ///
    /// let identifier_paths = HashSet::from(["foods.oranges.tangerines"]);
    ///
    /// value.replace_to_binary_types_of_root_value_when_setting_at_path("foods.oranges", identifier_paths, HashSet::new()).expect("expected to replace at paths with identifier");
    ///
    /// assert_eq!(value.get_value_at_path("foods.oranges.tangerines"), Ok(&Value::Identifier([104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101])));
    ///
    /// ```
    pub fn replace_to_binary_types_of_root_value_when_setting_at_path(
        &mut self,
        path: &str,
        identifier_paths: HashSet<&str>,
        binary_paths: HashSet<&str>,
    ) -> Result<(), Error> {
        if identifier_paths.contains(path) {
            ReplacementType::Identifier.replace_value_in_place(self)?;
        } else if binary_paths.contains(path) {
            ReplacementType::BinaryBytes.replace_value_in_place(self)?;
        } else {
            identifier_paths
                .into_iter()
                .try_for_each(|identifier_path| {
                    if identifier_path.starts_with(path) {
                        self.replace_at_path(identifier_path, ReplacementType::Identifier)
                            .map(|_| ())
                    } else {
                        Ok(())
                    }
                })?;

            binary_paths.into_iter().try_for_each(|binary_path| {
                if binary_path.starts_with(path) {
                    self.replace_at_path(binary_path, ReplacementType::BinaryBytes)
                        .map(|_| ())
                } else {
                    Ok(())
                }
            })?;
        }
        Ok(())
    }

    /// `replace_to_binary_types_when_setting_with_path` will replace a value with a corresponding
    /// binary type (Identifier or Binary Data) if that data is in one of the given paths.
    /// Paths can either be terminal, or can represent an object or an array (with values) where
    /// all subvalues must be set to the binary type.
    /// Either returns `Err(Error::Structure("reason"))` or `Err(Error::ByteLengthNot32BytesError))`
    /// if the replacement can not happen.
    ///
    /// ```
    /// # use std::collections::HashSet;
    /// use platform_value::{Error, Identifier, ReplacementType, Value};
    /// #
    /// let mut inner_inner_value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("mandarins")), Value::Text("6oFRdsUNiAtXscRn52atKYCiF8RBnH9vbUzhtzY3d83e".to_string())),
    ///         (Value::Text(String::from("tangerines")), Value::Array(vec![Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101), Value::U8(108),Value::U8(104), Value::U8(101)])),
    ///     ]
    /// );
    /// let mut inner_value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("grapes")), Value::Text("6oFRdsUNiAtXscRn52atKYCiF8RBnH9vbUzhtzY3d83e".to_string())),
    ///         (Value::Text(String::from("oranges")), inner_inner_value),
    ///     ]
    /// );
    /// let mut value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("foods")), inner_value),
    ///     ]
    /// );
    ///
    ///
    /// let identifier_paths = HashSet::from(["foods.oranges.tangerines"]);
    ///
    /// let oranges = value.get_mut_value_at_path("foods.oranges").unwrap();
    /// oranges.replace_to_binary_types_when_setting_with_path("foods.oranges", identifier_paths, HashSet::new()).expect("expected to replace at paths with identifier");
    ///
    /// assert_eq!(value.get_value_at_path("foods.oranges.tangerines"), Ok(&Value::Identifier([104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101, 108, 104, 101])));
    ///
    /// ```
    pub fn replace_to_binary_types_when_setting_with_path(
        &mut self,
        path: &str,
        identifier_paths: HashSet<&str>,
        binary_paths: HashSet<&str>,
    ) -> Result<(), Error> {
        if identifier_paths.contains(path) {
            ReplacementType::Identifier.replace_value_in_place(self)?;
        } else if binary_paths.contains(path) {
            ReplacementType::BinaryBytes.replace_value_in_place(self)?;
        } else {
            let mut path = path.to_string();
            path.push('.');
            identifier_paths
                .into_iter()
                .try_for_each(|identifier_path| {
                    if let Some(suffix) = identifier_path.strip_prefix(path.as_str()) {
                        self.replace_at_path(suffix, ReplacementType::Identifier)
                            .map(|_| ())
                    } else {
                        Ok(())
                    }
                })?;
            binary_paths.into_iter().try_for_each(|binary_path| {
                if let Some(suffix) = binary_path.strip_prefix(path.as_str()) {
                    self.replace_at_path(suffix, ReplacementType::BinaryBytes)
                        .map(|_| ())
                } else {
                    Ok(())
                }
            })?;
        }
        Ok(())
    }

    /// Cleans all values and removes null inner values at any depth.
    /// if the replacement can not happen.
    ///
    /// ```
    /// # use platform_value::{Error, Identifier, IntegerReplacementType, ReplacementType, Value};
    /// #
    /// let mut inner_value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("grapes")), Value::Null),
    ///         (Value::Text(String::from("oranges")), Value::I32(6)),
    ///     ]
    /// );
    /// let mut value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("foods")), inner_value),
    ///     ]
    /// );
    ///
    /// value = value.clean_recursive().unwrap();
    ///
    /// assert_eq!(value.get_optional_value_at_path("foods.grapes"), Ok(None));
    ///
    pub fn clean_recursive(self) -> Result<Value, Error> {
        Ok(Value::Map(
            self.into_map()?
                .into_iter()
                .filter_map(|(key, value)| {
                    if value.is_null() {
                        None
                    } else if value.is_map() {
                        match value.clean_recursive() {
                            Ok(value) => Some(Ok((key, value))),
                            Err(e) => Some(Err(e)),
                        }
                    } else {
                        Some(Ok((key, value)))
                    }
                })
                .collect::<Result<Vec<_>, Error>>()?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{IntegerReplacementType, ReplacementType, Value};

    // ---------------------------------------------------------------
    // Helper: builds a 32-byte base58-encoded string from a seed byte
    // ---------------------------------------------------------------
    fn base58_of_32_bytes(seed: u8) -> String {
        bs58::encode([seed; 32]).into_string()
    }

    fn make_32_u8_array(seed: u8) -> Vec<Value> {
        vec![Value::U8(seed); 32]
    }

    // ===============================================================
    // replace_at_path — single segment, ReplacementType::Identifier
    // ===============================================================

    #[test]
    fn replace_at_path_single_segment_identifier_from_text() {
        let b58 = base58_of_32_bytes(1);
        let mut value = Value::Map(vec![(Value::Text("id".into()), Value::Text(b58))]);
        value
            .replace_at_path("id", ReplacementType::Identifier)
            .unwrap();
        assert_eq!(
            value.get_value_at_path("id").unwrap(),
            &Value::Identifier([1u8; 32])
        );
    }

    #[test]
    fn replace_at_path_single_segment_identifier_from_u8_array() {
        let mut value = Value::Map(vec![(
            Value::Text("id".into()),
            Value::Array(make_32_u8_array(7)),
        )]);
        value
            .replace_at_path("id", ReplacementType::Identifier)
            .unwrap();
        assert_eq!(
            value.get_value_at_path("id").unwrap(),
            &Value::Identifier([7u8; 32])
        );
    }

    // ===============================================================
    // replace_at_path — single segment, ReplacementType::BinaryBytes
    // ===============================================================

    #[test]
    fn replace_at_path_single_segment_binary_bytes_from_base64() {
        use base64::prelude::*;
        let raw = vec![10u8, 20, 30];
        let b64 = BASE64_STANDARD.encode(&raw);
        let mut value = Value::Map(vec![(Value::Text("data".into()), Value::Text(b64))]);
        value
            .replace_at_path("data", ReplacementType::BinaryBytes)
            .unwrap();
        assert_eq!(value.get_value_at_path("data").unwrap(), &Value::Bytes(raw));
    }

    // ===============================================================
    // replace_at_path — single segment, ReplacementType::TextBase58
    // ===============================================================

    #[test]
    fn replace_at_path_single_segment_text_base58() {
        let mut value = Value::Map(vec![(
            Value::Text("id".into()),
            Value::Bytes(vec![1, 2, 3]),
        )]);
        value
            .replace_at_path("id", ReplacementType::TextBase58)
            .unwrap();
        let expected_b58 = bs58::encode(vec![1u8, 2, 3]).into_string();
        assert_eq!(
            value.get_value_at_path("id").unwrap(),
            &Value::Text(expected_b58)
        );
    }

    // ===============================================================
    // replace_at_path — single segment, ReplacementType::TextBase64
    // ===============================================================

    #[test]
    fn replace_at_path_single_segment_text_base64() {
        use base64::prelude::*;
        let raw = vec![1u8, 2, 3];
        let mut value = Value::Map(vec![(Value::Text("bin".into()), Value::Bytes(raw.clone()))]);
        value
            .replace_at_path("bin", ReplacementType::TextBase64)
            .unwrap();
        let expected_b64 = BASE64_STANDARD.encode(&raw);
        assert_eq!(
            value.get_value_at_path("bin").unwrap(),
            &Value::Text(expected_b64)
        );
    }

    // ===============================================================
    // replace_at_path — multi-segment nested path
    // ===============================================================

    #[test]
    fn replace_at_path_multi_segment_nested() {
        let b58 = base58_of_32_bytes(5);
        let inner = Value::Map(vec![(Value::Text("owner_id".into()), Value::Text(b58))]);
        let mut value = Value::Map(vec![(Value::Text("doc".into()), inner)]);

        value
            .replace_at_path("doc.owner_id", ReplacementType::Identifier)
            .unwrap();
        assert_eq!(
            value.get_value_at_path("doc.owner_id").unwrap(),
            &Value::Identifier([5u8; 32])
        );
    }

    // ===============================================================
    // replace_at_path — array path with [] (all members)
    // ===============================================================

    #[test]
    fn replace_at_path_array_all_members() {
        let b58 = base58_of_32_bytes(3);
        let elem1 = Value::Map(vec![(Value::Text("id".into()), Value::Text(b58.clone()))]);
        let elem2 = Value::Map(vec![(Value::Text("id".into()), Value::Text(b58))]);
        let arr = Value::Array(vec![elem1, elem2]);
        let mut value = Value::Map(vec![(Value::Text("items".into()), arr)]);

        value
            .replace_at_path("items[].id", ReplacementType::Identifier)
            .unwrap();
        assert_eq!(
            value.get_value_at_path("items[0].id").unwrap(),
            &Value::Identifier([3u8; 32])
        );
        assert_eq!(
            value.get_value_at_path("items[1].id").unwrap(),
            &Value::Identifier([3u8; 32])
        );
    }

    // ===============================================================
    // replace_at_path — optional key missing returns Ok
    // ===============================================================

    #[test]
    fn replace_at_path_missing_key_returns_ok() {
        let mut value = Value::Map(vec![(Value::Text("a".into()), Value::U32(42))]);
        // "b" does not exist — filter_map filters it out
        let result = value.replace_at_path("b", ReplacementType::Identifier);
        assert!(result.is_ok());
    }

    // ===============================================================
    // replace_at_path — non-map at root gives error
    // ===============================================================

    #[test]
    fn replace_at_path_on_non_map_errors() {
        let mut value = Value::U32(42);
        let result = value.replace_at_path("key", ReplacementType::Identifier);
        assert!(result.is_err());
    }

    // ===============================================================
    // replace_at_path — non-32-byte data for Identifier errors
    // ===============================================================

    #[test]
    fn replace_at_path_identifier_wrong_length_errors() {
        // base58-encode only 10 bytes, not 32
        let b58 = bs58::encode([1u8; 10]).into_string();
        let mut value = Value::Map(vec![(Value::Text("id".into()), Value::Text(b58))]);
        let result = value.replace_at_path("id", ReplacementType::Identifier);
        assert!(result.is_err());
    }

    // ===============================================================
    // replace_at_paths — multiple paths
    // ===============================================================

    #[test]
    fn replace_at_paths_replaces_multiple() {
        let b58 = base58_of_32_bytes(9);
        let inner = Value::Map(vec![
            (Value::Text("a".into()), Value::Text(b58.clone())),
            (Value::Text("b".into()), Value::Text(b58)),
        ]);
        let mut value = Value::Map(vec![(Value::Text("root".into()), inner)]);
        value
            .replace_at_paths(vec!["root.a", "root.b"], ReplacementType::Identifier)
            .unwrap();
        assert_eq!(
            value.get_value_at_path("root.a").unwrap(),
            &Value::Identifier([9u8; 32])
        );
        assert_eq!(
            value.get_value_at_path("root.b").unwrap(),
            &Value::Identifier([9u8; 32])
        );
    }

    // ===============================================================
    // replace_integer_type_at_path — single segment, U32
    // ===============================================================

    #[test]
    fn replace_integer_type_single_segment_u32() {
        let mut value = Value::Map(vec![(Value::Text("count".into()), Value::U8(5))]);
        value
            .replace_integer_type_at_path("count", IntegerReplacementType::U32)
            .unwrap();
        assert_eq!(value.get_value_at_path("count").unwrap(), &Value::U32(5));
    }

    // ===============================================================
    // replace_integer_type_at_path — single segment, various types
    // ===============================================================

    #[test]
    fn replace_integer_type_single_segment_u16() {
        let mut value = Value::Map(vec![(Value::Text("v".into()), Value::U128(100))]);
        value
            .replace_integer_type_at_path("v", IntegerReplacementType::U16)
            .unwrap();
        assert_eq!(value.get_value_at_path("v").unwrap(), &Value::U16(100));
    }

    #[test]
    fn replace_integer_type_single_segment_u64() {
        let mut value = Value::Map(vec![(Value::Text("v".into()), Value::U8(42))]);
        value
            .replace_integer_type_at_path("v", IntegerReplacementType::U64)
            .unwrap();
        assert_eq!(value.get_value_at_path("v").unwrap(), &Value::U64(42));
    }

    #[test]
    fn replace_integer_type_single_segment_i32() {
        let mut value = Value::Map(vec![(Value::Text("v".into()), Value::I8(-3))]);
        value
            .replace_integer_type_at_path("v", IntegerReplacementType::I32)
            .unwrap();
        assert_eq!(value.get_value_at_path("v").unwrap(), &Value::I32(-3));
    }

    #[test]
    fn replace_integer_type_single_segment_u128() {
        let mut value = Value::Map(vec![(Value::Text("v".into()), Value::U64(999))]);
        value
            .replace_integer_type_at_path("v", IntegerReplacementType::U128)
            .unwrap();
        assert_eq!(value.get_value_at_path("v").unwrap(), &Value::U128(999));
    }

    #[test]
    fn replace_integer_type_single_segment_i128() {
        let mut value = Value::Map(vec![(Value::Text("v".into()), Value::I64(-500))]);
        value
            .replace_integer_type_at_path("v", IntegerReplacementType::I128)
            .unwrap();
        assert_eq!(value.get_value_at_path("v").unwrap(), &Value::I128(-500));
    }

    #[test]
    fn replace_integer_type_single_segment_u8() {
        let mut value = Value::Map(vec![(Value::Text("v".into()), Value::U16(7))]);
        value
            .replace_integer_type_at_path("v", IntegerReplacementType::U8)
            .unwrap();
        assert_eq!(value.get_value_at_path("v").unwrap(), &Value::U8(7));
    }

    #[test]
    fn replace_integer_type_single_segment_i8() {
        let mut value = Value::Map(vec![(Value::Text("v".into()), Value::I16(-2))]);
        value
            .replace_integer_type_at_path("v", IntegerReplacementType::I8)
            .unwrap();
        assert_eq!(value.get_value_at_path("v").unwrap(), &Value::I8(-2));
    }

    #[test]
    fn replace_integer_type_single_segment_i16() {
        let mut value = Value::Map(vec![(Value::Text("v".into()), Value::I32(-100))]);
        value
            .replace_integer_type_at_path("v", IntegerReplacementType::I16)
            .unwrap();
        assert_eq!(value.get_value_at_path("v").unwrap(), &Value::I16(-100));
    }

    #[test]
    fn replace_integer_type_single_segment_i64() {
        let mut value = Value::Map(vec![(Value::Text("v".into()), Value::I128(-9999))]);
        value
            .replace_integer_type_at_path("v", IntegerReplacementType::I64)
            .unwrap();
        assert_eq!(value.get_value_at_path("v").unwrap(), &Value::I64(-9999));
    }

    // ===============================================================
    // replace_integer_type_at_path — multi-segment
    // ===============================================================

    #[test]
    fn replace_integer_type_multi_segment() {
        let inner = Value::Map(vec![(Value::Text("level".into()), Value::U8(10))]);
        let mut value = Value::Map(vec![(Value::Text("nested".into()), inner)]);
        value
            .replace_integer_type_at_path("nested.level", IntegerReplacementType::U32)
            .unwrap();
        assert_eq!(
            value.get_value_at_path("nested.level").unwrap(),
            &Value::U32(10)
        );
    }

    // ===============================================================
    // replace_integer_type_at_path — array path
    // ===============================================================

    #[test]
    fn replace_integer_type_array_all_members() {
        let elem1 = Value::Map(vec![(Value::Text("n".into()), Value::U128(8))]);
        let elem2 = Value::Map(vec![(Value::Text("n".into()), Value::U32(2))]);
        let arr = Value::Array(vec![elem1, elem2]);
        let mut value = Value::Map(vec![(Value::Text("items".into()), arr)]);
        value
            .replace_integer_type_at_path("items[].n", IntegerReplacementType::U16)
            .unwrap();
        assert_eq!(
            value.get_value_at_path("items[0].n").unwrap(),
            &Value::U16(8)
        );
        assert_eq!(
            value.get_value_at_path("items[1].n").unwrap(),
            &Value::U16(2)
        );
    }

    // ===============================================================
    // replace_integer_type_at_path — missing key returns Ok
    // ===============================================================

    #[test]
    fn replace_integer_type_missing_key_returns_ok() {
        let mut value = Value::Map(vec![(Value::Text("a".into()), Value::U32(1))]);
        let result = value.replace_integer_type_at_path("missing", IntegerReplacementType::U32);
        assert!(result.is_ok());
    }

    // ===============================================================
    // replace_integer_type_at_path — non-map errors
    // ===============================================================

    #[test]
    fn replace_integer_type_on_non_map_errors() {
        let mut value = Value::U32(42);
        let result = value.replace_integer_type_at_path("key", IntegerReplacementType::U32);
        assert!(result.is_err());
    }

    // ===============================================================
    // replace_integer_type_at_paths — multiple paths
    // ===============================================================

    #[test]
    fn replace_integer_type_at_paths_replaces_multiple() {
        let inner = Value::Map(vec![
            (Value::Text("x".into()), Value::U16(5)),
            (Value::Text("y".into()), Value::I32(6)),
        ]);
        let mut value = Value::Map(vec![(Value::Text("root".into()), inner)]);
        value
            .replace_integer_type_at_paths(vec!["root.x", "root.y"], IntegerReplacementType::U32)
            .unwrap();
        assert_eq!(value.get_value_at_path("root.x").unwrap(), &Value::U32(5));
        assert_eq!(value.get_value_at_path("root.y").unwrap(), &Value::U32(6));
    }

    // ===============================================================
    // replace_to_binary_types_of_root_value_when_setting_at_path
    // — identifier match (exact path in identifier_paths)
    // ===============================================================

    #[test]
    fn replace_root_binary_types_identifier_exact_match() {
        let b58 = base58_of_32_bytes(2);
        let mut value = Value::Text(b58);
        let identifier_paths = HashSet::from(["my_id"]);
        value
            .replace_to_binary_types_of_root_value_when_setting_at_path(
                "my_id",
                identifier_paths,
                HashSet::new(),
            )
            .unwrap();
        assert_eq!(value, Value::Identifier([2u8; 32]));
    }

    // ===============================================================
    // replace_to_binary_types_of_root_value_when_setting_at_path
    // — binary match (exact path in binary_paths)
    // ===============================================================

    #[test]
    fn replace_root_binary_types_binary_exact_match() {
        let b58 = base58_of_32_bytes(4);
        let mut value = Value::Text(b58);
        let binary_paths = HashSet::from(["my_data"]);
        value
            .replace_to_binary_types_of_root_value_when_setting_at_path(
                "my_data",
                HashSet::new(),
                binary_paths,
            )
            .unwrap();
        // BinaryBytes uses into_identifier_bytes (base58 decode) then replace_for_bytes -> Value::Bytes
        assert_eq!(value, Value::Bytes([4u8; 32].to_vec()));
    }

    // ===============================================================
    // replace_to_binary_types_of_root_value_when_setting_at_path
    // — prefix-based partial replacement (path starts_with)
    // ===============================================================

    #[test]
    fn replace_root_binary_types_prefix_based() {
        let b58 = base58_of_32_bytes(6);
        let inner = Value::Map(vec![(Value::Text("sub_id".into()), Value::Text(b58))]);
        let mut value = Value::Map(vec![(Value::Text("nested".into()), inner)]);
        let identifier_paths = HashSet::from(["root.nested.sub_id"]);
        value
            .replace_to_binary_types_of_root_value_when_setting_at_path(
                "root",
                identifier_paths,
                HashSet::new(),
            )
            .unwrap();
        // The identifier_path "root.nested.sub_id" starts_with "root", so
        // replace_at_path("root.nested.sub_id", Identifier) is called on self.
        // But self is the map starting at "nested", so the full path from self's
        // perspective is "root.nested.sub_id" which includes the "root" prefix --
        // this means it tries to find "root" key in self. Since our value doesn't
        // have a "root" key, the replacement is silently skipped.
        // This is the actual behavior of the method.
    }

    #[test]
    fn replace_root_binary_types_prefix_replaces_sub_path() {
        let b58 = base58_of_32_bytes(6);
        let inner = Value::Map(vec![(Value::Text("sub_id".into()), Value::Text(b58))]);
        let mut value = Value::Map(vec![(Value::Text("nested".into()), inner)]);
        // The identifier path starts with the path prefix
        let identifier_paths = HashSet::from(["doc.nested.sub_id"]);
        value
            .replace_to_binary_types_of_root_value_when_setting_at_path(
                "doc",
                identifier_paths,
                HashSet::new(),
            )
            .unwrap();
        // "doc.nested.sub_id".starts_with("doc") is true, so
        // self.replace_at_path("doc.nested.sub_id", Identifier) is called.
        // self doesn't have key "doc", so the filter_map returns empty vec.
        // This is the expected silent skip behavior.
    }

    // ===============================================================
    // replace_to_binary_types_of_root_value_when_setting_at_path
    // — no match at all, returns Ok
    // ===============================================================

    #[test]
    fn replace_root_binary_types_no_match_returns_ok() {
        let mut value = Value::Map(vec![(Value::Text("a".into()), Value::U32(1))]);
        let result = value.replace_to_binary_types_of_root_value_when_setting_at_path(
            "unrelated",
            HashSet::from(["other.path"]),
            HashSet::new(),
        );
        assert!(result.is_ok());
    }

    // ===============================================================
    // replace_to_binary_types_when_setting_with_path — identifier
    // ===============================================================

    #[test]
    fn replace_when_setting_with_path_identifier_exact() {
        let b58 = base58_of_32_bytes(11);
        let mut value = Value::Text(b58);
        let identifier_paths = HashSet::from(["doc.owner"]);
        value
            .replace_to_binary_types_when_setting_with_path(
                "doc.owner",
                identifier_paths,
                HashSet::new(),
            )
            .unwrap();
        assert_eq!(value, Value::Identifier([11u8; 32]));
    }

    // ===============================================================
    // replace_to_binary_types_when_setting_with_path — strip prefix
    // ===============================================================

    #[test]
    fn replace_when_setting_with_path_strip_prefix() {
        let b58 = base58_of_32_bytes(15);
        let mut value = Value::Map(vec![(Value::Text("sub_id".into()), Value::Text(b58))]);
        let identifier_paths = HashSet::from(["container.sub_id"]);
        value
            .replace_to_binary_types_when_setting_with_path(
                "container",
                identifier_paths,
                HashSet::new(),
            )
            .unwrap();
        assert_eq!(
            value.get_value_at_path("sub_id").unwrap(),
            &Value::Identifier([15u8; 32])
        );
    }

    // ===============================================================
    // replace_to_binary_types_when_setting_with_path — binary strip prefix
    // ===============================================================

    #[test]
    fn replace_when_setting_with_path_binary_strip_prefix() {
        use base64::prelude::*;
        let raw = vec![1u8, 2, 3, 4, 5];
        let b64 = BASE64_STANDARD.encode(&raw);
        let mut value = Value::Map(vec![(Value::Text("blob".into()), Value::Text(b64))]);
        let binary_paths = HashSet::from(["container.blob"]);
        value
            .replace_to_binary_types_when_setting_with_path(
                "container",
                HashSet::new(),
                binary_paths,
            )
            .unwrap();
        assert_eq!(value.get_value_at_path("blob").unwrap(), &Value::Bytes(raw));
    }

    // ===============================================================
    // replace_to_binary_types_when_setting_with_path — no match
    // ===============================================================

    #[test]
    fn replace_when_setting_with_path_no_match_ok() {
        let mut value = Value::Map(vec![(Value::Text("a".into()), Value::U32(1))]);
        let result = value.replace_to_binary_types_when_setting_with_path(
            "container",
            HashSet::from(["other.path"]),
            HashSet::new(),
        );
        assert!(result.is_ok());
    }

    // ===============================================================
    // clean_recursive — removes nulls from nested maps
    // ===============================================================

    #[test]
    fn clean_recursive_removes_nulls() {
        let inner = Value::Map(vec![
            (Value::Text("keep".into()), Value::U32(1)),
            (Value::Text("drop".into()), Value::Null),
        ]);
        let value = Value::Map(vec![
            (Value::Text("inner".into()), inner),
            (Value::Text("also_drop".into()), Value::Null),
        ]);
        let cleaned = value.clean_recursive().unwrap();
        let map = cleaned.to_map().unwrap();
        assert_eq!(map.len(), 1);
        let inner_map = map.get_key("inner").unwrap().to_map().unwrap();
        assert_eq!(inner_map.len(), 1);
        assert!(inner_map.get_optional_key("keep").is_some());
        assert!(inner_map.get_optional_key("drop").is_none());
    }

    #[test]
    fn clean_recursive_deeply_nested() {
        let deep = Value::Map(vec![
            (Value::Text("a".into()), Value::Null),
            (Value::Text("b".into()), Value::U8(1)),
        ]);
        let mid = Value::Map(vec![
            (Value::Text("deep".into()), deep),
            (Value::Text("c".into()), Value::Null),
        ]);
        let value = Value::Map(vec![(Value::Text("mid".into()), mid)]);
        let cleaned = value.clean_recursive().unwrap();
        let mid_map = cleaned.get_value_at_path("mid").unwrap().to_map().unwrap();
        assert_eq!(mid_map.len(), 1); // only "deep" remains
        let deep_map = cleaned
            .get_value_at_path("mid.deep")
            .unwrap()
            .to_map()
            .unwrap();
        assert_eq!(deep_map.len(), 1); // only "b" remains
    }

    #[test]
    fn clean_recursive_preserves_non_null_non_map_values() {
        let value = Value::Map(vec![
            (Value::Text("num".into()), Value::U64(42)),
            (Value::Text("text".into()), Value::Text("hello".into())),
            (
                Value::Text("arr".into()),
                Value::Array(vec![Value::Null, Value::U8(1)]),
            ),
        ]);
        let cleaned = value.clean_recursive().unwrap();
        let map = cleaned.to_map().unwrap();
        assert_eq!(map.len(), 3);
        // Arrays with Null inside are NOT cleaned (clean_recursive only filters map entries)
        let arr = map.get_key("arr").unwrap().to_array_ref().unwrap();
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn clean_recursive_all_null() {
        let value = Value::Map(vec![
            (Value::Text("a".into()), Value::Null),
            (Value::Text("b".into()), Value::Null),
        ]);
        let cleaned = value.clean_recursive().unwrap();
        let map = cleaned.to_map().unwrap();
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn clean_recursive_on_non_map_errors() {
        let value = Value::U32(42);
        let result = value.clean_recursive();
        assert!(result.is_err());
    }

    // ===============================================================
    // replace_at_path — intermediate non-map value errors
    // ===============================================================

    #[test]
    fn replace_at_path_intermediate_non_map_errors() {
        let mut value = Value::Map(vec![(Value::Text("a".into()), Value::U32(42))]);
        // "a" is U32, not a map, so traversing "a.b" should error
        let result = value.replace_at_path("a.b", ReplacementType::Identifier);
        assert!(result.is_err());
    }
}
