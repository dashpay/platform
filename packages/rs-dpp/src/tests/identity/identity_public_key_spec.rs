mod from_raw_object {
    use serde_json::json;

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

        let public_key = IdentityPublicKey::from_raw_object(public_key_json).unwrap();

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

        let public_key = IdentityPublicKey::from_raw_object(public_key_json).unwrap();

        assert_eq!(public_key.id, 0);
        assert_eq!(public_key.key_type, KeyType::ECDSA_SECP256K1);
        assert_eq!(public_key.purpose, Purpose::WITHDRAW);
        assert_eq!(public_key.security_level, SecurityLevel::MASTER);
        assert!(!public_key.read_only);
        assert_eq!(
            public_key.data,
            [
                2, 234, 242, 34, 227, 45, 70, 185, 127, 86, 248, 144, 187, 34, 195, 214, 94, 39,
                155, 24, 189, 162, 3, 243, 11, 210, 211, 238, 215, 105, 163, 71, 98
            ]
        );
    }
}
