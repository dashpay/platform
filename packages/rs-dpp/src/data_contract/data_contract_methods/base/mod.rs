mod v0;
pub use v0::*;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::prelude::DataContract;
use crate::ProtocolError;

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

    fn optional_document_type_for_name<'a>(&self, document_type_name: &str) -> Option<DocumentTypeRef<'a>> {
        match self {
            DataContract::V0(v0) => v0.optional_document_type_for_name(document_type_name),
        }
    }

    fn document_type_for_name<'a>(&self, document_type_name: &str) -> Result<DocumentTypeRef<'a>, ProtocolError> {
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