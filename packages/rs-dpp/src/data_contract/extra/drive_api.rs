use std::collections::BTreeMap;

use crate::data_contract::DataContract;
use crate::ProtocolError;

use super::document_type::DocumentType;
use super::errors::ContractError;
use super::mutability;
use super::root_tree::RootTree;

pub enum DriveEncoding {
    DriveCbor,
    DriveProtobuf,
}

/// The traits provides method specific for RS-Drive
pub trait DriveContractExt {
    fn id(&self) -> &[u8; 32];

    fn document_types(&self) -> &BTreeMap<String, DocumentType>;
    fn document_types_mut(&mut self) -> &mut BTreeMap<String, DocumentType>;
    fn set_document_types(&mut self, document_types: BTreeMap<String, DocumentType>);

    fn keeps_history(&self) -> bool;
    fn set_keeps_history(&mut self, value: bool);

    fn readonly(&self) -> bool;
    fn set_readonly(&mut self, is_read_only: bool);

    fn documents_keep_history_contract_default(&self) -> bool;
    fn set_documents_keep_history_contract_default(&mut self, value: bool);

    fn documents_mutable_contract_default(&self) -> bool;
    fn set_documents_mutable_contract_default(&mut self, value: bool);

    fn deserialize(
        serialized_contract: &[u8],
        contract_id: Option<[u8; 32]>,
        encoding: DriveEncoding,
    ) -> Result<Self, ContractError>
    where
        Self: Sized;

    fn from_cbor(
        contract_cbor: &[u8],
        contract_id: Option<[u8; 32]>,
    ) -> Result<Self, ContractError>
    where
        Self: Sized;

    fn to_cbor(&self) -> Result<Vec<u8>, ContractError>;

    fn document_type_for_name(
        &self,
        document_type_name: &str,
    ) -> Result<&DocumentType, ContractError>;

    fn root_path(&self) -> [&[u8]; 2];
    fn documents_path(&self) -> [&[u8]; 3];
    fn document_type_path<'a>(&'a self, document_type_name: &'a str) -> [&'a [u8]; 4];
    fn documents_primary_key_path<'a>(&'a self, document_type_name: &'a str) -> [&'a [u8]; 5];
    fn documents_with_history_primary_key_path<'a>(
        &'a self,
        document_type_name: &'a str,
        id: &'a [u8],
    ) -> [&'a [u8]; 6];
}

impl DriveContractExt for DataContract {
    fn id(&self) -> &[u8; 32] {
        &self.id.buffer
    }
    fn document_types(&self) -> &BTreeMap<String, DocumentType> {
        &self.document_types
    }
    fn document_types_mut(&mut self) -> &mut BTreeMap<String, DocumentType> {
        &mut self.document_types
    }
    fn set_document_types(&mut self, document_types: BTreeMap<String, DocumentType>) {
        self.document_types = document_types
    }

    fn keeps_history(&self) -> bool {
        self.config.keeps_history
    }

    fn set_keeps_history(&mut self, value: bool) {
        self.config.keeps_history = value
    }

    fn readonly(&self) -> bool {
        self.config.readonly
    }
    fn set_readonly(&mut self, is_read_only: bool) {
        self.config.readonly = is_read_only;
    }

    fn documents_keep_history_contract_default(&self) -> bool {
        self.config.documents_keep_history_contract_default
    }
    fn set_documents_keep_history_contract_default(&mut self, value: bool) {
        self.config.documents_keep_history_contract_default = value;
    }

    fn documents_mutable_contract_default(&self) -> bool {
        self.config.documents_mutable_contract_default
    }
    fn set_documents_mutable_contract_default(&mut self, value: bool) {
        self.config.documents_mutable_contract_default = value
    }

    fn deserialize(
        serialized_contract: &[u8],
        contract_id: Option<[u8; 32]>,
        encoding: DriveEncoding,
    ) -> Result<Self, ContractError>
    where
        Self: Sized,
    {
        let mut data_contract = match encoding {
            DriveEncoding::DriveCbor => DataContract::from_cbor(serialized_contract)?,
            DriveEncoding::DriveProtobuf => {
                todo!()
            }
        };
        if let Some(id) = contract_id {
            data_contract.id.buffer = id
        }
        Ok(data_contract)
    }

    fn from_cbor(contract_cbor: &[u8], contract_id: Option<[u8; 32]>) -> Result<Self, ContractError>
    where
        Self: Sized,
    {
        let mut data_contract = DataContract::from_cbor(contract_cbor)?;
        if let Some(id) = contract_id {
            data_contract.id.buffer = id
        }

        Ok(data_contract)
    }

    /// `to_cbor` overloads the original method from [`DataContract`] and adds the properties
    /// from [`super::Mutability`].
    fn to_cbor(&self) -> Result<Vec<u8>, ContractError> {
        let mut buf = self.protocol_version().to_le_bytes().to_vec();

        let mut contract_cbor_map = self.to_cbor_canonical_map()?;

        contract_cbor_map.insert(mutability::property::READONLY, self.readonly());
        contract_cbor_map.insert(mutability::property::KEEPS_HISTORY, self.keeps_history());
        contract_cbor_map.insert(
            mutability::property::DOCUMENTS_KEEP_HISTORY_CONTRACT_DEFAULT,
            self.documents_keep_history_contract_default(),
        );
        contract_cbor_map.insert(
            mutability::property::DOCUMENTS_MUTABLE_CONTRACT_DEFAULT,
            self.documents_mutable_contract_default(),
        );

        let mut contract_buf = contract_cbor_map
            .to_bytes()
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;

        buf.append(&mut contract_buf);
        Ok(buf)
    }

    fn document_type_for_name(
        &self,
        document_type_name: &str,
    ) -> Result<&DocumentType, ContractError> {
        self.document_types.get(document_type_name).ok_or({
            ContractError::DocumentTypeNotFound("can not get document type from contract")
        })
    }

    fn root_path(&self) -> [&[u8]; 2] {
        [
            Into::<&[u8; 1]>::into(RootTree::ContractDocuments),
            self.id().as_bytes(),
        ]
    }

    fn documents_path(&self) -> [&[u8]; 3] {
        [
            Into::<&[u8; 1]>::into(RootTree::ContractDocuments),
            self.id().as_bytes(),
            &[1],
        ]
    }

    fn document_type_path<'a>(&'a self, document_type_name: &'a str) -> [&'a [u8]; 4] {
        [
            Into::<&[u8; 1]>::into(RootTree::ContractDocuments),
            self.id.as_bytes(),
            &[1],
            document_type_name.as_bytes(),
        ]
    }

    fn documents_primary_key_path<'a>(&'a self, document_type_name: &'a str) -> [&'a [u8]; 5] {
        [
            Into::<&[u8; 1]>::into(RootTree::ContractDocuments),
            self.id.as_bytes(),
            &[1],
            document_type_name.as_bytes(),
            &[0],
        ]
    }

    fn documents_with_history_primary_key_path<'a>(
        &'a self,
        document_type_name: &'a str,
        id: &'a [u8],
    ) -> [&'a [u8]; 6] {
        [
            Into::<&[u8; 1]>::into(RootTree::ContractDocuments),
            self.id().as_bytes(),
            &[1],
            document_type_name.as_bytes(),
            &[0],
            id,
        ]
    }
}

#[cfg(test)]
mod test {
    use mutability::ContractConfig;

    use crate::{
        data_contract::extra::common::json_document_to_cbor, data_contract::DataContract,
        util::json_schema::JsonSchemaExt,
    };

    use super::*;

    type IndexName = &'static str;
    type IsIndexUnique = bool;
    type IndexPropertyName = &'static str;
    type IndexOrderDirection = &'static str;
    type IndexProperties = &'static [(IndexPropertyName, IndexOrderDirection)];

    #[derive(Default)]
    struct ExpectedDocumentsData {
        document_name: &'static str,
        required_properties: &'static [&'static str],
        indexes: &'static [(IndexName, IsIndexUnique, IndexProperties)],
    }

    fn expected_documents() -> Vec<ExpectedDocumentsData> {
        vec![
            ExpectedDocumentsData {
                document_name: "niceDocument",
                required_properties: &["$cratedAt"],
                ..Default::default()
            },
            ExpectedDocumentsData {
                document_name: "prettyDocument",
                required_properties: &["lastName", "$cratedAt"],
                ..Default::default()
            },
            ExpectedDocumentsData {
                document_name: "indexedDocument",
                required_properties: &["firstName", "$createdAt", "$updatedAt", "lastName"],
                indexes: &[
                    (
                        "index1",
                        true,
                        &[("$ownerId", "asc"), ("firstName", "desc")],
                    ),
                    (
                        "index2",
                        true,
                        &[("$ownerId", "asc"), ("$lastName", "desc")],
                    ),
                    ("index3", false, &[("lastName", "asc")]),
                    (
                        "index4",
                        false,
                        &[("$createdAt", "asc"), ("$updatedAt", "asc")],
                    ),
                    ("index5", false, &[("$updatedAt", "asc")]),
                    ("index6", false, &[("$createdAt", "asc")]),
                ],
            },
            ExpectedDocumentsData {
                document_name: "noTimeDocument",
                ..Default::default()
            },
            ExpectedDocumentsData {
                document_name: "uniqueDates",
                required_properties: &["firstName", "$createdAt", "$updatedAt"],
                indexes: &[
                    (
                        "index1",
                        true,
                        &[("$createdAt", "asc"), ("$updatedAt", "asc")],
                    ),
                    ("index2", false, &[("$updatedAt", "asc")]),
                ],
            },
            ExpectedDocumentsData {
                document_name: "withByteArrays",
                indexes: &[("index1", false, &[("byteArrayField", "asc")])],
                required_properties: &["byteArrayField"],
            },
            ExpectedDocumentsData {
                document_name: "optionalUniqueIndexedDocument",
                indexes: &[
                    ("index1", false, &[("firstName", "desc")]),
                    (
                        "index2",
                        true,
                        &[
                            ("$ownerId", "asc"),
                            ("firstName", "asc"),
                            ("lastName", "asc"),
                        ],
                    ),
                    ("index3", true, &[("country", "asc"), ("city", "asc")]),
                ],
                required_properties: &["firstName", "lastName"],
            },
        ]
    }

    #[test]
    fn deserialize_from_cbor_with_contract_inner() {
        let cbor_bytes = std::fs::read("src/tests/payloads/contract/contract.bin").unwrap();
        let expect_id_base58 = "2CAHCVpYLMw8uheSydQ4CTNrPYkFwdPmRVqYgWAeN9pL";
        let expect_owner_id_base58 = "6C7w6XJxXWbb12iJj2aLcQU3T9wn8CZ8pimiWXGfWb55";
        let expect_id = bs58::decode(expect_id_base58).into_vec().unwrap();
        let expect_owner_id = bs58::decode(expect_owner_id_base58).into_vec().unwrap();

        let data_contract =
            DataContract::from_cbor(&cbor_bytes).expect("contract should be deserialized");

        assert_eq!(1, data_contract.protocol_version());
        assert_eq!(expect_id, data_contract.id().as_bytes());
        assert_eq!(expect_owner_id, data_contract.owner_id().as_bytes());

        assert_eq!(7, data_contract.documents().len());
        assert_eq!(7, data_contract.document_types().len());
        assert_eq!(1, data_contract.version());
        assert_eq!(
            "https://schema.dash.org/dpp-0-4-0/meta/data-contract",
            data_contract.schema()
        );

        for expect in expected_documents() {
            assert!(
                data_contract.is_document_defined(expect.document_name),
                "'{}' document should be defined",
                expect.document_name
            );
            assert!(
                data_contract
                    .document_type_for_name(expect.document_name)
                    .is_ok(),
                "'{}' document type should be defined",
                expect.document_name
            );

            // document_type  - Drive API
            let document_type = data_contract
                .document_type_for_name(expect.document_name)
                .unwrap();
            assert_eq!(expect.indexes.len(), document_type.indices.len());

            // document type - JS API
            let document = data_contract
                .get_document_schema(expect.document_name)
                .unwrap();

            let document_indices = document.get_indices().unwrap_or_default();
            assert_eq!(expect.indexes.len(), document_indices.len());
        }
    }

    #[test]
    fn should_drive_api_methods_contain_contract_data() {
        let dashpay_cbor =
            json_document_to_cbor("src/tests/payloads/contract/dashpay-contract.json", Some(1));
        let contract = DataContract::from_cbor(&dashpay_cbor).unwrap();

        assert!(contract.documents_mutable_contract_default());
        assert!(!contract.keeps_history());
        assert!(!contract.readonly()); // the contract shouldn't be readonly
        assert!(!contract.documents_keep_history_contract_default());
        assert_eq!(contract.document_types().len(), 3);
        assert!(contract.document_types().get("profile").is_some());
        assert!(
            contract
                .document_types()
                .get("profile")
                .unwrap()
                .documents_mutable
        );
        assert!(contract.document_types().get("contactInfo").is_some());
        assert!(
            contract
                .document_types()
                .get("contactInfo")
                .unwrap()
                .documents_mutable
        );
        assert!(contract.document_types().get("contactRequest").is_some());
        assert!(
            !contract
                .document_types()
                .get("contactRequest")
                .unwrap()
                .documents_mutable
        );
        assert!(contract.document_types().get("non_existent_key").is_none());

        let contact_info_indices = &contract
            .document_types()
            .get("contactInfo")
            .unwrap()
            .indices;
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

    #[test]
    fn mutability_properties_should_be_stored_and_restored_during_serialization() {
        let dashpay_cbor =
            json_document_to_cbor("src/tests/payloads/contract/dashpay-contract.json", Some(1));
        let mut contract = DataContract::from_cbor(&dashpay_cbor).unwrap();

        assert!(!contract.readonly());
        assert!(!contract.keeps_history());
        assert!(contract.documents_mutable_contract_default());
        assert!(!contract.documents_keep_history_contract_default());

        contract.set_readonly(true);
        contract.set_keeps_history(true);
        contract.set_documents_mutable_contract_default(false);
        contract.set_documents_keep_history_contract_default(true);

        let contract_cbor =
            DriveContractExt::to_cbor(&contract).expect("serialization shouldn't fail");
        let deserialized_contract =
            DataContract::from_cbor(&contract_cbor).expect("deserialization shouldn't fail");

        assert!(matches!(
            deserialized_contract.config,
            ContractConfig {
                readonly: true,
                keeps_history: true,
                documents_mutable_contract_default: false,
                documents_keep_history_contract_default: true,
            }
        ));
    }
}
