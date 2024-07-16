//! Fee Refunds
//!
//! Fee refunds are calculated based on removed bytes per epoch.
//!

use crate::block::epoch::{Epoch, EpochIndex};
use crate::fee::default_costs::KnownCostItem::StorageDiskUsageCreditPerByte;
use crate::fee::default_costs::{CachedEpochIndexFeeVersions, EpochCosts};
use crate::fee::epoch::distribution::calculate_storage_fee_refund_amount_and_leftovers;
use crate::fee::epoch::{BytesPerEpoch, CreditsPerEpoch};
use crate::fee::Credits;
use crate::ProtocolError;
use bincode::{Decode, Encode};

use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use std::collections::btree_map::Iter;
use std::collections::BTreeMap;

/// There are additional work and storage required to process refunds
/// To protect system from the spam and unnecessary work
/// a dust refund limit is used
const MIN_REFUND_LIMIT_BYTES: u32 = 32;

/// Credits per Epoch by Identifier
pub type CreditsPerEpochByIdentifier = BTreeMap<[u8; 32], CreditsPerEpoch>;

/// Bytes per Epoch by Identifier
pub type BytesPerEpochByIdentifier = BTreeMap<[u8; 32], BytesPerEpoch>;

/// Fee refunds to identities based on removed data from specific epochs
#[derive(Debug, Clone, Eq, PartialEq, Default, Serialize, Deserialize, Encode, Decode)]
pub struct FeeRefunds(pub CreditsPerEpochByIdentifier);

impl FeeRefunds {
    /// Create fee refunds from GroveDB's StorageRemovalPerEpochByIdentifier
    pub fn from_storage_removal<I, C, E>(
        storage_removal: I,
        current_epoch_index: EpochIndex,
        epochs_per_era: u16,
        previous_fee_versions: &CachedEpochIndexFeeVersions,
    ) -> Result<Self, ProtocolError>
    where
        I: IntoIterator<Item = ([u8; 32], C)>,
        C: IntoIterator<Item = (E, u32)>,
        E: TryInto<u16>,
    {
        let refunds_per_epoch_by_identifier = storage_removal
            .into_iter()
            .map(|(identifier, bytes_per_epochs)| {
                bytes_per_epochs
                    .into_iter()
                    .filter(|(_, bytes)| bytes >= &MIN_REFUND_LIMIT_BYTES)
                    .map(|(encoded_epoch_index, bytes)| {
                        let epoch_index : u16 = encoded_epoch_index.try_into().map_err(|_| ProtocolError::Overflow("can't fit u64 epoch index from StorageRemovalPerEpochByIdentifier to u16 EpochIndex"))?;

                        // TODO Add in multipliers once they have been made

                        let credits: Credits = (bytes as Credits)
                            .checked_mul(Epoch::new(current_epoch_index)?.cost_for_known_cost_item(previous_fee_versions, StorageDiskUsageCreditPerByte))
                            .ok_or(ProtocolError::Overflow("storage written bytes cost overflow"))?;

                        let (amount, _) = calculate_storage_fee_refund_amount_and_leftovers(
                            credits,
                            epoch_index,
                            current_epoch_index,
                            epochs_per_era,
                        )?;

                        Ok((epoch_index, amount))
                    })
                    .collect::<Result<CreditsPerEpoch, ProtocolError>>()
                    .map(|credits_per_epochs| (identifier, credits_per_epochs))
            })
            .collect::<Result<CreditsPerEpochByIdentifier, ProtocolError>>()?;

        Ok(Self(refunds_per_epoch_by_identifier))
    }

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
    pub fn get(&self, key: &[u8; 32]) -> Option<&CreditsPerEpoch> {
        self.0.get(key)
    }

    /// Passthrough method for iteration
    pub fn iter(&self) -> Iter<[u8; 32], CreditsPerEpoch> {
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
                    .calculate_refunds_amount_for_identity(identifier.into())
                    .unwrap();

                Some((identifier.into(), credits))
            })
            .collect()
    }

    /// Calculates a refund amount of credits for specified identity id
    pub fn calculate_refunds_amount_for_identity(
        &self,
        identity_id: Identifier,
    ) -> Option<Credits> {
        let Some(credits_per_epoch) = self.get(identity_id.as_bytes()) else {
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
    type Item = ([u8; 32], CreditsPerEpoch);
    type IntoIter = std::collections::btree_map::IntoIter<[u8; 32], CreditsPerEpoch>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;
    use platform_version::version::PlatformVersion;

    static EPOCH_CHANGE_FEE_VERSION_TEST: Lazy<CachedEpochIndexFeeVersions> =
        Lazy::new(|| BTreeMap::from([(0, PlatformVersion::first().fee_version.clone())]));

    mod from_storage_removal {
        use super::*;
        use nohash_hasher::IntMap;
        use std::iter::FromIterator;

        #[test]
        fn should_filter_out_refunds_under_the_limit() {
            let identity_id = [0; 32];

            let bytes_per_epoch = IntMap::from_iter([(0, 31), (1, 100)]);
            let storage_removal =
                BytesPerEpochByIdentifier::from_iter([(identity_id, bytes_per_epoch)]);

            let fee_refunds = FeeRefunds::from_storage_removal(
                storage_removal,
                3,
                20,
                &EPOCH_CHANGE_FEE_VERSION_TEST,
            )
            .expect("should create fee refunds");

            let credits_per_epoch = fee_refunds.get(&identity_id).expect("should exists");

            assert!(credits_per_epoch.get(&0).is_none());
            assert!(credits_per_epoch.get(&1).is_some());
        }
    }
}
