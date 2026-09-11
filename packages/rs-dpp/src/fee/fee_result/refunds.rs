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
    ///
    /// A refund is the unpaid remainder of the storage fee originally charged for the removed
    /// bytes. That fee was priced with the storage table active when the bytes were written, so
    /// the rate is resolved at the storage epoch (the key of each removal entry) through the fee
    /// history. The current epoch only decides how many era shares of that fee were already paid
    /// out to proposers.
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

                        let storage_rate = Epoch::new(epoch_index)?
                            .cost_for_known_cost_item(previous_fee_versions, StorageDiskUsageCreditPerByte);

                        let credits: Credits = (bytes as Credits)
                            .checked_mul(storage_rate)
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
    pub fn iter(&self) -> Iter<'_, [u8; 32], CreditsPerEpoch> {
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
        let credits_per_epoch = self.get(identity_id.as_bytes())?;

        let credits = credits_per_epoch.values().sum();

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
    use platform_version::version::fee::storage::FeeStorageVersion;
    use platform_version::version::fee::v1::FEE_VERSION1;
    use platform_version::version::fee::FeeVersion;

    static EPOCH_CHANGE_FEE_VERSION_TEST: Lazy<CachedEpochIndexFeeVersions> =
        Lazy::new(|| BTreeMap::from([(0, FeeVersion::first())]));

    /// Storage rate of the first registered generation, which priced every byte written so far.
    const FIRST_GENERATION_RATE: Credits = 27000;

    /// A second storage table that is not registered anywhere. It exists only so a rate boundary
    /// in the fee history is observable in these tests.
    const SYNTHETIC_RATE: Credits = 54000;

    static SYNTHETIC_FEE_VERSION_2: FeeVersion = FeeVersion {
        fee_version_number: 2,
        storage: FeeStorageVersion {
            storage_disk_usage_credit_per_byte: SYNTHETIC_RATE,
            ..FEE_VERSION1.storage
        },
        ..FEE_VERSION1
    };

    mod from_storage_removal {
        use super::*;
        use nohash_hasher::IntMap;
        use std::iter::FromIterator;

        const EPOCHS_PER_ERA: u16 = 20;

        fn expected_refund(
            bytes: u32,
            rate: Credits,
            storage_epoch: EpochIndex,
            current_epoch: EpochIndex,
        ) -> Credits {
            let (amount, _) = calculate_storage_fee_refund_amount_and_leftovers(
                bytes as Credits * rate,
                storage_epoch,
                current_epoch,
                EPOCHS_PER_ERA,
            )
            .expect("refund amount");
            amount
        }

        fn refunds_for_one_identity(
            bytes_per_epoch: IntMap<u16, u32>,
            current_epoch: EpochIndex,
            fee_history: &CachedEpochIndexFeeVersions,
        ) -> CreditsPerEpoch {
            let identity_id = [7; 32];
            let storage_removal =
                BytesPerEpochByIdentifier::from_iter([(identity_id, bytes_per_epoch)]);

            FeeRefunds::from_storage_removal(
                storage_removal,
                current_epoch,
                EPOCHS_PER_ERA,
                fee_history,
            )
            .expect("should create fee refunds")
            .get(&identity_id)
            .expect("identity has refunds")
            .clone()
        }

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

        #[test]
        fn should_price_each_removed_epoch_at_the_storage_table_active_when_the_bytes_were_stored()
        {
            // Rate boundary at epoch 10: bytes written before it were charged at the first
            // generation's rate, bytes written from it on at the synthetic rate.
            let fee_history: CachedEpochIndexFeeVersions = BTreeMap::from([
                (0, FeeVersion::get(1).expect("registered")),
                (10, &SYNTHETIC_FEE_VERSION_2),
            ]);
            let current_epoch = 15;

            let refunds = refunds_for_one_identity(
                IntMap::from_iter([(5, 100), (12, 100)]),
                current_epoch,
                &fee_history,
            );

            assert_eq!(
                refunds.get(&5).copied(),
                Some(expected_refund(
                    100,
                    FIRST_GENERATION_RATE,
                    5,
                    current_epoch
                )),
                "bytes stored before the boundary refund at the rate they were charged"
            );
            assert_eq!(
                refunds.get(&12).copied(),
                Some(expected_refund(100, SYNTHETIC_RATE, 12, current_epoch)),
                "bytes stored after the boundary refund at the new rate"
            );
            assert_ne!(
                refunds.get(&5).copied(),
                Some(expected_refund(100, SYNTHETIC_RATE, 5, current_epoch)),
                "pre-boundary bytes must not be re-priced at the current epoch's rate"
            );
        }

        #[test]
        fn should_price_removals_before_the_earliest_history_entry_with_the_first_generation() {
            let fee_history: CachedEpochIndexFeeVersions =
                BTreeMap::from([(10, &SYNTHETIC_FEE_VERSION_2)]);
            let current_epoch = 12;

            let refunds = refunds_for_one_identity(
                IntMap::from_iter([(3, 100)]),
                current_epoch,
                &fee_history,
            );

            assert_eq!(
                refunds.get(&3).copied(),
                Some(expected_refund(
                    100,
                    FIRST_GENERATION_RATE,
                    3,
                    current_epoch
                ))
            );
        }

        #[test]
        fn should_match_the_current_epoch_rate_whenever_every_generation_shares_one_storage_table()
        {
            // Every shipped input: an empty history (the fee version number 1 path in Drive) or a
            // history whose entries all resolve to number 1. The storage epoch and the current
            // epoch then resolve to the same rate, so the result is identical to what pricing at
            // the current epoch produced before the rule changed.
            let same_table_histories: [CachedEpochIndexFeeVersions; 2] = [
                BTreeMap::default(),
                BTreeMap::from([
                    (0, FeeVersion::get(1).expect("registered")),
                    (7, FeeVersion::get(1).expect("registered")),
                ]),
            ];
            let current_epoch = 9;

            for fee_history in &same_table_histories {
                let refunds = refunds_for_one_identity(
                    IntMap::from_iter([(2, 100), (8, 100)]),
                    current_epoch,
                    fee_history,
                );
                let current_epoch_rate = Epoch::new(current_epoch)
                    .expect("epoch")
                    .cost_for_known_cost_item(fee_history, StorageDiskUsageCreditPerByte);
                assert_eq!(current_epoch_rate, FIRST_GENERATION_RATE);

                for storage_epoch in [2, 8] {
                    assert_eq!(
                        refunds.get(&storage_epoch).copied(),
                        Some(expected_refund(
                            100,
                            current_epoch_rate,
                            storage_epoch,
                            current_epoch
                        ))
                    );
                }
            }
        }
    }
}
