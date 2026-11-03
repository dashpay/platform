use bincode::config;
use dpp::address_funds::AddressWitness;
use dpp::identity::core_script::CoreScript;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::prelude::AssetLockProof;
use dpp::serialization::PlatformDeserializableUntrusted;
use dpp::state_transition::StateTransition;

// The derive's default crate path mirrors its in-crate DPP use.
mod serialization {
    pub use dpp::serialization::PlatformDeserializableTrusted;
}
use dpp::ProtocolError;

#[derive(
    bincode::Encode, bincode::Decode, platform_serialization_derive::PlatformDeserializeTrusted,
)]
#[platform_serialize(unversioned)]
struct LocalFixture(Vec<u8>);

/// A type that only ever decodes bytes it wrote itself derives the trusted
/// side alone and needs no `DecodeUntrusted` on its graph.
#[test]
fn should_allow_trusted_only_fixtures_without_untrusted_traits() {
    use dpp::serialization::PlatformDeserializableTrusted;

    let bytes = bincode::encode_to_vec(
        LocalFixture(vec![1, 2, 3]),
        config::standard().with_big_endian(),
    )
    .unwrap();
    assert_eq!(
        LocalFixture::deserialize_from_bytes_trusted(&bytes)
            .unwrap()
            .0,
        [1, 2, 3]
    );
}

#[test]
fn should_reject_missing_script_in_unlimited_state_transition_decoder() {
    let config = config::standard().with_big_endian();
    // IdentityCreditWithdrawal V0, through the first allocating field.
    let mut bytes =
        bincode::encode_to_vec((5u32, 0u32, [0u8; 32], 1_000u64, 1u32, 0u32), config).unwrap();
    bytes.extend(bincode::encode_to_vec(u64::MAX, config).unwrap());
    assert!(StateTransition::deserialize_from_bytes_untrusted_no_limit(&bytes).is_err());
    assert!(StateTransition::deserialize_from_bytes_untrusted(&bytes).is_err());
}

#[test]
fn should_decode_binary_scripts_with_owned_and_borrowed_untrusted_apis() {
    // Include embedded NULs and cross the old custom borrowed decoder's chunk
    // size. Both untrusted APIs must preserve the length-prefixed wire format.
    let script = CoreScript::from_bytes(vec![0; 1_025]);
    let config = config::standard().with_big_endian();
    let bytes = bincode::encode_to_vec(&script, config).unwrap();
    let ordinary = bincode::decode_from_slice::<CoreScript, _>(&bytes, config).unwrap();
    let guarded = bincode::decode_from_slice_untrusted::<CoreScript, _>(&bytes, config).unwrap();
    let borrowed =
        bincode::borrow_decode_from_slice_untrusted::<CoreScript, _>(&bytes, config).unwrap();
    assert_eq!(ordinary, (script, bytes.len()));
    assert_eq!(guarded, ordinary);
    assert_eq!(borrowed, ordinary);
}

#[test]
fn should_preserve_witness_wire_format_and_signature_count_validation() {
    let config = config::standard();
    let witness = AddressWitness::P2sh {
        signatures: vec![vec![1; 65].into()],
        redeem_script: vec![0, 1, 0, 2].into(),
    };
    let bytes = bincode::encode_to_vec(&witness, config).unwrap();
    let decoded =
        bincode::decode_from_slice_untrusted::<AddressWitness, _>(&bytes, config).unwrap();
    assert_eq!(decoded, (witness, bytes.len()));

    let witness = AddressWitness::P2sh {
        signatures: vec![vec![1; 65].into(); 100],
        redeem_script: vec![0].into(),
    };
    let bytes = bincode::encode_to_vec(&witness, config).unwrap();
    let ordinary = bincode::decode_from_slice::<AddressWitness, _>(&bytes, config).unwrap_err();
    let guarded =
        bincode::decode_from_slice_untrusted::<AddressWitness, _>(&bytes, config).unwrap_err();
    assert_eq!(ordinary.to_string(), guarded.to_string());
}

#[test]
fn should_preserve_asset_lock_serde_wire_format_and_reject_missing_payloads() {
    let proof = AssetLockProof::Chain(ChainAssetLockProof {
        core_chain_locked_height: 42,
        out_point: dpp::dashcore::OutPoint::null(),
    });
    let config = config::standard().with_big_endian();
    let bytes = bincode::encode_to_vec(&proof, config).unwrap();
    let ordinary = bincode::decode_from_slice::<AssetLockProof, _>(&bytes, config).unwrap();
    let guarded =
        bincode::decode_from_slice_untrusted::<AssetLockProof, _>(&bytes, config).unwrap();
    assert_eq!(ordinary, (proof, bytes.len()));
    assert_eq!(guarded, ordinary);

    // Instant proof, whose first raw Serde field is length-prefixed bytes.
    let mut bytes = bincode::encode_to_vec(0u32, config).unwrap();
    bytes.extend(bincode::encode_to_vec(u64::MAX, config).unwrap());
    assert!(bincode::decode_from_slice_untrusted::<AssetLockProof, _>(&bytes, config).is_err());
}

#[test]
fn should_preserve_foreign_txid_serde_encoding_in_consensus_errors() {
    use dpp::consensus::basic::identity::IdentityAssetLockProofLockedTransactionMismatchError;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::Txid;

    let value = IdentityAssetLockProofLockedTransactionMismatchError::new(
        Txid::from_byte_array([1; 32]),
        Txid::from_byte_array([2; 32]),
    );
    let config = config::standard().with_big_endian();
    let bytes = bincode::encode_to_vec(&value, config).unwrap();
    let decoded =
        IdentityAssetLockProofLockedTransactionMismatchError::deserialize_from_bytes_untrusted(
            &bytes,
        )
        .unwrap();
    assert_eq!(decoded, value);
    let ordinary = bincode::decode_from_slice::<
        IdentityAssetLockProofLockedTransactionMismatchError,
        _,
    >(&bytes, config)
    .unwrap();
    assert_eq!(ordinary, (value, bytes.len()));
}

/// The derived decoders report how many bytes the value took, so the exact
/// entry points can refuse a suffix the plain ones ignore.
#[test]
fn should_report_consumed_bytes_and_refuse_a_suffix() {
    use dpp::serialization::PlatformDeserializableTrusted;

    let config = config::standard().with_big_endian();
    let mut bytes = bincode::encode_to_vec(LocalFixture(vec![1, 2, 3]), config).unwrap();
    let value_len = bytes.len();
    bytes.extend([9, 9]);

    let (fixture, consumed) =
        LocalFixture::deserialize_from_bytes_trusted_with_bytes_len(&bytes).unwrap();
    assert_eq!(fixture.0, [1, 2, 3]);
    assert_eq!(consumed, value_len);

    assert_eq!(
        LocalFixture::deserialize_from_bytes_trusted(&bytes)
            .unwrap()
            .0,
        [1, 2, 3]
    );
    assert!(matches!(
        LocalFixture::deserialize_from_bytes_trusted_exact(&bytes),
        Err(ProtocolError::PlatformDeserializationError(message))
            if message.contains("2 bytes left over")
    ));
    assert_eq!(
        LocalFixture::deserialize_from_bytes_trusted_exact(&bytes[..value_len])
            .unwrap()
            .0,
        [1, 2, 3]
    );
}

/// The exact decoder runs under the type's own byte budget: an identity
/// public key is capped at 2000 bytes, whatever other types allow.
#[test]
fn should_apply_the_types_own_budget_to_the_exact_untrusted_decoder() {
    let key: IdentityPublicKey = IdentityPublicKeyV0 {
        id: 0,
        purpose: Purpose::AUTHENTICATION,
        security_level: SecurityLevel::MASTER,
        contract_bounds: None,
        key_type: KeyType::ECDSA_SECP256K1,
        read_only: false,
        data: BinaryData::new(vec![0; 2_100]),
        disabled_at: None,
    }
    .into();
    let bytes = bincode::encode_to_vec(&key, config::standard().with_big_endian()).unwrap();

    assert!(matches!(
        IdentityPublicKey::deserialize_from_bytes_untrusted_exact(&bytes),
        Err(ProtocolError::MaxEncodedBytesReachedError {
            max_size_kbytes: 2000,
            ..
        })
    ));
}
