use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::errors::DataContractError;
use crate::data_contract::DataContract;
use crate::document::document_methods::DocumentGetRawForDocumentTypeV0;
use crate::document::DocumentV0Getters;
use crate::version::PlatformVersion;
use crate::ProtocolError;

pub trait DocumentGetRawForContractV0: DocumentV0Getters + DocumentGetRawForDocumentTypeV0 {
    /// Return a value given the path to its key and the document type for a contract.
    fn get_raw_for_contract_v0(
        &self,
        key: &str,
        document_type_name: &str,
        contract: &DataContract,
        owner_id: Option<[u8; 32]>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<u8>>, ProtocolError> {
        let document_type = contract.document_types().get(document_type_name).ok_or({
            ProtocolError::DataContractError(DataContractError::DocumentTypeNotFound(
                "document type should exist for name".to_string(),
            ))
        })?;
        self.get_raw_for_document_type_v0(key, document_type.as_ref(), owner_id, platform_version)
    }
}
