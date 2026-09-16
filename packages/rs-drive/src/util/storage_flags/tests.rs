//! Tests for the Drive storage flags: codec, split, combine and the batch
//! closure entry points.

use super::*;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::fee::refund_owner::{RefundOwnersByIdentifier, SYSTEM_REFUND_CARRIER_KEY};
use grovedb_costs::storage_cost::removal::StorageRemovalPerEpochByIdentifier;
use grovedb_costs::storage_cost::removal::StorageRemovedBytes;
use grovedb_costs::storage_cost::removal::StorageRemovedBytes::{
    BasicStorageRemoval, NoStorageRemoval, SectionedStorageRemoval,
};
use grovedb_costs::storage_cost::StorageCost;
use grovedb_epoch_based_storage_flags::error::StorageFlagsError;
use intmap::IntMap;

const CONTRACT_ID: ContractId = [0x11; 32];
const OWNER_ID: OwnerId = [0x22; 32];

fn bucket_owner(position: ContractCreditBucketPosition) -> RefundOwner {
    RefundOwner::ContractBucket {
        contract_id: Identifier::from(CONTRACT_ID),
        position,
    }
}

fn epochs(entries: &[(EpochIndex, BytesAddedInEpoch)]) -> BTreeMap<EpochIndex, BytesAddedInEpoch> {
    entries.iter().copied().collect()
}

fn every_variant() -> Vec<StorageFlags> {
    vec![
        SingleEpoch(3),
        MultiEpoch(3, epochs(&[(4, 10), (5, 300)])),
        SingleEpochOwned(3, OWNER_ID),
        MultiEpochOwned(3, epochs(&[(4, 10), (5, 300)]), OWNER_ID),
        SingleEpochContractBucket(3, CONTRACT_ID, 7),
        MultiEpochContractBucket(3, epochs(&[(4, 10), (5, 300)]), CONTRACT_ID, 7),
        MultiEpochContractBucket(0, epochs(&[(65535, u32::MAX)]), CONTRACT_ID, u16::MAX),
    ]
}

fn sectioned(key: [u8; 32], entries: &[(u16, u32)]) -> StorageRemovedBytes {
    let mut map = StorageRemovalPerEpochByIdentifier::new();
    map.insert(key, IntMap::from_iter(entries.iter().copied()));
    SectionedStorageRemoval(map)
}

mod codec {
    use super::*;

    #[test]
    fn should_round_trip_every_variant() {
        for flags in every_variant() {
            let bytes = flags.serialize();
            let decoded = StorageFlags::deserialize(&bytes)
                .expect("should decode")
                .expect("should be some");
            assert_eq!(decoded, flags, "{}", flags);
            assert_eq!(bytes.first().copied(), Some(flags.type_byte()));
        }
    }

    #[test]
    fn should_encode_the_historical_variants_exactly_as_the_crate() {
        let pairs = [
            (SingleEpoch(3), CrateStorageFlags::SingleEpoch(3)),
            (
                MultiEpoch(3, epochs(&[(4, 10)])),
                CrateStorageFlags::MultiEpoch(3, epochs(&[(4, 10)])),
            ),
            (
                SingleEpochOwned(3, OWNER_ID),
                CrateStorageFlags::SingleEpochOwned(3, OWNER_ID),
            ),
            (
                MultiEpochOwned(3, epochs(&[(4, 10), (5, 300)]), OWNER_ID),
                CrateStorageFlags::MultiEpochOwned(3, epochs(&[(4, 10), (5, 300)]), OWNER_ID),
            ),
        ];
        for (ours, theirs) in pairs {
            assert_eq!(ours.serialize(), theirs.serialize(), "{}", ours);
            assert_eq!(ours.serialized_size(), theirs.serialized_size());
            assert_eq!(
                StorageFlags::deserialize(&theirs.serialize()).expect("should decode"),
                Some(ours)
            );
        }
    }

    #[test]
    fn should_pin_the_byte_layout_of_every_variant() {
        assert_eq!(SingleEpoch(0x0102).serialize(), vec![0, 1, 2]);
        assert_eq!(
            MultiEpoch(0x0102, epochs(&[(0x0304, 300)])).serialize(),
            vec![1, 1, 2, 3, 4, 0xac, 0x02]
        );

        let mut owned = vec![2];
        owned.extend_from_slice(&OWNER_ID);
        owned.extend_from_slice(&[1, 2]);
        assert_eq!(SingleEpochOwned(0x0102, OWNER_ID).serialize(), owned);
        assert_eq!(owned.len(), 35);

        let mut multi_owned = vec![3];
        multi_owned.extend_from_slice(&OWNER_ID);
        multi_owned.extend_from_slice(&[1, 2, 3, 4, 0xac, 0x02]);
        assert_eq!(
            MultiEpochOwned(0x0102, epochs(&[(0x0304, 300)]), OWNER_ID).serialize(),
            multi_owned
        );

        let mut bucket = vec![4];
        bucket.extend_from_slice(&CONTRACT_ID);
        bucket.extend_from_slice(&[0, 7]);
        bucket.extend_from_slice(&[1, 2]);
        assert_eq!(
            SingleEpochContractBucket(0x0102, CONTRACT_ID, 7).serialize(),
            bucket
        );
        assert_eq!(bucket.len(), 37);

        let mut multi_bucket = vec![5];
        multi_bucket.extend_from_slice(&CONTRACT_ID);
        multi_bucket.extend_from_slice(&[0, 7]);
        multi_bucket.extend_from_slice(&[1, 2, 3, 4, 0xac, 0x02]);
        assert_eq!(
            MultiEpochContractBucket(0x0102, epochs(&[(0x0304, 300)]), CONTRACT_ID, 7).serialize(),
            multi_bucket
        );
    }

    #[test]
    fn should_report_the_serialized_size_of_every_variant() {
        for flags in every_variant() {
            assert_eq!(
                flags.serialized_size() as usize,
                flags.serialize().len(),
                "{}",
                flags
            );
        }
    }

    #[test]
    fn should_reject_an_unknown_type_byte() {
        let mut bytes = vec![6];
        bytes.extend_from_slice(&[0; 36]);
        let error = StorageFlags::deserialize(&bytes).expect_err("should reject");
        assert!(matches!(
            error,
            StorageFlagsError::DeserializeUnknownStorageFlagsType(_)
        ));
        assert!(matches!(
            StorageFlags::deserialize(&[255, 1, 2]),
            Err(StorageFlagsError::DeserializeUnknownStorageFlagsType(_))
        ));
    }

    #[test]
    fn should_reject_truncated_and_oversized_bucket_flags() {
        let single = SingleEpochContractBucket(3, CONTRACT_ID, 7).serialize();
        for cut in [1usize, 32, 33, 34, 35, 36] {
            assert!(
                matches!(
                    StorageFlags::deserialize(&single[..cut]),
                    Err(StorageFlagsError::StorageFlagsWrongSize(_))
                ),
                "single bucket cut at {}",
                cut
            );
        }
        let mut too_long = single.clone();
        too_long.push(0);
        assert!(matches!(
            StorageFlags::deserialize(&too_long),
            Err(StorageFlagsError::StorageFlagsWrongSize(_))
        ));

        let multi = MultiEpochContractBucket(3, epochs(&[(4, 300)]), CONTRACT_ID, 7).serialize();
        // header only, header plus one epoch byte, and a cut inside the varint
        for cut in [37usize, 38, 39, multi.len() - 1] {
            assert!(
                matches!(
                    StorageFlags::deserialize(&multi[..cut]),
                    Err(StorageFlagsError::StorageFlagsWrongSize(_))
                ),
                "multi bucket cut at {}",
                cut
            );
        }
        // a dangling epoch index with no byte count
        let mut dangling = multi.clone();
        dangling.extend_from_slice(&[0, 9]);
        assert!(matches!(
            StorageFlags::deserialize(&dangling),
            Err(StorageFlagsError::StorageFlagsWrongSize(_))
        ));
    }

    #[test]
    fn should_decode_empty_flags_as_none() {
        assert_eq!(StorageFlags::deserialize(&[]).expect("should decode"), None);
        assert_eq!(
            StorageFlags::map_some_element_flags_ref(&None).expect("should decode"),
            None
        );
    }

    #[test]
    fn should_size_typed_owners_for_estimation() {
        let identity = RefundOwner::Identity(Identifier::from(OWNER_ID));
        let bucket = bucket_owner(7);

        assert_eq!(
            StorageFlags::approximate_size_for_owner(None, None),
            SingleEpoch(0).serialized_size()
        );
        assert_eq!(
            StorageFlags::approximate_size_for_owner(Some(&identity), None),
            SingleEpochOwned(0, OWNER_ID).serialized_size()
        );
        assert_eq!(
            StorageFlags::approximate_size_for_owner(Some(&bucket), None),
            SingleEpochContractBucket(0, CONTRACT_ID, 7).serialized_size()
        );
        assert_eq!(
            StorageFlags::approximate_size_for_owner(Some(&identity), Some((2, 1))),
            StorageFlags::approximate_size(true, Some((2, 1)))
        );
        assert_eq!(
            StorageFlags::approximate_size_for_owner(Some(&bucket), Some((2, 1))),
            37 + 2 * 3
        );
    }

    #[test]
    fn should_expose_the_typed_owner_of_every_variant() {
        let identity = RefundOwner::Identity(Identifier::from(OWNER_ID));
        let bucket = bucket_owner(7);

        assert_eq!(SingleEpoch(1).refund_owner(), None);
        assert_eq!(MultiEpoch(1, epochs(&[(2, 3)])).refund_owner(), None);
        assert_eq!(SingleEpochOwned(1, OWNER_ID).refund_owner(), Some(identity));
        assert_eq!(
            MultiEpochOwned(1, epochs(&[(2, 3)]), OWNER_ID).refund_owner(),
            Some(identity)
        );
        assert_eq!(
            SingleEpochContractBucket(1, CONTRACT_ID, 7).refund_owner(),
            Some(bucket)
        );
        assert_eq!(
            MultiEpochContractBucket(1, epochs(&[(2, 3)]), CONTRACT_ID, 7).refund_owner(),
            Some(bucket)
        );

        // the identity accessor never reads a bucket as an identity
        assert_eq!(
            SingleEpochContractBucket(1, CONTRACT_ID, 7).owner_id(),
            None
        );
        assert_eq!(SingleEpochOwned(1, OWNER_ID).owner_id(), Some(&OWNER_ID));

        assert_eq!(
            StorageFlags::new_single_epoch_for_owner(9, Some(bucket)),
            SingleEpochContractBucket(9, CONTRACT_ID, 7)
        );
        assert_eq!(
            StorageFlags::new_single_epoch_for_owner(9, Some(identity)),
            SingleEpochOwned(9, OWNER_ID)
        );
        assert_eq!(
            StorageFlags::new_single_epoch_for_owner(9, None),
            SingleEpoch(9)
        );
    }
}

mod split {
    use super::*;

    #[test]
    fn should_section_a_deleted_bucket_element_under_its_carrier_key_and_record_the_owner() {
        let owner = bucket_owner(7);
        let mut flags = SingleEpochContractBucket(5, CONTRACT_ID, 7).to_element_flags();
        let mut refund_owners = RefundOwnersByIdentifier::new();

        let (key_removal, value_removal) =
            StorageFlags::split_removal_bytes_typed(&mut flags, 50, 150, &mut refund_owners)
                .expect("should split");

        assert_eq!(key_removal, sectioned(owner.removal_key(), &[(5, 50)]));
        assert_eq!(value_removal, sectioned(owner.removal_key(), &[(5, 150)]));
        assert_eq!(
            refund_owners,
            RefundOwnersByIdentifier::from([(owner.removal_key(), owner)])
        );
    }

    #[test]
    fn should_take_from_the_latest_epochs_first_when_a_bucket_element_shrinks() {
        let owner = bucket_owner(7);
        let flags = MultiEpochContractBucket(5, epochs(&[(6, 300), (7, 400)]), CONTRACT_ID, 7);
        let mut refund_owners = RefundOwnersByIdentifier::new();

        let (key_removal, value_removal) = StorageFlags::split_removal_bytes_typed(
            &mut flags.to_element_flags(),
            0,
            700,
            &mut refund_owners,
        )
        .expect("should split");

        assert_eq!(key_removal, NoStorageRemoval);
        // the same LIFO sectioning the crate applies to identity-owned flags
        assert_eq!(
            value_removal,
            sectioned(owner.removal_key(), &[(5, 6), (6, 297), (7, 397)])
        );
        assert_eq!(refund_owners.get(&owner.removal_key()), Some(&owner));

        let crate_equivalent = CrateStorageFlags::MultiEpochOwned(
            5,
            epochs(&[(6, 300), (7, 400)]),
            owner.removal_key(),
        )
        .split_storage_removed_bytes(0, 700);
        assert_eq!(value_removal, crate_equivalent.1);
    }

    #[test]
    fn should_record_an_identity_owner() {
        let owner = RefundOwner::Identity(Identifier::from(OWNER_ID));
        let mut refund_owners = RefundOwnersByIdentifier::new();

        let (key_removal, value_removal) = StorageFlags::split_removal_bytes_typed(
            &mut SingleEpochOwned(2, OWNER_ID).to_element_flags(),
            10,
            20,
            &mut refund_owners,
        )
        .expect("should split");

        assert_eq!(key_removal, sectioned(OWNER_ID, &[(2, 10)]));
        assert_eq!(value_removal, sectioned(OWNER_ID, &[(2, 20)]));
        assert_eq!(
            refund_owners,
            RefundOwnersByIdentifier::from([(OWNER_ID, owner)])
        );
    }

    #[test]
    fn should_section_unowned_bytes_under_the_system_key_and_record_nothing() {
        let mut refund_owners = RefundOwnersByIdentifier::new();

        let (key_removal, value_removal) = StorageFlags::split_removal_bytes_typed(
            &mut SingleEpoch(2).to_element_flags(),
            10,
            20,
            &mut refund_owners,
        )
        .expect("should split");

        assert_eq!(
            key_removal,
            sectioned(SYSTEM_REFUND_CARRIER_KEY, &[(2, 10)])
        );
        assert_eq!(
            value_removal,
            sectioned(SYSTEM_REFUND_CARRIER_KEY, &[(2, 20)])
        );
        assert!(refund_owners.is_empty());

        let (key_removal, value_removal) =
            StorageFlags::split_removal_bytes_typed(&mut vec![], 10, 20, &mut refund_owners)
                .expect("should split");
        assert_eq!(key_removal, BasicStorageRemoval(10));
        assert_eq!(value_removal, BasicStorageRemoval(20));
        assert!(refund_owners.is_empty());
    }

    #[test]
    fn should_keep_the_all_zero_identity_as_system_bytes_and_record_nothing() {
        let mut refund_owners = RefundOwnersByIdentifier::new();

        let (key_removal, _) = StorageFlags::split_removal_bytes_typed(
            &mut SingleEpochOwned(2, SYSTEM_REFUND_CARRIER_KEY).to_element_flags(),
            10,
            20,
            &mut refund_owners,
        )
        .expect("should split");

        assert_eq!(
            key_removal,
            sectioned(SYSTEM_REFUND_CARRIER_KEY, &[(2, 10)])
        );
        assert!(refund_owners.is_empty());
    }

    #[test]
    fn should_reject_one_carrier_key_recorded_for_two_owners_within_a_batch() {
        let owner = bucket_owner(7);
        let mut refund_owners = RefundOwnersByIdentifier::from([(
            owner.removal_key(),
            RefundOwner::Identity(Identifier::from(owner.removal_key())),
        )]);

        let result = StorageFlags::split_removal_bytes_typed(
            &mut SingleEpochContractBucket(5, CONTRACT_ID, 7).to_element_flags(),
            1,
            1,
            &mut refund_owners,
        );

        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::CorruptedCodeExecution(_)))
        ));
    }

    #[test]
    fn should_keep_the_same_owner_recorded_once_across_two_elements() {
        let owner = bucket_owner(7);
        let mut refund_owners = RefundOwnersByIdentifier::new();
        for _ in 0..2 {
            StorageFlags::split_removal_bytes_typed(
                &mut SingleEpochContractBucket(5, CONTRACT_ID, 7).to_element_flags(),
                1,
                1,
                &mut refund_owners,
            )
            .expect("should split");
        }
        assert_eq!(refund_owners.len(), 1);
        assert_eq!(refund_owners.get(&owner.removal_key()), Some(&owner));
    }

    #[test]
    fn should_fail_closed_on_bucket_flags_through_the_shipped_closure_entry_points() {
        let mut flags = SingleEpochContractBucket(5, CONTRACT_ID, 7).to_element_flags();
        assert!(matches!(
            StorageFlags::split_removal_bytes(&mut flags, 1, 1),
            Err(StorageFlagsError::DeserializeUnknownStorageFlagsType(_))
        ));

        let cost = StorageCost {
            added_bytes: 10,
            replaced_bytes: 1,
            removed_bytes: NoStorageRemoval,
        };
        let mut new_flags = SingleEpochContractBucket(6, CONTRACT_ID, 7).to_element_flags();
        assert!(matches!(
            StorageFlags::update_element_flags(
                &cost,
                Some(SingleEpochContractBucket(5, CONTRACT_ID, 7).to_element_flags()),
                &mut new_flags
            ),
            Err(StorageFlagsError::DeserializeUnknownStorageFlagsType(_))
        ));

        // the shipped entry points still work for the historical variants
        let mut owned = SingleEpochOwned(5, OWNER_ID).to_element_flags();
        let (key_removal, _) =
            StorageFlags::split_removal_bytes(&mut owned, 3, 4).expect("should split");
        assert_eq!(key_removal, sectioned(OWNER_ID, &[(5, 3)]));
    }
}

mod combine {
    use super::*;

    #[test]
    fn should_keep_the_bucket_owner_when_combining_the_same_base_epoch() {
        let ours = SingleEpochContractBucket(1, CONTRACT_ID, 7);
        let theirs = SingleEpochContractBucket(1, CONTRACT_ID, 7);

        let combined = ours
            .combine_added_bytes(theirs, 10, MergingOwnersStrategy::RaiseIssue)
            .expect("should combine");

        assert_eq!(combined, SingleEpochContractBucket(1, CONTRACT_ID, 7));
    }

    #[test]
    fn should_transfer_ownership_across_kinds_with_use_theirs() {
        let identity_to_bucket = SingleEpochOwned(1, OWNER_ID)
            .combine_added_bytes(
                SingleEpochContractBucket(1, CONTRACT_ID, 7),
                10,
                MergingOwnersStrategy::UseTheirs,
            )
            .expect("should combine");
        assert_eq!(
            identity_to_bucket,
            SingleEpochContractBucket(1, CONTRACT_ID, 7)
        );

        let bucket_to_identity = SingleEpochContractBucket(1, CONTRACT_ID, 7)
            .combine_added_bytes(
                SingleEpochOwned(1, OWNER_ID),
                10,
                MergingOwnersStrategy::UseTheirs,
            )
            .expect("should combine");
        assert_eq!(bucket_to_identity, SingleEpochOwned(1, OWNER_ID));

        let kept_ours = SingleEpochContractBucket(1, CONTRACT_ID, 7)
            .combine_added_bytes(
                SingleEpochOwned(1, OWNER_ID),
                10,
                MergingOwnersStrategy::UseOurs,
            )
            .expect("should combine");
        assert_eq!(kept_ours, SingleEpochContractBucket(1, CONTRACT_ID, 7));
    }

    #[test]
    fn should_raise_an_issue_across_kinds_and_across_buckets() {
        assert!(matches!(
            SingleEpochOwned(1, OWNER_ID).combine_added_bytes(
                SingleEpochContractBucket(1, CONTRACT_ID, 7),
                10,
                MergingOwnersStrategy::RaiseIssue,
            ),
            Err(StorageFlagsError::MergingStorageFlagsFromDifferentOwners(_))
        ));
        assert!(matches!(
            SingleEpochContractBucket(1, CONTRACT_ID, 7).combine_added_bytes(
                SingleEpochContractBucket(1, CONTRACT_ID, 8),
                10,
                MergingOwnersStrategy::RaiseIssue,
            ),
            Err(StorageFlagsError::MergingStorageFlagsFromDifferentOwners(_))
        ));
    }

    #[test]
    fn should_never_merge_an_identity_whose_id_equals_a_bucket_carrier_key() {
        let bucket = bucket_owner(7);
        let colliding_identity = SingleEpochOwned(1, bucket.removal_key());

        let result = colliding_identity.combine_added_bytes(
            SingleEpochContractBucket(1, CONTRACT_ID, 7),
            10,
            MergingOwnersStrategy::UseTheirs,
        );

        assert!(matches!(
            result,
            Err(StorageFlagsError::MergingStorageFlagsFromDifferentOwners(_))
        ));
    }

    #[test]
    fn should_keep_the_typed_owner_when_adding_bytes_in_a_higher_epoch() {
        let combined = SingleEpochContractBucket(1, CONTRACT_ID, 7)
            .combine_added_bytes(
                SingleEpochContractBucket(2, CONTRACT_ID, 7),
                10,
                MergingOwnersStrategy::UseTheirs,
            )
            .expect("should combine");
        assert_eq!(
            combined,
            MultiEpochContractBucket(1, epochs(&[(2, 10)]), CONTRACT_ID, 7)
        );

        let again = combined
            .combine_added_bytes(
                SingleEpochContractBucket(2, CONTRACT_ID, 7),
                5,
                MergingOwnersStrategy::UseTheirs,
            )
            .expect("should combine");
        assert_eq!(
            again,
            MultiEpochContractBucket(1, epochs(&[(2, 15)]), CONTRACT_ID, 7)
        );

        // same arithmetic as the crate performs for an identity owner
        let crate_combined = CrateStorageFlags::SingleEpochOwned(1, OWNER_ID)
            .combine_added_bytes(
                CrateStorageFlags::SingleEpochOwned(2, OWNER_ID),
                10,
                MergingOwnersStrategy::UseTheirs,
            )
            .expect("should combine");
        assert_eq!(
            crate_combined,
            CrateStorageFlags::MultiEpochOwned(1, epochs(&[(2, 10)]), OWNER_ID)
        );
    }

    #[test]
    fn should_keep_the_typed_owner_when_removing_bytes_in_a_higher_epoch() {
        let owner = bucket_owner(7);
        let flags = MultiEpochContractBucket(1, epochs(&[(2, 20)]), CONTRACT_ID, 7);

        let removed = sectioned(owner.removal_key(), &[(2, 5)]);
        let combined = flags
            .clone()
            .combine_removed_bytes(
                SingleEpochContractBucket(2, CONTRACT_ID, 7),
                &removed,
                MergingOwnersStrategy::UseTheirs,
            )
            .expect("should combine");
        assert_eq!(
            combined,
            MultiEpochContractBucket(1, epochs(&[(2, 15)]), CONTRACT_ID, 7)
        );

        // removing the whole epoch collapses back to the single epoch variant
        let removed_all = sectioned(owner.removal_key(), &[(2, 18)]);
        let collapsed = flags
            .combine_removed_bytes(
                SingleEpochContractBucket(2, CONTRACT_ID, 7),
                &removed_all,
                MergingOwnersStrategy::UseTheirs,
            )
            .expect("should combine");
        assert_eq!(collapsed, SingleEpochContractBucket(1, CONTRACT_ID, 7));
    }

    #[test]
    fn should_reject_a_newer_base_epoch_merging_into_an_older_one() {
        assert!(matches!(
            SingleEpochContractBucket(2, CONTRACT_ID, 7).combine_added_bytes(
                SingleEpochContractBucket(1, CONTRACT_ID, 7),
                10,
                MergingOwnersStrategy::UseTheirs,
            ),
            Err(StorageFlagsError::MergingStorageFlagsWithDifferentBaseEpoch(_))
        ));
    }
}

mod update {
    use super::*;

    fn bigger(added_bytes: u32) -> StorageCost {
        StorageCost {
            added_bytes,
            replaced_bytes: 1,
            removed_bytes: NoStorageRemoval,
        }
    }

    #[test]
    fn should_behave_like_the_crate_for_identity_owned_flags() {
        let old = SingleEpochOwned(1, OWNER_ID).to_element_flags();
        let mut typed_new = SingleEpochOwned(2, OWNER_ID).to_element_flags();
        let mut crate_new = typed_new.clone();

        let typed_changed = StorageFlags::update_element_flags_typed(
            &bigger(10),
            Some(old.clone()),
            &mut typed_new,
        )
        .expect("should update");
        let crate_changed =
            CrateStorageFlags::update_element_flags(&bigger(10), Some(old), &mut crate_new)
                .expect("should update");

        assert_eq!(typed_changed, crate_changed);
        assert_eq!(typed_new, crate_new);
        assert_eq!(
            StorageFlags::deserialize(&typed_new).expect("should decode"),
            Some(MultiEpochOwned(1, epochs(&[(2, 10)]), OWNER_ID))
        );
    }

    #[test]
    fn should_grow_a_bucket_owned_element_into_a_multi_epoch_bucket() {
        let old = SingleEpochContractBucket(1, CONTRACT_ID, 7).to_element_flags();
        let mut new_flags = SingleEpochContractBucket(2, CONTRACT_ID, 7).to_element_flags();

        let changed =
            StorageFlags::update_element_flags_typed(&bigger(10), Some(old), &mut new_flags)
                .expect("should update");

        assert!(changed);
        assert_eq!(
            StorageFlags::deserialize(&new_flags).expect("should decode"),
            Some(MultiEpochContractBucket(
                1,
                epochs(&[(2, 10)]),
                CONTRACT_ID,
                7
            ))
        );
    }

    #[test]
    fn should_transfer_a_replaced_element_to_the_new_typed_owner() {
        let old = SingleEpochOwned(1, OWNER_ID).to_element_flags();
        let mut new_flags = SingleEpochContractBucket(1, CONTRACT_ID, 7).to_element_flags();

        let changed =
            StorageFlags::update_element_flags_typed(&bigger(10), Some(old), &mut new_flags)
                .expect("should update");

        assert!(!changed, "new flags already name the new owner");
        assert_eq!(
            StorageFlags::deserialize(&new_flags).expect("should decode"),
            Some(SingleEpochContractBucket(1, CONTRACT_ID, 7))
        );
    }

    #[test]
    fn should_shrink_a_bucket_owned_element_against_its_sectioned_removal() {
        let owner = bucket_owner(7);
        let old =
            MultiEpochContractBucket(1, epochs(&[(2, 20)]), CONTRACT_ID, 7).to_element_flags();
        let mut new_flags = SingleEpochContractBucket(2, CONTRACT_ID, 7).to_element_flags();
        let cost = StorageCost {
            added_bytes: 0,
            replaced_bytes: 1,
            removed_bytes: sectioned(owner.removal_key(), &[(2, 5)]),
        };

        let changed = StorageFlags::update_element_flags_typed(&cost, Some(old), &mut new_flags)
            .expect("should update");

        assert!(changed);
        assert_eq!(
            StorageFlags::deserialize(&new_flags).expect("should decode"),
            Some(MultiEpochContractBucket(
                1,
                epochs(&[(2, 15)]),
                CONTRACT_ID,
                7
            ))
        );
    }

    #[test]
    fn should_keep_old_flags_on_a_same_size_update_and_pass_through_inserts() {
        let old = SingleEpochContractBucket(9, CONTRACT_ID, 7).to_element_flags();
        let mut new_flags = SingleEpochContractBucket(1, CONTRACT_ID, 7).to_element_flags();
        let same_size = StorageCost {
            added_bytes: 0,
            replaced_bytes: 1,
            removed_bytes: NoStorageRemoval,
        };
        let changed =
            StorageFlags::update_element_flags_typed(&same_size, Some(old.clone()), &mut new_flags)
                .expect("should update");
        assert!(changed);
        assert_eq!(new_flags, old);

        let mut inserted = SingleEpochContractBucket(1, CONTRACT_ID, 7).to_element_flags();
        let changed = StorageFlags::update_element_flags_typed(&bigger(10), None, &mut inserted)
            .expect("should update");
        assert!(!changed);
    }

    #[test]
    fn should_reject_removing_flags_from_a_flagged_element() {
        let old = SingleEpochContractBucket(1, CONTRACT_ID, 7).to_element_flags();
        let mut new_flags = vec![];
        let result =
            StorageFlags::update_element_flags_typed(&bigger(10), Some(old), &mut new_flags);
        assert!(matches!(
            result,
            Err(Error::StorageFlags(StorageFlagsError::RemovingFlagsError(
                _
            )))
        ));
    }
}
