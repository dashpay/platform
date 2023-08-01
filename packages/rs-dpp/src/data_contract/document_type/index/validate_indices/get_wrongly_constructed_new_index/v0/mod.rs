use std::collections::HashSet;
use crate::data_contract::document_type::{Index, IndexProperty};
use crate::ProtocolError;
impl Index {
    // Get one of the new indices that have old properties in them in the wrong order
// Explanation:
// Lets say we have two EXISTING Indexes: IndexA and IndexB.
// IndexA has properties: a,b,c
// IndexB has properties: b,c
// The function checks if a NEW index (i.e IndexC) contains one of possible sequences of properties.
// In the example, all possible sequences are: [a], [a,b], [a,b,c], [b], [b,c].
    pub(super) fn get_wrongly_constructed_new_index_v0<'a>(
        existing_schema_indices: impl IntoIterator<Item=&'a Index>,
        new_schema_indices: impl IntoIterator<Item=&'a Index>,
        added_properties: impl IntoIterator<Item=&'a str>,
    ) -> Result<Option<&'a Index>, ProtocolError> {
        let mut existing_index_names: HashSet<&String> = Default::default();
        let mut existing_indexed_properties: HashSet<&String> = Default::default();
        let mut possible_sequences_of_properties: HashSet<&[IndexProperty]> = Default::default();
        let added_properties_set: HashSet<&str> = added_properties.into_iter().collect();

        for existing_index in existing_schema_indices {
            existing_index_names.insert(&existing_index.name);
            existing_indexed_properties.extend(existing_index.properties.iter().map(|p| &p.name));
            possible_sequences_of_properties
                .extend(Index::get_all_possible_sequences_of_properties(existing_index));
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
    ) -> impl Iterator<Item=&[IndexProperty]> {
        (0..index.properties.len()).map(move |i| &index.properties[..i + 1])
    }
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
            sequences.extend(Index::get_all_possible_sequences_of_properties(index));
        }
        assert_eq!(5, sequences.len());
        assert!(sequences.contains(&indices[0].properties[..1]));
        assert!(sequences.contains(&indices[0].properties[..2]));
        assert!(sequences.contains(&indices[1].properties[..1]));
        assert!(sequences.contains(&indices[1].properties[..2]));
        assert!(sequences.contains(&indices[1].properties[..3]));
    }
}