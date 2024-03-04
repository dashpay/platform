use std::collections::HashSet;

use crate::data_contract::document_type::index::{Index, IndexProperty};
use crate::ProtocolError;
use rand::prelude::StdRng;
use rand::seq::SliceRandom;
use rand::Rng;

impl Index {
    pub fn random(
        field_names: &[String],
        existing_indices: &[Index],
        rng: &mut StdRng,
    ) -> Result<Self, ProtocolError> {
        let index_name = format!("index_{}", rng.gen::<u16>());

        let mut unique_combinations = existing_indices
            .iter()
            .map(|index| {
                let mut names = index.properties.iter().map(|prop| prop.name.as_str()).collect::<Vec<&str>>();
                names.sort(); // Ensure the order does not affect the comparison
                names.join(",")
            })
            .collect::<HashSet<_>>();

        let mut properties = Vec::new();
        let mut attempts = 0;
        let max_attempts = 1000; // Adjust as needed

        while attempts < max_attempts {
            attempts += 1;

            let num_properties = rng.gen_range(1..=field_names.len());
            let selected_fields = field_names
                .choose_multiple(rng, num_properties)
                .cloned()
                .collect::<Vec<_>>();

            let combination_key = {
                let mut names = selected_fields.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
                names.sort(); // Ensure the order does not affect the comparison
                names.join(",")
            };
                
            if !unique_combinations.contains(&combination_key) {
                // Found a unique combination
                properties = selected_fields
                    .into_iter()
                    .map(|field_name| IndexProperty {
                        name: field_name,
                        ascending: rng.gen(), // Keep ascending/descending randomness
                    })
                    .collect();

                unique_combinations.insert(combination_key); // Mark this combination as used
                break;
            }
        }

        if properties.is_empty() {
            return Err(ProtocolError::Generic(
                "Unable to generate a unique index after maximum attempts".to_string(),
            ));
        }

        let unique = rng.gen();

        Ok(Index {
            name: index_name,
            properties,
            unique,
        })
    }
}
