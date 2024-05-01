mod v0;
pub use v0::*;

use crate::data_contract::{DefinitionName, DocumentName};
use crate::prelude::DataContract;
use crate::validation::operations::ProtocolValidationOperation;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl DataContractSchemaMethodsV0 for DataContract {
    fn set_document_schemas(
        &mut self,
        schemas: BTreeMap<DocumentName, Value>,
        defs: Option<BTreeMap<DefinitionName, Value>>,
        validate: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.set_document_schemas(
                schemas,
                defs,
                validate,
                validation_operations,
                platform_version,
            ),
        }
    }

    fn set_document_schema(
        &mut self,
        name: &str,
        schema: Value,
        validate: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.set_document_schema(
                name,
                schema,
                validate,
                validation_operations,
                platform_version,
            ),
        }
    }

    fn document_schemas(&self) -> BTreeMap<DocumentName, &Value> {
        match self {
            DataContract::V0(v0) => v0.document_schemas(),
        }
    }

    fn schema_defs(&self) -> Option<&BTreeMap<DefinitionName, Value>> {
        match self {
            DataContract::V0(v0) => v0.schema_defs(),
        }
    }

    fn set_schema_defs(
        &mut self,
        defs: Option<BTreeMap<DefinitionName, Value>>,
        validate: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        match self {
            DataContract::V0(v0) => {
                v0.set_schema_defs(defs, validate, validation_operations, platform_version)
            }
        }
    }
}
