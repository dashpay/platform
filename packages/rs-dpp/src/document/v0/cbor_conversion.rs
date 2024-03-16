use crate::document::property_names;

use crate::identity::TimestampMillis;
use crate::prelude::{BlockHeight, CoreBlockHeight, Revision};

use crate::ProtocolError;

use crate::document::serialization_traits::{
    DocumentCborMethodsV0, DocumentPlatformValueMethodsV0,
};
use crate::document::v0::DocumentV0;
use crate::version::PlatformVersion;
use ciborium::Value as CborValue;
use integer_encoding::VarIntWriter;
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::{Identifier, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};

#[cfg(feature = "cbor")]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DocumentForCbor {
    /// The unique document ID.
    #[serde(rename = "$id")]
    pub id: [u8; 32],

    /// The document's properties (data).
    #[serde(flatten)]
    pub properties: BTreeMap<String, CborValue>,

    /// The ID of the document's owner.
    #[serde(rename = "$ownerId")]
    pub owner_id: [u8; 32],

    /// The document revision.
    #[serde(rename = "$revision")]
    pub revision: Option<Revision>,

    #[serde(rename = "$createdAt")]
    pub created_at: Option<TimestampMillis>,
    #[serde(rename = "$updatedAt")]
    pub updated_at: Option<TimestampMillis>,

    #[serde(rename = "$createdAtBlockHeight")]
    pub created_at_block_height: Option<BlockHeight>,
    #[serde(rename = "$updatedAtBlockHeight")]
    pub updated_at_block_height: Option<BlockHeight>,

    #[serde(rename = "$createdAtCoreBlockHeight")]
    pub created_at_core_block_height: Option<CoreBlockHeight>,
    #[serde(rename = "$updatedAtCoreBlockHeight")]
    pub updated_at_core_block_height: Option<CoreBlockHeight>,
}

#[cfg(feature = "cbor")]
impl TryFrom<DocumentV0> for DocumentForCbor {
    type Error = ProtocolError;

    fn try_from(value: DocumentV0) -> Result<Self, Self::Error> {
        let DocumentV0 {
            id,
            properties,
            owner_id,
            revision,
            created_at,
            updated_at,
            created_at_block_height,
            updated_at_block_height,
            created_at_core_block_height,
            updated_at_core_block_height,
        } = value;
        Ok(DocumentForCbor {
            id: id.to_buffer(),
            properties: Value::convert_to_cbor_map(properties)
                .map_err(ProtocolError::ValueError)?,
            owner_id: owner_id.to_buffer(),
            revision,
            created_at,
            updated_at,
            created_at_block_height,
            updated_at_block_height,
            created_at_core_block_height,
            updated_at_core_block_height,
        })
    }
}

impl DocumentV0 {
    /// Reads a CBOR-serialized document and creates a Document from it.
    /// If Document and Owner IDs are provided, they are used, otherwise they are created.
    fn from_map(
        mut document_map: BTreeMap<String, Value>,
        document_id: Option<[u8; 32]>,
        owner_id: Option<[u8; 32]>,
    ) -> Result<Self, ProtocolError> {
        let owner_id = match owner_id {
            None => document_map
                .remove_hash256_bytes(property_names::OWNER_ID)
                .map_err(ProtocolError::ValueError)?,
            Some(owner_id) => owner_id,
        };

        let id = match document_id {
            None => document_map
                .remove_hash256_bytes(property_names::ID)
                .map_err(ProtocolError::ValueError)?,
            Some(document_id) => document_id,
        };

        let revision = document_map.remove_optional_integer(property_names::REVISION)?;

        let created_at = document_map.remove_optional_integer(property_names::CREATED_AT)?;
        let updated_at = document_map.remove_optional_integer(property_names::UPDATED_AT)?;
        let created_at_block_height =
            document_map.remove_optional_integer(property_names::CREATED_AT_BLOCK_HEIGHT)?;
        let updated_at_block_height =
            document_map.remove_optional_integer(property_names::UPDATED_AT_BLOCK_HEIGHT)?;
        let created_at_core_block_height =
            document_map.remove_optional_integer(property_names::CREATED_AT_CORE_BLOCK_HEIGHT)?;
        let updated_at_core_block_height =
            document_map.remove_optional_integer(property_names::UPDATED_AT_CORE_BLOCK_HEIGHT)?;

        // dev-note: properties is everything other than the id and owner id
        Ok(DocumentV0 {
            properties: document_map,
            owner_id: Identifier::new(owner_id),
            id: Identifier::new(id),
            revision,
            created_at,
            updated_at,
            created_at_block_height,
            updated_at_block_height,
            created_at_core_block_height,
            updated_at_core_block_height,
        })
    }
}

impl DocumentCborMethodsV0 for DocumentV0 {
    /// Reads a CBOR-serialized document and creates a Document from it.
    /// If Document and Owner IDs are provided, they are used, otherwise they are created.
    fn from_cbor(
        document_cbor: &[u8],
        document_id: Option<[u8; 32]>,
        owner_id: Option<[u8; 32]>,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        // first we need to deserialize the document and contract indices
        // we would need dedicated deserialization functions based on the document type
        let document_cbor_map: BTreeMap<String, CborValue> =
            ciborium::de::from_reader(document_cbor).map_err(|_| {
                ProtocolError::InvalidCBOR(
                    "unable to decode document for document call".to_string(),
                )
            })?;
        let document_map: BTreeMap<String, Value> =
            Value::convert_from_cbor_map(document_cbor_map).map_err(ProtocolError::ValueError)?;
        Self::from_map(document_map, document_id, owner_id)
    }

    fn to_cbor_value(&self) -> Result<CborValue, ProtocolError> {
        self.to_object()
            .map(|v| v.try_into().map_err(ProtocolError::ValueError))?
    }

    /// Serializes the Document to CBOR.
    fn to_cbor(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut buffer: Vec<u8> = Vec::new();
        buffer.write_varint(0).map_err(|_| {
            ProtocolError::EncodingError("error writing protocol version".to_string())
        })?;
        let cbor_document = DocumentForCbor::try_from(self.clone())?;
        ciborium::ser::into_writer(&cbor_document, &mut buffer).map_err(|_| {
            ProtocolError::EncodingError("unable to serialize into cbor".to_string())
        })?;
        Ok(buffer)
    }
}
