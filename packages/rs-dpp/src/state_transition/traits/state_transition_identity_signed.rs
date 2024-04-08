#[cfg(any(
    feature = "state-transition-signing",
    feature = "state-transition-validation"
))]
use crate::consensus::signature::{
    InvalidSignaturePublicKeySecurityLevelError, PublicKeyIsDisabledError,
};
use anyhow::anyhow;
use dashcore::secp256k1::{PublicKey as RawPublicKey, SecretKey as RawSecretKey};

#[cfg(feature = "state-transition-validation")]
use crate::state_transition::errors::WrongPublicKeyPurposeError;

#[cfg(any(
    feature = "state-transition-signing",
    feature = "state-transition-validation"
))]
use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use crate::state_transition::StateTransitionLike;

#[cfg(any(
    feature = "state-transition-signing",
    feature = "state-transition-validation"
))]
use crate::identity::IdentityPublicKey;
use crate::identity::Purpose;
use crate::{
    identity::{KeyID, SecurityLevel},
    prelude::*,
};

pub trait StateTransitionIdentitySigned: StateTransitionLike {
    fn signature_public_key_id(&self) -> KeyID;
    fn set_signature_public_key_id(&mut self, key_id: KeyID);

    //this is not versioned because of it just being the base trait default
    #[cfg(any(
        feature = "state-transition-signing",
        feature = "state-transition-validation"
    ))]
    /// Verifies that the supplied public key has the correct security level
    /// and purpose to sign the state transition
    /// This should only be used for authentication
    fn verify_public_key_level_and_purpose(
        &self,
        public_key: &IdentityPublicKey,
    ) -> Result<(), ProtocolError> {
        // Otherwise, key security level should be less than MASTER but more or equal than required
        if !self
            .security_level_requirement()
            .contains(&public_key.security_level())
        {
            return Err(ProtocolError::InvalidSignaturePublicKeySecurityLevelError(
                InvalidSignaturePublicKeySecurityLevelError::new(
                    public_key.security_level(),
                    self.security_level_requirement(),
                ),
            ));
        }

        if public_key.purpose() != self.purpose_requirement() {
            return Err(ProtocolError::WrongPublicKeyPurposeError(
                WrongPublicKeyPurposeError::new(public_key.purpose(), self.purpose_requirement()),
            ));
        }
        Ok(())
    }

    //this is not versioned because of it just being the base trait default
    #[cfg(any(
        feature = "state-transition-signing",
        feature = "state-transition-validation"
    ))]
    fn verify_public_key_is_enabled(
        &self,
        public_key: &IdentityPublicKey,
    ) -> Result<(), ProtocolError> {
        if public_key.disabled_at().is_some() {
            return Err(ProtocolError::PublicKeyIsDisabledError(
                PublicKeyIsDisabledError::new(public_key.id()),
            ));
        }
        Ok(())
    }

    /// Returns minimal key security level that can be used to sign this ST.
    /// Override this method if the ST requires a different security level.
    fn security_level_requirement(&self) -> Vec<SecurityLevel>;

    /// The purpose requirement for the signing key
    /// The default is authentication
    /// However for Withdrawals and Fund Transfers the requirement is TRANSFER
    fn purpose_requirement(&self) -> Purpose {
        Purpose::AUTHENTICATION
    }
}

pub fn get_compressed_public_ec_key(private_key: &[u8]) -> Result<[u8; 33], ProtocolError> {
    let sk = RawSecretKey::from_slice(private_key)
        .map_err(|e| anyhow!("Invalid ECDSA private key: {}", e))?;

    let secp = dashcore::secp256k1::Secp256k1::new();
    let public_key_compressed = RawPublicKey::from_secret_key(&secp, &sk).serialize();
    Ok(public_key_compressed)
}

//
// #[cfg(test)]
// mod test {
//     use chrono::Utc;
//     use platform_value::{BinaryData, Value};
//     use rand::rngs::StdRng;
//     use rand::SeedableRng;
//     use serde::{Deserialize, Serialize};
//     use serde_json::json;
//     use std::convert::TryInto;
//     use std::vec;
//
//     use crate::ProtocolError::InvalidSignaturePublicKeySecurityLevelError;
//     use crate::{
//         assert_error_contains,
//         identity::{KeyID, SecurityLevel},
//         state_transition::{
//             StateTransition, StateTransitionFieldTypes, StateTransitionLike, StateTransitionType,
//         },
//         util::hash::ripemd160_sha256,
//         NativeBlsModule,
//     };
//     use platform_value::string_encoding::Encoding;
//
//     use super::StateTransitionIdentitySignedV0;
//     use super::*;
//     use crate::serialization::PlatformDeserializable;
//     use crate::serialization::PlatformSerializable;
//     use crate::serialization::Signable;
//     use crate::version::FeatureVersion;
//     use bincode::{config, Decode, Encode};
//     use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
//
//     #[derive(
//         Debug,
//         Clone,
//         Encode,
//         Decode,
//         Serialize,
//         Deserialize,
//         PlatformDeserialize,
//         PlatformSerialize,
//         PlatformSignable,
//     )]
//
//     #[serde(rename_all = "camelCase")]
//     struct ExampleStateTransition {
//         pub protocol_version: u32,
//         pub transition_type: StateTransitionType,
//         pub owner_id: Identifier,
//         #[platform_signable(exclude_from_sig_hash)]
//         pub signature: BinaryData,
//         #[platform_signable(exclude_from_sig_hash)]
//         pub signature_public_key_id: KeyID,
//     }
//
//     impl StateTransitionFieldTypes for ExampleStateTransition {
//         fn binary_property_paths() -> Vec<&'static str> {
//             vec!["signature"]
//         }
//         fn identifiers_property_paths() -> Vec<&'static str> {
//             vec![]
//         }
//         fn signature_property_paths() -> Vec<&'static str> {
//             vec!["signature", "signaturePublicKeyId"]
//         }
//
//         fn to_cleaned_object(&self, _skip_signature: bool) -> Result<Value, ProtocolError> {
//             todo!()
//         }
//     }
//
//     impl From<ExampleStateTransition> for StateTransition {
//         fn from(_val: ExampleStateTransition) -> Self {
//             let st = DocumentsBatchTransition::default();
//             StateTransition::DocumentsBatch(st)
//         }
//     }
//
//     impl StateTransitionLike for ExampleStateTransition {
//         fn state_transition_protocol_version(&self) -> FeatureVersion {
//             1
//         }
//         fn state_transition_type(&self) -> StateTransitionType {
//             StateTransitionType::DocumentsBatch
//         }
//         fn signature(&self) -> &BinaryData {
//             &self.signature
//         }
//         fn set_signature(&mut self, signature: BinaryData) {
//             self.signature = signature
//         }
//
//         fn set_signature_bytes(&mut self, signature: Vec<u8>) {
//             self.signature = BinaryData::new(signature)
//         }
//
//         fn modified_data_ids(&self) -> Vec<Identifier> {
//             vec![]
//         }
//     }
//
//     impl StateTransitionIdentitySignedV0 for ExampleStateTransition {
//         fn get_owner_id(&self) -> &Identifier {
//             &self.owner_id
//         }
//         fn get_security_level_requirement(&self) -> Vec<SecurityLevel> {
//             vec![SecurityLevel::HIGH]
//         }
//
//         fn get_signature_public_key_id(&self) -> Option<KeyID> {
//             self.signature_public_key_id
//         }
//
//         fn set_signature_public_key_id(&mut self, key_id: KeyID) {
//             self.signature_public_key_id = key_id;
//         }
//     }
//
//     fn get_mock_state_transition() -> ExampleStateTransition {
//         let owner_id = Identifier::from_string(
//             "AX5o22ARWFYZE9JZTA5SSeyvprtetBcvbQLSBZ7cR7Gw",
//             Encoding::Base58,
//         )
//         .unwrap();
//         ExampleStateTransition {
//             protocol_version: 1,
//             transition_type: StateTransitionType::DocumentsBatch,
//             signature: Default::default(),
//             signature_public_key_id: 1,
//             owner_id,
//         }
//     }
//
//     struct Keys {
//         pub ec_private: Vec<u8>,
//         pub ec_public_compressed: Vec<u8>,
//         pub ec_public_uncompressed: Vec<u8>,
//         pub bls_private: Vec<u8>,
//         pub bls_public: Vec<u8>,
//         pub identity_public_key: IdentityPublicKey,
//         pub public_key_id: KeyID,
//     }
//
//     fn get_test_keys() -> Keys {
//         let secp = dashcore::secp256k1::Secp256k1::new();
//         let mut rng = dashcore::secp256k1::rand::thread_rng();
//         let mut std_rng = StdRng::seed_from_u64(99999);
//         let (private_key, public_key) = secp.generate_keypair(&mut rng);
//
//         let public_key_id = 1;
//         let ec_private_key_bytes = private_key.secret_bytes();
//         let ec_public_compressed_bytes = public_key.serialize();
//         let ec_public_uncompressed_bytes = public_key.serialize_uncompressed();
//
//         let bls_private =
//             bls_signatures::PrivateKey::generate_dash(&mut std_rng).expect("expected private key");
//         let bls_public = bls_private
//             .g1_element()
//             .expect("expected to make public key");
//         let bls_private_bytes = bls_private.to_bytes().to_vec();
//         let bls_public_bytes = bls_public.to_bytes().to_vec();
//
//         let identity_public_key = IdentityPublicKey {
//             id: public_key_id,
//             key_type: KeyType::ECDSA_SECP256K1,
//             purpose: Purpose::AUTHENTICATION,
//             security_level: SecurityLevel::HIGH,
//             data: BinaryData::new(ec_public_compressed_bytes.try_into().unwrap()),
//             read_only: false,
//             disabled_at: None,
//         };
//
//         Keys {
//             ec_private: ec_private_key_bytes.to_vec(),
//             ec_public_compressed: ec_public_compressed_bytes.to_vec(),
//             ec_public_uncompressed: ec_public_uncompressed_bytes.to_vec(),
//             bls_private: bls_private_bytes,
//             bls_public: bls_public_bytes,
//             identity_public_key,
//             public_key_id,
//         }
//     }
//
//     #[test]
//     fn to_object_with_signature() {
//         let st = get_mock_state_transition();
//         let st_object = st.to_object(false).unwrap();
//
//         assert_eq!(st_object["protocolVersion"].to_integer::<u32>().unwrap(), 1);
//         assert_eq!(st_object["transitionType"].to_integer::<u8>().unwrap(), 1);
//         assert_eq!(
//             st_object["signaturePublicKeyId"]
//                 .to_integer::<u32>()
//                 .unwrap(),
//             1
//         );
//         assert!(st_object["signature"].as_bytes().unwrap().is_empty());
//     }
//
//     #[test]
//     fn to_object_without_signature() {
//         let st = get_mock_state_transition();
//         let st_object = st.to_object(true).unwrap();
//
//         assert_eq!(st_object["protocolVersion"].to_integer::<u32>().unwrap(), 1);
//         assert_eq!(st_object["transitionType"].to_integer::<u8>().unwrap(), 1);
//         assert!(!st_object.has("signaturePublicKeyId").unwrap());
//         assert!(!st_object.has("signature").unwrap());
//     }
//
//     #[test]
//     fn to_json() {
//         let st = get_mock_state_transition();
//         let st_json = st.to_json(false).unwrap();
//         assert_eq!(
//             st_json,
//             json!({
//                 "protocolVersion" : 1,
//                 "signature": "",
//                 "signaturePublicKeyId": 1,
//                 "transitionType" : 1,
//                 "ownerId" : "AX5o22ARWFYZE9JZTA5SSeyvprtetBcvbQLSBZ7cR7Gw"
//             })
//         );
//     }
//
//     #[test]
//     fn to_hash() {
//         let st = get_mock_state_transition();
//         let hash = st.hash(false).unwrap();
//         assert_eq!(
//             "39b9c5951e5d83668f98909bb73d390d49867c47bbfe043a42ac83de898142c0",
//             hex::encode(hash)
//         )
//     }
//
//     #[test]
//     fn to_buffer() {
//         let st = get_mock_state_transition();
//         let hash = st.to_cbor_buffer(false).unwrap();
//         let result = hex::encode(hash);
//
//         assert_eq!("01a4676f776e6572496458208d6e06cac6cd2c4b9020806a3f1a4ec48fc90defd314330a5ce7d8548dfc2524697369676e617475726540747369676e61747572655075626c69634b65794964016e7472616e736974696f6e5479706501", result.as_str());
//     }
//
//     #[test]
//     fn to_buffer_no_signature() {
//         let st = get_mock_state_transition();
//         let hash = st.to_cbor_buffer(true).unwrap();
//         let result = hex::encode(hash);
//
//         assert_eq!("01a2676f776e6572496458208d6e06cac6cd2c4b9020806a3f1a4ec48fc90defd314330a5ce7d8548dfc25246e7472616e736974696f6e5479706501", result);
//     }
//
//     #[test]
//     fn get_signature_public_key_id() {
//         let st = get_mock_state_transition();
//         let keys = get_test_keys();
//         assert_eq!(Some(keys.public_key_id), st.get_signature_public_key_id())
//     }
//
//     #[test]
//     fn sign_validate_with_private_key() {
//         let bls = NativeBlsModule::default();
//         let mut st = get_mock_state_transition();
//         let keys = get_test_keys();
//
//         st.sign(&keys.identity_public_key, &keys.ec_private, &bls)
//             .unwrap();
//         st.verify_signature(&keys.identity_public_key, &bls)
//             .expect("the verification shouldn't fail");
//     }
//
//     #[test]
//     fn sign_validate_signature_ecdsa_hash160() {
//         let bls = NativeBlsModule::default();
//         let mut st = get_mock_state_transition();
//         let mut keys = get_test_keys();
//         keys.identity_public_key.key_type = KeyType::ECDSA_HASH160;
//         keys.identity_public_key.data =
//             BinaryData::new(ripemd160_sha256(keys.identity_public_key.data.as_slice()).to_vec());
//
//         st.sign(&keys.identity_public_key, &keys.ec_private, &bls)
//             .unwrap();
//         st.verify_signature(&keys.identity_public_key, &bls)
//             .expect("the verification shouldn't fail");
//     }
//
//     #[test]
//     fn error_when_sign_with_wrong_public_key() {
//         let bls = NativeBlsModule::default();
//         let mut st = get_mock_state_transition();
//         let mut keys = get_test_keys();
//
//         let secp = dashcore::secp256k1::Secp256k1::new();
//         let mut rng = dashcore::secp256k1::rand::thread_rng();
//         let (_, public_key) = secp.generate_keypair(&mut rng);
//
//         keys.identity_public_key.data = BinaryData::new(public_key.serialize().to_vec());
//
//         let sign_result = st.sign(&keys.identity_public_key, &keys.ec_private, &bls);
//         assert_error_contains!(sign_result, "Invalid signature public key");
//     }
//
//     #[test]
//     fn error_if_security_level_is_not_met() {
//         let bls = NativeBlsModule::default();
//         let mut st = get_mock_state_transition();
//         let mut keys = get_test_keys();
//         keys.identity_public_key.security_level = SecurityLevel::MEDIUM;
//
//         let sign_error = st
//             .sign(&keys.identity_public_key, &keys.ec_private, &bls)
//             .unwrap_err();
//         match sign_error {
//             InvalidSignaturePublicKeySecurityLevelError(err) => {
//                 assert_eq!(SecurityLevel::MEDIUM, err.public_key_security_level());
//                 assert_eq!(vec![SecurityLevel::HIGH], err.allowed_key_security_levels());
//             }
//             error => {
//                 panic!("invalid error type: {}", error)
//             }
//         };
//     }
//
//     #[test]
//     fn error_if_key_purpose_not_authenticated() {
//         let bls = NativeBlsModule::default();
//         let mut st = get_mock_state_transition();
//         let mut keys = get_test_keys();
//         keys.identity_public_key.purpose = Purpose::ENCRYPTION;
//
//         let sign_error = st
//             .sign(&keys.identity_public_key, &keys.ec_private, &bls)
//             .unwrap_err();
//         match sign_error {
//             ProtocolError::WrongPublicKeyPurposeError(err) => {
//                 assert_eq!(Purpose::ENCRYPTION, err.public_key_purpose());
//                 assert_eq!(Purpose::AUTHENTICATION, err.key_purpose_requirement());
//             }
//             error => {
//                 panic!("invalid error type: {}", error)
//             }
//         };
//     }
//
//     #[test]
//     fn should_sign_validate_with_bls_signature() {
//         let bls = NativeBlsModule::default();
//         let mut st = get_mock_state_transition();
//         let mut keys = get_test_keys();
//         keys.identity_public_key.key_type = KeyType::BLS12_381;
//         keys.identity_public_key.data = BinaryData::new(keys.bls_public.clone());
//
//         st.sign(&keys.identity_public_key, &keys.bls_private, &bls)
//             .expect("validation should be successful");
//     }
//
//     #[test]
//     fn error_if_transition_is_not_signed_ecdsa() {
//         let bls = NativeBlsModule::default();
//         let st = get_mock_state_transition();
//         let keys = get_test_keys();
//
//         let verify_error = st
//             .verify_signature(&keys.identity_public_key, &bls)
//             .unwrap_err();
//         match verify_error {
//             ProtocolError::StateTransitionIsNotSignedError { .. } => {}
//             error => {
//                 panic!("invalid error type: {}", error)
//             }
//         };
//     }
//
//     #[test]
//     fn error_if_transition_is_not_signed_bls() {
//         let bls = NativeBlsModule::default();
//         let st = get_mock_state_transition();
//         let mut keys = get_test_keys();
//         keys.identity_public_key.key_type = KeyType::BLS12_381;
//         keys.identity_public_key.data = BinaryData::new(keys.bls_public.clone());
//
//         let verify_error = st
//             .verify_signature(&keys.identity_public_key, &bls)
//             .unwrap_err();
//         match verify_error {
//             ProtocolError::StateTransitionIsNotSignedError { .. } => {}
//             error => {
//                 panic!("invalid error type: {}", error)
//             }
//         };
//     }
//
//     #[test]
//     fn set_signature() {
//         let mut st = get_mock_state_transition();
//         let signature = "some_signature";
//         st.set_signature(BinaryData::new(signature.as_bytes().to_owned()));
//         assert_eq!(signature.as_bytes(), st.signature().as_slice());
//     }
//
//     #[test]
//     fn set_signature_public_key_id() {
//         let mut st = get_mock_state_transition();
//         let public_key_id = 2;
//         st.set_signature_public_key_id(public_key_id);
//         assert_eq!(Some(public_key_id), st.get_signature_public_key_id());
//     }
//
//     #[test]
//     fn should_throw_public_key_is_disabled_error_if_public_key_is_disabled() {
//         let bls = NativeBlsModule::default();
//         let mut st = get_mock_state_transition();
//         let mut keys = get_test_keys();
//         keys.identity_public_key
//             .set_disabled_at(Utc::now().timestamp_millis() as u64);
//
//         let result = st
//             .sign(&keys.identity_public_key, &keys.bls_private, &bls)
//             .expect_err("the protocol error should be returned");
//
//         assert!(matches!(
//             result,
//             ProtocolError::PublicKeyIsDisabledError { .. }
//         ))
//     }
//
//     #[test]
//     fn should_throw_invalid_signature_public_key_security_level_error() {
//         let bls = NativeBlsModule::default();
//         // should throw InvalidSignaturePublicKeySecurityLevel Error if public key with master level is used to sign non update state transition
//         let mut st = get_mock_state_transition();
//         let mut keys = get_test_keys();
//
//         st.transition_type = StateTransitionType::DataContractCreate;
//         keys.identity_public_key.security_level = SecurityLevel::MASTER;
//
//         let result = st
//             .sign(&keys.identity_public_key, &keys.bls_private, &bls)
//             .expect_err("the protocol error should be returned");
//
//         match result {
//             ProtocolError::InvalidSignaturePublicKeySecurityLevelError(err) => {
//                 assert_eq!(err.public_key_security_level(), SecurityLevel::MASTER);
//                 assert_eq!(err.allowed_key_security_levels(), vec![SecurityLevel::HIGH]);
//             }
//             error => panic!(
//                 "expected InvalidSignaturePublicKeySecurityLevelError, got {}",
//                 error
//             ),
//         }
//     }
// }
