use dash_pkc::bls::{BlsPublicKey, BlsScChia, BlsScIetf, BlsSecretKey, BlsSignature};
use dpp::bls_signatures::{
    AggregateSignature, Bls12381G2Impl, Pairing, PublicKey, SecretKey, Signature, SignatureSchemes,
};
use dpp::core_types::validator::v0::ValidatorV0;
use dpp::core_types::validator_set::v0::ValidatorSetV0;
use dpp::native_bls::NativeBlsModule;
use dpp::{BlsModule, ProtocolError};
use serde::Deserialize;

#[derive(Deserialize)]
struct Corpus {
    key_generation: Vec<KeyGeneration>,
    basic: Vec<Basic>,
    secure_aggregation: Vec<Aggregate>,
    scalars: Vec<Scalar>,
    storage: Vec<Storage>,
}

#[derive(Deserialize)]
struct KeyGeneration {
    ikm: String,
    secret_key: String,
    public_key: String,
}

#[derive(Deserialize)]
struct Basic {
    secret_key: String,
    message: String,
    public_key: String,
    legacy_public_key: String,
    signature: String,
}

#[derive(Deserialize)]
struct Aggregate {
    secret_keys: Vec<String>,
    message: String,
    public_keys: Vec<String>,
    signature: String,
}

#[derive(Deserialize)]
struct Scalar {
    name: String,
    input: String,
    normalized: Option<String>,
    public_key: Option<String>,
    signature: Option<String>,
}

#[derive(Deserialize)]
struct Storage {
    name: String,
    validator: serde_json::Value,
    validator_bytes: String,
    validator_set: serde_json::Value,
    validator_set_bytes: String,
}

fn corpus() -> Corpus {
    serde_json::from_str(include_str!("fixtures/bls_compatibility/vectors.json")).unwrap()
}

fn bytes<const N: usize>(hex: &str) -> [u8; N] {
    hex::decode(hex).unwrap().try_into().unwrap()
}

#[test]
fn should_preserve_key_generation_from_fixed_material() {
    let vectors = corpus().key_generation;
    assert_eq!(vectors.len(), 3);
    for v in vectors {
        let ikm = bytes::<32>(&v.ikm);
        let current = SecretKey::<Bls12381G2Impl>::from_hash(ikm);
        let candidate = BlsSecretKey::<BlsScIetf>::from_ikm(&ikm).unwrap();
        assert_eq!(current.to_be_bytes(), bytes::<32>(&v.secret_key));
        assert_eq!(*candidate.to_bytes(), current.to_be_bytes());
        assert_eq!(
            current.public_key().0.to_compressed(),
            bytes::<48>(&v.public_key)
        );
        assert_eq!(
            candidate.public_key().to_bytes(),
            bytes::<48>(&v.public_key)
        );
    }
}

#[test]
fn should_match_frozen_basic_signatures_in_both_backends() {
    let vectors = corpus().basic;
    assert_eq!(vectors.len(), 16);
    for v in vectors {
        let secret = bytes(&v.secret_key);
        let message = hex::decode(v.message).unwrap();
        let public: [u8; 48] = bytes(&v.public_key);
        let signature: [u8; 96] = bytes(&v.signature);
        assert_eq!(
            NativeBlsModule.private_key_to_public_key(&secret).unwrap(),
            public
        );
        assert_eq!(NativeBlsModule.sign(&message, &secret).unwrap(), signature);
        assert!(NativeBlsModule
            .verify_signature(&signature, &message, &public)
            .unwrap());
        let candidate = BlsSecretKey::<BlsScIetf>::from_bytes(&secret).unwrap();
        assert_eq!(candidate.public_key().to_bytes(), public);
        assert_eq!(candidate.sign(&message).to_bytes(), signature);
        let decoded = BlsSignature::<BlsScIetf>::from_bytes(&signature).unwrap();
        assert!(candidate.public_key().verify(&message, &decoded).is_ok());
        assert_eq!(
            candidate
                .public_key()
                .to_scheme::<BlsScChia>()
                .unwrap()
                .to_bytes(),
            bytes::<48>(&v.legacy_public_key)
        );
        let mut wrong_message = message.clone();
        wrong_message.push(1);
        assert!(!NativeBlsModule
            .verify_signature(&signature, &wrong_message, &public)
            .unwrap());
        assert!(candidate
            .public_key()
            .verify(&wrong_message, &decoded)
            .is_err());
        let mut other_secret = [0; 32];
        other_secret[31] = 3;
        let other = BlsSecretKey::<BlsScIetf>::from_bytes(&other_secret)
            .unwrap()
            .public_key();
        assert!(!NativeBlsModule
            .verify_signature(&signature, &message, &other.to_bytes())
            .unwrap());
        assert!(other.verify(&message, &decoded).is_err());
    }
}

#[test]
fn should_match_frozen_secure_aggregates_in_both_backends() {
    let vectors = corpus().secure_aggregation;
    assert_eq!(vectors.len(), 5);
    for v in vectors {
        let message = hex::decode(v.message).unwrap();
        let public_keys: Vec<_> = v
            .public_keys
            .iter()
            .map(|key| PublicKey::<Bls12381G2Impl>::try_from(bytes::<48>(key).as_slice()).unwrap())
            .collect();
        let signatures: Vec<_> = v
            .secret_keys
            .iter()
            .map(|key| {
                SecretKey::<Bls12381G2Impl>::from_be_bytes(&bytes(key))
                    .unwrap()
                    .sign(SignatureSchemes::Basic, &message)
                    .unwrap()
            })
            .collect();
        let AggregateSignature::Basic(point) =
            AggregateSignature::from_signatures_secure(&signatures, &public_keys).unwrap()
        else {
            panic!("expected Basic");
        };
        assert_eq!(point.to_compressed(), bytes::<96>(&v.signature));
        assert!(Signature::Basic(point)
            .verify_secure(&public_keys, &message)
            .is_ok());
        let candidate_keys: Vec<_> = v
            .secret_keys
            .iter()
            .map(|key| BlsSecretKey::<BlsScIetf>::from_bytes(&bytes(key)).unwrap())
            .collect();
        let pks: Vec<_> = candidate_keys.iter().map(|key| key.public_key()).collect();
        let sigs: Vec<_> = candidate_keys
            .iter()
            .map(|key| key.sign(&message))
            .collect();
        let refs: Vec<_> = pks.iter().collect();
        let candidate =
            BlsSignature::secure_aggregate(&sigs.iter().collect::<Vec<_>>(), &refs).unwrap();
        assert_eq!(candidate.to_bytes(), bytes::<96>(&v.signature));
        assert!(candidate.secure_verify_aggregates(&message, &refs).is_ok());
        let mut wrong_message = message;
        wrong_message.push(1);
        assert!(Signature::Basic(point)
            .verify_secure(&public_keys, &wrong_message)
            .is_err());
        assert!(candidate
            .secure_verify_aggregates(&wrong_message, &refs)
            .is_err());
    }
}

#[test]
fn should_preserve_historical_scalar_reduction() {
    let vectors = corpus().scalars;
    assert_eq!(vectors.len(), 4);
    for v in vectors {
        let input = bytes(&v.input);
        let key = SecretKey::<Bls12381G2Impl>::from_be_bytes(&input).into_option();
        assert_eq!(
            key.as_ref().map(|key| hex::encode(key.to_be_bytes())),
            v.normalized,
            "{}",
            v.name
        );
        // The strict upstream constructor cannot replace the historical DPP parser directly.
        assert!(
            BlsSecretKey::<BlsScIetf>::from_bytes(&input).is_err(),
            "{}",
            v.name
        );
        if let Some(normalized) = v.normalized {
            assert_eq!(
                hex::encode(NativeBlsModule.private_key_to_public_key(&input).unwrap()),
                v.public_key.unwrap()
            );
            assert_eq!(
                hex::encode(NativeBlsModule.sign(b"scalar boundary", &input).unwrap()),
                v.signature.unwrap()
            );
            let reduced = dash_pkc::bls::Fr::from_bendian_reduce(&input).unwrap();
            let candidate = BlsSecretKey::<BlsScIetf>::try_from(reduced).unwrap();
            assert_eq!(*candidate.to_bytes(), bytes::<32>(&normalized));
        } else {
            assert!(matches!(
                NativeBlsModule.sign(b"scalar boundary", &input),
                Err(ProtocolError::InvalidBLSPrivateKeyError(_))
            ));
            assert!(matches!(
                NativeBlsModule.private_key_to_public_key(&input),
                Err(ProtocolError::InvalidBLSPrivateKeyError(_))
            ));
        }
    }
}

#[test]
fn should_preserve_identity_key_parsing_without_accepting_identity_signatures() {
    let mut public_key = [0; 48];
    public_key[0] = 0xc0;
    let mut signature = [0; 96];
    signature[0] = 0xc0;
    // This is historical parsing behavior, not permission to verify an identity signature.
    assert!(NativeBlsModule.validate_public_key(&public_key).is_ok());
    assert!(BlsPublicKey::<BlsScIetf>::from_bytes(&public_key).is_err());
    assert!(!NativeBlsModule
        .verify_signature(&signature, b"identity", &public_key)
        .unwrap());
    let valid = bytes::<48>(&corpus().basic[0].public_key);
    assert!(!NativeBlsModule
        .verify_signature(&signature, b"identity", &valid)
        .unwrap());
}

#[test]
fn should_preserve_invalid_input_results_and_validation_order() {
    let v = &corpus().basic[0];
    let public_key = bytes::<48>(&v.public_key);
    let secret = bytes::<32>(&v.secret_key);
    for len in [0, 31, 33] {
        assert!(
            matches!(NativeBlsModule.sign(b"", &vec![1;len]), Err(ProtocolError::PrivateKeySizeError { got }) if got == len as u32)
        );
        assert!(
            matches!(NativeBlsModule.private_key_to_public_key(&vec![1;len]), Err(ProtocolError::PrivateKeySizeError { got }) if got == len as u32)
        );
    }
    for len in [0, 95, 97] {
        assert!(
            matches!(NativeBlsModule.verify_signature(&vec![0;len], b"", &public_key), Err(ProtocolError::BlsSignatureSizeError { got }) if got == len as u32)
        );
        assert!(matches!(
            NativeBlsModule.verify_signature(&vec![0; len], b"", &[0; 48]),
            Err(ProtocolError::BlsError(_))
        ));
    }
    for public in [vec![], vec![0; 47], vec![0; 48], vec![255; 48], vec![0; 49]] {
        assert!(NativeBlsModule.validate_public_key(&public).is_err());
        assert!(matches!(
            NativeBlsModule.verify_signature(&[0; 96], b"", &public),
            Err(ProtocolError::BlsError(_))
        ));
    }
    for signature in [[0; 96], [255; 96]] {
        assert!(!NativeBlsModule
            .verify_signature(&signature, b"", &public_key)
            .unwrap());
        assert!(BlsSignature::<BlsScIetf>::from_bytes(&signature).is_err());
    }
    assert!(NativeBlsModule.sign(b"", &secret).is_ok());
}

#[test]
fn should_reject_points_outside_the_prime_order_subgroup() {
    // On-curve points x=4 in G1 and x=(2,0) in G2, in compressed IETF encoding.
    let mut public_key = [0; 48];
    public_key[0] = 0x80;
    public_key[47] = 4;
    let mut signature = [0; 96];
    signature[0] = 0xa0;
    signature[95] = 2;
    assert!(NativeBlsModule.validate_public_key(&public_key).is_err());
    assert!(BlsPublicKey::<BlsScIetf>::from_bytes(&public_key).is_err());
    assert!(bool::from(
        <Bls12381G2Impl as Pairing>::Signature::from_compressed(&signature).is_none()
    ));
    let valid = bytes::<48>(&corpus().basic[0].public_key);
    assert!(!NativeBlsModule
        .verify_signature(&signature, b"", &valid)
        .unwrap());
    assert!(BlsSignature::<BlsScIetf>::from_bytes(&signature).is_err());
}

#[test]
fn should_read_and_reemit_frozen_validator_storage() {
    let config = bincode::config::standard().with_big_endian();
    let vectors = corpus().storage;
    assert_eq!(vectors.len(), 3);
    for v in vectors {
        let encoded = hex::decode(v.validator_bytes).unwrap();
        let (validator, consumed): (ValidatorV0, _) =
            bincode::decode_from_slice(&encoded, config).unwrap();
        assert_eq!(consumed, encoded.len(), "{}", v.name);
        assert_eq!(bincode::encode_to_vec(&validator, config).unwrap(), encoded);
        assert_eq!(serde_json::to_value(&validator).unwrap(), v.validator);
        assert_eq!(
            serde_json::from_value::<ValidatorV0>(v.validator.clone()).unwrap(),
            validator
        );
        assert_eq!(
            bincode::decode_from_slice_untrusted::<ValidatorV0, _>(&encoded, config)
                .unwrap()
                .0,
            validator
        );
        assert_eq!(
            bincode::borrow_decode_from_slice_untrusted::<ValidatorV0, _>(&encoded, config)
                .unwrap()
                .0,
            validator
        );
        let encoded = hex::decode(v.validator_set_bytes).unwrap();
        let (set, consumed): (ValidatorSetV0, _) =
            bincode::decode_from_slice(&encoded, config).unwrap();
        assert_eq!(consumed, encoded.len());
        assert_eq!(bincode::encode_to_vec(&set, config).unwrap(), encoded);
        assert_eq!(serde_json::to_value(&set).unwrap(), v.validator_set);
        assert_eq!(
            serde_json::from_value::<ValidatorSetV0>(v.validator_set).unwrap(),
            set
        );
    }
}
