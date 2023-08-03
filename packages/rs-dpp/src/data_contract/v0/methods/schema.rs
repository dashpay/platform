use crate::data_contract::data_contract_methods::schema::DataContractSchemaMethodsV0;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::v0::DataContractV0;
use crate::data_contract::{DefinitionName, DocumentName};
use platform_value::Value;
use std::collections::BTreeMap;

impl DataContractSchemaMethodsV0 for DataContractV0 {
    fn set_document_schemas(
        &self,
        schemas: BTreeMap<DocumentName, Value>,
        defs: Option<BTreeMap<DefinitionName, Value>>,
    ) {
        todo!()
    }

    fn document_schemas(&self) -> BTreeMap<DocumentName, &Value> {
        self.document_types
            .iter()
            .map(|(name, r#type)| (name.to_owned(), r#type.schema()))
            .collect()
    }

    fn schema_defs(&self) -> &Option<BTreeMap<DefinitionName, Value>> {
        &self.schema_defs
    }

    fn set_schema_defs(&self, defs: Option<BTreeMap<DefinitionName, Value>>) {
        todo!()
    }
}
