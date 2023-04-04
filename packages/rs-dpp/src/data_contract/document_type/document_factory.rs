use crate::data_contract::document_type::property_names::{CREATED_AT, UPDATED_AT};
use crate::data_contract::document_type::DocumentType;
use crate::document::document_transition::INITIAL_REVISION;
use crate::document::Document;
use crate::prelude::TimestampMillis;
use crate::ProtocolError;
use chrono::Utc;
use platform_value::Value;

use platform_value::Identifier;
use std::collections::BTreeMap;

impl DocumentType {
    /// Creates a document at the current time based on document type information
    /// Properties set here must be pre validated
    pub fn create_document_with_valid_properties(
        &self,
        id: Identifier,
        owner_id: Identifier,
        properties: BTreeMap<String, Value>,
    ) -> Result<Document, ProtocolError> {
        let created_at = if self.properties.contains_key(CREATED_AT) {
            Some(Utc::now().timestamp_millis() as TimestampMillis)
        } else {
            None
        };

        let updated_at = if self.properties.contains_key(UPDATED_AT) {
            Some(Utc::now().timestamp_millis() as TimestampMillis)
        } else {
            None
        };

        let revision = if self.documents_mutable {
            Some(INITIAL_REVISION)
        } else {
            None
        };

        Ok(Document {
            id,
            owner_id,
            properties,
            revision,
            created_at,
            updated_at,
        })
    }
}
