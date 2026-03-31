#[cfg(feature = "serde-conversion")]
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, PartialEq, PartialOrd, Clone, Eq)]
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
pub enum OrderBy {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "asc"))]
    Asc,
    #[cfg_attr(feature = "serde-conversion", serde(rename = "desc"))]
    Desc,
}

use crate::data_contract::errors::DataContractError;

use crate::ProtocolError;
use anyhow::anyhow;

use crate::data_contract::document_type::ContestedIndexResolution::MasternodeVote;
#[cfg(feature = "validation")]
use crate::data_contract::errors::DataContractError::RegexError;
use platform_value::{Value, ValueMap};
use rand::distributions::{Alphanumeric, DistString};
use regex::Regex;
#[cfg(feature = "serde-conversion")]
use serde::de::{VariantAccess, Visitor};
use std::cmp::Ordering;
#[cfg(feature = "serde-conversion")]
use std::fmt;
use std::sync::OnceLock;
use std::{collections::BTreeMap, convert::TryFrom};

pub mod random_index;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd)]
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
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
    Regex(LazyRegex),
    PositiveIntegerMatch(u128),
}

#[derive(Debug, Clone)]
pub struct LazyRegex {
    regex: OnceLock<Regex>,
    regex_str: String,
}

impl LazyRegex {
    pub fn new(regex_str: String) -> Self {
        LazyRegex {
            regex: OnceLock::new(),
            regex_str,
        }
    }

    pub fn is_match(&self, string: &str) -> bool {
        let regexp = self
            .regex
            .get_or_init(|| Regex::new(&self.regex_str).expect("valid regexp"));

        regexp.is_match(string)
    }

    pub fn as_str(&self) -> &str {
        self.regex_str.as_str()
    }
}

#[cfg(feature = "serde-conversion")]
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

#[cfg(feature = "serde-conversion")]
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

        impl Visitor<'_> for FieldVisitor {
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
                        let regex_str: String = v.newtype_variant()?;

                        Ok(ContestedIndexFieldMatch::Regex(LazyRegex::new(regex_str)))
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

#[allow(clippy::non_canonical_partial_ord_impl)]
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
                ContestedIndexFieldMatch::Regex(regex.clone())
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
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
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
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
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
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
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
                                                let regex_str =
                                                    field_match_value.to_str()?.to_owned();

                                                #[cfg(feature = "validation")]
                                                Regex::new(&regex_str).map_err(|e| {
                                                    RegexError(format!(
                                                        "invalid field match regex: {}",
                                                        e
                                                    ))
                                                })?;

                                                field_matches =
                                                    Some(ContestedIndexFieldMatch::Regex(
                                                        LazyRegex::new(regex_str),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_index_property(name: &str, ascending: bool) -> IndexProperty {
        IndexProperty {
            name: name.to_string(),
            ascending,
        }
    }

    fn make_index(name: &str, properties: Vec<(&str, bool)>, unique: bool) -> Index {
        Index {
            name: name.to_string(),
            properties: properties
                .into_iter()
                .map(|(n, asc)| make_index_property(n, asc))
                .collect(),
            unique,
            null_searchable: true,
            contested_index: None,
        }
    }

    // -----------------------------------------------------------------------
    // ContestedIndexResolution tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_contested_index_resolution_try_from_valid() {
        let res = ContestedIndexResolution::try_from(0u8).unwrap();
        assert_eq!(res, ContestedIndexResolution::MasternodeVote);
    }

    #[test]
    fn test_contested_index_resolution_try_from_invalid() {
        let res = ContestedIndexResolution::try_from(1u8);
        assert!(res.is_err());
    }

    #[test]
    fn test_contested_index_resolution_try_from_255() {
        let res = ContestedIndexResolution::try_from(255u8);
        assert!(res.is_err());
    }

    // -----------------------------------------------------------------------
    // LazyRegex tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_lazy_regex_match() {
        let lr = LazyRegex::new("^[a-z]+$".to_string());
        assert!(lr.is_match("hello"));
        assert!(!lr.is_match("Hello"));
        assert!(!lr.is_match("123"));
    }

    #[test]
    fn test_lazy_regex_as_str() {
        let lr = LazyRegex::new("test_pattern".to_string());
        assert_eq!(lr.as_str(), "test_pattern");
    }

    // -----------------------------------------------------------------------
    // ContestedIndexFieldMatch tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_contested_index_field_match_regex_matches() {
        let m = ContestedIndexFieldMatch::Regex(LazyRegex::new("^dash".to_string()));
        assert!(m.matches(&Value::Text("dashname".to_string())));
        assert!(!m.matches(&Value::Text("notdash".to_string())));
    }

    #[test]
    fn test_contested_index_field_match_regex_non_string() {
        let m = ContestedIndexFieldMatch::Regex(LazyRegex::new(".*".to_string()));
        assert!(!m.matches(&Value::U64(42)));
    }

    #[test]
    fn test_contested_index_field_match_positive_integer_matches() {
        let m = ContestedIndexFieldMatch::PositiveIntegerMatch(42);
        assert!(m.matches(&Value::U64(42)));
        assert!(!m.matches(&Value::U64(43)));
    }

    #[test]
    fn test_contested_index_field_match_positive_integer_non_integer() {
        let m = ContestedIndexFieldMatch::PositiveIntegerMatch(42);
        assert!(!m.matches(&Value::Text("42".to_string())));
    }

    // -----------------------------------------------------------------------
    // ContestedIndexFieldMatch ordering tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_contested_index_field_match_ord_integers() {
        let a = ContestedIndexFieldMatch::PositiveIntegerMatch(10);
        let b = ContestedIndexFieldMatch::PositiveIntegerMatch(20);
        assert!(a < b);
    }

    #[test]
    fn test_contested_index_field_match_ord_regex_vs_integer() {
        let regex = ContestedIndexFieldMatch::Regex(LazyRegex::new("abc".to_string()));
        let integer = ContestedIndexFieldMatch::PositiveIntegerMatch(10);
        assert!(regex < integer);
        assert!(integer > regex);
    }

    #[test]
    fn test_contested_index_field_match_ord_regex_vs_regex() {
        let short = ContestedIndexFieldMatch::Regex(LazyRegex::new("a".to_string()));
        let long = ContestedIndexFieldMatch::Regex(LazyRegex::new("abc".to_string()));
        assert!(short < long);
    }

    // -----------------------------------------------------------------------
    // ContestedIndexFieldMatch equality tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_contested_index_field_match_eq_regex() {
        let a = ContestedIndexFieldMatch::Regex(LazyRegex::new("^test$".to_string()));
        let b = ContestedIndexFieldMatch::Regex(LazyRegex::new("^test$".to_string()));
        assert_eq!(a, b);
    }

    #[test]
    fn test_contested_index_field_match_eq_different_regex() {
        let a = ContestedIndexFieldMatch::Regex(LazyRegex::new("^a$".to_string()));
        let b = ContestedIndexFieldMatch::Regex(LazyRegex::new("^b$".to_string()));
        assert_ne!(a, b);
    }

    #[test]
    fn test_contested_index_field_match_eq_integer() {
        let a = ContestedIndexFieldMatch::PositiveIntegerMatch(42);
        let b = ContestedIndexFieldMatch::PositiveIntegerMatch(42);
        assert_eq!(a, b);
    }

    #[test]
    fn test_contested_index_field_match_eq_different_types() {
        let regex = ContestedIndexFieldMatch::Regex(LazyRegex::new("42".to_string()));
        let integer = ContestedIndexFieldMatch::PositiveIntegerMatch(42);
        assert_ne!(regex, integer);
    }

    // -----------------------------------------------------------------------
    // ContestedIndexFieldMatch clone tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_contested_index_field_match_clone_regex() {
        let original = ContestedIndexFieldMatch::Regex(LazyRegex::new("^test$".to_string()));
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_contested_index_field_match_clone_integer() {
        let original = ContestedIndexFieldMatch::PositiveIntegerMatch(100);
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    // -----------------------------------------------------------------------
    // ContestedIndexInformation default tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_contested_index_information_default() {
        let info = ContestedIndexInformation::default();
        assert!(info.field_matches.is_empty());
        assert_eq!(info.resolution, ContestedIndexResolution::MasternodeVote);
    }

    // -----------------------------------------------------------------------
    // Index::objects_are_conflicting tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_objects_are_conflicting_non_unique_always_false() {
        let index = make_index("idx", vec![("name", true)], false);
        let obj1: ValueMap = vec![(
            Value::Text("name".to_string()),
            Value::Text("Sam".to_string()),
        )];
        let obj2: ValueMap = vec![(
            Value::Text("name".to_string()),
            Value::Text("Sam".to_string()),
        )];
        assert!(!index.objects_are_conflicting(&obj1, &obj2));
    }

    #[test]
    fn test_objects_are_conflicting_unique_same_values() {
        let index = make_index("idx", vec![("name", true)], true);
        let obj1: ValueMap = vec![(
            Value::Text("name".to_string()),
            Value::Text("Sam".to_string()),
        )];
        let obj2: ValueMap = vec![(
            Value::Text("name".to_string()),
            Value::Text("Sam".to_string()),
        )];
        assert!(index.objects_are_conflicting(&obj1, &obj2));
    }

    #[test]
    fn test_objects_are_conflicting_unique_different_values() {
        let index = make_index("idx", vec![("name", true)], true);
        let obj1: ValueMap = vec![(
            Value::Text("name".to_string()),
            Value::Text("Sam".to_string()),
        )];
        let obj2: ValueMap = vec![(
            Value::Text("name".to_string()),
            Value::Text("Alice".to_string()),
        )];
        assert!(!index.objects_are_conflicting(&obj1, &obj2));
    }

    #[test]
    fn test_objects_are_conflicting_one_missing_property() {
        let index = make_index("idx", vec![("name", true)], true);
        let obj1: ValueMap = vec![(
            Value::Text("name".to_string()),
            Value::Text("Sam".to_string()),
        )];
        let obj2: ValueMap = vec![];
        assert!(!index.objects_are_conflicting(&obj1, &obj2));
    }

    #[test]
    fn test_objects_are_conflicting_multi_property() {
        let index = make_index("idx", vec![("name", true), ("age", true)], true);
        let obj1: ValueMap = vec![
            (
                Value::Text("name".to_string()),
                Value::Text("Sam".to_string()),
            ),
            (Value::Text("age".to_string()), Value::U64(30)),
        ];
        let obj2: ValueMap = vec![
            (
                Value::Text("name".to_string()),
                Value::Text("Sam".to_string()),
            ),
            (Value::Text("age".to_string()), Value::U64(30)),
        ];
        assert!(index.objects_are_conflicting(&obj1, &obj2));

        let obj3: ValueMap = vec![
            (
                Value::Text("name".to_string()),
                Value::Text("Sam".to_string()),
            ),
            (Value::Text("age".to_string()), Value::U64(25)),
        ];
        assert!(!index.objects_are_conflicting(&obj1, &obj3));
    }

    // -----------------------------------------------------------------------
    // Index::property_names() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_property_names() {
        let index = make_index("idx", vec![("name", true), ("age", false)], false);
        let names = index.property_names();
        assert_eq!(names, vec!["name".to_string(), "age".to_string()]);
    }

    // -----------------------------------------------------------------------
    // Index::extract_values() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_extract_values_with_matching_data() {
        let index = make_index("idx", vec![("name", true), ("age", true)], false);
        let mut data = BTreeMap::new();
        data.insert("name".to_string(), Value::Text("Sam".to_string()));
        data.insert("age".to_string(), Value::U64(30));
        let values = index.extract_values(&data);
        assert_eq!(values.len(), 2);
        assert_eq!(values[0], Value::Text("Sam".to_string()));
        assert_eq!(values[1], Value::U64(30));
    }

    #[test]
    fn test_extract_values_with_missing_data() {
        let index = make_index("idx", vec![("name", true), ("age", true)], false);
        let mut data = BTreeMap::new();
        data.insert("name".to_string(), Value::Text("Sam".to_string()));
        let values = index.extract_values(&data);
        assert_eq!(values.len(), 2);
        assert_eq!(values[0], Value::Text("Sam".to_string()));
        assert_eq!(values[1], Value::Null); // missing key returns Null
    }

    // -----------------------------------------------------------------------
    // Index::matches() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_matches_exact_match() {
        let index = make_index("idx", vec![("name", true), ("age", true)], false);
        let result = index.matches(&["name", "age"], None, &[]);
        assert_eq!(result, Some(0));
    }

    #[test]
    fn test_matches_partial_match() {
        let index = make_index("idx", vec![("name", true), ("age", true)], false);
        let result = index.matches(&["name"], None, &[]);
        assert_eq!(result, Some(1));
    }

    #[test]
    fn test_matches_no_match() {
        let index = make_index("idx", vec![("name", true), ("age", true)], false);
        let result = index.matches(&["email"], None, &[]);
        assert_eq!(result, None);
    }

    #[test]
    fn test_matches_with_order_by() {
        let index = make_index("idx", vec![("name", true), ("age", true)], false);
        // Matching on "name" with order_by "age": d starts at 2, one match decrements to 1
        let result = index.matches(&["name"], None, &["age"]);
        assert_eq!(result, Some(1));
    }

    #[test]
    fn test_matches_in_field_last_property() {
        let index = make_index("idx", vec![("name", true), ("age", true)], false);
        let result = index.matches(&["name"], Some("age"), &[]);
        assert_eq!(result, Some(1));
    }

    #[test]
    fn test_matches_in_field_before_last() {
        let index = make_index("idx", vec![("name", true), ("age", true)], false);
        let result = index.matches(&["age"], Some("name"), &[]);
        assert_eq!(result, Some(1));
    }

    #[test]
    fn test_matches_in_field_not_matching() {
        let index = make_index("idx", vec![("name", true), ("age", true)], false);
        let result = index.matches(&["name"], Some("email"), &[]);
        assert_eq!(result, None);
    }

    // -----------------------------------------------------------------------
    // IndexProperty::try_from tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_index_property_try_from_asc() {
        let mut map = BTreeMap::new();
        map.insert("name".to_string(), "asc".to_string());
        let prop = IndexProperty::try_from(map).unwrap();
        assert_eq!(prop.name, "name");
        assert!(prop.ascending);
    }

    #[test]
    fn test_index_property_try_from_desc() {
        let mut map = BTreeMap::new();
        map.insert("age".to_string(), "desc".to_string());
        let prop = IndexProperty::try_from(map).unwrap();
        assert_eq!(prop.name, "age");
        assert!(!prop.ascending);
    }

    #[test]
    fn test_index_property_try_from_empty_map_error() {
        let map: BTreeMap<String, String> = BTreeMap::new();
        let result = IndexProperty::try_from(map);
        assert!(result.is_err());
    }

    #[test]
    fn test_index_property_try_from_multiple_entries_error() {
        let mut map = BTreeMap::new();
        map.insert("name".to_string(), "asc".to_string());
        map.insert("age".to_string(), "desc".to_string());
        let result = IndexProperty::try_from(map);
        assert!(result.is_err());
    }

    #[test]
    fn test_index_property_try_from_invalid_sort_order_error() {
        let mut map = BTreeMap::new();
        map.insert("name".to_string(), "random".to_string());
        let result = IndexProperty::try_from(map);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // IndexProperty::from_platform_value() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_index_property_from_platform_value_asc() {
        let map = vec![(
            Value::Text("fieldName".to_string()),
            Value::Text("asc".to_string()),
        )];
        let prop = IndexProperty::from_platform_value(&map).unwrap();
        assert_eq!(prop.name, "fieldName");
        assert!(prop.ascending);
    }

    #[test]
    fn test_index_property_from_platform_value_desc() {
        let map = vec![(
            Value::Text("fieldName".to_string()),
            Value::Text("desc".to_string()),
        )];
        let prop = IndexProperty::from_platform_value(&map).unwrap();
        assert_eq!(prop.name, "fieldName");
        assert!(!prop.ascending);
    }

    #[test]
    fn test_index_property_from_platform_value_bad_key_type() {
        let map = vec![(Value::U64(42), Value::Text("asc".to_string()))];
        let result = IndexProperty::from_platform_value(&map);
        assert!(result.is_err());
    }

    #[test]
    fn test_index_property_from_platform_value_bad_value_type() {
        let map = vec![(Value::Text("field".to_string()), Value::U64(1))];
        let result = IndexProperty::from_platform_value(&map);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Index TryFrom<&[(Value, Value)]> tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_index_try_from_basic() {
        let index_map: Vec<(Value, Value)> = vec![
            (
                Value::Text("name".to_string()),
                Value::Text("test_index".to_string()),
            ),
            (Value::Text("unique".to_string()), Value::Bool(true)),
            (
                Value::Text("nullSearchable".to_string()),
                Value::Bool(false),
            ),
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![Value::Map(vec![(
                    Value::Text("fieldA".to_string()),
                    Value::Text("asc".to_string()),
                )])]),
            ),
        ];
        let index = Index::try_from(index_map.as_slice()).unwrap();
        assert_eq!(index.name, "test_index");
        assert!(index.unique);
        assert!(!index.null_searchable);
        assert_eq!(index.properties.len(), 1);
        assert_eq!(index.properties[0].name, "fieldA");
        assert!(index.properties[0].ascending);
        assert!(index.contested_index.is_none());
    }

    #[test]
    fn test_index_try_from_without_name_generates_random() {
        let index_map: Vec<(Value, Value)> = vec![(
            Value::Text("properties".to_string()),
            Value::Array(vec![Value::Map(vec![(
                Value::Text("fieldA".to_string()),
                Value::Text("asc".to_string()),
            )])]),
        )];
        let index = Index::try_from(index_map.as_slice()).unwrap();
        assert!(!index.name.is_empty());
        assert_eq!(index.name.len(), 24); // Alphanumeric.sample_string with len 24
    }

    #[test]
    fn test_index_try_from_default_null_searchable_true() {
        let index_map: Vec<(Value, Value)> = vec![(
            Value::Text("properties".to_string()),
            Value::Array(vec![Value::Map(vec![(
                Value::Text("fieldA".to_string()),
                Value::Text("asc".to_string()),
            )])]),
        )];
        let index = Index::try_from(index_map.as_slice()).unwrap();
        assert!(index.null_searchable); // default is true
    }

    #[test]
    fn test_index_try_from_unknown_key_error() {
        let index_map: Vec<(Value, Value)> =
            vec![(Value::Text("unknownKey".to_string()), Value::Bool(true))];
        let result = Index::try_from(index_map.as_slice());
        assert!(result.is_err());
    }

    #[test]
    fn test_index_try_from_contested_without_unique_error() {
        let index_map: Vec<(Value, Value)> = vec![
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![Value::Map(vec![(
                    Value::Text("fieldA".to_string()),
                    Value::Text("asc".to_string()),
                )])]),
            ),
            (
                Value::Text("contested".to_string()),
                Value::Map(vec![(Value::Text("resolution".to_string()), Value::U64(0))]),
            ),
        ];
        let result = Index::try_from(index_map.as_slice());
        assert!(result.is_err()); // contest supported only for unique indexes
    }

    #[test]
    fn test_index_try_from_contested_with_unique() {
        let index_map: Vec<(Value, Value)> = vec![
            (Value::Text("unique".to_string()), Value::Bool(true)),
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![Value::Map(vec![(
                    Value::Text("fieldA".to_string()),
                    Value::Text("asc".to_string()),
                )])]),
            ),
            (
                Value::Text("contested".to_string()),
                Value::Map(vec![
                    (Value::Text("resolution".to_string()), Value::U64(0)),
                    (
                        Value::Text("fieldMatches".to_string()),
                        Value::Array(vec![Value::Map(vec![
                            (
                                Value::Text("field".to_string()),
                                Value::Text("normalizedLabel".to_string()),
                            ),
                            (
                                Value::Text("regexPattern".to_string()),
                                Value::Text("^[a-zA-Z]+$".to_string()),
                            ),
                        ])]),
                    ),
                ]),
            ),
        ];
        let index = Index::try_from(index_map.as_slice()).unwrap();
        assert!(index.unique);
        assert!(index.contested_index.is_some());
        let contested = index.contested_index.unwrap();
        assert_eq!(
            contested.resolution,
            ContestedIndexResolution::MasternodeVote
        );
        assert!(contested.field_matches.contains_key("normalizedLabel"));
    }

    // -----------------------------------------------------------------------
    // OrderBy tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_order_by_partial_ord() {
        assert!(OrderBy::Asc < OrderBy::Desc);
    }
}
