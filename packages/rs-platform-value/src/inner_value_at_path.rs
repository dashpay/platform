use crate::value_map::ValueMapHelper;
use crate::{error, Error, Value, ValueMap};
use std::collections::BTreeMap;

pub(crate) fn is_array_path(text: &str) -> Result<Option<(&str, Option<usize>)>, Error> {
    // 1. Find the last '[' character.
    let Some(open_bracket_pos) = text.rfind('[') else {
        return Ok(None);
    };

    // 2. Check if `text` ends with ']'.
    if !text.ends_with(']') {
        return Ok(None);
    }

    // 3. Extract the portion before the '[' as the field name.
    let field_name = &text[..open_bracket_pos];

    // 4. Ensure the field name consists only of word characters
    if field_name.is_empty() || !field_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Ok(None);
    }

    // 5. Extract the portion inside the brackets.
    let inside_brackets = &text[open_bracket_pos + 1..text.len() - 1];

    // 6. If the inside is empty, there is no index number
    if inside_brackets.is_empty() {
        return Ok(Some((field_name, None)));
    }

    // 7. Otherwise, parse the inside as a number (usize).
    let index = inside_brackets
        .parse::<usize>()
        .map_err(|_| Error::IntegerSizeError)?;

    Ok(Some((field_name, Some(index))))
}

impl Value {
    pub fn remove_value_at_path(&mut self, path: &str) -> Result<Value, Error> {
        let mut split = path.split('.').peekable();
        let mut current_value = self;
        let mut last_path_component = None;
        while let Some(path_component) = split.next() {
            if split.peek().is_none() {
                last_path_component = Some(path_component);
            } else {
                let map = current_value.to_map_mut()?;
                current_value = map.get_optional_key_mut(path_component).ok_or_else(|| {
                    Error::StructureError(format!(
                        "unable to remove property {path_component} in {path}"
                    ))
                })?;
            };
        }
        let Some(last_path_component) = last_path_component else {
            return Err(Error::StructureError("path was empty".to_string()));
        };
        let map = current_value.as_map_mut_ref()?;
        map.remove_key(last_path_component)
    }

    pub fn remove_optional_value_at_path(&mut self, path: &str) -> Result<Option<Value>, Error> {
        let mut split = path.split('.').peekable();
        let mut current_value = self;
        let mut last_path_component = None;
        while let Some(path_component) = split.next() {
            if split.peek().is_none() {
                last_path_component = Some(path_component);
            } else {
                let map = current_value.to_map_mut()?;
                if let Some(maybe_value) = map.get_optional_key_mut(path_component) {
                    current_value = maybe_value;
                } else {
                    return Ok(None);
                }
            };
        }
        let Some(last_path_component) = last_path_component else {
            return Err(Error::StructureError("path was empty".to_string()));
        };
        let map = current_value.as_map_mut_ref()?;
        Ok(map.remove_optional_key(last_path_component))
    }

    pub fn remove_values_matching_path(&mut self, path: &str) -> Result<Vec<Value>, Error> {
        let mut split = path.split('.').peekable();
        let mut current_values = vec![self];
        let mut removed_values = vec![];
        while let Some(path_component) = split.next() {
            if let Some((string_part, number_part)) = is_array_path(path_component)? {
                current_values = current_values
                    .into_iter()
                    .filter_map(|current_value| {
                        if current_value.is_null() {
                            return None;
                        }
                        let Some(map) = current_value.as_map_mut() else {
                            return Some(Err(Error::StructureError(
                                "value is not a map during removal".to_string(),
                            )));
                        };

                        let array_value = map.get_optional_key_mut(string_part)?;

                        if array_value.is_null() {
                            return None;
                        }
                        let Some(array) = array_value.as_array_mut() else {
                            return Some(Err(Error::StructureError(
                                "value is not an array during removal".to_string(),
                            )));
                        };
                        if let Some(number_part) = number_part {
                            if array.len() < number_part {
                                //this already exists
                                Some(Ok(vec![array.get_mut(number_part).unwrap()]))
                            } else {
                                Some(Err(Error::StructureError(format!(
                                    "element at position {number_part} in array does not exist"
                                ))))
                            }
                        } else {
                            // we are replacing all members in array
                            Some(Ok(array.iter_mut().collect()))
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
                        if current_value.is_null() {
                            return None;
                        }

                        let map = match current_value.as_map_mut_ref() {
                            Ok(map) => map,
                            Err(err) => return Some(Err(err)),
                        };

                        if split.peek().is_none() {
                            if let Some(removed) = map.remove_optional_key(path_component) {
                                removed_values.push(removed)
                            }
                            None
                        } else {
                            let new_value = map.get_optional_key_mut(path_component)?;
                            Some(Ok(new_value))
                        }
                    })
                    .collect::<Result<Vec<&mut Value>, Error>>()?;
            }
        }
        Ok(removed_values)
    }

    pub fn remove_value_at_path_into<T: TryFrom<Value, Error = error::Error>>(
        &mut self,
        path: &str,
    ) -> Result<T, Error> {
        self.remove_value_at_path(path)?.try_into()
    }

    pub fn remove_value_at_path_as_bytes(&mut self, path: &str) -> Result<Vec<u8>, Error> {
        self.remove_value_at_path(path)?.try_into()
    }

    pub fn remove_values_at_paths<'a>(
        &'a mut self,
        paths: Vec<&'a str>,
    ) -> Result<BTreeMap<&'a str, Value>, Error> {
        paths
            .into_iter()
            .map(|path| Ok((path, self.remove_value_at_path(path)?)))
            .collect()
    }

    pub fn remove_values_matching_paths<'a>(
        &'a mut self,
        paths: Vec<&'a str>,
    ) -> Result<BTreeMap<&'a str, Vec<Value>>, Error> {
        paths
            .into_iter()
            .map(|path| Ok((path, self.remove_values_matching_path(path)?)))
            .collect()
    }

    pub fn get_value_at_path<'a>(&'a self, path: &str) -> Result<&'a Value, Error> {
        let split = path.split('.');
        let mut current_value = self;
        for path_component in split {
            if let Some((string_part, number_part)) = is_array_path(path_component)? {
                let map = current_value.to_map_ref()?;
                let array_value = map.get_key(string_part)?;
                let array = array_value.to_array_ref()?;
                let Some(number_part) = number_part else {
                    return Err(Error::Unsupported("getting values of more than 1 member of an array is currently not supported".to_string()));
                };
                // We are setting the value of just member of the array
                if number_part < array.len() {
                    //this already exists
                    current_value = array.get(number_part).unwrap()
                } else {
                    return Err(Error::StructureError(
                        format!("trying to get the value in an array at an index {} higher than current array length {}", number_part, array.len()),
                    ));
                }
            } else {
                let map = current_value.to_map_ref()?;
                current_value = map.get_optional_key(path_component).ok_or_else(|| {
                    Error::StructureError(format!(
                        "unable to get property {path_component} in {path}"
                    ))
                })?;
            }
        }
        Ok(current_value)
    }

    pub fn get_optional_value_at_path<'a>(
        &'a self,
        path: &'a str,
    ) -> Result<Option<&'a Value>, Error> {
        let split = path.split('.');
        let mut current_value = self;
        for path_component in split {
            if let Some((string_part, number_part)) = is_array_path(path_component)? {
                let map = current_value.to_map_ref()?;
                let Some(array_value) = map.get_optional_key(string_part) else {
                    return Ok(None);
                };
                let array = array_value.to_array_ref()?;
                let Some(number_part) = number_part else {
                    return Err(Error::Unsupported(
                        "setting values of all members in an array is currently not supported"
                            .to_string(),
                    ));
                };
                // We are setting the value of just member of the array
                if number_part < array.len() {
                    //this already exists
                    current_value = array.get(number_part).unwrap()
                } else {
                    return Ok(None);
                }
            } else {
                let map = current_value.to_map_ref()?;
                let Some(new_value) = map.get_optional_key(path_component) else {
                    return Ok(None);
                };
                current_value = new_value;
            }
        }
        Ok(Some(current_value))
    }

    pub fn get_mut_value_at_path<'a>(&'a mut self, path: &'a str) -> Result<&'a mut Value, Error> {
        let split = path.split('.');
        let mut current_value = self;
        for path_component in split {
            let map = current_value.to_map_mut()?;
            current_value = map.get_optional_key_mut(path_component).ok_or_else(|| {
                Error::StructureError(format!(
                    "unable to get mut property {path_component} in {path}"
                ))
            })?;
        }
        Ok(current_value)
    }

    pub fn get_optional_mut_value_at_path<'a>(
        &'a mut self,
        path: &'a str,
    ) -> Result<Option<&'a mut Value>, Error> {
        let split = path.split('.');
        let mut current_value = self;
        for path_component in split {
            let map = current_value.to_map_mut()?;
            let Some(new_value) = map.get_optional_key_mut(path_component) else {
                return Ok(None);
            };
            current_value = new_value;
        }
        Ok(Some(current_value))
    }

    pub fn get_integer_at_path<T>(&self, path: &str) -> Result<T, Error>
    where
        T: TryFrom<i128>
            + TryFrom<u128>
            + TryFrom<u64>
            + TryFrom<i64>
            + TryFrom<u32>
            + TryFrom<i32>
            + TryFrom<u16>
            + TryFrom<i16>
            + TryFrom<u8>
            + TryFrom<i8>,
    {
        self.get_value_at_path(path)?.to_integer()
    }

    pub fn get_optional_integer_at_path<T>(&self, path: &str) -> Result<Option<T>, Error>
    where
        T: TryFrom<i128>
            + TryFrom<u128>
            + TryFrom<u64>
            + TryFrom<i64>
            + TryFrom<u32>
            + TryFrom<i32>
            + TryFrom<u16>
            + TryFrom<i16>
            + TryFrom<u8>
            + TryFrom<i8>,
    {
        self.get_optional_value_at_path(path)?
            .map(|value| value.to_integer())
            .transpose()
    }

    pub fn set_value_at_full_path(&mut self, path: &str, value: Value) -> Result<(), Error> {
        let mut split = path.split('.').peekable();
        let mut current_value = self;
        let mut last_path_component = None;
        while let Some(path_component) = split.next() {
            if split.peek().is_none() {
                last_path_component = Some(path_component);
            } else if let Some((string_part, number_part)) = is_array_path(path_component)? {
                let map = current_value.to_map_mut()?;
                let array_value = map.get_key_mut_or_insert(string_part, Value::Array(vec![]));
                let array = array_value.to_array_mut()?;
                let Some(number_part) = number_part else {
                    return Err(Error::Unsupported(
                        "setting values of all members in an array is currently not supported"
                            .to_string(),
                    ));
                };
                // We are setting the value of just member of the array
                match number_part.cmp(&array.len()) {
                    std::cmp::Ordering::Less => {
                        //this already exists
                        current_value = array.get_mut(number_part).unwrap();
                    }
                    std::cmp::Ordering::Equal => {
                        //we should create a new map
                        array.push(Value::Map(ValueMap::new()));
                        current_value = array.get_mut(number_part).unwrap();
                    }
                    std::cmp::Ordering::Greater => {
                        return Err(Error::StructureError(
                            "trying to insert into an array path higher than current array length"
                                .to_string(),
                        ));
                    }
                }
            } else {
                let map = current_value.to_map_mut()?;
                current_value =
                    map.get_key_mut_or_insert(path_component, Value::Map(ValueMap::new()));
            };
        }
        let Some(last_path_component) = last_path_component else {
            return Err(Error::StructureError("path was empty".to_string()));
        };
        let map = current_value.to_map_mut()?;
        Self::insert_in_map(map, last_path_component, value);
        Ok(())
    }

    pub fn set_value_at_path(&mut self, path: &str, key: &str, value: Value) -> Result<(), Error> {
        let map = self.get_mut_value_at_path(path)?.as_map_mut_ref()?;
        Self::insert_in_map(map, key, value);
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform_value;

    #[test]
    fn insert_with_parents() {
        let mut document = platform_value!({
            "root" :  {
                "from" : {
                    "id": "123",
                    "message": "text_message",
                },
            }
        });

        document
            .set_value_at_full_path("root.to.new_field", platform_value!("new_value"))
            .expect("no errors");
        document
            .set_value_at_full_path("root.array[0].new_field", platform_value!("new_value"))
            .expect("no errors");

        assert_eq!(document["root"]["from"]["id"], platform_value!("123"));
        assert_eq!(
            document["root"]["from"]["message"],
            platform_value!("text_message")
        );
        assert_eq!(
            document["root"]["to"]["new_field"],
            platform_value!("new_value")
        );
        assert_eq!(
            document["root"]["array"][0]["new_field"],
            platform_value!("new_value")
        );
    }

    mod is_array_path {
        use super::*;

        #[test]
        fn test_valid_no_index() {
            let result = is_array_path("array[]");
            assert!(result.is_ok());
            let maybe_tuple = result.unwrap();
            assert!(maybe_tuple.is_some());
            let (field_name, index) = maybe_tuple.unwrap();
            assert_eq!(field_name, "array");
            assert_eq!(index, None);
        }

        #[test]
        fn test_valid_with_index() {
            let result = is_array_path("arr[123]");
            assert!(result.is_ok());
            let maybe_tuple = result.unwrap();
            assert!(maybe_tuple.is_some());
            let (field_name, index) = maybe_tuple.unwrap();
            assert_eq!(field_name, "arr");
            assert_eq!(index, Some(123));
        }

        #[test]
        fn test_no_brackets() {
            let result = is_array_path("no_brackets");
            assert!(result.is_ok());
            assert!(result.unwrap().is_none());
        }

        #[test]
        fn test_missing_closing_bracket() {
            let result = is_array_path("array[");
            assert!(result.is_ok());
            assert!(result.unwrap().is_none());
        }

        #[test]
        fn test_non_alphanumeric_field() {
            let result = is_array_path("arr-test[123]");
            assert!(result.is_ok());
            assert!(result.unwrap().is_none());
        }

        #[test]
        fn test_empty_field_name() {
            let result = is_array_path("[123]");
            assert!(result.is_ok());
            assert!(result.unwrap().is_none());
        }

        #[test]
        fn test_non_numeric_index() {
            let result = is_array_path("array[abc]");
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), Error::IntegerSizeError);
        }

        #[test]
        fn test_empty_index() {
            let result = is_array_path("array[]");
            assert!(result.is_ok());
            let maybe_tuple = result.unwrap();
            assert!(maybe_tuple.is_some());
            let (field_name, index) = maybe_tuple.unwrap();
            assert_eq!(field_name, "array");
            assert_eq!(index, None);
        }
    }
}
