use crate::data_contract::{DefinitionName, DocumentName, JsonSchema};
use std::collections::BTreeMap;

pub trait DataContractSchemaMethodsV0 {
    fn set_document_schemas(
        &self,
        schemas: BTreeMap<DocumentName, JsonSchema>,
        defs: Option<BTreeMap<DefinitionName, JsonSchema>>,
    );
    fn document_schemas(&self) -> &BTreeMap<DocumentName, JsonSchema>;
    fn schema_defs(&self) -> &Option<BTreeMap<DefinitionName, JsonSchema>>;
    fn set_schema_defs(&self, defs: Option<BTreeMap<DefinitionName, JsonSchema>>);
}
