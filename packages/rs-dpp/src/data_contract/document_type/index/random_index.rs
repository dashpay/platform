use crate::data_contract::document_type::index::{Index, IndexProperty};
use crate::ProtocolError;
use rand::prelude::StdRng;
use rand::seq::SliceRandom;
use rand::Rng;
use std::borrow::Borrow;

impl Index {
    pub fn random<I, T>(
        field_names: &[String],
        existing_indices: I,
        rng: &mut StdRng,
    ) -> Result<Self, ProtocolError>
    where
        I: Clone + IntoIterator<Item = T>, // T is the type of elements in the collection
        T: Borrow<Index>,                  // Assuming Index is the type stored in the collection
    {
        let index_name = format!("index_{}", rng.gen::<u16>());

        let mut properties;
        let mut attempts = 0;
        let max_attempts = 1000; // You can adjust this value based on your requirements

        loop {
            let num_properties = rng.gen_range(1..=field_names.len());
            let mut selected_fields = field_names
                .choose_multiple(rng, num_properties)
                .cloned()
                .collect::<Vec<_>>();

            properties = selected_fields
                .drain(..)
                .map(|field_name| IndexProperty {
                    name: field_name,
                    ascending: true,
                })
                .collect::<Vec<_>>();

            if !existing_indices
                .clone()
                .into_iter()
                .any(|index| index.borrow().properties == properties)
            {
                break;
            }

            attempts += 1;
            if attempts >= max_attempts {
                return Err(ProtocolError::Generic(
                    "Unable to generate a unique index after maximum attempts".to_string(),
                ));
            }
        }

        let unique = rng.gen();

        Ok(Index {
            name: index_name,
            properties,
            unique,
            contested_index: None,
        })
    }
}
