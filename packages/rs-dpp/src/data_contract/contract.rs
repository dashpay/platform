use super::common::*;
use super::document_type::DocumentType;
use super::drive_api::DriveEncoding;
use super::errors::common::StructureError;
use super::errors::contract::ContractError;
use ciborium::value::Value as CborValue;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::convert::TryInto;

pub const DEFAULT_CONTRACT_KEEPS_HISTORY: bool = false;
pub const DEFAULT_CONTRACT_MUTABILITY: bool = true;
pub const DEFAULT_CONTRACT_DOCUMENTS_KEEPS_HISTORY: bool = false;
pub const DEFAULT_CONTRACT_DOCUMENT_MUTABILITY: bool = true;

// TODO drive dependency
#[repr(u8)]
pub enum RootTree {
    // Input data errors
    Identities = 0,
    ContractDocuments = 1,
    PublicKeyHashesToIdentities = 2,
    Misc = 3,
}

// TODO drive dependency
impl From<RootTree> for u8 {
    fn from(root_tree: RootTree) -> Self {
        root_tree as u8
    }
}

// TODO drive dependency
impl From<RootTree> for [u8; 1] {
    fn from(root_tree: RootTree) -> Self {
        [root_tree as u8]
    }
}

// TODO drive dependency
impl From<RootTree> for &'static [u8; 1] {
    fn from(root_tree: RootTree) -> Self {
        match root_tree {
            RootTree::Identities => &[0],
            RootTree::ContractDocuments => &[1],
            RootTree::PublicKeyHashesToIdentities => &[2],
            RootTree::Misc => &[3],
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
pub struct Contract {
    pub id: [u8; 32],
    pub document_types: BTreeMap<String, DocumentType>,
    pub keeps_history: bool,
    pub readonly: bool,
    pub documents_keep_history_contract_default: bool,
    pub documents_mutable_contract_default: bool,
}

impl Contract {
    pub fn deserialize(
        serialized_contract: &[u8],
        contract_id: Option<[u8; 32]>,
        encoding: DriveEncoding,
    ) -> Result<Self, ContractError> {
        match encoding {
            DriveEncoding::DriveCbor => Contract::from_cbor(serialized_contract, contract_id),
            DriveEncoding::DriveProtobuf => {
                todo!()
            }
        }
    }

    pub fn from_cbor(
        contract_cbor: &[u8],
        contract_id: Option<[u8; 32]>,
    ) -> Result<Self, ContractError> {
        let (version, read_contract_cbor) = contract_cbor.split_at(4);
        if !check_protocol_version_bytes(version) {
            return Err(StructureError::InvalidProtocolVersion("invalid protocol version").into());
        }
        // Deserialize the contract
        let contract: BTreeMap<String, CborValue> =
            ciborium::de::from_reader(read_contract_cbor)
                .map_err(|_| StructureError::InvalidCBOR("unable to decode contract"))?;

        // Get the contract id
        let contract_id: [u8; 32] = if let Some(contract_id) = contract_id {
            contract_id
        } else {
            bytes_for_system_value_from_tree_map(&contract, "$id")?
                .ok_or({ ContractError::MissingRequiredKey("unable to get contract id") })?
                .try_into()
                .map_err(|_| ContractError::FieldRequirementUnmet("contract_id must be 32 bytes"))?
        };

        // Does the contract keep history when the contract itself changes?
        let keeps_history: bool = bool_for_system_value_from_tree_map(
            &contract,
            "keepsHistory",
            DEFAULT_CONTRACT_KEEPS_HISTORY,
        )?;

        // Is the contract mutable?
        let readonly: bool = bool_for_system_value_from_tree_map(
            &contract,
            "readOnly",
            !DEFAULT_CONTRACT_MUTABILITY,
        )?;

        // Do documents in the contract keep history?
        let documents_keep_history_contract_default: bool = bool_for_system_value_from_tree_map(
            &contract,
            "documentsKeepHistoryContractDefault",
            DEFAULT_CONTRACT_DOCUMENTS_KEEPS_HISTORY,
        )?;

        // Are documents in the contract mutable?
        let documents_mutable_contract_default: bool = bool_for_system_value_from_tree_map(
            &contract,
            "documentsMutableContractDefault",
            DEFAULT_CONTRACT_DOCUMENT_MUTABILITY,
        )?;

        let definition_references = match contract.get("$defs") {
            None => BTreeMap::new(),
            Some(definition_value) => {
                let definition_map = definition_value.as_map();
                match definition_map {
                    None => BTreeMap::new(),
                    Some(key_value) => cbor_map_to_btree_map(key_value),
                }
            }
        };

        let documents_cbor_value = contract
            .get("documents")
            .ok_or({ ContractError::MissingRequiredKey("unable to get documents") })?;
        let contract_document_types_raw = documents_cbor_value
            .as_map()
            .ok_or({ ContractError::InvalidContractStructure("documents must be a map") })?;

        let mut contract_document_types: BTreeMap<String, DocumentType> = BTreeMap::new();

        // Build the document type hashmap
        for (type_key_value, document_type_value) in contract_document_types_raw {
            if !type_key_value.is_text() {
                return Err(ContractError::InvalidContractStructure(
                    "document type name is not a string as expected",
                ));
            }

            // Make sure the document_type_value is a map
            if !document_type_value.is_map() {
                return Err(ContractError::InvalidContractStructure(
                    "document type data is not a map as expected",
                ));
            }

            let document_type = DocumentType::from_cbor_value(
                type_key_value.as_text().expect("confirmed as text"),
                document_type_value.as_map().expect("confirmed as map"),
                &definition_references,
                documents_keep_history_contract_default,
                documents_mutable_contract_default,
            )?;
            contract_document_types.insert(
                String::from(type_key_value.as_text().expect("confirmed as text")),
                document_type,
            );
        }

        Ok(Contract {
            id: contract_id,
            document_types: contract_document_types,
            keeps_history,
            readonly,
            documents_keep_history_contract_default,
            documents_mutable_contract_default,
        })
    }

    pub fn root_path(&self) -> [&[u8]; 2] {
        [
            Into::<&[u8; 1]>::into(RootTree::ContractDocuments),
            &self.id,
        ]
    }

    pub fn documents_path(&self) -> [&[u8]; 3] {
        [
            Into::<&[u8; 1]>::into(RootTree::ContractDocuments),
            &self.id,
            &[1],
        ]
    }

    pub fn document_type_path<'a>(&'a self, document_type_name: &'a str) -> [&'a [u8]; 4] {
        [
            Into::<&[u8; 1]>::into(RootTree::ContractDocuments),
            &self.id,
            &[1],
            document_type_name.as_bytes(),
        ]
    }

    pub fn documents_primary_key_path<'a>(&'a self, document_type_name: &'a str) -> [&'a [u8]; 5] {
        [
            Into::<&[u8; 1]>::into(RootTree::ContractDocuments),
            &self.id,
            &[1],
            document_type_name.as_bytes(),
            &[0],
        ]
    }

    pub fn documents_with_history_primary_key_path<'a>(
        &'a self,
        document_type_name: &'a str,
        id: &'a [u8],
    ) -> [&'a [u8]; 6] {
        [
            Into::<&[u8; 1]>::into(RootTree::ContractDocuments),
            &self.id,
            &[1],
            document_type_name.as_bytes(),
            &[0],
            id,
        ]
    }

    pub fn document_type_for_name(
        &self,
        document_type_name: &str,
    ) -> Result<&DocumentType, ContractError> {
        self.document_types.get(document_type_name).ok_or({
            ContractError::DocumentTypeNotFound("can not get document type from contract")
        })
    }
}

const fn check_protocol_version(_version: u32) -> bool {
    // Temporary disabled due protocol version is dynamic and goes from consensus params
    true
}

fn check_protocol_version_bytes(version_bytes: &[u8]) -> bool {
    if version_bytes.len() != 4 {
        false
    } else {
        let version_set_bytes: [u8; 4] = version_bytes
            .try_into()
            .expect("slice with incorrect length");
        let version = u32::from_be_bytes(version_set_bytes);
        // todo despite the const this will be use as dynamic content
        check_protocol_version(version)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::super::common::json_document_to_cbor;
    use super::super::contract::Contract;
    use super::*;

    #[test]
    fn test_cbor_deserialization() {
        let document_cbor = json_document_to_cbor("src/tests/payloads/simple.json", Some(1));
        let (version, read_document_cbor) = document_cbor.split_at(4);
        assert!(check_protocol_version_bytes(version));
        let document: HashMap<String, ciborium::value::Value> =
            ciborium::de::from_reader(read_document_cbor).expect("cannot deserialize cbor");
        assert!(document.get("a").is_some());
    }

    #[test]
    #[ignore = "test uses the `Document`. It is not a unit test"]
    fn test_document_cbor_serialization() {
        // let dashpay_cbor = json_document_to_cbor(
        //     "tests/supporting_files/contract/dashpay/dashpay-contract.json",
        //     Some(1),
        // );
        // let contract = Contract::from_cbor(&dashpay_cbor, None).unwrap();

        // let document_type = contract
        //     .document_type_for_name("profile")
        //     .expect("expected to get profile document type");
        // let document = document_type.random_document(Some(3333));

        // let document_cbor = document.to_cbor();

        // let recovered_document = Document::from_cbor(document_cbor.as_slice(), None, None)
        //     .expect("expected to get document");

        // assert_eq!(recovered_document, document);
    }

    #[test]
    #[ignore = "it is a Document structure test"]
    fn test_document_display() {
        // let dashpay_cbor = json_document_to_cbor(
        //     "tests/supporting_files/contract/dashpay/dashpay-contract.json",
        //     Some(1),
        // );
        // let contract = Contract::from_cbor(&dashpay_cbor, None).unwrap();

        // let document_type = contract
        //     .document_type_for_name("profile")
        //     .expect("expected to get profile document type");
        // let document = document_type.random_document(Some(3333));

        // let document_string = format!("{}", document);
        // assert_eq!(document_string.as_str(), "id:2vq574DjKi7ZD8kJ6dMHxT5wu6ZKD2bW5xKAyKAGW7qZ owner_id:ChTEGXJcpyknkADUC5s6tAzvPqVG7x6Lo1Nr5mFtj2mk $createdAt:1627081806.116 $updatedAt:1575820087.909 avatarUrl:W18RuyblDX7hxB38OJYt[...(894)] displayName:wvAD8Grs2h publicMessage:LdWpGtOzOkYXStdxU3G0[...(105)] ")
    }

    #[test]
    fn test_import_contract() {
        let dashpay_cbor =
            json_document_to_cbor("src/tests/payloads/contract/dashpay-contract.json", Some(1));
        let contract = Contract::from_cbor(&dashpay_cbor, None).unwrap();

        assert!(contract.documents_mutable_contract_default);
        assert!(!contract.keeps_history);
        assert!(!contract.readonly); // the contract shouldn't be readonly
        assert!(!contract.documents_keep_history_contract_default);
        assert_eq!(contract.document_types.len(), 3);
        assert!(contract.document_types.get("profile").is_some());
        assert!(
            contract
                .document_types
                .get("profile")
                .unwrap()
                .documents_mutable
        );
        assert!(contract.document_types.get("contactInfo").is_some());
        assert!(
            contract
                .document_types
                .get("contactInfo")
                .unwrap()
                .documents_mutable
        );
        assert!(contract.document_types.get("contactRequest").is_some());
        assert!(
            !contract
                .document_types
                .get("contactRequest")
                .unwrap()
                .documents_mutable
        );
        assert!(contract.document_types.get("non_existent_key").is_none());

        let contact_info_indices = &contract.document_types.get("contactInfo").unwrap().indices;
        assert_eq!(contact_info_indices.len(), 2);
        assert!(contact_info_indices[0].unique);
        assert!(!contact_info_indices[1].unique);
        assert_eq!(contact_info_indices[0].properties.len(), 3);

        assert_eq!(contact_info_indices[0].properties[0].name, "$ownerId");
        assert_eq!(
            contact_info_indices[0].properties[1].name,
            "rootEncryptionKeyIndex"
        );
        assert_eq!(
            contact_info_indices[0].properties[2].name,
            "derivationEncryptionKeyIndex"
        );

        assert!(contact_info_indices[0].properties[0].ascending);
    }
}
