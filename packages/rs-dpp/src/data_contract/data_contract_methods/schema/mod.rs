mod v0;
use crate::data_contract::JsonValue;
use crate::prelude::DataContract;
use crate::version::PlatformVersion;
use crate::ProtocolError;
pub use v0::*;

impl DataContractSchemaMethodsV0 for DataContract {
    fn document_json_schema_ref(&self, doc_type: &str) -> Result<String, ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.document_json_schema_ref(doc_type),
        }
    }

    fn document_json_schema(&self, doc_type: &str) -> Result<&JsonValue, ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.document_json_schema(doc_type),
        }
    }

    fn set_document_json_schema(
        &mut self,
        doc_type: String,
        schema: JsonValue,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.set_document_json_schema(doc_type, schema, platform_version),
        }
    }
}
