use crate::data_contract::document_type::property_names::{CREATED_AT, UPDATED_AT};
use crate::document::{Document, DocumentV0};
use crate::prelude::TimestampMillis;
use crate::ProtocolError;
use chrono::Utc;
use platform_value::Value;

use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::document::INITIAL_REVISION;
use crate::version::{FeatureVersion, PlatformVersion};
use platform_value::Identifier;
use std::collections::BTreeMap;

impl DocumentTypeV0 {
    /// Creates a document at the current time based on document type information
    /// Properties set here must be pre validated
    pub(in crate::data_contract::document_type) fn create_document_with_prevalidated_properties_v0(
        &self,
        id: Identifier,
        owner_id: Identifier,
        properties: BTreeMap<String, Value>,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        let created_at = if self.required_fields.contains(CREATED_AT) {
            Some(Utc::now().timestamp_millis() as TimestampMillis)
        } else {
            None
        };

        let updated_at = if self.required_fields.contains(UPDATED_AT) {
            Some(Utc::now().timestamp_millis() as TimestampMillis)
        } else {
            None
        };

        let revision = if self.documents_mutable {
            Some(INITIAL_REVISION)
        } else {
            None
        };

        match platform_version
            .dpp
            .document_versions
            .document_structure_version
        {
            0 => Ok(DocumentV0 {
                id,
                owner_id,
                properties,
                revision,
                created_at,
                updated_at,
            }
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "create_document_with_prevalidated_properties_v0 (for document version)"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
