use super::contract::Contract;
use super::errors::contract::ContractError;
use super::{document_type::DocumentType, DataContract};
use std::collections::BTreeMap;

pub enum DriveEncoding {
    DriveCbor,
    DriveProtobuf,
}

// AntiCorruption layer - the goal is to maintain the compatibility with rs-drive, despite changing
// the implementation details
pub trait DriveContractExt {
    // setters/getters
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

    // methods
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

    fn root_path(&self) -> [&[u8]; 2];
    fn documents_path(&self) -> [&[u8]; 3];
    fn document_type_path<'a>(&'a self, document_type_name: &'a str) -> [&'a [u8]; 4];
    fn documents_primary_key_path<'a>(&'a self, document_type_name: &'a str) -> [&'a [u8]; 5];
    fn documents_with_history_primary_key_path<'a>(
        &'a self,
        document_type_name: &'a str,
        id: &'a [u8],
    ) -> [&'a [u8]; 6];

    fn document_type_for_name(
        &self,
        document_type_name: &str,
    ) -> Result<&DocumentType, ContractError>;
}

impl DriveContractExt for DataContract {
    fn id(&self) -> &[u8; 32] {
        &self.id.buffer
    }
    fn document_types(&self) -> &BTreeMap<String, DocumentType> {
        &self.contract.document_types
    }
    fn document_types_mut(&mut self) -> &mut BTreeMap<String, DocumentType> {
        &mut self.contract.document_types
    }
    fn set_document_types(&mut self, document_types: BTreeMap<String, DocumentType>) {
        self.contract.document_types = document_types
    }

    fn keeps_history(&self) -> bool {
        self.contract.keeps_history
    }

    fn set_keeps_history(&mut self, value: bool) {
        self.contract.keeps_history = value
    }

    fn readonly(&self) -> bool {
        self.contract.readonly
    }
    fn set_readonly(&mut self, is_read_only: bool) {
        self.contract.readonly = is_read_only;
    }

    fn documents_keep_history_contract_default(&self) -> bool {
        self.contract.documents_keep_history_contract_default
    }
    fn set_documents_keep_history_contract_default(&mut self, value: bool) {
        self.contract.documents_keep_history_contract_default = value;
    }

    fn documents_mutable_contract_default(&self) -> bool {
        self.contract.documents_mutable_contract_default
    }
    fn set_documents_mutable_contract_default(&mut self, value: bool) {
        self.contract.documents_mutable_contract_default = value
    }

    fn deserialize(
        serialized_contract: &[u8],
        contract_id: Option<[u8; 32]>,
        encoding: DriveEncoding,
    ) -> Result<Self, ContractError>
    where
        Self: Sized,
    {
        Ok(DataContract {
            contract: Contract::deserialize(serialized_contract, contract_id, encoding)?,
            ..Default::default()
        })
    }

    fn from_cbor(contract_cbor: &[u8], contract_id: Option<[u8; 32]>) -> Result<Self, ContractError>
    where
        Self: Sized,
    {
        let mut data_contract =
            DataContract::from_cbor(contract_cbor).expect("data contract should be created");

        data_contract.contract = Contract::from_cbor(contract_cbor, contract_id)?;
        Ok(data_contract)
    }

    fn root_path(&self) -> [&[u8]; 2] {
        self.contract.root_path()
    }
    fn documents_path(&self) -> [&[u8]; 3] {
        self.contract.documents_path()
    }
    fn document_type_path<'a>(&'a self, document_type_name: &'a str) -> [&'a [u8]; 4] {
        self.contract.document_type_path(document_type_name)
    }
    fn documents_primary_key_path<'a>(&'a self, document_type_name: &'a str) -> [&'a [u8]; 5] {
        self.contract.documents_primary_key_path(document_type_name)
    }
    fn documents_with_history_primary_key_path<'a>(
        &'a self,
        document_type_name: &'a str,
        id: &'a [u8],
    ) -> [&'a [u8]; 6] {
        self.contract
            .documents_with_history_primary_key_path(document_type_name, id)
    }

    fn document_type_for_name(
        &self,
        document_type_name: &str,
    ) -> Result<&DocumentType, ContractError> {
        self.contract.document_type_for_name(document_type_name)
    }
}
