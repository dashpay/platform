//! Whether the `fieldMatches` of a contested index judge alike every document with the same
//! index values. When they do not, a document outside the contest can take the index values
//! of a document in it, and the award at the end of the contest finds its unique index entry
//! taken.

use crate::data_contract::document_type::property::{DocumentProperty, DocumentPropertyType};
use crate::data_contract::document_type::Index;
use indexmap::IndexMap;
use platform_value::Value;

impl Index {
    /// Whether this index's contest, if it has one, is decided by the index values alone, so
    /// that two documents with the same index values are both in the contest or both outside
    /// it. Every field a `fieldMatches` entry names must be a property of the index, and a
    /// match on a string property that admits the empty string must judge the empty string and
    /// `"\0"` alike: the tree key encoder stores both under the one key `[0]`. A match on a
    /// field that is not a string judges every document with the same value alike. An index
    /// without a contest is decided by its values, as is a contest without `fieldMatches`,
    /// which takes every document.
    ///
    /// `document_properties` are the type's flattened properties. Shared by registration,
    /// which refuses a contest its values do not decide from protocol version 15, and the
    /// document create's state validation, which keeps a document outside such a contest,
    /// registered earlier, from taking the values of a live one.
    pub fn contest_is_decided_by_index_values(
        &self,
        document_properties: &IndexMap<String, DocumentProperty>,
    ) -> bool {
        let Some(contested_index) = &self.contested_index else {
            return true;
        };
        contested_index
            .field_matches
            .iter()
            .all(|(field, field_match)| {
                if !self
                    .properties
                    .iter()
                    .any(|property| &property.name == field)
                {
                    return false;
                }
                let admits_the_empty_string = matches!(
                    document_properties.get(field).map(|property| &property.property_type),
                    Some(DocumentPropertyType::String(sizes)) if sizes.min_length.unwrap_or(0) == 0
                );
                !admits_the_empty_string
                    || field_match.matches(&Value::Text(String::new()))
                        == field_match.matches(&Value::Text("\0".to_string()))
            })
    }
}
