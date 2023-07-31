use crate::data_contract::JsonSchema;
use crate::version::PlatformVersion;
use crate::ProtocolError;

pub trait DataContractDocumentSchemaMethodsV0 {
    fn get_document_schema_ref(&self, doc_type: &str) -> Result<String, ProtocolError>;
    fn get_document_schema(&self, doc_type: &str) -> Result<&JsonSchema, ProtocolError>;
    fn set_document_schema(
        &mut self,
        doc_type: String,
        schema: JsonSchema,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError>;
}
