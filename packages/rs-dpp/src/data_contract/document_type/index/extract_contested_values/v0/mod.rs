use super::contested_index_identifier;
use crate::data_contract::document_type::property::{DocumentProperty, DocumentPropertyType};
use crate::data_contract::document_type::Index;
use indexmap::IndexMap;
use platform_value::Value;
use std::collections::BTreeMap;

impl Index {
    /// The index values of `data` with every identifier property written as
    /// `Value::Identifier`.
    #[inline(always)]
    pub(super) fn extract_contested_values_v0(
        &self,
        data: &BTreeMap<String, Value>,
        document_properties: &IndexMap<String, DocumentProperty>,
    ) -> Vec<Value> {
        self.properties
            .iter()
            .zip(self.extract_values(data))
            .map(|(index_property, value)| {
                let is_identifier = matches!(
                    document_properties
                        .get(&index_property.name)
                        .map(|property| &property.property_type),
                    Some(
                        DocumentPropertyType::Identifier
                            | DocumentPropertyType::IdentifierWithReference(_)
                    )
                );
                if is_identifier {
                    canonical_identifier_value(value)
                } else {
                    value
                }
            })
            .collect()
    }
}

/// An identifier value in any accepted form as `Value::Identifier`; any other value as it is.
fn canonical_identifier_value(value: Value) -> Value {
    contested_index_identifier(&value).map_or(value, Value::Identifier)
}
