use super::for_saving_v1::QuorumForSavingV1;
use super::quorum_set::PreviousPastQuorumsV0;
use crate::platform_types::platform_state::platform_state_for_saving::PlatformStateForSaving;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::signature_verification_quorum_set::{
    Quorums, SignatureVerificationQuorumSet, SignatureVerificationQuorumSetForSaving,
    SignatureVerificationQuorumSetV0Methods, VerificationQuorum,
};
use dpp::bls::PublicKey;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::QuorumHash;
use dpp::dashcore_rpc::json::QuorumType;
use dpp::serialization::PlatformDeserializableFromVersionedStructureTrusted;
use dpp::version::PlatformVersion;

const KEYS: [&str; 4] = [
    "97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb",
    "a572cbea904d67468808c8eb50a9450c9721db309128012543902d0ac358a62ae28f75bb8f1c7c42c39a8c5529bf0f4e",
    "95fde78acd5f6886ddaf5d0056610167c513d09c1c0efabbc7cdcc69beea113779c4a81e2d24daafc5387dbf6ac5fe48",
    "b7f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb",
];

fn config() -> bincode::config::Configuration<
    bincode::config::BigEndian,
    bincode::config::Varint,
    bincode::config::NoLimit,
> {
    bincode::config::standard()
        .with_big_endian()
        .with_no_limit()
}

fn assert_quorum(quorums: &Quorums<VerificationQuorum>, hash: u8, key: usize, index: Option<u32>) {
    let quorum = quorums
        .get(&QuorumHash::from_byte_array([hash; 32]))
        .unwrap();
    assert_eq!(
        quorum.public_key.to_bytes().as_slice(),
        hex::decode(KEYS[key]).unwrap()
    );
    assert_eq!(quorum.index, index);
}

fn assert_quorums(set: &SignatureVerificationQuorumSet) {
    assert_eq!(set.config().quorum_type, QuorumType::Llmq400_60);
    assert_eq!(set.config().active_signers, 4);
    assert_eq!(set.config().window, 288);
    assert!(!set.config().rotation);
    assert_eq!(set.current_quorums().len(), 2);
    assert_quorum(set.current_quorums(), 0x11, 0, None);
    assert_quorum(set.current_quorums(), 0x22, 1, Some(0));
    let SignatureVerificationQuorumSet::V0(runtime) = set;
    let PreviousPastQuorumsV0 {
        quorums,
        last_active_core_height,
        updated_at_core_height,
        previous_change_height,
    } = runtime.previous.as_ref().unwrap();
    assert_eq!(
        (
            *last_active_core_height,
            *updated_at_core_height,
            *previous_change_height
        ),
        (1000, 1008, Some(900))
    );
    assert_eq!(quorums.len(), 2);
    assert_quorum(quorums, 0x33, 2, Some(1));
    assert_quorum(quorums, 0x44, 3, None);
}

#[test]
fn should_preserve_current_and_previous_quorums_in_old_v1_and_v2_records() {
    for fixture in [
        include_str!("fixtures/quorum-storage-v1.hex"),
        include_str!("fixtures/quorum-storage-v2.hex"),
    ] {
        let bytes = hex::decode(fixture.trim()).unwrap();
        let (stored, read): (SignatureVerificationQuorumSetForSaving, _) =
            bincode::decode_from_slice(&bytes, config()).unwrap();
        assert_eq!(read, bytes.len());
        assert_eq!(bincode::encode_to_vec(&stored, config()).unwrap(), bytes);
        assert_quorums(&stored.into());
    }
}

#[test]
fn should_load_old_saved_state_and_checkpoint_with_both_quorum_sets() {
    for fixture in [
        include_str!("fixtures/platform-state-v1.hex"),
        include_str!("fixtures/checkpoint-platform-state.hex"),
        include_str!("fixtures/platform-state-v2.hex"),
    ] {
        let bytes = hex::decode(fixture.trim()).unwrap();
        let (stored, read): (PlatformStateForSaving, _) =
            bincode::decode_from_slice(&bytes, config()).unwrap();
        assert_eq!(read, bytes.len());
        assert_eq!(bincode::encode_to_vec(&stored, config()).unwrap(), bytes);
        let state = match stored {
            PlatformStateForSaving::V2(record) => {
                record.into_platform_state(vec![], vec![]).unwrap()
            }
            _ => PlatformState::versioned_deserialize_trusted(&bytes, PlatformVersion::latest())
                .unwrap(),
        };
        assert_eq!(state.current_protocol_version_in_consensus, 13);
        assert_eq!(state.next_epoch_protocol_version, 14);
        assert_quorums(&state.chain_lock_validating_quorums);
        assert_quorums(&state.instant_lock_validating_quorums);
        if bytes[0] == 1 {
            assert_eq!(state.serialize_standalone_to_bytes().unwrap(), bytes);
        }
    }
}

fn saved_quorum(key: PublicKey) -> QuorumForSavingV1 {
    let quorums = Quorums::from_iter([(
        QuorumHash::from_byte_array([0x11; 32]),
        VerificationQuorum {
            public_key: key,
            index: Some(1000),
        },
    )]);
    Vec::<QuorumForSavingV1>::from(quorums).pop().unwrap()
}

#[test]
fn should_store_exactly_48_key_bytes_without_a_prefix_and_preserve_the_following_index() {
    for key in [
        PublicKey::try_from(hex::decode(KEYS[0]).unwrap().as_slice()).unwrap(),
        PublicKey::default(),
    ] {
        let stored = saved_quorum(key);
        let mut expected = vec![0x11; 32];
        expected.extend_from_slice(&key.to_bytes());
        expected.extend_from_slice(&[1, 251, 3, 232]); // Some(1000), big-endian varint.
        assert_eq!(bincode::encode_to_vec(&stored, config()).unwrap(), expected);
        let (decoded, read): (QuorumForSavingV1, _) =
            bincode::decode_from_slice(&expected, config()).unwrap();
        assert_eq!(read, expected.len());
        let quorums: Quorums<VerificationQuorum> = vec![decoded].into();
        let quorum = quorums
            .get(&QuorumHash::from_byte_array([0x11; 32]))
            .unwrap();
        assert_eq!(quorum.public_key, key);
        assert_eq!(quorum.index, Some(1000));
    }
}

#[test]
fn should_reject_truncated_and_invalid_stored_quorum_keys() {
    let bytes = bincode::encode_to_vec(saved_quorum(PublicKey::default()), config()).unwrap();
    for len in 32..80 {
        assert!(
            bincode::decode_from_slice::<QuorumForSavingV1, _>(&bytes[..len], config()).is_err()
        );
    }
    let mut invalid = bytes;
    invalid[32..80].fill(0);
    assert!(bincode::decode_from_slice::<QuorumForSavingV1, _>(&invalid, config()).is_err());
}
