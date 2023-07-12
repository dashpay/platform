use crate::data_contract::DataContract;
use crate::data_contract::errors::DataContractError;
use crate::document::DocumentV0Getters;
use crate::ProtocolError;

pub trait DocumentGetRawForContractV0 : DocumentV0Getters {
    /// Return a value given the path to its key and the document type for a contract.
    fn get_raw_for_contract_v0<'a>(
        &'a self,
        key: &str,
        document_type_name: &str,
        contract: &DataContract,
        owner_id: Option<[u8; 32]>,
    ) -> Result<Option<Vec<u8>>, ProtocolError> {
        let document_type = contract.document_types().get(document_type_name).ok_or({
            ProtocolError::DataContractError(DataContractError::DocumentTypeNotFound(
                "document type should exist for name",
            ))
        })?;
        self.get_raw_for_document_type(key, document_type, owner_id)
    }
}