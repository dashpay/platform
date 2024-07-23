#[cfg(feature = "index-serde-conversion")]
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, PartialEq, PartialOrd, Clone, Eq)]
#[cfg_attr(feature = "index-serde-conversion", derive(Serialize, Deserialize))]
pub enum OrderBy {
    #[cfg_attr(feature = "index-serde-conversion", serde(rename = "asc"))]
    Asc,
    #[cfg_attr(feature = "index-serde-conversion", serde(rename = "desc"))]
    Desc,
}

use crate::data_contract::errors::DataContractError;

use crate::ProtocolError;
use anyhow::anyhow;

use crate::data_contract::document_type::ContestedIndexResolution::MasternodeVote;
use crate::data_contract::errors::DataContractError::RegexError;
use platform_value::{Value, ValueMap};
use rand::distributions::{Alphanumeric, DistString};
use regex::Regex;
#[cfg(feature = "index-serde-conversion")]
use serde::de::{VariantAccess, Visitor};
use std::cmp::Ordering;
#[cfg(feature = "index-serde-conversion")]
use std::fmt;
use std::{collections::BTreeMap, convert::TryFrom};

pub mod random_index;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd)]
#[cfg_attr(feature = "index-serde-conversion", derive(Serialize, Deserialize))]
pub enum ContestedIndexResolution {
    MasternodeVote = 0,
}

impl TryFrom<u8> for ContestedIndexResolution {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(MasternodeVote),
            value => Err(ProtocolError::UnknownStorageKeyRequirements(format!(
                "contested index resolution unknown: {}",
                value
            ))),
        }
    }
}

#[repr(u8)]
#[derive(Debug)]
pub enum ContestedIndexFieldMatch {
    Regex(regex::Regex),
    PositiveIntegerMatch(u128),
}

#[cfg(feature = "index-serde-conversion")]
impl Serialize for ContestedIndexFieldMatch {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            ContestedIndexFieldMatch::Regex(ref regex) => serializer.serialize_newtype_variant(
                "ContestedIndexFieldMatch",
                0,
                "Regex",
                regex.as_str(),
            ),
            ContestedIndexFieldMatch::PositiveIntegerMatch(ref num) => serializer
                .serialize_newtype_variant(
                    "ContestedIndexFieldMatch",
                    1,
                    "PositiveIntegerMatch",
                    num,
                ),
        }
    }
}

#[cfg(feature = "index-serde-conversion")]
impl<'de> Deserialize<'de> for ContestedIndexFieldMatch {
    fn deserialize<D>(deserializer: D) -> Result<ContestedIndexFieldMatch, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Regex,
            PositiveIntegerMatch,
        }

        struct FieldVisitor;

        impl<'de> Visitor<'de> for FieldVisitor {
            type Value = Field;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("`regex` or `positive_integer_match`")
            }

            fn visit_str<E>(self, value: &str) -> Result<Field, E>
            where
                E: de::Error,
            {
                match value {
                    "regex" => Ok(Field::Regex),
                    "positive_integer_match" => Ok(Field::PositiveIntegerMatch),
                    _ => Err(de::Error::unknown_variant(
                        value,
                        &["regex", "positive_integer_match"],
                    )),
                }
            }
        }

        struct ContestedIndexFieldMatchVisitor;

        impl<'de> Visitor<'de> for ContestedIndexFieldMatchVisitor {
            type Value = ContestedIndexFieldMatch;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("enum ContestedIndexFieldMatch")
            }

            fn visit_enum<V>(self, visitor: V) -> Result<ContestedIndexFieldMatch, V::Error>
            where
                V: de::EnumAccess<'de>,
            {
                match visitor.variant()? {
                    (Field::Regex, v) => {
                        let regex_str: &str = v.newtype_variant()?;
                        Regex::new(regex_str)
                            .map(ContestedIndexFieldMatch::Regex)
                            .map_err(de::Error::custom)
                    }
                    (Field::PositiveIntegerMatch, v) => {
                        let num: u128 = v.newtype_variant()?;
                        Ok(ContestedIndexFieldMatch::PositiveIntegerMatch(num))
                    }
                }
            }
        }

        deserializer.deserialize_enum(
            "ContestedIndexFieldMatch",
            &["regex", "positive_integer_match"],
            ContestedIndexFieldMatchVisitor,
        )
    }
}

impl PartialOrd for ContestedIndexFieldMatch {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        use ContestedIndexFieldMatch::*;
        match (self, other) {
            // Comparing two integers
            (PositiveIntegerMatch(a), PositiveIntegerMatch(b)) => a.partial_cmp(b),

            // Arbitrarily decide that any Regex is less than any PositiveIntegerMatch
            (Regex(_), PositiveIntegerMatch(_)) => Some(Ordering::Less),
            (PositiveIntegerMatch(_), Regex(_)) => Some(Ordering::Greater),

            // Comparing Regex with Regex, perhaps based on pattern length
            (Regex(a), Regex(b)) => a.as_str().len().partial_cmp(&b.as_str().len()),
        }
    }
}

impl Ord for ContestedIndexFieldMatch {
    fn cmp(&self, other: &Self) -> Ordering {
        use ContestedIndexFieldMatch::*;
        match (self, other) {
            // Directly compare integers
            (PositiveIntegerMatch(a), PositiveIntegerMatch(b)) => a.cmp(b),

            // Compare Regex based on pattern string length
            (Regex(a), Regex(b)) => a.as_str().len().cmp(&b.as_str().len()),

            // Regex is considered less than a positive integer
            (Regex(_), PositiveIntegerMatch(_)) => Ordering::Less,
            (PositiveIntegerMatch(_), Regex(_)) => Ordering::Greater,
        }
    }
}

impl Clone for ContestedIndexFieldMatch {
    fn clone(&self) -> Self {
        match self {
            ContestedIndexFieldMatch::Regex(regex) => {
                ContestedIndexFieldMatch::Regex(regex::Regex::new(regex.as_str()).unwrap())
            }
            ContestedIndexFieldMatch::PositiveIntegerMatch(int) => {
                ContestedIndexFieldMatch::PositiveIntegerMatch(*int)
            }
        }
    }
}

impl PartialEq for ContestedIndexFieldMatch {
    fn eq(&self, other: &Self) -> bool {
        match self {
            ContestedIndexFieldMatch::Regex(regex) => match other {
                ContestedIndexFieldMatch::Regex(other_regex) => {
                    regex.as_str() == other_regex.as_str()
                }
                _ => false,
            },
            ContestedIndexFieldMatch::PositiveIntegerMatch(int) => match other {
                ContestedIndexFieldMatch::PositiveIntegerMatch(other_int) => int == other_int,
                _ => false,
            },
        }
    }
}

impl Eq for ContestedIndexFieldMatch {}

impl ContestedIndexFieldMatch {
    pub fn matches(&self, value: &Value) -> bool {
        match self {
            ContestedIndexFieldMatch::Regex(regex) => {
                if let Some(string) = value.as_str() {
                    regex.is_match(string)
                } else {
                    false
                }
            }
            ContestedIndexFieldMatch::PositiveIntegerMatch(int) => value
                .as_integer::<u128>()
                .map(|i| i == *int)
                .unwrap_or(false),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
#[cfg_attr(feature = "index-serde-conversion", derive(Serialize, Deserialize))]
pub struct ContestedIndexInformation {
    pub field_matches: BTreeMap<String, ContestedIndexFieldMatch>,
    pub resolution: ContestedIndexResolution,
}

impl Default for ContestedIndexInformation {
    fn default() -> Self {
        ContestedIndexInformation {
            field_matches: BTreeMap::new(),
            resolution: ContestedIndexResolution::MasternodeVote,
        }
    }
}

// Indices documentation:  https://dashplatform.readme.io/docs/reference-data-contracts#document-indices
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "index-serde-conversion", derive(Serialize, Deserialize))]
pub struct Index {
    pub name: String,
    pub properties: Vec<IndexProperty>,
    pub unique: bool,
    /// Null searchable indicates what to do if all members of the index are null
    /// If this is set to false then we do not insert references which makes such items non-searchable
    pub null_searchable: bool,
    /// Contested indexes are useful when a resource is considered valuable
    pub contested_index: Option<ContestedIndexInformation>,
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

    /// Get values
    pub fn extract_values(&self, data: &BTreeMap<String, Value>) -> Vec<Value> {
        self.properties
            .iter()
            .map(|property| data.get(&property.name).cloned().unwrap_or(Value::Null))
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[cfg_attr(feature = "index-serde-conversion", derive(Serialize, Deserialize))]
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
        // The default for null searchable should be true. Do not change this without very
        // careful thought and consideration.
        let mut null_searchable = true;
        let mut name = None;
        let mut contested_index = None;
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
                "nullSearchable" => {
                    if value_value.is_bool() {
                        null_searchable = value_value.as_bool().expect("confirmed as bool");
                    }
                }
                "contested" => {
                    let contested_properties_value_map = value_value.to_map()?;

                    let mut contested_index_information = ContestedIndexInformation::default();

                    for (contested_key_value, contested_value) in contested_properties_value_map {
                        let contested_key = contested_key_value
                            .to_str()
                            .map_err(|e| DataContractError::ValueDecodingError(e.to_string()))?;
                        match contested_key {
                            "fieldMatches" => {
                                let field_matches_array = contested_value.to_array_ref()?;
                                for field_match in field_matches_array {
                                    let field_match_map = field_match.to_map()?;
                                    let mut name = None;
                                    let mut field_matches = None;
                                    for (field_match_key_as_value, field_match_value) in
                                        field_match_map
                                    {
                                        let field_match_key =
                                            field_match_key_as_value.to_str().map_err(|e| {
                                                DataContractError::ValueDecodingError(e.to_string())
                                            })?;
                                        match field_match_key {
                                            "field" => {
                                                let field = field_match_value.to_str()?.to_owned();
                                                name = Some(field);
                                            }
                                            "regexPattern" => {
                                                let regex = field_match_value.to_str()?.to_owned();
                                                field_matches =
                                                    Some(ContestedIndexFieldMatch::Regex(
                                                        Regex::new(&regex).map_err(|e| {
                                                            RegexError(format!(
                                                                "invalid field match regex: {}",
                                                                e.to_string()
                                                            ))
                                                        })?,
                                                    ));
                                            }
                                            key => {
                                                return Err(DataContractError::ValueWrongType(
                                                    format!("unexpected field match key {}", key),
                                                ));
                                            }
                                        }
                                    }
                                    if name.is_none() {
                                        return Err(DataContractError::FieldRequirementUnmet(
                                            format!(
                                                "field not present in contested fieldMatches {}",
                                                key
                                            ),
                                        ));
                                    }
                                    if field_matches.is_none() {
                                        return Err(DataContractError::FieldRequirementUnmet(
                                            format!(
                                                "field not present in contested fieldMatches {}",
                                                key
                                            ),
                                        ));
                                    }
                                    contested_index_information
                                        .field_matches
                                        .insert(name.unwrap(), field_matches.unwrap());
                                }
                            }
                            "resolution" => {
                                let resolution_int = contested_value.to_integer::<u8>()?;
                                contested_index_information.resolution =
                                    resolution_int.try_into().map_err(|e: ProtocolError| {
                                        DataContractError::ValueWrongType(e.to_string())
                                    })?;
                            }
                            "description" => {}
                            key => {
                                return Err(DataContractError::ValueWrongType(format!(
                                    "unexpected contested key {}",
                                    key
                                )));
                            }
                        }
                    }
                    contested_index = Some(contested_index_information);
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

        if contested_index.is_some() && !unique {
            return Err(DataContractError::InvalidContractStructure(
                "contest supported only for unique indexes".to_string(),
            ));
        }

        // if the index didn't have a name let's make one
        let name = name.unwrap_or_else(|| Alphanumeric.sample_string(&mut rand::thread_rng(), 24));

        Ok(Index {
            name,
            properties: index_properties,
            unique,
            null_searchable,
            contested_index,
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
