mod v0;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::data_contract::{DefinitionName, DocumentName};
use crate::prelude::DataContract;
use crate::serialization::PlatformSerializableWithPlatformVersion;
use crate::util::hash::hash_to_vec;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;
pub use v0::*;

impl DataContractMethodsV0 for DataContract {
    fn set_document_schemas(
        &mut self,
        schemas: BTreeMap<DocumentName, Value>,
        defs: Option<BTreeMap<DefinitionName, Value>>,
        validate: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        match self {
            DataContract::V0(v0) => {
                v0.set_document_schemas(schemas, defs, validate, platform_version)
            }
        }
    }

    fn set_document_schema(
        &mut self,
        name: &str,
        schema: Value,
        validate: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        match self {
            DataContract::V0(v0) => {
                v0.set_document_schema(name, schema, validate, platform_version)
            }
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
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.set_schema_defs(defs, validate, platform_version),
        }
    }
}
