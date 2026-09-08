use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::accessors::v1::DataContractV1Getters;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::update_values::DescriptionUpdate;
use crate::data_contract::DataContract;
use crate::state_transition::data_contract_update_transition::DataContractUpdateTransitionV1;

impl DataContractUpdateTransitionV1 {
    /// Checks that `updated_contract` is a contract this delta could have
    /// produced, and names the first field that says otherwise.
    ///
    /// A full-contract update is verified by comparing the proven contract
    /// with the embedded one field by field. A delta has no embedded
    /// contract, so verification checks every change it carries against
    /// the proven contract instead: identity, version, config when
    /// supplied, every new and updated schema and definition, every new
    /// group and token, added keywords present, removed keywords absent,
    /// and the description.
    ///
    /// It says nothing about what the delta did not touch: the proven
    /// contract is bound to the signed root hash, so untouched document
    /// types, definitions, groups, tokens and config are whatever state
    /// holds, not something this check re-derives.
    pub fn first_mismatch(&self, updated_contract: &DataContract) -> Option<String> {
        if updated_contract.id() != self.data_contract_id {
            return Some("contract id differs".to_string());
        }
        if updated_contract.owner_id() != self.owner_id {
            return Some("owner id differs".to_string());
        }
        if updated_contract.version() != self.version {
            return Some(format!(
                "contract version is {}, the update produces {}",
                updated_contract.version(),
                self.version
            ));
        }
        if let Some(config) = &self.config {
            if updated_contract.config() != config {
                return Some("config differs".to_string());
            }
        }

        let document_schemas = updated_contract.document_schemas();
        for (name, schema) in self
            .new_document_schemas
            .iter()
            .chain(self.updated_document_schemas.iter())
        {
            if document_schemas.get(name) != Some(&schema) {
                return Some(format!("document type '{name}' differs"));
            }
        }

        let schema_defs = updated_contract.schema_defs();
        for (name, definition) in self
            .new_schema_defs
            .iter()
            .chain(self.updated_schema_defs.iter())
        {
            if schema_defs.and_then(|definitions| definitions.get(name)) != Some(definition) {
                return Some(format!("schema definition '{name}' differs"));
            }
        }

        for (position, group) in &self.new_groups {
            if updated_contract.groups().get(position) != Some(group) {
                return Some(format!("group at position {position} differs"));
            }
        }

        for (position, token) in &self.new_tokens {
            if updated_contract.tokens().get(position) != Some(token) {
                return Some(format!("token at position {position} differs"));
            }
        }

        let keywords = updated_contract.keywords();
        if let Some(keyword) = self
            .add_keywords
            .iter()
            .find(|keyword| !keywords.contains(*keyword))
        {
            return Some(format!("added keyword '{keyword}' is missing"));
        }
        if let Some(keyword) = self
            .remove_keywords
            .iter()
            .find(|keyword| keywords.contains(*keyword))
        {
            return Some(format!("removed keyword '{keyword}' is still present"));
        }

        match &self.description {
            DescriptionUpdate::Keep => {}
            DescriptionUpdate::Clear => {
                if updated_contract.description().is_some() {
                    return Some("description was not cleared".to_string());
                }
            }
            DescriptionUpdate::Set(description) => {
                if updated_contract.description() != Some(description) {
                    return Some("description differs".to_string());
                }
            }
        }

        None
    }
}
