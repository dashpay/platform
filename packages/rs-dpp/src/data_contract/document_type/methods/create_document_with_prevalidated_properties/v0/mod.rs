use crate::document::{Document, DocumentV0};
use crate::prelude::{BlockHeight, CoreBlockHeight};
use crate::ProtocolError;
use chrono::Utc;
use platform_value::Value;

use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::methods::DocumentTypeV0Methods;
use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::document::property_names::{
    CREATED_AT_BLOCK_HEIGHT, CREATED_AT_CORE_BLOCK_HEIGHT, TRANSFERRED_AT,
    TRANSFERRED_AT_BLOCK_HEIGHT, TRANSFERRED_AT_CORE_BLOCK_HEIGHT, UPDATED_AT_BLOCK_HEIGHT,
    UPDATED_AT_CORE_BLOCK_HEIGHT,
};
use crate::document::INITIAL_REVISION;
use crate::version::PlatformVersion;
use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::Identifier;
use std::collections::BTreeMap;

impl DocumentTypeV0 {
    /// Creates a document at the current time based on document type information
    /// Properties set here must be pre validated
    pub(in crate::data_contract::document_type) fn create_document_with_prevalidated_properties_v0(
        &self,
        id: Identifier,
        owner_id: Identifier,
        block_height: BlockHeight,
        core_block_height: CoreBlockHeight,
        properties: BTreeMap<String, Value>,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        // Set timestamps if they are required and not exist
        let mut created_at: Option<crate::identity::TimestampMillis> = properties
            .get_optional_integer(crate::document::property_names::CREATED_AT)
            .map_err(ProtocolError::ValueError)?;

        let mut updated_at: Option<crate::identity::TimestampMillis> = properties
            .get_optional_integer(crate::document::property_names::UPDATED_AT)
            .map_err(ProtocolError::ValueError)?;

        let mut transferred_at: Option<crate::identity::TimestampMillis> = properties
            .get_optional_integer(TRANSFERRED_AT)
            .map_err(ProtocolError::ValueError)?;

        let mut created_at_block_height: Option<BlockHeight> = properties
            .get_optional_integer(CREATED_AT_BLOCK_HEIGHT)
            .map_err(ProtocolError::ValueError)?;

        let mut updated_at_block_height: Option<BlockHeight> = properties
            .get_optional_integer(UPDATED_AT_BLOCK_HEIGHT)
            .map_err(ProtocolError::ValueError)?;

        let mut transferred_at_block_height: Option<BlockHeight> = properties
            .get_optional_integer(TRANSFERRED_AT_BLOCK_HEIGHT)
            .map_err(ProtocolError::ValueError)?;

        let mut created_at_core_block_height: Option<CoreBlockHeight> = properties
            .get_optional_integer(CREATED_AT_CORE_BLOCK_HEIGHT)
            .map_err(ProtocolError::ValueError)?;

        let mut updated_at_core_block_height: Option<CoreBlockHeight> = properties
            .get_optional_integer(UPDATED_AT_CORE_BLOCK_HEIGHT)
            .map_err(ProtocolError::ValueError)?;

        let mut transferred_at_core_block_height: Option<CoreBlockHeight> = properties
            .get_optional_integer(TRANSFERRED_AT_CORE_BLOCK_HEIGHT)
            .map_err(ProtocolError::ValueError)?;

        let is_created_at_required = self
            .required_fields()
            .contains(crate::document::property_names::CREATED_AT);
        let is_updated_at_required = self
            .required_fields()
            .contains(crate::document::property_names::UPDATED_AT);
        let is_transferred_at_required = self.required_fields().contains(TRANSFERRED_AT);

        let is_created_at_block_height_required =
            self.required_fields().contains(CREATED_AT_BLOCK_HEIGHT);
        let is_updated_at_block_height_required =
            self.required_fields().contains(UPDATED_AT_BLOCK_HEIGHT);
        let is_transferred_at_block_height_required =
            self.required_fields().contains(TRANSFERRED_AT_BLOCK_HEIGHT);

        let is_created_at_core_block_height_required = self
            .required_fields()
            .contains(CREATED_AT_CORE_BLOCK_HEIGHT);
        let is_updated_at_core_block_height_required = self
            .required_fields()
            .contains(UPDATED_AT_CORE_BLOCK_HEIGHT);
        let is_transferred_at_core_block_height_required = self
            .required_fields()
            .contains(TRANSFERRED_AT_CORE_BLOCK_HEIGHT);

        if (is_created_at_required && created_at.is_none())
            || (is_updated_at_required && updated_at.is_none()
                || (is_transferred_at_required && transferred_at.is_none()))
        {
            //we want only one call to get current time
            let now = Utc::now().timestamp_millis() as crate::identity::TimestampMillis;

            if is_created_at_required {
                created_at = created_at.or(Some(now));
            };

            if is_updated_at_required {
                updated_at = updated_at.or(Some(now));
            };

            if is_transferred_at_required {
                transferred_at = transferred_at.or(Some(now));
            };
        };

        if is_created_at_block_height_required {
            created_at_block_height = created_at_block_height.or(Some(block_height));
        };

        if is_updated_at_block_height_required {
            updated_at_block_height = updated_at_block_height.or(Some(block_height));
        };

        if is_transferred_at_block_height_required {
            transferred_at_block_height = transferred_at_block_height.or(Some(block_height));
        };

        if is_created_at_core_block_height_required {
            created_at_core_block_height = created_at_core_block_height.or(Some(core_block_height));
        };

        if is_updated_at_core_block_height_required {
            updated_at_core_block_height = updated_at_core_block_height.or(Some(core_block_height));
        };

        if is_transferred_at_core_block_height_required {
            transferred_at_core_block_height =
                transferred_at_core_block_height.or(Some(core_block_height));
        };

        let revision = if self.requires_revision() {
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
                transferred_at,
                created_at_block_height,
                updated_at_block_height,
                transferred_at_block_height,
                created_at_core_block_height,
                updated_at_core_block_height,
                transferred_at_core_block_height,
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
