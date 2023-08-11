use crate::data_contract::document_type::v0::DocumentTypeV0;

// If another document type (like V1) ever were to exist we would need to implement max_size_v0 again

impl DocumentTypeV0 {
    pub(in crate::data_contract::document_type) fn max_size_v0(&self) -> u16 {
        let mut iter = self
            .flattened_properties
            .iter()
            .filter_map(|(_, document_property)| document_property.property_type.max_byte_size());
        let first = Some(iter.next().unwrap_or_default());

        iter.fold(first, |acc, item| acc.and_then(|acc| acc.checked_add(item)))
            .unwrap_or(u16::MAX)
    }
}
