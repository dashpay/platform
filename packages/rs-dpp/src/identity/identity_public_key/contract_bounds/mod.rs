use crate::identifier::Identifier;
use crate::identity::identity_public_key::contract_bounds::ContractBounds::{
    MultipleContractsOfSameOwner, SingleContract, SingleContractDocumentType,
};
use crate::util::cbor_value::{CborCanonicalMap, CborMapExtension};
use crate::ProtocolError;
use bincode::{Decode, Encode};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub type ContractBoundsType = u8;

/// A contract bounds is the bounds that the key has influence on.
/// For authentication keys the bounds mean that the keys can only be used to sign
/// within the specified contract.
/// For encryption decryption this tells clients to only use these keys for specific
/// contracts.
///
#[repr(u8)]
#[derive(
    Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Encode, Decode, Ord, PartialOrd, Hash,
)]
#[serde(tag = "type")]
pub enum ContractBounds {
    /// this key can only be used within a specific contract
    #[serde(rename = "singleContract")]
    SingleContract { id: Identifier } = 0,
    /// this key can only be used within a specific contract and for a specific document type
    #[serde(rename = "documentType")]
    SingleContractDocumentType {
        id: Identifier,
        document_type: String,
    } = 1,
    /// this key can only be used within contracts owned by a specified owner
    #[serde(rename = "multipleContractsOfSameOwner")]
    MultipleContractsOfSameOwner { owner_id: Identifier } = 2,
}

impl ContractBounds {
    /// Creates a new contract bounds for the key
    pub fn new_from_type(
        contract_bounds_type: u8,
        identifier: Vec<u8>,
        document_type: String,
    ) -> Result<Self, ProtocolError> {
        Ok(match contract_bounds_type {
            0 => SingleContract {
                id: Identifier::from_bytes(identifier.as_slice())?,
            },
            1 => SingleContractDocumentType {
                id: Identifier::from_bytes(identifier.as_slice())?,
                document_type,
            },
            _ => {
                return Err(ProtocolError::InvalidKeyContractBoundsError(format!(
                    "unrecognized contract bounds type: {}",
                    contract_bounds_type
                )))
            }
        })
    }

    /// Gets the contract bounds type
    pub fn contract_bounds_type(&self) -> ContractBoundsType {
        match self {
            SingleContract { .. } => 0,
            SingleContractDocumentType { .. } => 1,
            MultipleContractsOfSameOwner { .. } => 2,
        }
    }

    pub fn contract_bounds_type_from_str(str: &str) -> Result<ContractBoundsType, ProtocolError> {
        match str {
            "singleContract" => Ok(0),
            "documentType" => Ok(1),
            _ => Err(ProtocolError::DecodingError(String::from(
                "Expected type to be one of none, singleContract or singleContractDocumentType",
            ))),
        }
    }
    /// Gets the contract bounds type
    pub fn contract_bounds_type_string(&self) -> &str {
        match self {
            SingleContract { .. } => "singleContract",
            SingleContractDocumentType { .. } => "documentType",
            MultipleContractsOfSameOwner { .. } => "multipleContractsOfSameOwner",
        }
    }

    /// Gets the identifier
    pub fn identifier(&self) -> &Identifier {
        match self {
            SingleContract { id } => id,
            SingleContractDocumentType { id, .. } => id,
            MultipleContractsOfSameOwner { owner_id } => owner_id,
        }
    }

    /// Gets the document type
    pub fn document_type(&self) -> Option<&String> {
        match self {
            SingleContract { .. } => None,
            SingleContractDocumentType { document_type, .. } => Some(document_type),
            MultipleContractsOfSameOwner { .. } => None,
        }
    }

    /// Gets the cbor value
    pub fn to_cbor_value(&self) -> CborValue {
        let mut pk_map = CborCanonicalMap::new();

        let contract_bounds_type = self.contract_bounds_type();
        pk_map.insert("type", self.contract_bounds_type_string());

        pk_map.insert("identifier", self.identifier().to_buffer_vec());

        if contract_bounds_type == 1 {
            pk_map.insert("documentType", self.document_type().unwrap().clone());
        }
        pk_map.to_value_sorted()
    }

    /// Gets the cbor value
    pub fn from_cbor_value(cbor_value: &CborValue) -> Result<Self, ProtocolError> {
        let key_value_map = cbor_value.as_map().ok_or_else(|| {
            ProtocolError::DecodingError(String::from(
                "Expected identity public key to be a key value map",
            ))
        })?;

        let contract_bounds_type_string =
            key_value_map.as_string("type", "Contract bounds must have a type")?;
        let contract_bounds_type =
            Self::contract_bounds_type_from_str(contract_bounds_type_string.as_str())?;
        let contract_bounds_identifier = if contract_bounds_type > 0 {
            key_value_map.as_vec(
                "identifier",
                "Contract bounds must have an identifier if it is not type 0",
            )?
        } else {
            vec![]
        };
        let contract_bounds_document_type = if contract_bounds_type == 2 {
            key_value_map.as_string(
                "documentType",
                "Contract bounds must have a document type if it is type 2",
            )?
        } else {
            String::new()
        };
        ContractBounds::new_from_type(
            contract_bounds_type,
            contract_bounds_identifier,
            contract_bounds_document_type,
        )
    }
}
