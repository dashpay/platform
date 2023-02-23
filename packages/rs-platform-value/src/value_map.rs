use crate::{Error, Value, ValueMap};
use std::collections::BTreeMap;

impl Value {
    /// If the `Value` is a `Map`, returns a the associated `BTreeMap<String, Value>` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use std::collections::BTreeMap;
    /// # use platform_value::{Error, Value};
    /// #
    /// let mut value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("key")), Value::Float(18.)),
    ///     ]
    /// );
    /// assert_eq!(value.into_map(), Ok(BTreeMap::from([(String::from("key"), Value::Float(18.))])));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.into_map(), Err(Error::StructureError("value is not a map".to_string())))
    /// ```
    pub fn into_btree_map(self) -> Result<BTreeMap<String, Value>, Error> {
        Self::map_into_btree_map(self.into_map()?)
    }

    /// Takes a ValueMap which is a `Vec<(Value, Value)>`
    /// Returns a BTreeMap<String, Value> as long as each Key is a String
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    pub fn map_into_btree_map(map: ValueMap) -> Result<BTreeMap<String, Value>, Error> {
        map.into_iter()
            .map(|(key, value)| {
                let key = key
                    .into_text()
                    .map_err(|_| Error::StructureError("expected key to be string".to_string()))?;
                Ok((key, value))
            })
            .collect::<Result<BTreeMap<String, Value>, Error>>()
    }

    /// Takes a ref to a ValueMap which is a `&Vec<(Value, Value)>`
    /// Returns a BTreeMap<String, &Value> as long as each Key is a String
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    pub fn map_ref_into_btree_map(map: &ValueMap) -> Result<BTreeMap<String, &Value>, Error> {
        map.iter()
            .map(|(key, value)| {
                let key = key
                    .to_text()
                    .map_err(|_| Error::StructureError("expected key to be string".to_string()))?;
                Ok((key, value))
            })
            .collect::<Result<BTreeMap<String, &Value>, Error>>()
    }
}
