mod from_raw_object {
    use bls_signatures::Serialize;
    use dashcore::PublicKey;
    use serde_json::{json, Value};

    use crate::identity::{KeyType, Purpose, SecurityLevel};
    use crate::prelude::IdentityPublicKey;

    #[test]
    pub fn should_parse_raw_key() {
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
        assert_eq!(public_key.read_only, false);
        assert_eq!(
            public_key.data,
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
            "data": "AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di",
            "readOnly": false
        });

        let public_key = IdentityPublicKey::from_json_object(public_key_json)
            .expect("the public key should be created");
        assert_eq!(public_key.get_type(), KeyType::BIP13_SCRIPT_HASH);
        assert_eq!(
            public_key.hash().unwrap(),
            [
                2, 234, 242, 34, 227, 45, 70, 185, 127, 86, 248, 144, 187, 34, 195, 214, 94, 39,
                155, 24, 189, 162, 3, 243, 11, 210, 211, 238, 215, 105, 163, 71, 98
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
        let public_key: IdentityPublicKey = serde_json::from_str(pk_str).unwrap();

        // let public_key = IdentityPublicKey::from_raw_object(public_key_json).unwrap();

        assert_eq!(public_key.id, 0);
        assert_eq!(public_key.key_type, KeyType::ECDSA_SECP256K1);
        assert_eq!(public_key.purpose, Purpose::AUTHENTICATION);
        assert_eq!(public_key.security_level, SecurityLevel::MASTER);
        assert_eq!(public_key.read_only, false);
        assert_eq!(
            public_key.data,
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

        let public_key_json = json!({
            "id": 0,
            "type": KeyType::ECDSA_SECP256K1,
            "purpose": Purpose::AUTHENTICATION,
            "securityLevel": SecurityLevel::MASTER,
            "data": public_key,
            "readOnly": false
        });

        let public_key = IdentityPublicKey::from_raw_object(public_key_json)
            .expect("the public key should be created");
        assert_eq!(public_key.get_type(), KeyType::ECDSA_SECP256K1);
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
        let bls_public_key = bls_signatures::PrivateKey::new(private_key)
            .public_key()
            .as_bytes();

        let public_key_json = json!({
            "id": 0,
            "type": KeyType::BLS12_381,
            "purpose": Purpose::AUTHENTICATION,
            "securityLevel": SecurityLevel::MASTER,
            "data": bls_public_key,
            "readOnly": false
        });

        let public_key = IdentityPublicKey::from_raw_object(public_key_json)
            .expect("the public key should be created");
        assert_eq!(public_key.get_type(), KeyType::BLS12_381);
        assert_eq!(
            vec![
                111, 89, 76, 223, 228, 50, 201, 143, 165, 74, 149, 193, 215, 143, 217, 170, 49,
                108, 229, 150
            ],
            public_key.hash().unwrap()
        );
    }
}
