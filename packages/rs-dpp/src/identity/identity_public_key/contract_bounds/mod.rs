use crate::identifier::Identifier;
use crate::identity::identity_public_key::contract_bounds::ContractBounds::{
    SingleContract, SingleContractDocumentType,
};
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

pub type ContractBoundsType = u8;

/// A contract bounds is the bounds that the key has influence on.
/// For authentication keys the bounds mean that the keys can only be used to sign
/// within the specified contract.
/// For encryption decryption this tells clients to only use these keys for specific
/// contracts.
///
#[cfg_attr(feature = "json-conversion", derive(JsonConvertible))]
#[repr(u8)]
#[derive(
    Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Encode, Decode, Ord, PartialOrd, Hash,
)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[serde(tag = "$type", rename_all = "camelCase")]
pub enum ContractBounds {
    /// this key can only be used within a specific contract
    #[serde(rename = "singleContract")]
    SingleContract { id: Identifier } = 0,
    /// this key can only be used within a specific contract and for a specific document type
    #[serde(rename = "documentType", rename_all = "camelCase")]
    SingleContractDocumentType {
        id: Identifier,
        document_type_name: String,
    } = 1,
    // /// this key can only be used within contracts owned by a specified owner
    // #[serde(rename = "multipleContractsOfSameOwner")]
    // MultipleContractsOfSameOwner { owner_id: Identifier } = 2,
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
                document_type_name: document_type,
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
            // MultipleContractsOfSameOwner { .. } => 2,
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
            // MultipleContractsOfSameOwner { .. } => "multipleContractsOfSameOwner",
        }
    }

    /// Gets the identifier
    pub fn identifier(&self) -> &Identifier {
        match self {
            SingleContract { id } => id,
            SingleContractDocumentType { id, .. } => id,
            // MultipleContractsOfSameOwner { owner_id } => owner_id,
        }
    }

    /// Gets the document type
    pub fn document_type(&self) -> Option<&String> {
        match self {
            SingleContract { .. } => None,
            SingleContractDocumentType {
                document_type_name: document_type,
                ..
            } => Some(document_type),
            // MultipleContractsOfSameOwner { .. } => None,
        }
    }
    //
    // /// Gets the cbor value
    // pub fn to_cbor_value(&self) -> CborValue {
    //     let mut pk_map = CborCanonicalMap::new();
    //
    //     let contract_bounds_type = self.contract_bounds_type();
    //     pk_map.insert("type", self.contract_bounds_type_string());
    //
    //     pk_map.insert("identifier", self.identifier().to_buffer_vec());
    //
    //     if contract_bounds_type == 1 {
    //         pk_map.insert("documentType", self.document_type().unwrap().clone());
    //     }
    //     pk_map.to_value_sorted()
    // }
    //
    // /// Gets the cbor value
    // pub fn from_cbor_value(cbor_value: &CborValue) -> Result<Self, ProtocolError> {
    //     let key_value_map = cbor_value.as_map().ok_or_else(|| {
    //         ProtocolError::DecodingError(String::from(
    //             "Expected identity public key to be a key value map",
    //         ))
    //     })?;
    //
    //     let contract_bounds_type_string =
    //         key_value_map.as_string("type", "Contract bounds must have a type")?;
    //     let contract_bounds_type =
    //         Self::contract_bounds_type_from_str(contract_bounds_type_string.as_str())?;
    //     let contract_bounds_identifier = if contract_bounds_type > 0 {
    //         key_value_map.as_vec(
    //             "identifier",
    //             "Contract bounds must have an identifier if it is not type 0",
    //         )?
    //     } else {
    //         vec![]
    //     };
    //     let contract_bounds_document_type = if contract_bounds_type == 2 {
    //         key_value_map.as_string(
    //             "documentType",
    //             "Contract bounds must have a document type if it is type 2",
    //         )?
    //     } else {
    //         String::new()
    //     };
    //     ContractBounds::new_from_type(
    //         contract_bounds_type,
    //         contract_bounds_identifier,
    //         contract_bounds_document_type,
    //     )
    // }
}

#[cfg(test)]
mod core_tests {
    use super::*;

    // -- new_from_type: valid types --
    #[test]
    fn test_new_from_type_single_contract() {
        let id_bytes = vec![0xAAu8; 32];
        let bounds =
            ContractBounds::new_from_type(0, id_bytes.clone(), "ignored".to_string()).unwrap();
        assert!(matches!(bounds, ContractBounds::SingleContract { .. }));
        assert_eq!(bounds.contract_bounds_type(), 0);
        assert_eq!(bounds.contract_bounds_type_string(), "singleContract");
        assert_eq!(bounds.identifier().as_bytes(), id_bytes.as_slice());
        // document_type is None for SingleContract regardless of what we passed in.
        assert!(bounds.document_type().is_none());
    }

    #[test]
    fn test_new_from_type_single_contract_document_type() {
        let id_bytes = vec![0xBBu8; 32];
        let bounds = ContractBounds::new_from_type(1, id_bytes.clone(), "myDoc".to_string())
            .expect("expected to construct SingleContractDocumentType");
        assert!(matches!(
            bounds,
            ContractBounds::SingleContractDocumentType { .. }
        ));
        assert_eq!(bounds.contract_bounds_type(), 1);
        assert_eq!(bounds.contract_bounds_type_string(), "documentType");
        assert_eq!(bounds.identifier().as_bytes(), id_bytes.as_slice());
        assert_eq!(bounds.document_type().map(String::as_str), Some("myDoc"));
    }

    // -- new_from_type: invalid type --
    #[test]
    fn test_new_from_type_unrecognized_type_returns_error() {
        let id_bytes = vec![0xCCu8; 32];
        let err = ContractBounds::new_from_type(99, id_bytes, "".to_string()).unwrap_err();
        match err {
            ProtocolError::InvalidKeyContractBoundsError(msg) => {
                assert!(msg.contains("99"), "expected error message to mention 99");
            }
            other => panic!("expected InvalidKeyContractBoundsError, got {:?}", other),
        }
    }

    // -- new_from_type: identifier wrong length --
    #[test]
    fn test_new_from_type_invalid_identifier_length_returns_error() {
        // Identifier::from_bytes requires exactly 32 bytes.
        let short = vec![0x01u8; 10];
        assert!(ContractBounds::new_from_type(0, short, "".to_string()).is_err());
    }

    // -- contract_bounds_type_from_str --
    #[test]
    fn test_contract_bounds_type_from_str_single_contract() {
        assert_eq!(
            ContractBounds::contract_bounds_type_from_str("singleContract").unwrap(),
            0
        );
    }

    #[test]
    fn test_contract_bounds_type_from_str_document_type() {
        assert_eq!(
            ContractBounds::contract_bounds_type_from_str("documentType").unwrap(),
            1
        );
    }

    #[test]
    fn test_contract_bounds_type_from_str_unknown_returns_error() {
        let err = ContractBounds::contract_bounds_type_from_str("garbage").unwrap_err();
        match err {
            ProtocolError::DecodingError(_) => {}
            other => panic!("expected ProtocolError::DecodingError, got {:?}", other),
        }
    }

    // -- equality / clone / hash (derives) --
    #[test]
    fn test_contract_bounds_equality_and_clone() {
        let id = Identifier::from([0x11u8; 32]);
        let a = ContractBounds::SingleContract { id };
        let b = a.clone();
        assert_eq!(a, b);

        let different = ContractBounds::SingleContractDocumentType {
            id,
            document_type_name: "foo".to_string(),
        };
        assert_ne!(a, different);
    }

    #[test]
    fn test_contract_bounds_type_string_roundtrip_with_from_str() {
        // The string form produced by the variant should round-trip through from_str
        // back to its numeric discriminant.
        let id_bytes = vec![0xD0u8; 32];
        let sc = ContractBounds::new_from_type(0, id_bytes.clone(), "".to_string()).unwrap();
        let sctd = ContractBounds::new_from_type(1, id_bytes, "docType".to_string()).unwrap();

        assert_eq!(
            ContractBounds::contract_bounds_type_from_str(sc.contract_bounds_type_string())
                .unwrap(),
            sc.contract_bounds_type()
        );
        assert_eq!(
            ContractBounds::contract_bounds_type_from_str(sctd.contract_bounds_type_string())
                .unwrap(),
            sctd.contract_bounds_type()
        );
    }
}

#[cfg(all(test, feature = "json-conversion"))]
mod tests {
    use super::*;
    use crate::serialization::JsonConvertible;

    #[test]
    fn contract_bounds_single_contract_json_round_trip() {
        let id = Identifier::from([0xABu8; 32]);
        let bounds = ContractBounds::SingleContract { id };

        let json = bounds.to_json().expect("to_json should succeed");
        assert!(
            json["id"].is_string(),
            "Identifier should be a base58 string, got: {:?}",
            json["id"]
        );

        let expected_base58 = id.to_string(platform_value::string_encoding::Encoding::Base58);
        assert_eq!(json["id"].as_str().unwrap(), expected_base58);

        let restored = ContractBounds::from_json(json).expect("from_json should succeed");
        assert_eq!(bounds, restored);
    }

    #[test]
    fn contract_bounds_document_type_json_round_trip() {
        let id = Identifier::from([0xCDu8; 32]);
        let bounds = ContractBounds::SingleContractDocumentType {
            id,
            document_type_name: "myDocument".to_string(),
        };

        let json = bounds.to_json().expect("to_json should succeed");
        assert!(json["id"].is_string());
        assert_eq!(json["documentTypeName"].as_str().unwrap(), "myDocument");

        let restored = ContractBounds::from_json(json).expect("from_json should succeed");
        assert_eq!(bounds, restored);
    }

    #[test]
    fn contract_bounds_value_round_trip() {
        let id = Identifier::from([0x55u8; 32]);
        let bounds = ContractBounds::SingleContractDocumentType {
            id,
            document_type_name: "note".to_string(),
        };

        let obj = bounds.to_object().expect("to_object should succeed");
        let restored = ContractBounds::from_object(obj).expect("from_object should succeed");
        assert_eq!(bounds, restored);
    }
}
