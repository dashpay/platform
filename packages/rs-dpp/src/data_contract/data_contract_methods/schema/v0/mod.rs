use crate::data_contract::JsonValue;
use crate::version::PlatformVersion;
use crate::ProtocolError;

pub trait DataContractSchemaMethodsV0 {
    // TODO: Remove json?
    fn json_schema(&self) -> Result<&JsonValue, ProtocolError>;

    fn set_json_schema(
        &mut self,
        schema: JsonValue,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError>;

    fn document_json_schema_ref(&self, doc_type: &str) -> Result<String, ProtocolError>;

    fn document_json_schema(&self, doc_type: &str) -> Result<&JsonValue, ProtocolError>;

    fn set_document_json_schema(
        &mut self,
        doc_type: String,
        schema: JsonValue,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError>;
}
