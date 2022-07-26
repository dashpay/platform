use std::collections::BTreeMap;
use std::io::{BufReader, Read};

use byteorder::{BigEndian, WriteBytesExt};
use ciborium::value::Value;
use dpp::data_contract::extra::DriveContractExt;
use serde::{Deserialize, Serialize};

use crate::common::{bytes_for_system_value_from_tree_map, get_key_from_cbor_map};
use crate::contract::Contract;
use crate::drive::defaults::PROTOCOL_VERSION;
use crate::drive::Drive;
use dpp::data_contract::extra::{ContractError, DocumentType};

use crate::error::drive::DriveError;
use crate::error::structure::StructureError;
use crate::error::Error;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Document {
    #[serde(rename = "$id")]
    pub id: [u8; 32],
    #[serde(flatten)]
    pub properties: BTreeMap<String, Value>,
    #[serde(rename = "$ownerId")]
    pub owner_id: [u8; 32],
}

impl Document {
    // The serialization of a document follows the following pattern
    // id 32 bytes
    // owner_id 32 bytes
    //
    pub fn serialize(&self, document_type: &DocumentType) -> Result<Vec<u8>, Error> {
        let mut buffer: Vec<u8> = self.id.as_slice().to_vec();
        buffer.extend(self.owner_id.as_slice());
        document_type
            .properties
            .iter()
            .map(|(field_name, field)| {
                if let Some(value) = self.properties.get(field_name) {
                    let value = field
                        .document_type
                        .encode_value_ref_with_size(value, field.required)?;
                    buffer.extend(value.as_slice());
                    Ok(())
                } else if field.required {
                    Err(Error::Contract(ContractError::MissingRequiredKey(
                        "a required field is not present",
                    )))
                } else {
                    // We don't have something that wasn't required
                    buffer.push(0);
                    Ok(())
                }
            })
            .collect::<Result<(), Error>>()?;
        Ok(buffer)
    }

    pub fn serialize_consume(mut self, document_type: &DocumentType) -> Result<Vec<u8>, Error> {
        let mut buffer: Vec<u8> = Vec::try_from(self.id).unwrap();
        let mut owner_id = Vec::try_from(self.owner_id).unwrap();
        buffer.append(&mut owner_id);
        document_type
            .properties
            .iter()
            .map(|(field_name, field)| {
                if let Some(value) = self.properties.remove(field_name) {
                    let value = field
                        .document_type
                        .encode_value_with_size(value, field.required)?;
                    buffer.extend(value.as_slice());
                    Ok(())
                } else if field.required {
                    Err(Error::Contract(ContractError::MissingRequiredKey(
                        "a required field is not present",
                    )))
                } else {
                    // We don't have something that wasn't required
                    buffer.push(0);
                    Ok(())
                }
            })
            .collect::<Result<(), Error>>()?;
        Ok(buffer)
    }

    pub fn from_bytes(
        serialized_document: &[u8],
        document_type: &DocumentType,
    ) -> Result<Self, Error> {
        let mut buf = BufReader::new(serialized_document);
        if serialized_document.len() < 64 {
            return Err(Error::Drive(DriveError::CorruptedSerialization(
                "serialized document is too small, must have id and owner id",
            )));
        }
        let mut id = [0; 32];
        buf.read_exact(&mut id).map_err(|_| {
            Error::Drive(DriveError::CorruptedSerialization(
                "error reading from serialized document",
            ))
        })?;

        let mut owner_id = [0; 32];
        buf.read_exact(&mut owner_id).map_err(|_| {
            Error::Drive(DriveError::CorruptedSerialization(
                "error reading from serialized document",
            ))
        })?;

        let properties = document_type
            .properties
            .iter()
            .filter_map(|(key, field)| {
                let read_value = field.document_type.read_from(&mut buf, field.required);
                match read_value {
                    Ok(read_value) => {
                        if let Some(read_value) = read_value {
                            Some(Ok((key.clone(), read_value)))
                        } else {
                            None
                        }
                    }
                    Err(e) => Some(Err(e)),
                }
            })
            .collect::<Result<BTreeMap<String, Value>, ContractError>>()?;
        let id = <[u8; 32]>::try_from(id).unwrap();
        let owner_id = <[u8; 32]>::try_from(owner_id).unwrap();
        Ok(Document {
            id,
            properties,
            owner_id,
        })
    }

    pub fn from_cbor(
        document_cbor: &[u8],
        document_id: Option<&[u8]>,
        owner_id: Option<&[u8]>,
    ) -> Result<Self, Error> {
        let (version, read_document_cbor) = document_cbor.split_at(4);
        if !Drive::check_protocol_version_bytes(version) {
            return Err(Error::Structure(StructureError::InvalidProtocolVersion(
                "invalid protocol version",
            )));
        }
        // first we need to deserialize the document and contract indices
        // we would need dedicated deserialization functions based on the document type
        let mut document: BTreeMap<String, Value> = ciborium::de::from_reader(read_document_cbor)
            .map_err(|_| {
            Error::Structure(StructureError::InvalidCBOR("unable to decode contract"))
        })?;

        let owner_id: [u8; 32] = match owner_id {
            None => {
                let owner_id: Vec<u8> =
                    bytes_for_system_value_from_tree_map(&document, "$ownerId")?.ok_or({
                        Error::Contract(ContractError::DocumentOwnerIdMissing(
                            "unable to get document $ownerId",
                        ))
                    })?;
                document.remove("$ownerId");
                if owner_id.len() != 32 {
                    return Err(Error::Contract(ContractError::FieldRequirementUnmet(
                        "invalid owner id",
                    )));
                }
                owner_id.as_slice().try_into()
            }
            Some(owner_id) => {
                // we need to start by verifying that the owner_id is a 256 bit number (32 bytes)
                if owner_id.len() != 32 {
                    return Err(Error::Contract(ContractError::FieldRequirementUnmet(
                        "invalid owner id",
                    )));
                }
                owner_id.try_into()
            }
        }
        .expect("conversion to 32bytes shouldn't fail");

        let id: [u8; 32] = match document_id {
            None => {
                let document_id: Vec<u8> = bytes_for_system_value_from_tree_map(&document, "$id")?
                    .ok_or({
                        Error::Contract(ContractError::DocumentIdMissing(
                            "unable to get document $id",
                        ))
                    })?;
                document.remove("$id");
                if document_id.len() != 32 {
                    return Err(Error::Contract(ContractError::FieldRequirementUnmet(
                        "invalid document id",
                    )));
                }
                document_id.as_slice().try_into()
            }
            Some(document_id) => {
                // we need to start by verifying that the document_id is a 256 bit number (32 bytes)
                if document_id.len() != 32 {
                    return Err(Error::Contract(ContractError::FieldRequirementUnmet(
                        "invalid document id",
                    )));
                }
                document_id.try_into()
            }
        }
        .expect("document_id must be 32 bytes");

        // dev-note: properties is everything other than the id and owner id
        Ok(Document {
            properties: document,
            owner_id,
            id,
        })
    }

    pub fn from_cbor_with_id(
        document_cbor: &[u8],
        document_id: &[u8],
        owner_id: &[u8],
    ) -> Result<Self, Error> {
        // we need to start by verifying that the owner_id is a 256 bit number (32 bytes)
        if owner_id.len() != 32 {
            return Err(Error::Contract(ContractError::FieldRequirementUnmet(
                "invalid owner id",
            )));
        }

        if document_id.len() != 32 {
            return Err(Error::Contract(ContractError::FieldRequirementUnmet(
                "invalid document id",
            )));
        }

        let (version, read_document_cbor) = document_cbor.split_at(4);
        if !Drive::check_protocol_version_bytes(version) {
            return Err(Error::Structure(StructureError::InvalidProtocolVersion(
                "invalid protocol version",
            )));
        }

        // first we need to deserialize the document and contract indices
        // we would need dedicated deserialization functions based on the document type
        let properties: BTreeMap<String, Value> = ciborium::de::from_reader(read_document_cbor)
            .map_err(|_| {
                Error::Structure(StructureError::InvalidCBOR("unable to decode contract"))
            })?;

        // dev-note: properties is everything other than the id and owner id
        Ok(Document {
            properties,
            owner_id: owner_id
                .try_into()
                .expect("try_into shouldn't fail, document_id must be 32 bytes"),
            id: document_id
                .try_into()
                .expect("try_into shouldn't fail, document_id must be 32 bytes"),
        })
    }

    pub fn to_cbor(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::new();
        buffer
            .write_u32::<BigEndian>(PROTOCOL_VERSION)
            .expect("writing protocol version caused error");
        ciborium::ser::into_writer(&self, &mut buffer).expect("unable to serialize into cbor");
        buffer
    }

    pub fn get_raw_for_document_type<'a>(
        &'a self,
        key_path: &str,
        document_type: &DocumentType,
        owner_id: Option<&[u8]>,
    ) -> Result<Option<Vec<u8>>, Error> {
        if key_path == "$ownerId" && owner_id.is_some() {
            Ok(Some(Vec::from(owner_id.unwrap())))
        } else {
            match key_path {
                "$id" => return Ok(Some(Vec::from(self.id))),
                "$ownerId" => return Ok(Some(Vec::from(self.owner_id))),
                _ => {}
            }
            let key_paths: Vec<&str> = key_path.split('.').collect::<Vec<&str>>();
            let (key, rest_key_paths) = key_paths.split_first().ok_or({
                Error::Contract(ContractError::MissingRequiredKey(
                    "key must not be null when getting from document",
                ))
            })?;

            fn get_value_at_path<'a>(
                value: &'a Value,
                key_paths: &'a [&str],
            ) -> Result<Option<&'a Value>, Error> {
                if key_paths.is_empty() {
                    Ok(Some(value))
                } else {
                    let (key, rest_key_paths) = key_paths.split_first().ok_or({
                        Error::Contract(ContractError::MissingRequiredKey(
                            "key must not be null when getting from document",
                        ))
                    })?;
                    let map_values = value.as_map().ok_or({
                        Error::Contract(ContractError::ValueWrongType(
                            "inner key must refer to a value map",
                        ))
                    })?;
                    match get_key_from_cbor_map(map_values, key) {
                        None => Ok(None),
                        Some(value) => get_value_at_path(value, rest_key_paths),
                    }
                }
            }

            match self.properties.get(*key) {
                None => Ok(None),
                Some(value) => match get_value_at_path(value, rest_key_paths)? {
                    None => Ok(None),
                    Some(path_value) => Ok(Some(
                        document_type.serialize_value_for_key(key_path, path_value)?,
                    )),
                },
            }
        }
    }

    pub fn get_raw_for_contract<'a>(
        &'a self,
        key: &str,
        document_type_name: &str,
        contract: &Contract,
        owner_id: Option<&[u8]>,
    ) -> Result<Option<Vec<u8>>, Error> {
        let document_type = contract.document_types().get(document_type_name).ok_or({
            Error::Contract(ContractError::DocumentTypeNotFound(
                "document type should exist for name",
            ))
        })?;
        self.get_raw_for_document_type(key, document_type, owner_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::json_document_to_cbor;
    use crate::contract::CreateRandomDocument;
    use dpp::data_contract::extra::DriveContractExt;

    #[test]
    fn test_drive_serialization() {
        let dashpay_cbor = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/dashpay-contract.json",
            Some(1),
        );
        let contract = <Contract as DriveContractExt>::from_cbor(&dashpay_cbor, None).unwrap();

        let document_type = contract
            .document_type_for_name("contactRequest")
            .expect("expected to get profile document type");
        let document = document_type.random_document(Some(3333));

        let document_cbor = document.to_cbor();

        let serialized_document = document
            .serialize(document_type)
            .expect("expected to serialize");

        let deserialized_document = document_type
            .document_from_bytes(serialized_document.as_slice())
            .expect("expected to deserialize a document");
        assert_eq!(document, deserialized_document);
        assert!(serialized_document.len() < document_cbor.len());
        for _i in 0..10000 {
            let document = document_type.random_document(Some(3333));
            let _serialized_document = document
                .serialize_consume(document_type)
                .expect("expected to serialize");
        }
    }
}
