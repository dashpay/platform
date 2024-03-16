use crate::data_contract::accessors::v0::DataContractV0Setters;
#[cfg(feature = "data-contract-json-conversion")]
use crate::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
#[cfg(any(feature = "state-transitions", feature = "factories"))]
use crate::data_contract::created_data_contract::v0::CreatedDataContractV0;
#[cfg(any(feature = "state-transitions", feature = "factories"))]
use crate::data_contract::created_data_contract::CreatedDataContract;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::{Document, DocumentV0};
use crate::prelude::{DataContract, IdentityNonce};
#[cfg(feature = "data-contract-cbor-conversion")]
use crate::util::cbor_serializer::serializable_value_to_cbor;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{Identifier, ReplacementType};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Reads a JSON file and converts it to serde_value.
#[cfg(feature = "data-contract-json-conversion")]
pub fn json_document_to_json_value(
    path: impl AsRef<Path>,
) -> Result<serde_json::Value, ProtocolError> {
    let file = File::open(path.as_ref()).map_err(|_| {
        ProtocolError::FileNotFound(format!(
            "file not found at path {}",
            path.as_ref().to_str().unwrap()
        ))
    })?;

    let reader = BufReader::new(file);
    serde_json::from_reader(reader)
        .map_err(|_| ProtocolError::DecodingError("error decoding value from document".to_string()))
}

/// Reads a JSON file and converts it to serde_value.
pub fn json_document_to_platform_value(
    path: impl AsRef<Path>,
) -> Result<platform_value::Value, ProtocolError> {
    let file = File::open(path.as_ref()).map_err(|_| {
        ProtocolError::FileNotFound(format!(
            "file not found at path {}",
            path.as_ref().to_str().unwrap()
        ))
    })?;

    let reader = BufReader::new(file);
    serde_json::from_reader(reader)
        .map_err(|_| ProtocolError::DecodingError("error decoding value from document".to_string()))
}

/// Reads a JSON file and converts it to CBOR.
#[cfg(feature = "data-contract-cbor-conversion")]
pub fn json_document_to_cbor(
    path: impl AsRef<Path>,
    protocol_version: Option<u32>,
) -> Result<Vec<u8>, ProtocolError> {
    let json = json_document_to_json_value(path)?;
    serializable_value_to_cbor(&json, protocol_version)
}

/// Reads a JSON file and converts it a contract.
#[cfg(feature = "data-contract-json-conversion")]
pub fn json_document_to_contract(
    path: impl AsRef<Path>,
    validate: bool,
    platform_version: &PlatformVersion,
) -> Result<DataContract, ProtocolError> {
    let value = json_document_to_json_value(path)?;

    DataContract::from_json(value, validate, platform_version)
}

#[cfg(all(
    any(feature = "state-transitions", feature = "factories"),
    feature = "data-contract-json-conversion"
))]
/// Reads a JSON file and converts it a contract.
pub fn json_document_to_created_contract(
    path: impl AsRef<Path>,
    identity_nonce: IdentityNonce,
    validate: bool,
    platform_version: &PlatformVersion,
) -> Result<CreatedDataContract, ProtocolError> {
    let data_contract = json_document_to_contract(path, validate, platform_version)?;

    Ok(CreatedDataContractV0 {
        data_contract,
        identity_nonce,
    }
    .into())
}

/// Reads a JSON file and converts it a document.
#[cfg(feature = "data-contract-json-conversion")]
pub fn json_document_to_contract_with_ids(
    path: impl AsRef<Path>,
    id: Option<Identifier>,
    owner_id: Option<Identifier>,
    validate: bool,
    platform_version: &PlatformVersion,
) -> Result<DataContract, ProtocolError> {
    let value = json_document_to_json_value(path)?;

    let mut contract = DataContract::from_json(value, validate, platform_version)?;

    if let Some(id) = id {
        contract.set_id(id);
    }

    if let Some(owner_id) = owner_id {
        contract.set_owner_id(owner_id);
    }

    Ok(contract)
}

/// Reads a JSON file and converts it a document.
pub fn json_document_to_document(
    path: impl AsRef<Path>,
    owner_id: Option<Identifier>,
    document_type: DocumentTypeRef,
    _platform_version: &PlatformVersion,
) -> Result<Document, ProtocolError> {
    let mut data = json_document_to_platform_value(path)?;

    if let Some(owner_id) = owner_id {
        data.set_value(
            "$ownerId",
            platform_value::Value::Identifier(owner_id.into_buffer()),
        )?;
    }

    let mut document: DocumentV0 = DocumentV0 {
        id: data.remove_identifier("$id")?,
        owner_id: data.remove_identifier("$ownerId")?,
        properties: Default::default(),
        revision: data.remove_optional_integer("$revision")?,
        created_at: data.remove_optional_integer("$createdAt")?,
        updated_at: data.remove_optional_integer("$updatedAt")?,
        created_at_block_height: data.remove_optional_integer("$createdAtBlockHeight")?,
        updated_at_block_height: data.remove_optional_integer("$updatedAtBlockHeight")?,
        created_at_core_block_height: data.remove_optional_integer("$createdAtCoreBlockHeight")?,
        updated_at_core_block_height: data.remove_optional_integer("$updatedAtCoreBlockHeight")?,
    };

    data.replace_at_paths(
        document_type.identifier_paths().iter().map(|s| s.as_str()),
        ReplacementType::Identifier,
    )?;

    data.replace_at_paths(
        document_type.binary_paths().iter().map(|s| s.as_str()),
        ReplacementType::BinaryBytes,
    )?;

    document.properties = data
        .into_btree_string_map()
        .map_err(ProtocolError::ValueError)?;

    Ok(document.into())
}
