use crate::error::fee::FeeError;
use crate::error::Error;
use bincode::Options;
use costs::storage_cost::removal::Identifier;
use intmap::IntMap;
use serde::{Deserialize, Serialize};
use std::collections::btree_map::{IntoIter, Iter};
use std::collections::BTreeMap;

/// Fee Result
#[derive(Debug, Clone, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct RemovedBytesFromEpochsByIdentities(pub BTreeMap<Identifier, IntMap<u32>>);

impl RemovedBytesFromEpochsByIdentities {
    /// Adds and self assigns result between two Fee Results
    pub fn checked_add_assign(&mut self, rhs: Self) -> Result<(), Error> {
        for (identifier, mut int_map_b) in rhs.0.into_iter() {
            let to_insert_int_map = if let Some(sint_map_a) = self.0.remove(&identifier) {
                // other has an int_map with the same identifier
                let intersection = sint_map_a
                    .into_iter()
                    .map(|(k, v)| {
                        let combined = if let Some(value_b) = int_map_b.remove(k) {
                            v.checked_add(value_b)
                                .ok_or(Error::Fee(FeeError::Overflow("storage fee overflow error")))
                        } else {
                            Ok(v)
                        };
                        combined.map(|c| (k, c))
                    })
                    .collect::<Result<IntMap<u32>, Error>>()?;
                intersection.into_iter().chain(int_map_b).collect()
            } else {
                int_map_b
            };
            // reinsert the now combined intmap
            self.0.insert(identifier, to_insert_int_map);
        }
        Ok(())
    }

    /// Passthrough method for get
    pub fn get(&self, key: &Identifier) -> Option<&IntMap<u32>> {
        self.0.get(key)
    }

    /// Passthrough method for iteration
    pub fn iter(&self) -> Iter<Identifier, IntMap<u32>> {
        self.0.iter()
    }

    /// Passthrough method for into interation
    pub fn into_iter(self) -> IntoIter<Identifier, IntMap<u32>> {
        self.0.into_iter()
    }

    /// Serialize the structure
    pub fn serialize(&self) -> Result<Vec<u8>, Error> {
        bincode::DefaultOptions::default()
            .with_varint_encoding()
            .reject_trailing_bytes()
            .serialize(&self.0)
            .map_err(|_| {
                Error::Fee(FeeError::CorruptedRemovedBytesFromIdentitiesSerialization(
                    "unable to serialize",
                ))
            })
    }

    pub fn serialized_size(&self) -> Result<u64, Error> {
        bincode::DefaultOptions::default()
            .with_varint_encoding()
            .reject_trailing_bytes()
            .serialized_size(&self.0)
            .map_err(|_| {
                Error::Fee(FeeError::CorruptedRemovedBytesFromIdentitiesSerialization(
                    "unable to serialize and get size",
                ))
            })
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, Error> {
        Ok(RemovedBytesFromEpochsByIdentities(
            bincode::DefaultOptions::default()
                .with_varint_encoding()
                .reject_trailing_bytes()
                .deserialize(bytes)
                .map_err(|_| {
                    Error::Fee(FeeError::CorruptedRemovedBytesFromIdentitiesSerialization(
                        "unable to deserialize",
                    ))
                })?,
        ))
    }
}
