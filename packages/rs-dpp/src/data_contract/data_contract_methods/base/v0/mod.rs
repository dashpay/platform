use crate::data_contract::document_type::DocumentTypeRef;
use crate::ProtocolError;

pub trait DataContractBaseMethodsV0 {
    /// Increments version of Data Contract
    fn increment_version(&mut self);
    /// Returns true if document type is defined
    fn is_document_defined(&self, document_type_name: &str) -> bool;
    fn optional_document_type_for_name<'a>(
        &self,
        document_type_name: &str,
    ) -> Option<DocumentTypeRef<'a>>;
    fn document_type_for_name<'a>(
        &self,
        document_type_name: &str,
    ) -> Result<DocumentTypeRef<'a>, ProtocolError>;
    fn has_document_type_for_name(&self, document_type_name: &str) -> bool;
}
