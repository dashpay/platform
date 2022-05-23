use dashcore::{
    secp256k1::{PublicKey as RawPublicKey, SecretKey as RawSecretKey},
};

use anyhow::anyhow;
use bls_signatures::Serialize;
use std::convert::TryInto;

use crate::{
    identity::{IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel},
    prelude::*, util::hash::ripemd160_sha256,
};

use super::StateTransitionLike;

pub trait StateTransitionIdentitySigned
where
    Self: StateTransitionLike,
{
    fn get_signature_public_key_id(&self) -> KeyID;
    fn set_signature_public_key_id(&mut self, key_id: KeyID);

    fn sign(
        &mut self,
        identity_public_key: &IdentityPublicKey,
        private_key: &[u8],
    ) -> Result<(), ProtocolError> {
        Self::verify_public_key_level_and_purpose(identity_public_key)?;

        match identity_public_key.get_type() {
            KeyType::ECDSA_SECP256K1 => {
                let public_key_compressed = get_compressed_public_ec_key(private_key)?;

                // we store compressed public key in the identity ,
                // and here we compare the private key used to sing the state transition with
                // the compressed key stored in the identity

                if public_key_compressed.to_vec() != identity_public_key.get_data() {
                    return Err(ProtocolError::InvalidSignaturePublicKeyError {
                        public_key: identity_public_key.get_data().to_owned(),
                    });
                }

                self.sign_by_private_key(private_key, identity_public_key.get_type())
            }
            KeyType::ECDSA_HASH160 => {
                let public_key_compressed = get_compressed_public_ec_key(private_key)?;
                let pub_key_hash = ripemd160_sha256(&public_key_compressed);

                if pub_key_hash != identity_public_key.get_data() {
                    return Err(ProtocolError::InvalidSignaturePublicKeyError {
                        public_key: identity_public_key.get_data().to_owned(),
                    });
                }
                self.sign_by_private_key(private_key, identity_public_key.get_type())
            }
            KeyType::BLS12_381 => {
                let public_key = get_public_bls_key(private_key)?;

                if public_key != identity_public_key.get_data() {
                    return Err(ProtocolError::InvalidSignaturePublicKeyError {
                        public_key: identity_public_key.get_data().to_owned(),
                    });
                }
                self.sign_by_private_key(private_key, identity_public_key.get_type())
            }
        }
    }

    fn verify_signature(&self, public_key: &IdentityPublicKey) -> Result<(), ProtocolError> {
        Self::verify_public_key_level_and_purpose(public_key)?;

        let signature = self.get_signature();
        if signature.is_empty() {
            return Err(ProtocolError::StateTransitionIsNotIsSignedError {
                state_transition: self.clone().into(),
            });
        }

        if self.get_signature_public_key_id() != public_key.get_id() {
            return Err(ProtocolError::PublicKeyMismatchError {
                public_key: public_key.clone(),
            });
        }

        let public_key_bytes = public_key.get_data();
        match public_key.get_type() {
            KeyType::ECDSA_HASH160 => {
                self.verify_ecdsa_hash_160_signature_by_public_key_hash(public_key_bytes)
            }

            KeyType::ECDSA_SECP256K1 => {
                self.verify_ecdsa_signature_by_public_key(public_key_bytes)
            }

            KeyType::BLS12_381 => self.verify_bls_signature_by_public_key(public_key_bytes),
        }
    }

    /// Verifies that the supplied public key has the correct security level
    /// and purpose to sign the state transition
    fn verify_public_key_level_and_purpose(
        public_key: &IdentityPublicKey,
    ) -> Result<(), ProtocolError> {
        if Self::get_security_level_requirement() < public_key.get_security_level() {
            return Err(ProtocolError::PublicKeySecurityLevelNotMetError {
                public_key_security_level: public_key.get_security_level(),
                required_security_level: Self::get_security_level_requirement(),
            });
        }

        if public_key.get_purpose() != Purpose::AUTHENTICATION {
            return Err(ProtocolError::WrongPublicKeyPurposeError {
                public_key_purpose: public_key.get_purpose(),
                key_purpose_requirement: Purpose::AUTHENTICATION,
            });
        }
        Ok(())
    }

    fn get_security_level_requirement() -> SecurityLevel {
        SecurityLevel::MASTER
    }
}

pub fn get_compressed_public_ec_key(private_key: &[u8]) -> Result<[u8; 33], ProtocolError> {
    let sk = RawSecretKey::from_slice(private_key)
        .map_err(|e| anyhow!("Invalid ECDSA private key: {}", e))?;

    let secp = dashcore::secp256k1::Secp256k1::new();
    let public_key_compressed = RawPublicKey::from_secret_key(&secp, &sk).serialize();
    Ok(public_key_compressed)
}

pub fn get_public_bls_key(private_key: &[u8]) -> Result<Vec<u8>, ProtocolError> {
    let fixed_len_key: [u8; 32] = private_key
        .try_into()
        .map_err(|_| anyhow!("the BLS private key must be 32 bytes long"))?;
    let pk = bls_signatures::PrivateKey::from_bytes(&fixed_len_key).map_err(anyhow::Error::msg)?;
    let public_key = pk.public_key().as_bytes();
    Ok(public_key)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        identity::{KeyID, SecurityLevel},
        state_transition::{
            StateTransition, StateTransitionConvert, StateTransitionLike, StateTransitionType,
        }, util::hash::ripemd160_sha256, assert_error_contains, mocks,
    };
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    use bls_signatures::Serialize as BlsSerialize;

    use super::StateTransitionIdentitySigned;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExampleStateTransition {
        pub protocol_version: u32,
        pub signature: Vec<u8>,
        pub signature_public_key_id: KeyID,
        pub transition_type: StateTransitionType,
    }

    impl StateTransitionConvert for ExampleStateTransition {
        fn binary_property_paths() -> Vec<&'static str> {
            vec!["signature"]
        }
        fn identifiers_property_paths() -> Vec<&'static str> {
            vec![]
        }
        fn signature_property_paths() -> Vec<&'static str> {
            vec!["signature", "signaturePublicKeyId"]
        }
    }

    impl Into<StateTransition> for ExampleStateTransition {
        fn into(self) -> StateTransition {
            let st = mocks::DocumentsBatchTransition{};
            StateTransition::DocumentsBatch(st)
        }
    }

    impl StateTransitionLike for ExampleStateTransition {
        fn calculate_fee(&self) -> Result<u64, crate::ProtocolError> {
            unimplemented!()
        }
        fn get_protocol_version(&self) -> u32 {
            1
        }
        fn get_type(&self) -> StateTransitionType {
            StateTransitionType::DocumentsBatch
        }
        fn get_signature(&self) -> &Vec<u8> {
            &self.signature
        }
        fn set_signature(&mut self, signature: Vec<u8>) {
            self.signature = signature
        }
    }

    impl StateTransitionIdentitySigned for ExampleStateTransition {
        fn get_security_level_requirement() -> SecurityLevel {
            SecurityLevel::MASTER
        }

        fn get_signature_public_key_id(&self) -> KeyID {
            self.signature_public_key_id
        }

        fn set_signature_public_key_id(&mut self, key_id: KeyID) {
            self.signature_public_key_id = key_id;
        }
    }

    fn get_mock_state_transition() -> ExampleStateTransition {
        ExampleStateTransition {
            protocol_version: 1,
            transition_type: StateTransitionType::DocumentsBatch,
            signature: Default::default(),
            signature_public_key_id: 1,
        }
    }

    struct Keys  {
        pub ec_private : Vec<u8>,
        pub ec_public_compressed : Vec<u8>,
        pub ec_public_uncompressed : Vec<u8>,
        pub bls_private : Vec<u8>,
        pub bls_public  : Vec<u8>,
        pub identity_public_key : IdentityPublicKey,
        pub public_key_id : u64,

    }


    fn get_test_keys() -> Keys   {
        let secp = dashcore::secp256k1::Secp256k1::new();
        let mut rng = dashcore::secp256k1::rand::thread_rng();
        let (private_key, public_key) = secp.generate_keypair(&mut rng);

        let public_key_id = 1;
        let ec_private_key_bytes  = private_key.secret_bytes();
        let ec_public_compressed_bytes  =  public_key.serialize();
        let ec_public_uncompressed_bytes  =  public_key.serialize_uncompressed();


        let mut buffer = [0u8; 32];
        let _ = getrandom::getrandom(&mut buffer);
        let bls_private = bls_signatures::PrivateKey::new(buffer);
        let bls_public = bls_private.public_key();
        let bls_private_bytes =  bls_private.as_bytes();
        let bls_public_bytes =  bls_public.as_bytes();

        let identity_public_key = IdentityPublicKey {
            id : public_key_id,
            key_type : KeyType::ECDSA_SECP256K1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            data: ec_public_compressed_bytes.try_into().unwrap(),
            read_only: false
        };

        Keys {
            ec_private: ec_private_key_bytes.to_vec(),
            ec_public_compressed : ec_public_compressed_bytes.to_vec(),
            ec_public_uncompressed : ec_public_uncompressed_bytes.to_vec(),
            bls_private : bls_private_bytes,
            bls_public : bls_public_bytes,
            identity_public_key,
            public_key_id,
        }
    }



    #[test]
    fn to_object_with_signature() {
        let st = get_mock_state_transition();
        let st_object = st.to_object(false).unwrap();

        assert_eq!(st_object["protocolVersion"].as_i64().unwrap(), 1);
        assert_eq!(st_object["transitionType"].as_u64().unwrap(), 1);
        assert_eq!(st_object["signaturePublicKeyId"].as_u64().unwrap(), 1);
        assert!(st_object["signature"].as_array().unwrap().is_empty());
    }

    #[test]
    fn to_object_without_signature() {
        let st = get_mock_state_transition();
        let st_object = st.to_object(true).unwrap();

        assert_eq!(st_object["protocolVersion"].as_i64().unwrap(), 1);
        assert_eq!(st_object["transitionType"].as_u64().unwrap(), 1);
        assert!(st_object.get("signaturePublicKeyId").is_none());
        assert!(st_object.get("signature").is_none());
    }

    #[test]
    fn to_json() {
        let st = get_mock_state_transition();
        let st_json = st.to_json().unwrap();
        assert_eq!(
            st_json,
            json!({
                "protocolVersion" : 1,
                "signature": "",
                "signaturePublicKeyId": 1,
                "transitionType" : 1,
            })
        );
    }

    #[test]
    fn to_hash() {
        let st = get_mock_state_transition();
        let hash = st.hash(false).unwrap();
        assert_eq!(
            "86a734f974cb260d528079d2b47050891afce203c56077d603f9896a40044223",
            hex::encode(hash)
        )
    }

    #[test]
    fn to_buffer() {
        let st = get_mock_state_transition();
        let hash = st.to_buffer(false).unwrap();
        let result =  hex::encode(hash);

        assert_eq!(108, result.len());
        assert!(result.starts_with("01000000")
        )
    }

    #[test]
    fn to_buffer_no_signature() {
        let st = get_mock_state_transition();
        let hash = st.to_buffer(true).unwrap();
        let result =  hex::encode(hash);

        assert_eq!(42, result.len());
        assert_eq!("01000000a16e7472616e736974696f6e5479706501", result);
    }


    #[test]
    fn get_signature_public_key_id() {
        let st = get_mock_state_transition();
        let keys = get_test_keys();
        assert_eq!(keys.public_key_id, st.get_signature_public_key_id())
    }


    #[test]
    fn sign_validate_with_private_key() {
        let mut st = get_mock_state_transition();
        let keys = get_test_keys();

        st.sign(&keys.identity_public_key, &keys.ec_private).unwrap();
        st.verify_signature(&keys.identity_public_key).expect("the verification shouldn't fail");
    }

    #[test]
    fn sign_validate_signature_ecdsa_hash160() {
        let mut st = get_mock_state_transition();
        let mut keys = get_test_keys();
        keys.identity_public_key.key_type = KeyType::ECDSA_HASH160;
        keys.identity_public_key.data = ripemd160_sha256(&keys.identity_public_key.data);

        st.sign(&keys.identity_public_key, &keys.ec_private).unwrap();
        st.verify_signature(&keys.identity_public_key).expect("the verification shouldn't fail");
    }


    #[test]
    fn error_when_sign_with_wrong_public_key()  {
        let mut st = get_mock_state_transition();
        let mut keys = get_test_keys();

        let secp = dashcore::secp256k1::Secp256k1::new();
        let mut rng = dashcore::secp256k1::rand::thread_rng();
        let (_, public_key) = secp.generate_keypair(&mut rng);

        keys.identity_public_key.data = public_key.serialize().to_vec();

        let sign_result = st.sign(&keys.identity_public_key, &keys.ec_private);
        assert_error_contains!(sign_result, "Invalid signature public key");
    }


    #[test]
    fn error_if_security_level_is_not_met() {
        let mut st = get_mock_state_transition();
        let mut keys = get_test_keys();
        keys.identity_public_key.security_level = SecurityLevel::MEDIUM;

        let sign_error = st.sign(&keys.identity_public_key, &keys.ec_private).unwrap_err();
        match sign_error {
            ProtocolError::PublicKeySecurityLevelNotMetError { public_key_security_level: sec_level, required_security_level: req_sec_level } =>  {
                assert_eq!(SecurityLevel::MEDIUM, sec_level);
                assert_eq!(SecurityLevel::MASTER, req_sec_level);
            }
            error => {
                panic!("invalid error type: {}", error)
            }

        };
     }


     #[test]
     fn error_if_key_purpose_not_authenticated() {
        let mut st = get_mock_state_transition();
        let mut keys = get_test_keys();
        keys.identity_public_key.purpose  = Purpose::ENCRYPTION;

        let sign_error = st.sign(&keys.identity_public_key, &keys.ec_private).unwrap_err();
        match sign_error {
            ProtocolError::WrongPublicKeyPurposeError { public_key_purpose: purpose, key_purpose_requirement: req_purpose } =>  {
                assert_eq!(Purpose::ENCRYPTION, purpose);
                assert_eq!(Purpose::AUTHENTICATION, req_purpose);
            }
            error => {
                panic!("invalid error type: {}", error)
            }

        };

     }


     #[test]
     fn should_sign_validate_with_bls_signature() {
        let mut st = get_mock_state_transition();
        let mut keys = get_test_keys();
        keys.identity_public_key.key_type = KeyType::BLS12_381;
        keys.identity_public_key.data = keys.bls_public.clone();

        st.sign(&keys.identity_public_key, &keys.bls_private).expect("validation should be successful");
     }


     #[test]
     fn error_if_transition_is_not_signed_ecdsa()  {
        let  st = get_mock_state_transition();
        let  keys = get_test_keys();

        let verify_error = st.verify_signature(&keys.identity_public_key).unwrap_err();
        match verify_error {
            ProtocolError::StateTransitionIsNotIsSignedError {..} =>  {
            }
            error => {
                panic!("invalid error type: {}", error)
            }
        };
     }

     #[test]
     fn error_if_transition_is_not_signed_bls()  {
        let   st = get_mock_state_transition();
        let  mut keys = get_test_keys();
        keys.identity_public_key.key_type = KeyType::BLS12_381;
        keys.identity_public_key.data = keys.bls_public.clone();

        let verify_error = st.verify_signature(&keys.identity_public_key).unwrap_err();
        match verify_error {
            ProtocolError::StateTransitionIsNotIsSignedError {..} =>  {
            }
            error => {
                panic!("invalid error type: {}", error)
            }
        };
     }

     #[test]
     fn set_signature() {
        let   mut st = get_mock_state_transition();
        let signature = "some_signature";
        st.set_signature(signature.as_bytes().to_owned());
        assert_eq!(signature.as_bytes(), st.get_signature());
     }

     #[test]
     fn set_signature_public_key_id() {
        let   mut st = get_mock_state_transition();
        let public_key_id = 2;
        st.set_signature_public_key_id(public_key_id);
        assert_eq!(public_key_id, st.get_signature_public_key_id());
     }

}





