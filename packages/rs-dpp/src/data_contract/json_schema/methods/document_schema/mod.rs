mod v0;
use crate::data_contract::JsonSchema;
use crate::prelude::DataContract;
use crate::version::PlatformVersion;
use crate::ProtocolError;
pub use v0::*;

impl DataContractDocumentSchemaMethodsV0 for DataContract {
    fn get_document_schema_ref(&self, doc_type: &str) -> Result<String, ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.get_document_schema_ref(doc_type),
        }
    }

    fn get_document_schema(&self, doc_type: &str) -> Result<&JsonSchema, ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.get_document_schema(doc_type),
        }
    }

    fn set_document_schema(
        &mut self,
        doc_type: String,
        schema: JsonSchema,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.set_document_schema(doc_type, schema, platform_version),
        }
    }
}
