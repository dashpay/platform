use crate::data_contract::{DefinitionName, DocumentName};
use platform_value::Value;
use std::collections::BTreeMap;

pub trait DataContractSchemaMethodsV0 {
    fn set_document_schemas(
        &self,
        schemas: BTreeMap<DocumentName, Value>,
        defs: Option<BTreeMap<DefinitionName, Value>>,
    );
    fn document_schemas(&self) -> BTreeMap<DocumentName, &Value>;
    fn schema_defs(&self) -> &Option<BTreeMap<DefinitionName, Value>>;
    fn set_schema_defs(&self, defs: Option<BTreeMap<DefinitionName, Value>>);
}
