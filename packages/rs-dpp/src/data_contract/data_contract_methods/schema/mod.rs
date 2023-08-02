mod v0;

use crate::data_contract::{DefinitionName, DocumentName, JsonSchema};
use crate::prelude::DataContract;
use std::collections::BTreeMap;
pub use v0::*;

impl DataContractSchemaMethodsV0 for DataContract {
    fn set_document_schemas(
        &self,
        schemas: BTreeMap<DocumentName, JsonSchema>,
        defs: Option<BTreeMap<DefinitionName, JsonSchema>>,
    ) {
        match self {
            DataContract::V0(v0) => v0.set_document_schemas(schemas, defs),
        }
    }

    fn document_schemas(&self) -> BTreeMap<DocumentName, JsonSchema> {
        match self {
            DataContract::V0(v0) => v0.document_schemas(),
        }
    }

    fn schema_defs(&self) -> Option<BTreeMap<DefinitionName, JsonSchema>> {
        match self {
            DataContract::V0(v0) => v0.schema_defs(),
        }
    }

    fn set_schema_defs(&self, defs: Option<BTreeMap<DefinitionName, JsonSchema>>) {
        match self {
            DataContract::V0(v0) => v0.set_schema_defs(defs),
        }
    }
}
