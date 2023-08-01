
use std::collections::HashSet;
use std::collections::{BTreeMap, HashMap};

use crate::consensus::basic::data_contract::{
    DataContractHaveNewUniqueIndexError, DataContractInvalidIndexDefinitionUpdateError,
    DataContractUniqueIndicesChangedError,
};
use crate::consensus::basic::BasicError;
use crate::data_contract::document_type::{Index, IndexProperty};
use crate::util::json_schema::JsonSchemaExt;
use crate::util::json_value::JsonValueExt;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use anyhow::anyhow;
use platform_value::Value;
use platform_version::version::PlatformVersion;

type IndexName = String;
type DocumentType = String;

impl Index {
    //todo: change this to use Platform value and document types
    pub(super) fn validate_indices_are_backward_compatible_v0<'a>(
        existing_documents: impl IntoIterator<Item=(&'a DocumentType, &'a Value)>,
        new_documents: impl IntoIterator<Item=(&'a DocumentType, &'a Value)>,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let mut result = SimpleConsensusValidationResult::default();
        let new_documents_by_type: HashMap<&DocumentType, &Value> =
            new_documents.into_iter().collect();

        for (document_type, existing_schema) in existing_documents.into_iter() {
            let new_documents_schema = *new_documents_by_type.get(document_type).ok_or_else(|| {
                anyhow!(
                "the document '{}' type doesn't exist in new definitions",
                document_type
            )
            })?;
            let name_new_index_map = new_documents_schema.get_indices_map::<BTreeMap<_, _>>()?;

            let old_properties_set: HashSet<&str> = existing_schema
                .get_()?
                .as_object()
                .ok_or_else(|| {
                    anyhow!(
                    "the document '{}' properties in old schema must be an object",
                    document_type
                )
                })?
                .keys()
                .map(|x| x.as_ref())
                .collect();
            let new_properties_set: HashSet<&str> = new_documents_by_type
                .get(document_type)
                .expect("checked above")
                .get_schema_properties()?
                .as_object()
                .ok_or_else(|| {
                    anyhow!(
                    "the document '{}' properties in new schema must be an object",
                    document_type
                )
                })?
                .keys()
                .map(|x| x.as_ref())
                .collect();

            let added_properties = new_properties_set.difference(&old_properties_set);

            let existing_schema_indices = existing_schema.get_indices::<Vec<_>>().unwrap_or_default();

            let maybe_changed_unique_existing_index =
                Index::get_changed_old_unique_index(&existing_schema_indices, &name_new_index_map);
            if let Some(changed_index) = maybe_changed_unique_existing_index {
                result.add_error(BasicError::DataContractUniqueIndicesChangedError(
                    DataContractUniqueIndicesChangedError::new(
                        document_type.to_owned(),
                        changed_index.name.clone(),
                    ),
                ));
            }

            let maybe_wrongly_updated_index = Index::get_wrongly_updated_non_unique_index(
                &existing_schema_indices,
                &name_new_index_map,
                existing_schema,
                platform_version,
            )?;
            if let Some(index) = maybe_wrongly_updated_index {
                result.add_error(BasicError::DataContractInvalidIndexDefinitionUpdateError(
                    DataContractInvalidIndexDefinitionUpdateError::new(
                        document_type.to_owned(),
                        index.name.clone(),
                    ),
                ))
            }

            let maybe_new_unique_index =
                Index::get_new_unique_index(&existing_schema_indices, name_new_index_map.values())?;
            if let Some(index) = maybe_new_unique_index {
                result.add_error(BasicError::DataContractHaveNewUniqueIndexError(
                    DataContractHaveNewUniqueIndexError::new(
                        document_type.to_owned(),
                        index.name.clone(),
                    ),
                ))
            }
            let maybe_wrongly_constructed_new_index = Index::get_wrongly_constructed_new_index(
                existing_schema_indices.iter(),
                name_new_index_map.values(),
                added_properties.copied(),
                platform_version,
            )?;
            if let Some(index) = maybe_wrongly_constructed_new_index {
                result.add_error(BasicError::DataContractInvalidIndexDefinitionUpdateError(
                    DataContractInvalidIndexDefinitionUpdateError::new(
                        document_type.to_owned(),
                        index.name.clone(),
                    ),
                ))
            }
        }

        Ok(result)
    }

    // The old and UNIQUE indices cannot be modified.
// Returns the first unique index that has changed when comparing to the `new_indices`
    fn get_changed_old_unique_index<'a>(
        existing_indices: &'a [Index],
        new_indices: &'a BTreeMap<IndexName, Index>,
    ) -> Option<&'a Index> {
        existing_indices
            .iter()
            .find(|i| Index::indexes_are_not_equal(i, new_indices.get(&i.name)) && i.unique)
    }

    fn indexes_are_not_equal(index_a: &Index, index_b: Option<&Index>) -> bool {
        match index_b {
            None => true,
            Some(index) => index_a != index,
        }
    }


    fn get_new_unique_index<'a>(
        existing_schema_indices: impl IntoIterator<Item=&'a Index>,
        new_schema_indices: impl IntoIterator<Item=&'a Index>,
    ) -> Result<Option<&'a Index>, ProtocolError> {
        let existing_index_names: HashSet<&String> = existing_schema_indices
            .into_iter()
            .map(|i| &i.name)
            .collect();

        // Gather only new defined indexes
        let maybe_new_unique_index = new_schema_indices
            .into_iter()
            .filter(|i| !existing_index_names.contains(&i.name))
            .find(|i| i.unique);

        Ok(maybe_new_unique_index)
    }
}
