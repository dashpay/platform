use crate::document::document_methods::DocumentGetRawForDocumentTypeV0;
use crate::document::DocumentV0Getters;
use platform_value::Value;
use std::collections::BTreeMap;

pub trait DocumentIsEqualIgnoringTimestampsV0:
    DocumentV0Getters + DocumentGetRawForDocumentTypeV0
{
    /// Checks to see if a document is equal without time based fields.
    /// Since these fields are set on the network this function can be useful to make sure that
    /// fields that were supplied have not changed, while ignoring those that are set network side.
    /// Time based fields that are ignored are
    ///     created_at/updated_at
    ///     created_at_block_height/updated_at_block_height
    ///     created_at_core_block_height/updated_at_core_block_height
    fn is_equal_ignoring_time_based_fields_v0(
        &self,
        rhs: &Self,
        also_ignore_fields: Option<Vec<&str>>,
    ) -> bool {
        fn filtered_properties<'a>(
            properties: &'a BTreeMap<String, Value>,
            ignore_fields: &Vec<&str>,
        ) -> BTreeMap<&'a String, &'a Value> {
            properties
                .iter()
                .filter(|(key, _)| !ignore_fields.contains(&key.as_str()))
                .collect()
        }

        let properties_equal = if let Some(fields) = &also_ignore_fields {
            filtered_properties(self.properties(), fields)
                == filtered_properties(rhs.properties(), fields)
        } else {
            self.properties() == rhs.properties()
        };

        self.id() == rhs.id()
            && self.owner_id() == rhs.owner_id()
            && properties_equal
            && self.revision() == rhs.revision()
    }
}
