use crate::drive::RootTree;
use dpp::data_contract::document_type::DocumentType;
use dpp::data_contract::DataContract;

trait ContractPaths {
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

impl ContractPaths for DataContract {
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
