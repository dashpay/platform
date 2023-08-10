use crate::data_contract::document_type::methods::DocumentTypeV0Methods;
use crate::data_contract::document_type::property::DocumentPropertyType;
use crate::data_contract::document_type::v0::{DocumentTypeV0, DEFAULT_HASH_SIZE};

impl DocumentTypeV0 {
    pub(in crate::data_contract::document_type) fn document_field_type_for_property_v0(
        &self,
        property: &str,
    ) -> Option<DocumentPropertyType> {
        match property {
            "$id" => Some(DocumentPropertyType::ByteArray(
                Some(DEFAULT_HASH_SIZE as u16),
                Some(DEFAULT_HASH_SIZE as u16),
            )),
            "$ownerId" => Some(DocumentPropertyType::ByteArray(
                Some(DEFAULT_HASH_SIZE as u16),
                Some(DEFAULT_HASH_SIZE as u16),
            )),
            "$createdAt" => Some(DocumentPropertyType::Date),
            "$updatedAt" => Some(DocumentPropertyType::Date),
            &_ => self
                .document_field_for_property(property)
                .map(|document_field| document_field.r#type),
        }
    }
}
