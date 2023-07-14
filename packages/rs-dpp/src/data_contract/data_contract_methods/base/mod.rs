mod v0;
use crate::data_contract::document_type::{DocumentTypeMutRef, DocumentTypeRef};
use crate::prelude::DataContract;
use crate::ProtocolError;
pub use v0::*;

impl DataContractBaseMethodsV0 for DataContract {
    fn increment_version(&mut self) {
        match self {
            DataContract::V0(v0) => v0.increment_version(),
        }
    }

    fn is_document_defined(&self, document_type_name: &str) -> bool {
        match self {
            DataContract::V0(v0) => v0.is_document_defined(document_type_name),
        }
    }

    fn optional_document_type_for_name(&self, document_type_name: &str) -> Option<DocumentTypeRef> {
        match self {
            DataContract::V0(v0) => v0.optional_document_type_for_name(document_type_name),
        }
    }

    fn optional_document_type_mut_for_name(
        &mut self,
        document_type_name: &str,
    ) -> Option<DocumentTypeMutRef> {
        match self {
            DataContract::V0(v0) => v0.optional_document_type_mut_for_name(document_type_name),
        }
    }

    fn document_type_for_name(
        &self,
        document_type_name: &str,
    ) -> Result<DocumentTypeRef, ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.document_type_for_name(document_type_name),
        }
    }

    fn has_document_type_for_name(&self, document_type_name: &str) -> bool {
        match self {
            DataContract::V0(v0) => v0.has_document_type_for_name(document_type_name),
        }
    }
}
