use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::document::{Document, DocumentV0};
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{ReplacementType, Value};

impl DocumentTypeV0 {
    pub(in crate::data_contract::document_type) fn convert_value_to_document_v0(
        &self,
        mut data: Value,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match platform_version
            .dpp
            .document_versions
            .document_structure_version
        {
            0 => {
                let mut document: DocumentV0 = DocumentV0 {
                    id: data.remove_identifier("$id")?,
                    owner_id: data.remove_identifier("$ownerId")?,
                    properties: Default::default(),
                    revision: data.remove_optional_integer("$revision")?,
                    created_at: data.remove_optional_integer("$createdAt")?,
                    updated_at: data.remove_optional_integer("$updatedAt")?,
                };

                data.replace_at_paths(
                    self.identifier_paths.iter().map(|s| s.as_str()),
                    ReplacementType::Identifier,
                )?;

                data.replace_at_paths(
                    self.binary_paths.iter().map(|s| s.as_str()),
                    ReplacementType::BinaryBytes,
                )?;

                document.properties = data
                    .into_btree_string_map()
                    .map_err(ProtocolError::ValueError)?;

                Ok(document.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "convert_value_to_document_v0 inner match to document".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
