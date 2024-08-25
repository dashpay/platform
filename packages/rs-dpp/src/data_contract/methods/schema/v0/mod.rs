use crate::data_contract::{DefinitionName, DocumentName};
use crate::validation::operations::ProtocolValidationOperation;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

pub trait DataContractSchemaMethodsV0 {
    fn set_document_schemas(
        &mut self,
        schemas: BTreeMap<DocumentName, Value>,
        defs: Option<BTreeMap<DefinitionName, Value>>,
        full_validation: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError>;

    fn set_document_schema(
        &mut self,
        name: &str,
        schema: Value,
        full_validation: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError>;

    fn document_schemas(&self) -> BTreeMap<DocumentName, &Value>;
    fn schema_defs(&self) -> Option<&BTreeMap<DefinitionName, Value>>;
    fn set_schema_defs(
        &mut self,
        defs: Option<BTreeMap<DefinitionName, Value>>,
        full_validation: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError>;
}
