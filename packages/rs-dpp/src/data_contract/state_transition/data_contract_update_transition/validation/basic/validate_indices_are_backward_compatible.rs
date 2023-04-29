use std::collections::HashSet;
use std::collections::{BTreeMap, HashMap};

use crate::consensus::basic::data_contract::{
    DataContractHaveNewUniqueIndexError, DataContractInvalidIndexDefinitionUpdateError,
    DataContractUniqueIndicesChangedError,
};
use crate::consensus::basic::BasicError;
use crate::data_contract::document_type::IndexProperty;
use crate::util::json_schema::Index;
use crate::util::json_schema::JsonSchemaExt;
use crate::util::json_value::JsonValueExt;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use anyhow::anyhow;

type IndexName = String;
type DocumentType = String;
type JsonSchema = serde_json::Value;

//todo: change this to use Platform value and document types
pub fn validate_indices_are_backward_compatible<'a>(
    existing_documents: impl IntoIterator<Item = (&'a DocumentType, &'a JsonSchema)>,
    new_documents: impl IntoIterator<Item = (&'a DocumentType, &'a JsonSchema)>,
) -> Result<SimpleConsensusValidationResult, ProtocolError> {
    let mut result = SimpleConsensusValidationResult::default();
    let new_documents_by_type: HashMap<&DocumentType, &JsonSchema> =
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
            .get_schema_properties()?
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
            get_changed_old_unique_index(&existing_schema_indices, &name_new_index_map);
        if let Some(changed_index) = maybe_changed_unique_existing_index {
            result.add_error(BasicError::DataContractUniqueIndicesChangedError(
                DataContractUniqueIndicesChangedError::new(
                    document_type.to_owned(),
                    changed_index.name.clone(),
                ),
            ));
        }

        let maybe_wrongly_updated_index = get_wrongly_updated_non_unique_index(
            &existing_schema_indices,
            &name_new_index_map,
            existing_schema,
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
            get_new_unique_index(&existing_schema_indices, name_new_index_map.values())?;
        if let Some(index) = maybe_new_unique_index {
            result.add_error(BasicError::DataContractHaveNewUniqueIndexError(
                DataContractHaveNewUniqueIndexError::new(
                    document_type.to_owned(),
                    index.name.clone(),
                ),
            ))
        }
        let maybe_wrongly_constructed_new_index = get_wrongly_constructed_new_index(
            existing_schema_indices.iter(),
            name_new_index_map.values(),
            added_properties.copied(),
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
        .find(|i| indexes_are_not_equal(i, new_indices.get(&i.name)) && i.unique)
}

fn indexes_are_not_equal(index_a: &Index, index_b: Option<&Index>) -> bool {
    match index_b {
        None => true,
        Some(index) => index_a != index,
    }
}

// Get one of the new indices that have old properties in them in the wrong order
// Explanation:
// Lets say we have two EXISTING Indexes: IndexA and IndexB.
// IndexA has properties: a,b,c
// IndexB has properties: b,c
// The function checks if a NEW index (i.e IndexC) contains one of possible sequences of properties.
// In the example, all possible sequences are: [a], [a,b], [a,b,c], [b], [b,c].
fn get_wrongly_constructed_new_index<'a>(
    existing_schema_indices: impl IntoIterator<Item = &'a Index>,
    new_schema_indices: impl IntoIterator<Item = &'a Index>,
    added_properties: impl IntoIterator<Item = &'a str>,
) -> Result<Option<&'a Index>, ProtocolError> {
    let mut existing_index_names: HashSet<&String> = Default::default();
    let mut existing_indexed_properties: HashSet<&String> = Default::default();
    let mut possible_sequences_of_properties: HashSet<&[IndexProperty]> = Default::default();
    let added_properties_set: HashSet<&str> = added_properties.into_iter().collect();

    for existing_index in existing_schema_indices {
        existing_index_names.insert(&existing_index.name);
        existing_indexed_properties.extend(existing_index.properties.iter().map(|p| &p.name));
        possible_sequences_of_properties
            .extend(get_all_possible_sequences_of_properties(existing_index));
    }

    let new_indices = new_schema_indices
        .into_iter()
        .filter(|index| !existing_index_names.contains(&&index.name));

    for new_index in new_indices {
        let existing_indexed_properties_len = new_index
            .properties
            .iter()
            .filter(|prop| existing_indexed_properties.contains(&&prop.name))
            .count();

        if existing_indexed_properties_len == 0 {
            // Creating a new index for unindexed field is not ok unless it's a new field:
            if let Some(property) = new_index.properties.first() {
                if new_index.properties.len() == 1 && added_properties_set.contains(&*property.name)
                {
                    continue;
                }
            } else {
                return Ok(Some(new_index));
            }
        }

        let properties_sequence = &new_index.properties[..existing_indexed_properties_len];

        if !possible_sequences_of_properties.contains(properties_sequence) {
            return Ok(Some(new_index));
        }
    }

    Ok(None)
}
fn get_all_possible_sequences_of_properties(
    index: &Index,
) -> impl Iterator<Item = &[IndexProperty]> {
    (0..index.properties.len()).map(move |i| &index.properties[..i + 1])
}

fn get_new_unique_index<'a>(
    existing_schema_indices: impl IntoIterator<Item = &'a Index>,
    new_schema_indices: impl IntoIterator<Item = &'a Index>,
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

fn get_wrongly_updated_non_unique_index<'a>(
    existing_schema_indices: &'a [Index],
    new_indices: &'a BTreeMap<IndexName, Index>,
    existing_schema: &'a JsonSchema,
) -> Result<Option<&'a Index>, ProtocolError> {
    // Checking every existing non-unique index, and it's respective new index
    // if they are changed per spec
    for index_definition in existing_schema_indices.iter().filter(|i| !i.unique) {
        let maybe_new_index_definition = new_indices.get(&index_definition.name);
        if let Some(new_index_definition) = maybe_new_index_definition {
            // Non-unique index can be ONLY updated by appending. The 'old' properties in the new
            // index must remain intact.
            let index_properties_len = index_definition.properties.len();
            if new_index_definition.properties[0..index_properties_len]
                != index_definition.properties
            {
                return Ok(Some(index_definition));
            }

            // Check if the rest of new indexes are defined in the existing schema
            for property in
                new_index_definition.properties[index_definition.properties.len()..].iter()
            {
                if let Ok(indices) = existing_schema.get_value("indices") {
                    let indices_array = indices.as_array().ok_or_else(|| {
                        ProtocolError::ParsingError(
                            "Error parsing schema: indices is not an array".to_string(),
                        )
                    })?;

                    for index in indices_array {
                        let properties_value = index.get_value("properties")?;
                        let properties_array = properties_value.as_array().ok_or_else(|| {
                            ProtocolError::ParsingError(
                                "Error parsing schema: properties is not an array".to_string(),
                            )
                        })?;

                        for property_to_check in properties_array {
                            if property_to_check.get_value(&property.name).is_ok() {
                                return Ok(Some(index_definition));
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_collect_all_possible_sequences() {
        let indices: Vec<Index> = vec![
            Index {
                name: "bravo_index".to_string(),
                unique: false,
                properties: vec![
                    IndexProperty {
                        name: "bravo_index_property_1".to_string(),
                        ascending: true,
                    },
                    IndexProperty {
                        name: "bravo_index_property_2".to_string(),
                        ascending: true,
                    },
                ],
            },
            Index {
                name: "alpha_index".to_string(),
                unique: false,
                properties: vec![
                    IndexProperty {
                        name: "alpha_index_property_1".to_string(),
                        ascending: true,
                    },
                    IndexProperty {
                        name: "alpha_index_property_2".to_string(),
                        ascending: true,
                    },
                    IndexProperty {
                        name: "alpha_index_property_3".to_string(),
                        ascending: true,
                    },
                ],
            },
        ];
        let mut sequences: HashSet<&[IndexProperty]> = Default::default();
        for index in indices.iter() {
            sequences.extend(get_all_possible_sequences_of_properties(index));
        }
        assert_eq!(5, sequences.len());
        assert!(sequences.contains(&indices[0].properties[..1]));
        assert!(sequences.contains(&indices[0].properties[..2]));
        assert!(sequences.contains(&indices[1].properties[..1]));
        assert!(sequences.contains(&indices[1].properties[..2]));
        assert!(sequences.contains(&indices[1].properties[..3]));
    }
}
