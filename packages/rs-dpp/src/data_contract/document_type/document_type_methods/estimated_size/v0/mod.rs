use crate::data_contract::document_type::v0::DocumentTypeV0;

// If another document type (like V1) ever were to exist we would need to implement estimated_size_v0 again

impl DocumentTypeV0 {
    /// The estimated size uses the middle ceil size of all attributes
    pub(in crate::data_contract::document_type) fn estimated_size_v0(&self) -> u16 {
        let mut iter = self
            .flattened_properties
            .iter()
            .filter_map(|(_, document_field_type)| {
                document_field_type.r#type.middle_byte_size_ceil()
            });
        let first = Some(iter.next().unwrap_or_default());

        iter.fold(first, |acc, item| acc.and_then(|acc| acc.checked_add(item)))
            .unwrap_or(u16::MAX)
    }
}
