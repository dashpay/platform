use crate::{Error, ReplacementType, Value, ValueMapHelper};
use std::collections::{HashMap, HashSet};

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
    /// ```
    pub fn replace_at_path(
        &mut self,
        path: &str,
        replacement_type: ReplacementType,
    ) -> Result<bool, Error> {
        let mut split = path.split('.').peekable();
        let mut current_value = self;
        while let Some(path_component) = split.next() {
            let map = current_value.as_map_mut_ref()?;
            let Some(new_value) = map.get_key_mut(path_component) else {
                return Ok(false);
            };

            if split.peek().is_none() {
                let bytes = match replacement_type {
                    ReplacementType::Identifier | ReplacementType::TextBase58 => {
                        new_value.to_identifier_bytes()
                    }
                    ReplacementType::BinaryBytes | ReplacementType::TextBase64 => {
                        new_value.to_binary_bytes()
                    }
                }?;
                *new_value = replacement_type.replace_for_bytes(bytes)?;
                return Ok(true);
            }
            current_value = new_value;
        }
        Ok(false)
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
    ) -> Result<HashMap<&'a str, bool>, Error> {
        paths
            .into_iter()
            .map(|path| {
                let success = self.replace_at_path(path, replacement_type)?;
                Ok((path, success))
            })
            .collect()
    }

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
            identifier_paths
                .into_iter()
                .try_for_each(|identifier_path| {
                    if identifier_path.starts_with(path) {
                        let (_, suffix) = identifier_path.split_at(path.len() + 1);
                        self.replace_at_path(suffix, ReplacementType::Identifier)
                            .map(|_| ())
                    } else {
                        Ok(())
                    }
                })?;

            binary_paths.into_iter().try_for_each(|binary_path| {
                if binary_path.starts_with(path) {
                    let (_, suffix) = binary_path.split_at(path.len() + 1);
                    self.replace_at_path(suffix, ReplacementType::BinaryBytes)
                        .map(|_| ())
                } else {
                    Ok(())
                }
            })?;
        }
        Ok(())
    }
}
