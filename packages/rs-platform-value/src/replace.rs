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
    /// all subvalues must be set to the bianry type.
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
    /// all subvalues must be set to the bianry type.
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
