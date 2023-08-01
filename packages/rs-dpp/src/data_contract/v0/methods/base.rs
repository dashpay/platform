use crate::data_contract::base::DataContractBaseMethodsV0;
use crate::data_contract::data_contract::DataContractV0;
use crate::data_contract::document_type::{DocumentTypeMutRef, DocumentTypeRef};
use crate::data_contract::errors::DataContractError;
use crate::ProtocolError;

impl DataContractBaseMethodsV0 for DataContractV0 {
    /// Increments version of Data Contract
    fn increment_version(&mut self) {
        self.version += 1;
    }

    /// Returns true if document type is defined
    fn is_document_defined(&self, document_type_name: &str) -> bool {
        self.document_types.get(document_type_name).is_some()
    }

    fn optional_document_type_for_name(&self, document_type_name: &str) -> Option<DocumentTypeRef> {
        self.document_types
            .get(document_type_name)
            .map(|document_type| document_type.as_ref())
    }

    fn optional_document_type_mut_for_name(
        &mut self,
        document_type_name: &str,
    ) -> Option<DocumentTypeMutRef> {
        self.document_types
            .get_mut(document_type_name)
            .map(|document_type| document_type.as_mut_ref())
    }

    fn document_type_for_name(
        &self,
        document_type_name: &str,
    ) -> Result<DocumentTypeRef, ProtocolError> {
        Ok(self
            .document_types
            .get(document_type_name)
            .ok_or({
                ProtocolError::DataContractError(DataContractError::DocumentTypeNotFound(
                    "can not get document type from contract",
                ))
            })?
            .as_ref())
    }

    fn has_document_type_for_name(&self, document_type_name: &str) -> bool {
        self.document_types.get(document_type_name).is_some()
    }
}
