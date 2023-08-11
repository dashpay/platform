mod from_object {
    use dashcore::PublicKey;
    use platform_value::platform_value;
    use platform_value::BinaryData;
    use serde_json::json;
    use std::convert::TryInto;

    use crate::identity::{KeyType, Purpose, SecurityLevel};
    use crate::prelude::IdentityPublicKey;

    #[test]
    pub fn should_parse_raw_json_key() {
        let public_key_json = json!({
            "id": 0,
            "type": 0,
            "purpose": 0,
            "securityLevel": 0,
            "data": "AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di",
            "readOnly": false
        });

        let public_key = IdentityPublicKey::from_json_object(public_key_json).unwrap();

        assert_eq!(public_key.id, 0);
        assert_eq!(public_key.key_type, KeyType::ECDSA_SECP256K1);
        assert_eq!(public_key.purpose, Purpose::AUTHENTICATION);
        assert_eq!(public_key.security_level, SecurityLevel::MASTER);
        assert!(!public_key.read_only);
        assert_eq!(
            public_key.data.to_vec(),
            [
                2, 234, 242, 34, 227, 45, 70, 185, 127, 86, 248, 144, 187, 34, 195, 214, 94, 39,
                155, 24, 189, 162, 3, 243, 11, 210, 211, 238, 215, 105, 163, 71, 98
            ]
        );
    }

    #[test]
    pub fn should_parse_key_of_withdraw_purpose() {
        let public_key_json = json!({
            "id": 0,
            "type": 0,
            "purpose": 3,
            "securityLevel": 0,
            "data": "AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di",
            "readOnly": false
        });

        let public_key = IdentityPublicKey::from_json_object(public_key_json).unwrap();

        assert_eq!(public_key.id, 0);
        assert_eq!(public_key.key_type, KeyType::ECDSA_SECP256K1);
        assert_eq!(public_key.purpose, Purpose::WITHDRAW);
        assert_eq!(public_key.security_level, SecurityLevel::MASTER);
        assert!(!public_key.read_only);
    }

    #[test]
    pub fn should_return_data_in_case_bip13_script_hash() {
        let public_key_json = json!({
            "id": 0,
            "type": KeyType::BIP13_SCRIPT_HASH,
            "purpose": Purpose::AUTHENTICATION,
            "securityLevel": SecurityLevel::MASTER,
            "data": "n6I3y1cTf2efmnf3/oFvAmjpGQ8=",
            "readOnly": false
        });

        let public_key = IdentityPublicKey::from_json_object(public_key_json)
            .expect("the public key should be created");
        assert_eq!(public_key.key_type, KeyType::BIP13_SCRIPT_HASH);
        assert_eq!(
            public_key.hash().unwrap(),
            [
                159, 162, 55, 203, 87, 19, 127, 103, 159, 154, 119, 247, 254, 129, 111, 2, 104,
                233, 25, 15,
            ]
        );
    }

    #[test]
    pub fn should_parse_key_object_from_dpp() {
        let pk_str = "{\
            \"id\":0,\
            \"type\":0,\
            \"data\":[2, 234, 242, 34, 227, 45, 70, 185, 127, 86, 248, 144, 187, 34, 195, 214, 94, 39, 155, 24, 189, 162, 3, 243, 11, 210, 211, 238, 215, 105, 163, 71, 98 ],\
            \"purpose\":0,\
            \"securityLevel\":0, \
            \"readOnly\":false \
        }";
        let public_key: IdentityPublicKey = pk_str
            .try_into()
            .expect("expected to convert to IdentityPublicKey");

        // let public_key = IdentityPublicKey::from_object(public_key_json).unwrap();

        assert_eq!(public_key.id, 0);
        assert_eq!(public_key.key_type, KeyType::ECDSA_SECP256K1);
        assert_eq!(public_key.purpose, Purpose::AUTHENTICATION);
        assert_eq!(public_key.security_level, SecurityLevel::MASTER);
        assert!(!public_key.read_only);
        assert_eq!(
            public_key.data.to_vec(),
            [
                2, 234, 242, 34, 227, 45, 70, 185, 127, 86, 248, 144, 187, 34, 195, 214, 94, 39,
                155, 24, 189, 162, 3, 243, 11, 210, 211, 238, 215, 105, 163, 71, 98
            ]
        );
    }

    //"{\"id\":0,\"type\":0,\"data\":\"YWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFh\",\"purpose\":0,\"securityLevel\":0,\"readOnly\":false}"

    #[test]
    pub fn should_return_true_if_public_key_is_master() {
        let public_key_json = json!({
            "id": 0,
            "type": KeyType::BIP13_SCRIPT_HASH,
            "purpose": Purpose::AUTHENTICATION,
            "securityLevel": SecurityLevel::MASTER,
            "data": "AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di",
            "readOnly": false
        });

        let public_key = IdentityPublicKey::from_json_object(public_key_json)
            .expect("the public key should be created");
        assert!(public_key.is_master());
    }

    #[test]
    pub fn should_return_false_if_public_key_is_not_master() {
        let public_key_json = json!({
            "id": 0,
            "type": KeyType::BIP13_SCRIPT_HASH,
            "purpose": Purpose::AUTHENTICATION,
            "securityLevel": SecurityLevel::CRITICAL,

            "data": "AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di",
            "readOnly": false
        });

        let public_key = IdentityPublicKey::from_json_object(public_key_json)
            .expect("the public key should be created");

        assert!(!public_key.is_master());
    }

    #[test]
    pub fn should_return_hash_for_ecdsa_public_key() {
        let public_key_compressed =
            "02716899be7008396a0b34dd49d9707b01e86265f9556ab54a493e712d42946e7a";
        let public_key_compressed_bytes = hex::decode(public_key_compressed).unwrap();
        let public_key = PublicKey::from_slice(&public_key_compressed_bytes)
            .unwrap()
            .to_bytes();

        let public_key_json = platform_value!({
            "id": 0u32,
            "type": KeyType::ECDSA_SECP256K1 as u8,
            "purpose": Purpose::AUTHENTICATION as u8,
            "securityLevel": SecurityLevel::MASTER as u8,
            "data": BinaryData::new(public_key),
            "readOnly": false
        });

        let public_key = IdentityPublicKey::from_value(public_key_json)
            .expect("the public key should be created");
        assert_eq!(public_key.key_type, KeyType::ECDSA_SECP256K1);
        assert_eq!(
            vec![
                92, 158, 158, 5, 73, 213, 211, 15, 121, 255, 72, 55, 220, 47, 233, 182, 143, 218,
                51, 12
            ],
            public_key.hash().unwrap()
        );
    }

    #[test]
    pub fn should_return_hash_for_bls_public_key() {
        let private_key = [1u8; 32];
        let bls_public_key = bls_signatures::PrivateKey::from_bytes(private_key.as_slice(), false)
            .expect("expected to get private key")
            .g1_element()
            .expect("expected to make public key")
            .to_bytes()
            .to_vec();

        let public_key_json = platform_value!({
            "id": 0u32,
            "type": KeyType::BLS12_381 as u8,
            "purpose": Purpose::AUTHENTICATION as u8,
            "securityLevel": SecurityLevel::MASTER as u8,
            "data": BinaryData::new(bls_public_key),
            "readOnly": false
        });

        let public_key = IdentityPublicKey::from_value(public_key_json)
            .expect("the public key should be created");
        assert_eq!(public_key.key_type, KeyType::BLS12_381);
        assert_eq!(
            vec![
                41, 182, 195, 61, 168, 53, 154, 177, 166, 144, 85, 113, 1, 53, 136, 83, 16, 114,
                51, 82
            ],
            public_key.hash().unwrap()
        );
    }
}
