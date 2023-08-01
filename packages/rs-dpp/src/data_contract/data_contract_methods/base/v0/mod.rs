use crate::data_contract::document_type::{DocumentTypeMutRef, DocumentTypeRef};
use crate::ProtocolError;

pub trait DataContractBaseMethodsV0 {
    /// Increments version of Data Contract
    fn increment_version(&mut self);
    /// Returns true if document type is defined
    fn is_document_defined(&self, document_type_name: &str) -> bool;
    // TODO: Are these two methods necessary?
    fn optional_document_type_for_name(&self, document_type_name: &str) -> Option<DocumentTypeRef>;
    fn optional_document_type_mut_for_name(
        &mut self,
        document_type_name: &str,
    ) -> Option<DocumentTypeMutRef>;
    fn document_type_for_name(
        &self,
        document_type_name: &str,
    ) -> Result<DocumentTypeRef, ProtocolError>;
    fn has_document_type_for_name(&self, document_type_name: &str) -> bool;
}
