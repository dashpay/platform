//! Fee Refunds
//!
//! Fee refunds are calculated based on removed bytes per epoch.
//!
//! The carrier that GroveDB hands back keys removed bytes by a 32-byte
//! identifier. `FeeRefunds` keeps that carrier as its first field and records
//! the typed [`RefundOwner`] of every carrier key in its second field, so a
//! consumer that routes a refund reads the owner's kind from the record and
//! never infers it from the key.

use crate::block::epoch::{Epoch, EpochIndex};
use crate::fee::default_costs::KnownCostItem::StorageDiskUsageCreditPerByte;
use crate::fee::default_costs::{CachedEpochIndexFeeVersions, EpochCosts};
use crate::fee::epoch::distribution::calculate_storage_fee_refund_amount_and_leftovers;
use crate::fee::epoch::{BytesPerEpoch, CreditsPerEpoch};
use crate::fee::refund_owner::RefundOwner;
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

pub use crate::fee::refund_owner::RefundOwnersByIdentifier;

/// Fee refunds to owners based on removed data from specific epochs.
///
/// The first field is the per-owner, per-epoch credit carrier keyed by
/// [`RefundOwner::removal_key`]. The second field records the typed owner of
/// each carrier key. Every constructor that prices a storage removal fills
/// both; a key without a recorded owner cannot be routed and is treated as an
/// invariant failure by the typed accessors.
#[derive(Debug, Clone, Eq, PartialEq, Default, Serialize, Deserialize, Encode, Decode)]
pub struct FeeRefunds(
    pub CreditsPerEpochByIdentifier,
    pub RefundOwnersByIdentifier,
);

impl FeeRefunds {
    /// Create fee refunds from GroveDB's StorageRemovalPerEpochByIdentifier
    ///
    /// This is the untyped path: every carrier key is an identity id, as
    /// written by the identity-owned storage flags, and is recorded as
    /// [`RefundOwner::Identity`] explicitly.
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
        let refunds_per_epoch_by_identifier = Self::price_storage_removal(
            storage_removal,
            current_epoch_index,
            epochs_per_era,
            previous_fee_versions,
        )?;

        let owners = refunds_per_epoch_by_identifier
            .keys()
            .map(|identifier| {
                (
                    *identifier,
                    RefundOwner::Identity(Identifier::from(*identifier)),
                )
            })
            .collect();

        Ok(Self(refunds_per_epoch_by_identifier, owners))
    }

    /// Create fee refunds from GroveDB's StorageRemovalPerEpochByIdentifier
    /// with the owners recorded when the bytes were split.
    ///
    /// Every carrier key must have an entry in `owners`; a key without one is
    /// a removal whose owner was never recorded, which is an invariant
    /// failure rather than something to guess at. The system carrier key is
    /// never an owner and must be removed by the caller before pricing.
    pub fn from_typed_storage_removal<I, C, E>(
        storage_removal: I,
        owners: &RefundOwnersByIdentifier,
        current_epoch_index: EpochIndex,
        epochs_per_era: u16,
        previous_fee_versions: &CachedEpochIndexFeeVersions,
    ) -> Result<Self, ProtocolError>
    where
        I: IntoIterator<Item = ([u8; 32], C)>,
        C: IntoIterator<Item = (E, u32)>,
        E: TryInto<u16>,
    {
        let refunds_per_epoch_by_identifier = Self::price_storage_removal(
            storage_removal,
            current_epoch_index,
            epochs_per_era,
            previous_fee_versions,
        )?;

        let recorded_owners = refunds_per_epoch_by_identifier
            .keys()
            .map(|identifier| {
                owners
                    .get(identifier)
                    .map(|owner| (*identifier, *owner))
                    .ok_or_else(|| {
                        ProtocolError::CorruptedCodeExecution(format!(
                            "storage removal carrier key {} has no recorded refund owner",
                            hex::encode(identifier)
                        ))
                    })
            })
            .collect::<Result<RefundOwnersByIdentifier, ProtocolError>>()?;

        Ok(Self(refunds_per_epoch_by_identifier, recorded_owners))
    }

    fn price_storage_removal<I, C, E>(
        storage_removal: I,
        current_epoch_index: EpochIndex,
        epochs_per_era: u16,
        previous_fee_versions: &CachedEpochIndexFeeVersions,
    ) -> Result<CreditsPerEpochByIdentifier, ProtocolError>
    where
        I: IntoIterator<Item = ([u8; 32], C)>,
        C: IntoIterator<Item = (E, u32)>,
        E: TryInto<u16>,
    {
        storage_removal
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
            .collect::<Result<CreditsPerEpochByIdentifier, ProtocolError>>()
    }

    /// Adds and self assigns result between two Fee Results
    ///
    /// Credits merge per carrier key and epoch. Recorded owners merge by key;
    /// one carrier key naming two different owners is a collision that is
    /// reported, never resolved by picking one.
    pub fn checked_add_assign(&mut self, rhs: Self) -> Result<(), ProtocolError> {
        let Self(rhs_credits, rhs_owners) = rhs;
        // owners are checked before either map changes, so a rejected merge
        // leaves the accumulator exactly as it was
        for identifier in rhs_credits.keys() {
            let Some(owner) = rhs_owners.get(identifier) else {
                return Err(ProtocolError::CorruptedCodeExecution(format!(
                    "storage refund carrier key {} has no recorded refund owner",
                    hex::encode(identifier)
                )));
            };
            match self.1.get(identifier) {
                Some(existing) if existing != owner => {
                    return Err(ProtocolError::CorruptedCodeExecution(format!(
                        "storage removal carrier key {} is recorded for two different refund owners: {:?} and {:?}",
                        hex::encode(identifier),
                        existing,
                        owner
                    )));
                }
                // credits already held without an owner must not acquire
                // one from the other side
                None if self.0.contains_key(identifier) => {
                    return Err(ProtocolError::CorruptedCodeExecution(format!(
                        "storage refund carrier key {} has no recorded refund owner",
                        hex::encode(identifier)
                    )));
                }
                _ => {}
            }
        }
        for (identifier, owner) in &rhs_owners {
            match self.1.get(identifier) {
                Some(existing) if existing != owner => {
                    return Err(ProtocolError::CorruptedCodeExecution(format!(
                        "storage removal carrier key {} is recorded for two different refund owners: {:?} and {:?}",
                        hex::encode(identifier),
                        existing,
                        owner
                    )));
                }
                // an owner record arriving without credits must not lend an
                // owner to credits already held without one
                None if self.0.contains_key(identifier) => {
                    return Err(ProtocolError::CorruptedCodeExecution(format!(
                        "storage refund carrier key {} has no recorded refund owner",
                        hex::encode(identifier)
                    )));
                }
                _ => {}
            }
        }
        for (identifier, mut int_map_b) in rhs_credits.into_iter() {
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
        self.1.extend(rhs_owners);
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

    /// The recorded owner of a carrier key
    pub fn owner_of(&self, key: &[u8; 32]) -> Option<RefundOwner> {
        self.1.get(key).copied()
    }

    /// Iterates the refunds with their recorded owners.
    ///
    /// A carrier key without a recorded owner yields an error: a refund that
    /// cannot name its owner cannot be routed and must halt rather than be
    /// burned, minted or guessed.
    pub fn iter_typed(
        &self,
    ) -> impl Iterator<Item = Result<(RefundOwner, &[u8; 32], &CreditsPerEpoch), ProtocolError>>
    {
        self.0.iter().map(|(identifier, credits_per_epoch)| {
            self.owner_of(identifier)
                .map(|owner| (owner, identifier, credits_per_epoch))
                .ok_or_else(|| {
                    ProtocolError::CorruptedCodeExecution(format!(
                        "storage refund carrier key {} has no recorded refund owner",
                        hex::encode(identifier)
                    ))
                })
        })
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

    /// Checks that every recorded owner is an identity, so that the identity
    /// keyed accessors below can be used without reading a bucket's carrier
    /// key as an identity id. A carrier key without a recorded owner fails
    /// the same way.
    ///
    /// The identity keyed accessors keep their historical signatures because
    /// the shipped balance consumer calls them; that consumer runs only under
    /// generations that predate typed owners, and Drive fails closed there on
    /// a carrier key that has no identity balance. New callers check this
    /// guard first or use the typed accessors.
    pub fn ensure_identity_owners_only(&self) -> Result<(), ProtocolError> {
        for entry in self.iter_typed() {
            let (owner, identifier, _) = entry?;
            if owner.as_identity().is_none() {
                return Err(ProtocolError::CorruptedCodeExecution(format!(
                    "storage refund carrier key {} belongs to {:?}, not to an identity",
                    hex::encode(identifier),
                    owner
                )));
            }
        }
        Ok(())
    }

    /// Calculates a refund amount of credits per identity excluding specified identity id.
    ///
    /// Identity keyed view: every carrier key is returned as an identity id.
    /// Call [`Self::ensure_identity_owners_only`] first on a path that may
    /// hold bucket owned refunds, or use
    /// [`Self::calculate_all_refunds_except_owner`].
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

    /// Calculates the refund amount of credits per recorded owner, excluding
    /// the given owner.
    ///
    /// Owners are compared as typed values, so an identity and a contract
    /// bucket never match each other. A carrier key without a recorded owner
    /// is an error, as is a sum that overflows.
    pub fn calculate_all_refunds_except_owner(
        &self,
        skip_owner: &RefundOwner,
    ) -> Result<BTreeMap<RefundOwner, Credits>, ProtocolError> {
        let mut refunds_by_owner = BTreeMap::new();
        for entry in self.iter_typed() {
            let (owner, _, credits_per_epoch) = entry?;
            if owner == *skip_owner {
                continue;
            }
            let credits = credits_per_epoch.values().try_fold(0u64, |sum, credits| {
                sum.checked_add(*credits)
                    .ok_or(ProtocolError::Overflow("storage refund sum overflow"))
            })?;
            let total: &mut Credits = refunds_by_owner.entry(owner).or_insert(0);
            *total = total
                .checked_add(credits)
                .ok_or(ProtocolError::Overflow("storage refund sum overflow"))?;
        }
        Ok(refunds_by_owner)
    }

    /// Calculates a refund amount of credits for specified identity id.
    ///
    /// Identity keyed view: looks the identity id up as a carrier key. An
    /// identity id can never equal a bucket's carrier key except by a hash
    /// preimage, so this stays exact for identity payers.
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
    use nohash_hasher::IntMap;
    use once_cell::sync::Lazy;
    use platform_version::version::fee::FeeVersion;
    use std::iter::FromIterator;

    static EPOCH_CHANGE_FEE_VERSION_TEST: Lazy<CachedEpochIndexFeeVersions> =
        Lazy::new(|| BTreeMap::from([(0, FeeVersion::first())]));

    fn bucket_owner(contract_byte: u8, position: u16) -> RefundOwner {
        RefundOwner::ContractBucket {
            contract_id: Identifier::from([contract_byte; 32]),
            position,
        }
    }

    mod from_storage_removal {
        use super::*;

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
        fn should_record_an_identity_owner_for_every_carrier_key() {
            let first = [1u8; 32];
            let second = [2u8; 32];
            let storage_removal = BytesPerEpochByIdentifier::from_iter([
                (first, IntMap::from_iter([(0, 100)])),
                (second, IntMap::from_iter([(1, 100)])),
            ]);

            let fee_refunds = FeeRefunds::from_storage_removal(
                storage_removal,
                3,
                20,
                &EPOCH_CHANGE_FEE_VERSION_TEST,
            )
            .expect("should create fee refunds");

            assert_eq!(
                fee_refunds.owner_of(&first),
                Some(RefundOwner::Identity(Identifier::from(first)))
            );
            assert_eq!(
                fee_refunds.owner_of(&second),
                Some(RefundOwner::Identity(Identifier::from(second)))
            );
            assert_eq!(fee_refunds.1.len(), 2);
        }
    }

    mod from_typed_storage_removal {
        use super::*;

        #[test]
        fn should_record_the_owner_from_the_map_for_identity_and_bucket_keys() {
            let identity = RefundOwner::Identity(Identifier::from([5u8; 32]));
            let bucket = bucket_owner(6, 2);
            let owners = RefundOwnersByIdentifier::from_iter([
                (identity.removal_key(), identity),
                (bucket.removal_key(), bucket),
            ]);
            let storage_removal = BytesPerEpochByIdentifier::from_iter([
                (identity.removal_key(), IntMap::from_iter([(0, 100)])),
                (bucket.removal_key(), IntMap::from_iter([(1, 200)])),
            ]);

            let fee_refunds = FeeRefunds::from_typed_storage_removal(
                storage_removal.clone(),
                &owners,
                3,
                20,
                &EPOCH_CHANGE_FEE_VERSION_TEST,
            )
            .expect("should create fee refunds");

            assert_eq!(
                fee_refunds.owner_of(&identity.removal_key()),
                Some(identity)
            );
            assert_eq!(fee_refunds.owner_of(&bucket.removal_key()), Some(bucket));

            // the credits are priced exactly as the untyped path prices them
            let untyped = FeeRefunds::from_storage_removal(
                storage_removal,
                3,
                20,
                &EPOCH_CHANGE_FEE_VERSION_TEST,
            )
            .expect("should create fee refunds");
            assert_eq!(fee_refunds.0, untyped.0);
        }

        #[test]
        fn should_reject_a_carrier_key_without_a_recorded_owner() {
            let bucket = bucket_owner(6, 2);
            let owners = RefundOwnersByIdentifier::new();
            let storage_removal = BytesPerEpochByIdentifier::from_iter([(
                bucket.removal_key(),
                IntMap::from_iter([(1, 200)]),
            )]);

            let result = FeeRefunds::from_typed_storage_removal(
                storage_removal,
                &owners,
                3,
                20,
                &EPOCH_CHANGE_FEE_VERSION_TEST,
            );

            assert!(matches!(
                result,
                Err(ProtocolError::CorruptedCodeExecution(_))
            ));
        }
    }

    mod checked_add_assign {
        use super::*;

        fn refunds_for(owner: RefundOwner, epoch: u16, credits: Credits) -> FeeRefunds {
            FeeRefunds(
                CreditsPerEpochByIdentifier::from_iter([(
                    owner.removal_key(),
                    CreditsPerEpoch::from_iter([(epoch, credits)]),
                )]),
                RefundOwnersByIdentifier::from_iter([(owner.removal_key(), owner)]),
            )
        }

        #[test]
        fn should_merge_credits_and_owners_from_both_sides() {
            let identity = RefundOwner::Identity(Identifier::from([1u8; 32]));
            let bucket = bucket_owner(2, 0);

            let mut refunds = refunds_for(identity, 0, 10);
            refunds
                .checked_add_assign(refunds_for(identity, 1, 5))
                .expect("should merge the same owner");
            refunds
                .checked_add_assign(refunds_for(bucket, 0, 7))
                .expect("should merge a second owner");

            assert_eq!(
                refunds.get(&identity.removal_key()),
                Some(&CreditsPerEpoch::from_iter([(0, 10), (1, 5)]))
            );
            assert_eq!(
                refunds.get(&bucket.removal_key()),
                Some(&CreditsPerEpoch::from_iter([(0, 7)]))
            );
            assert_eq!(refunds.owner_of(&identity.removal_key()), Some(identity));
            assert_eq!(refunds.owner_of(&bucket.removal_key()), Some(bucket));
        }

        #[test]
        fn should_reject_one_carrier_key_recorded_for_two_owners() {
            let key = [9u8; 32];
            let identity = RefundOwner::Identity(Identifier::from(key));
            let bucket = bucket_owner(2, 0);

            let mut refunds = FeeRefunds(
                CreditsPerEpochByIdentifier::from_iter([(
                    key,
                    CreditsPerEpoch::from_iter([(0, 10)]),
                )]),
                RefundOwnersByIdentifier::from_iter([(key, identity)]),
            );
            let colliding = FeeRefunds(
                CreditsPerEpochByIdentifier::from_iter([(
                    key,
                    CreditsPerEpoch::from_iter([(0, 1)]),
                )]),
                RefundOwnersByIdentifier::from_iter([(key, bucket)]),
            );

            let before = refunds.clone();
            let result = refunds.checked_add_assign(colliding);

            assert!(matches!(
                result,
                Err(ProtocolError::CorruptedCodeExecution(_))
            ));
            assert_eq!(refunds, before, "a rejected merge changes nothing");
        }

        #[test]
        fn should_reject_a_merge_that_would_give_unowned_credits_an_owner() {
            let bucket = bucket_owner(2, 0);
            let key = bucket.removal_key();
            let unowned = FeeRefunds(
                CreditsPerEpochByIdentifier::from_iter([(
                    key,
                    CreditsPerEpoch::from_iter([(0, 10)]),
                )]),
                RefundOwnersByIdentifier::new(),
            );
            let owned = refunds_for(bucket, 0, 1);

            let mut left = unowned.clone();
            let before = left.clone();
            assert!(matches!(
                left.checked_add_assign(owned.clone()),
                Err(ProtocolError::CorruptedCodeExecution(_))
            ));
            assert_eq!(left, before, "a rejected merge changes nothing");

            let mut right = owned;
            let before = right.clone();
            assert!(matches!(
                right.checked_add_assign(unowned.clone()),
                Err(ProtocolError::CorruptedCodeExecution(_))
            ));
            assert_eq!(right, before, "a rejected merge changes nothing");

            // an owner record with no credits of its own must not attach
            // itself to credits held without an owner
            let owner_only = FeeRefunds(
                CreditsPerEpochByIdentifier::new(),
                RefundOwnersByIdentifier::from_iter([(key, bucket)]),
            );
            let mut left = unowned;
            let before = left.clone();
            assert!(matches!(
                left.checked_add_assign(owner_only),
                Err(ProtocolError::CorruptedCodeExecution(_))
            ));
            assert_eq!(left, before, "a rejected merge changes nothing");
        }
    }

    mod typed_accessors {
        use super::*;

        #[test]
        fn should_sum_refunds_per_owner_and_skip_the_given_owner() {
            let payer = RefundOwner::Identity(Identifier::from([1u8; 32]));
            let other = RefundOwner::Identity(Identifier::from([2u8; 32]));
            let bucket = bucket_owner(3, 4);

            let refunds = FeeRefunds(
                CreditsPerEpochByIdentifier::from_iter([
                    (payer.removal_key(), CreditsPerEpoch::from_iter([(0, 100)])),
                    (
                        other.removal_key(),
                        CreditsPerEpoch::from_iter([(0, 20), (1, 30)]),
                    ),
                    (bucket.removal_key(), CreditsPerEpoch::from_iter([(2, 7)])),
                ]),
                RefundOwnersByIdentifier::from_iter([
                    (payer.removal_key(), payer),
                    (other.removal_key(), other),
                    (bucket.removal_key(), bucket),
                ]),
            );

            let others = refunds
                .calculate_all_refunds_except_owner(&payer)
                .expect("should sum");

            assert_eq!(others, BTreeMap::from_iter([(other, 50), (bucket, 7)]));

            // the identity view keeps its historical shape for identity keys
            assert_eq!(
                refunds.calculate_refunds_amount_for_identity(Identifier::from([2u8; 32])),
                Some(50)
            );
            let by_identity =
                refunds.calculate_all_refunds_except_identity(Identifier::from([1u8; 32]));
            assert_eq!(by_identity.get(&Identifier::from([2u8; 32])), Some(&50));
        }

        #[test]
        fn should_fail_closed_on_a_carrier_key_without_a_recorded_owner() {
            let refunds = FeeRefunds(
                CreditsPerEpochByIdentifier::from_iter([(
                    [4u8; 32],
                    CreditsPerEpoch::from_iter([(0, 100)]),
                )]),
                RefundOwnersByIdentifier::new(),
            );

            let entries: Vec<_> = refunds.iter_typed().collect();
            assert_eq!(entries.len(), 1);
            assert!(matches!(
                entries[0],
                Err(ProtocolError::CorruptedCodeExecution(_))
            ));
            assert!(matches!(
                refunds.calculate_all_refunds_except_owner(&bucket_owner(1, 1)),
                Err(ProtocolError::CorruptedCodeExecution(_))
            ));
        }

        #[test]
        fn should_guard_the_identity_keyed_view_against_bucket_owned_refunds() {
            let identity = RefundOwner::Identity(Identifier::from([1u8; 32]));
            let bucket = bucket_owner(3, 4);
            let owners = RefundOwnersByIdentifier::from_iter([
                (identity.removal_key(), identity),
                (bucket.removal_key(), bucket),
            ]);
            let removal = BytesPerEpochByIdentifier::from_iter([
                (identity.removal_key(), IntMap::from_iter([(0, 100)])),
                (bucket.removal_key(), IntMap::from_iter([(0, 200)])),
            ]);
            let typed = FeeRefunds::from_typed_storage_removal(
                removal,
                &owners,
                3,
                20,
                &EPOCH_CHANGE_FEE_VERSION_TEST,
            )
            .expect("should create fee refunds");

            assert!(matches!(
                typed.ensure_identity_owners_only(),
                Err(ProtocolError::CorruptedCodeExecution(_))
            ));

            let identities_only = FeeRefunds::from_storage_removal(
                BytesPerEpochByIdentifier::from_iter([(
                    identity.removal_key(),
                    IntMap::from_iter([(0, 100)]),
                )]),
                3,
                20,
                &EPOCH_CHANGE_FEE_VERSION_TEST,
            )
            .expect("should create fee refunds");
            identities_only
                .ensure_identity_owners_only()
                .expect("identity owners pass");

            let unowned = FeeRefunds(
                CreditsPerEpochByIdentifier::from_iter([(
                    [4u8; 32],
                    CreditsPerEpoch::from_iter([(0, 100)]),
                )]),
                RefundOwnersByIdentifier::new(),
            );
            assert!(matches!(
                unowned.ensure_identity_owners_only(),
                Err(ProtocolError::CorruptedCodeExecution(_))
            ));
        }

        #[test]
        fn should_sum_per_epoch_across_owners_of_both_kinds() {
            let identity = RefundOwner::Identity(Identifier::from([1u8; 32]));
            let bucket = bucket_owner(3, 4);
            let refunds = FeeRefunds(
                CreditsPerEpochByIdentifier::from_iter([
                    (
                        identity.removal_key(),
                        CreditsPerEpoch::from_iter([(0, 100), (1, 1)]),
                    ),
                    (bucket.removal_key(), CreditsPerEpoch::from_iter([(0, 7)])),
                ]),
                RefundOwnersByIdentifier::from_iter([
                    (identity.removal_key(), identity),
                    (bucket.removal_key(), bucket),
                ]),
            );

            assert_eq!(
                refunds.sum_per_epoch(),
                CreditsPerEpoch::from_iter([(0, 107), (1, 1)])
            );
        }
    }
}
