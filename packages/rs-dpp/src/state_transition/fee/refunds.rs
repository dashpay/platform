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

use crate::credits::Credits;
use crate::ProtocolError;
use nohash_hasher::IntMap;
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use std::collections::btree_map::{IntoIter, Iter};
use std::collections::BTreeMap;

pub type EpochIndex = u16;

pub type CreditsPerEpoch = IntMap<EpochIndex, Credits>;

pub type CreditsPerEpochByIdentifier = BTreeMap<Identifier, CreditsPerEpoch>;

/// Fee refunds to identities based on removed data from specific epochs
#[derive(Debug, Clone, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct FeeRefunds(pub CreditsPerEpochByIdentifier);

impl FeeRefunds {
    /// Adds and self assigns result between two Fee Results
    pub fn checked_add_assign(&mut self, rhs: Self) -> Result<(), ProtocolError> {
        for (identifier, mut int_map_b) in rhs.0.into_iter() {
            let to_insert_int_map = if let Some(sint_map_a) = self.0.remove(&identifier) {
                // other has an int_map with the same identifier
                let intersection = sint_map_a
                    .into_iter()
                    .map(|(k, v)| {
                        let combined = if let Some(value_b) = int_map_b.remove(&k) {
                            v.checked_add(value_b)
                                .ok_or(ProtocolError::Overflow("storage fee overflow error"))
                        } else {
                            Ok(v)
                        };
                        combined.map(|c| (k, c))
                    })
                    .collect::<Result<CreditsPerEpoch, ProtocolError>>()?;
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

    /// Calculates all refunds
    pub fn sum(&self) -> Credits {
        self.iter()
            .map(|(_, creditsPerEpoch)| {
                credits_per_epoch.iter().map(|(_, credits)| credits).sum();
            })
            .sum()
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
        identity_id: &Identifier,
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
}

impl IntoIterator for FeeRefunds {
    type Item = (Identifier, CreditsPerEpoch);
    type IntoIter = IntoIter<Identifier, CreditsPerEpoch>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
