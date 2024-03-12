use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Clone, Eq)]
pub enum OrderBy {
    #[serde(rename = "asc")]
    Asc,
    #[serde(rename = "desc")]
    Desc,
}

use crate::data_contract::errors::DataContractError;

use crate::ProtocolError;
use anyhow::anyhow;

use platform_value::{Value, ValueMap};
use rand::distributions::{Alphanumeric, DistString};
use std::{collections::BTreeMap, convert::TryFrom};

pub mod random_index;

// Indices documentation:  https://dashplatform.readme.io/docs/reference-data-contracts#document-indices
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Index {
    pub name: String,
    pub properties: Vec<IndexProperty>,
    pub unique: bool,
}

impl Index {
    /// Check to see if two objects are conflicting
    pub fn objects_are_conflicting(&self, object1: &ValueMap, object2: &ValueMap) -> bool {
        if !self.unique {
            return false;
        }
        self.properties.iter().all(|property| {
            //if either or both are null then there can not be an overlap
            let Some(value1) = Value::get_optional_from_map(object1, property.name.as_str()) else {
                return false;
            };
            let Some(value2) = Value::get_optional_from_map(object2, property.name.as_str()) else {
                return false;
            };
            value1 == value2
        })
    }
    /// The field names of the index
    pub fn property_names(&self) -> Vec<String> {
        self.properties
            .iter()
            .map(|property| property.name.clone())
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IndexProperty {
    pub name: String,
    pub ascending: bool,
}

impl TryFrom<BTreeMap<String, String>> for IndexProperty {
    type Error = ProtocolError;

    fn try_from(value: BTreeMap<String, String>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(ProtocolError::Error(anyhow!(
                "property in the index definition cannot be empty"
            )));
        }
        if value.len() > 1 {
            return Err(ProtocolError::Error(anyhow!(
                "property in the index cannot contain more than one item: {:#?}",
                value
            )));
        }

        // the unwrap is safe because of the checks above
        let raw_property = value.into_iter().next().unwrap();
        let ascending = match raw_property.1.as_str() {
            "asc" => true,
            "desc" => false,
            sort_order => {
                return Err(ProtocolError::Error(anyhow!(
                    "invalid sorting order: '{}'",
                    sort_order
                )))
            }
        };

        Ok(Self {
            name: raw_property.0,
            ascending,
        })
    }
}

impl Index {
    // The matches function will take a slice of an array of strings and an optional sort on value.
    // An index matches if all the index_names in the slice are consecutively the index's properties
    // with leftovers permitted.
    // If a sort_on value is provided it must match the last index property.
    // The number returned is the number of unused index properties

    // A case for example if we have an index on person's name and age
    // where we say name == 'Sam' sort by age
    // there is no field operator on age
    // The return value for name == 'Sam' sort by age would be 0
    // The return value for name == 'Sam and age > 5 sort by age would be 0
    // the return value for sort by age would be 1
    pub fn matches(
        &self,
        index_names: &[&str],
        in_field_name: Option<&str>,
        order_by: &[&str],
    ) -> Option<u16> {
        // Here we are trying to figure out if the Index matches the order by
        // To do so we take the index and go backwards as we need the order by clauses to be
        // continuous, but they do not need to be at the end.
        let mut reduced_properties = self.properties.as_slice();
        // let mut should_ignore: Vec<String> = order_by.iter().map(|&str| str.to_string()).collect();
        if !order_by.is_empty() {
            for _ in 0..self.properties.len() {
                if reduced_properties.len() < order_by.len() {
                    return None;
                }
                let matched_ordering = reduced_properties
                    .iter()
                    .rev()
                    .zip(order_by.iter().rev())
                    .all(|(property, &sort)| property.name.as_str() == sort);
                if matched_ordering {
                    break;
                }
                if let Some((_last, elements)) = reduced_properties.split_last() {
                    // should_ignore.push(last.name.clone());
                    reduced_properties = elements;
                } else {
                    return None;
                }
            }
        }

        let last_property = self.properties.last()?;

        // the in field can only be on the last or before last property
        if let Some(in_field_name) = in_field_name {
            if last_property.name.as_str() != in_field_name {
                // it can also be on the before last
                if self.properties.len() == 1 {
                    return None;
                }
                let before_last_property = self.properties.get(self.properties.len() - 2)?;
                if before_last_property.name.as_str() != in_field_name {
                    return None;
                }
            }
        }

        let mut d = self.properties.len();

        for search_name in index_names.iter() {
            if !reduced_properties
                .iter()
                .any(|property| property.name.as_str() == *search_name)
            {
                return None;
            }
            d -= 1;
        }

        Some(d as u16)
    }
}

impl TryFrom<&[(Value, Value)]> for Index {
    type Error = DataContractError;

    fn try_from(index_type_value_map: &[(Value, Value)]) -> Result<Self, Self::Error> {
        // Decouple the map
        // It contains properties and a unique key
        // If the unique key is absent, then unique is false
        // If present, then use that value
        // For properties, we iterate each and move it to IndexProperty

        let mut unique = false;
        let mut name = None;
        let mut index_properties: Vec<IndexProperty> = Vec::new();

        for (key_value, value_value) in index_type_value_map {
            let key = key_value.to_str()?;

            match key {
                "name" => {
                    name = Some(
                        value_value
                            .as_text()
                            .ok_or(DataContractError::InvalidContractStructure(
                                "index name should be a string".to_string(),
                            ))?
                            .to_owned(),
                    );
                }
                "unique" => {
                    if value_value.is_bool() {
                        unique = value_value.as_bool().expect("confirmed as bool");
                    }
                }
                "properties" => {
                    let properties =
                        value_value
                            .as_array()
                            .ok_or(DataContractError::ValueWrongType(
                                "properties value should be an array".to_string(),
                            ))?;

                    // Iterate over this and get the index properties
                    for property in properties {
                        let property_map =
                            property.as_map().ok_or(DataContractError::ValueWrongType(
                                "each property of an index should be a map".to_string(),
                            ))?;

                        let index_property = IndexProperty::from_platform_value(property_map)?;
                        index_properties.push(index_property);
                    }
                }
                _ => {
                    return Err(DataContractError::ValueWrongType(
                        "unexpected property name".to_string(),
                    ))
                }
            }
        }

        // if the index didn't have a name let's make one
        //todo: we should remove the name altogether
        let name = name.unwrap_or_else(|| Alphanumeric.sample_string(&mut rand::thread_rng(), 24));

        Ok(Index {
            name,
            properties: index_properties,
            unique,
        })
    }
}

impl IndexProperty {
    pub fn from_platform_value(
        index_property_map: &[(Value, Value)],
    ) -> Result<Self, DataContractError> {
        let property = &index_property_map[0];

        let key = property
            .0 // key
            .as_text()
            .ok_or(DataContractError::KeyWrongType(
                "key should be of type string".to_string(),
            ))?;
        let value = property
            .1 // value
            .as_text()
            .ok_or(DataContractError::ValueWrongType(
                "value should be of type string".to_string(),
            ))?;

        let ascending = value == "asc";

        Ok(IndexProperty {
            name: key.to_string(),
            ascending,
        })
    }
}
