use crate::consensus::basic::data_contract::DataContractUpdateEntryKind;
use crate::consensus::state::data_contract::data_contract_update_entry_already_exists_error::DataContractUpdateEntryAlreadyExistsError;
use crate::consensus::state::data_contract::data_contract_update_entry_not_found_error::DataContractUpdateEntryNotFoundError;
use crate::consensus::state::data_contract::data_contract_update_permission_error::DataContractUpdatePermissionError;
use crate::consensus::ConsensusError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::accessors::v1::DataContractV1Getters;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::serialized_version::v1::DataContractInSerializationFormatV1;
use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::data_contract::update_values::{DataContractUpdateValues, DescriptionUpdate};
use crate::data_contract::{DataContract, DocumentName};
use crate::validation::ConsensusValidationResult;
use platform_value::Value;
use std::collections::BTreeMap;

impl DataContract {
    /// Generation 0 of the merge, which builds the contract in format version 1, the format
    /// state transitions carry at the protocol versions that select it.
    #[inline(always)]
    pub(super) fn apply_update_v0(
        &self,
        update_values: DataContractUpdateValues<'_>,
    ) -> ConsensusValidationResult<DataContractInSerializationFormat> {
        match self.merge_update_v0(update_values) {
            Ok(merged) => ConsensusValidationResult::new_with_data(merged.into()),
            Err(error) => ConsensusValidationResult::new_with_error(error),
        }
    }

    fn merge_update_v0(
        &self,
        update_values: DataContractUpdateValues<'_>,
    ) -> Result<DataContractInSerializationFormatV1, ConsensusError> {
        let contract_id = self.id();

        // A delta names its owner, as it embeds no contract to read one from
        if update_values.owner_id != self.owner_id() {
            return Err(DataContractUpdatePermissionError::new(
                contract_id,
                update_values.owner_id,
            )
            .into());
        }

        let not_found = |kind: DataContractUpdateEntryKind, name: String| {
            Err(DataContractUpdateEntryNotFoundError::new(contract_id, kind, name).into())
        };
        let already_exists = |kind: DataContractUpdateEntryKind, name: String| {
            Err(DataContractUpdateEntryAlreadyExistsError::new(contract_id, kind, name).into())
        };

        let mut document_schemas: BTreeMap<DocumentName, Value> = self
            .document_schemas()
            .into_iter()
            .map(|(name, schema)| (name, schema.clone()))
            .collect();
        for (name, schema) in update_values.updated_document_schemas {
            match document_schemas.get_mut(name) {
                Some(existing) => *existing = schema.clone(),
                None => return not_found(DataContractUpdateEntryKind::DocumentType, name.clone()),
            }
        }
        for (name, schema) in update_values.new_document_schemas {
            if document_schemas
                .insert(name.clone(), schema.clone())
                .is_some()
            {
                return already_exists(DataContractUpdateEntryKind::DocumentType, name.clone());
            }
        }

        let mut schema_defs = self.schema_defs().cloned();
        for (name, definition) in update_values.updated_schema_defs {
            match schema_defs
                .as_mut()
                .and_then(|definitions| definitions.get_mut(name))
            {
                Some(existing) => *existing = definition.clone(),
                None => return not_found(DataContractUpdateEntryKind::SchemaDef, name.clone()),
            }
        }
        for (name, definition) in update_values.new_schema_defs {
            if schema_defs
                .get_or_insert_with(BTreeMap::new)
                .insert(name.clone(), definition.clone())
                .is_some()
            {
                return already_exists(DataContractUpdateEntryKind::SchemaDef, name.clone());
            }
        }

        let mut groups = self.groups().clone();
        for (position, group) in update_values.new_groups {
            if groups.insert(*position, group.clone()).is_some() {
                return already_exists(DataContractUpdateEntryKind::Group, position.to_string());
            }
        }

        let mut tokens = self.tokens().clone();
        for (position, token) in update_values.new_tokens {
            if tokens.insert(*position, token.clone()).is_some() {
                return already_exists(DataContractUpdateEntryKind::Token, position.to_string());
            }
        }

        // The stored keywords that stay keep their order, and the added ones follow them
        let mut keywords = self.keywords().clone();
        for keyword in update_values.remove_keywords {
            match keywords.iter().position(|existing| existing == keyword) {
                Some(index) => {
                    keywords.remove(index);
                }
                None => return not_found(DataContractUpdateEntryKind::Keyword, keyword.clone()),
            }
        }
        for keyword in update_values.add_keywords {
            if keywords.contains(keyword) {
                return already_exists(DataContractUpdateEntryKind::Keyword, keyword.clone());
            }
            keywords.push(keyword.clone());
        }

        let description = match update_values.description {
            DescriptionUpdate::Keep => self.description().cloned(),
            DescriptionUpdate::Clear => None,
            DescriptionUpdate::Set(description) => Some(description.clone()),
        };

        Ok(DataContractInSerializationFormatV1 {
            id: contract_id,
            config: update_values.config.unwrap_or(self.config()).clone(),
            version: update_values.version,
            owner_id: self.owner_id(),
            schema_defs,
            document_schemas,
            created_at: self.created_at(),
            updated_at: self.updated_at(),
            created_at_block_height: self.created_at_block_height(),
            updated_at_block_height: self.updated_at_block_height(),
            created_at_epoch: self.created_at_epoch(),
            updated_at_epoch: self.updated_at_epoch(),
            groups,
            tokens,
            keywords,
            description,
        })
    }
}
