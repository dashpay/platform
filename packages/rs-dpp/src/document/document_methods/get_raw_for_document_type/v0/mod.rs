use crate::data_contract::document_type::methods::DocumentTypeV0Methods;
use crate::data_contract::document_type::DocumentPropertyType;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::DocumentV0Getters;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::btreemap_extensions::BTreeValueMapPathHelper;

pub trait DocumentGetRawForDocumentTypeV0: DocumentV0Getters {
    /// Return a value given the path to its key for a document type.
    fn get_raw_for_document_type_v0(
        &self,
        key_path: &str,
        document_type: DocumentTypeRef,
        owner_id: Option<[u8; 32]>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<u8>>, ProtocolError> {
        // todo: maybe merge with document_type.serialize_value_for_key() because we use different
        //   code paths for query and index creation
        // returns the owner id if the key path is $ownerId and an owner id is given
        if key_path == "$ownerId" {
            if let Some(owner_id) = owner_id {
                return Ok(Some(Vec::from(owner_id)));
            }
        }

        match key_path {
            // returns self.id or self.owner_id if key path is $id or $ownerId
            "$id" => return Ok(Some(self.id().to_vec())),
            "$ownerId" => return Ok(Some(self.owner_id().to_vec())),
            "$createdAt" => {
                return Ok(self
                    .created_at()
                    .map(DocumentPropertyType::encode_date_timestamp))
            }
            "$createdAtBlockHeight" => {
                return Ok(self
                    .created_at_block_height()
                    .map(DocumentPropertyType::encode_u64))
            }
            "$createdAtCoreBlockHeight" => {
                return Ok(self
                    .created_at_core_block_height()
                    .map(DocumentPropertyType::encode_u32))
            }
            "$updatedAt" => {
                return Ok(self
                    .updated_at()
                    .map(DocumentPropertyType::encode_date_timestamp))
            }
            "$updatedAtBlockHeight" => {
                return Ok(self
                    .updated_at_block_height()
                    .map(DocumentPropertyType::encode_u64))
            }
            "$updatedAtCoreBlockHeight" => {
                return Ok(self
                    .updated_at_core_block_height()
                    .map(DocumentPropertyType::encode_u32))
            }
            "$transferredAt" => {
                return Ok(self
                    .transferred_at()
                    .map(DocumentPropertyType::encode_date_timestamp))
            }
            "$transferredAtBlockHeight" => {
                return Ok(self
                    .transferred_at_block_height()
                    .map(DocumentPropertyType::encode_u64))
            }
            "$transferredAtCoreBlockHeight" => {
                return Ok(self
                    .transferred_at_core_block_height()
                    .map(DocumentPropertyType::encode_u32))
            }
            _ => {}
        }
        self.properties()
            .get_optional_at_path(key_path)?
            .map(|value| document_type.serialize_value_for_key(key_path, value, platform_version))
            .transpose()
    }
}
