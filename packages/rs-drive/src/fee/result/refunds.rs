// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Fee Refunds
//!
//! Fee refunds are calculated based on removed bytes per epoch.
//!

/// There are additional work and storage required to process refunds
/// To protect system from the spam and unnecessary work
/// a dust refund limit is used
const MIN_REFUND_LIMIT_BYTES: u32 = 32;

use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fee::credits::Credits;
use crate::fee::default_costs::KnownCostItem::StorageDiskUsageCreditPerByte;
use crate::fee::epoch::distribution::calculate_storage_fee_refund_amount_and_leftovers;
use crate::fee::epoch::{CreditsPerEpoch, EpochIndex};
use crate::fee::get_overflow_error;
use crate::fee_pools::epochs::Epoch;
use bincode::Options;
use costs::storage_cost::removal::{Identifier, StorageRemovalPerEpochByIdentifier};
use serde::{Deserialize, Serialize};
use std::collections::btree_map::{IntoIter, Iter};
use std::collections::BTreeMap;

/// Credits per Epoch by Identifier
pub type CreditsPerEpochByIdentifier = BTreeMap<Identifier, CreditsPerEpoch>;

/// Fee refunds to identities based on removed data from specific epochs
#[derive(Debug, Clone, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct FeeRefunds(pub CreditsPerEpochByIdentifier);

impl FeeRefunds {
    /// Create fee refunds from GroveDB's StorageRemovalPerEpochByIdentifier
    pub fn from_storage_removal(
        storage_removal: StorageRemovalPerEpochByIdentifier,
        current_epoch_index: EpochIndex,
    ) -> Result<Self, Error> {
        let refunds_per_epoch_by_identifier = storage_removal
            .into_iter()
            .map(|(identifier, bytes_per_epochs)| {
                bytes_per_epochs
                    .into_iter()
                    .filter(|(_, bytes)| bytes >= &MIN_REFUND_LIMIT_BYTES)
                    .map(|(encoded_epoch_index, bytes)| {
                        let epoch_index = u16::try_from(encoded_epoch_index).map_err(|_| get_overflow_error("can't fit u64 epoch index from StorageRemovalPerEpochByIdentifier to u16 EpochIndex"))?;

                        // TODO We should use multipliers

                        let credits: Credits = (bytes as Credits)
                            .checked_mul(Epoch::new(current_epoch_index).cost_for_known_cost_item(StorageDiskUsageCreditPerByte))
                            .ok_or_else(|| {
                                get_overflow_error("storage written bytes cost overflow")
                            })?;

                        let (amount, _) = calculate_storage_fee_refund_amount_and_leftovers(
                            credits,
                            epoch_index,
                            current_epoch_index,
                        )?;

                        Ok((epoch_index, amount))
                    })
                    .collect::<Result<CreditsPerEpoch, Error>>()
                    .map(|credits_per_epochs| (identifier, credits_per_epochs))
            })
            .collect::<Result<CreditsPerEpochByIdentifier, Error>>()?;

        Ok(Self(refunds_per_epoch_by_identifier))
    }

    /// Adds and self assigns result between two Fee Results
    pub fn checked_add_assign(&mut self, rhs: Self) -> Result<(), Error> {
        for (identifier, mut int_map_b) in rhs.0.into_iter() {
            let to_insert_int_map = if let Some(sint_map_a) = self.0.remove(&identifier) {
                // other has an int_map with the same identifier
                let intersection = sint_map_a
                    .into_iter()
                    .map(|(k, v)| {
                        let combined = if let Some(value_b) = int_map_b.remove(&k) {
                            v.checked_add(value_b)
                                .ok_or(Error::Fee(FeeError::Overflow("storage fee overflow error")))
                        } else {
                            Ok(v)
                        };
                        combined.map(|c| (k, c))
                    })
                    .collect::<Result<CreditsPerEpoch, Error>>()?;
                intersection.into_iter().chain(int_map_b).collect()
            } else {
                int_map_b
            };
            // reinsert the now combined IntMap
            self.0.insert(identifier, to_insert_int_map);
        }
        Ok(())
    }

    /// Passthrough method for get
    pub fn get(&self, key: &Identifier) -> Option<&CreditsPerEpoch> {
        self.0.get(key)
    }

    /// Passthrough method for iteration
    pub fn iter(&self) -> Iter<Identifier, CreditsPerEpoch> {
        self.0.iter()
    }

    /// Sums the fee result among all identities
    pub fn sum_per_epoch(self) -> CreditsPerEpoch {
        let mut summed_credits = CreditsPerEpoch::default();

        self.into_iter().for_each(|(_, credits_per_epoch)| {
            credits_per_epoch
                .into_iter()
                .for_each(|(epoch_index, credits)| {
                    summed_credits
                        .entry(epoch_index)
                        .and_modify(|base_credits| *base_credits += credits)
                        .or_insert(credits);
                });
        });
        summed_credits
    }

    /// Calculates a refund amount of credits per identity excluding specified identity id
    pub fn calculate_all_refunds_except_identity(
        &self,
        identity_id: Identifier,
    ) -> BTreeMap<Identifier, Credits> {
        self.iter()
            .filter_map(|(&identifier, _)| {
                if identifier == identity_id {
                    return None;
                }

                let credits = self
                    .calculate_refunds_amount_for_identity(identifier)
                    .unwrap();

                Some((identifier, credits))
            })
            .collect()
    }

    /// Calculates a refund amount of credits for specified identity id
    pub fn calculate_refunds_amount_for_identity(
        &self,
        identity_id: Identifier,
    ) -> Option<Credits> {
        let Some(credits_per_epoch) = self.get(&identity_id) else {
            return None;
        };

        let credits = credits_per_epoch
            .iter()
            .map(|(_epoch_index, credits)| credits)
            .sum();

        Some(credits)
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

    /// Returns serialized size
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

    /// Deserialized struct from bytes
    pub fn deserialize(bytes: &[u8]) -> Result<Self, Error> {
        Ok(FeeRefunds(
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

impl IntoIterator for FeeRefunds {
    type Item = (Identifier, CreditsPerEpoch);
    type IntoIter = IntoIter<Identifier, CreditsPerEpoch>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod from_storage_removal {
        use super::*;
        use intmap::IntMap;

        #[test]
        fn should_filter_out_refunds_under_the_limit() {
            let identity_id = [0; 32];

            let bytes_per_epoch = IntMap::from_iter([(0, 31), (1, 100)]);
            let storage_removal =
                StorageRemovalPerEpochByIdentifier::from_iter([(identity_id, bytes_per_epoch)]);

            let fee_refunds = FeeRefunds::from_storage_removal(storage_removal, 3)
                .expect("should create fee refunds");

            let credits_per_epoch = fee_refunds.get(&identity_id).expect("should exists");

            assert!(credits_per_epoch.get(&0).is_none());
            assert!(credits_per_epoch.get(&1).is_some());
        }
    }
}
