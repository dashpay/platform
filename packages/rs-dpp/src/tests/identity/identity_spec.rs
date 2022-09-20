use serde::{Deserialize, Serialize};

// {
// protocolVersion: 1,
// id: Buffer(32) [Uint8Array] [
// 136,  29,  45,  41, 192, 117, 244,  93,
// 108, 167,  36, 206,  22, 227,  39, 247,
// 228, 166,  68,  27,  19,  27, 119, 192,
// 10, 232, 143,   8,  36, 226,  31, 148
// ],
// publicKeys: [
// {
// id: 0,
// type: 0,
// purpose: 0,
// securityLevel: 0,
// data: Buffer(33) [Uint8Array] [
// 2, 234, 242,  34, 227,  45,  70, 185,
// 127,  86, 248, 144, 187,  34, 195, 214,
// 94,  39, 155,  24, 189, 162,   3, 243,
// 11, 210, 211, 238, 215, 105, 163,  71,
// 98
// ],
// readOnly: false
// },
// {
// id: 1,
// type: 0,
// purpose: 1,
// securityLevel: 3,
// data: Buffer(33) [Uint8Array] [
// 3, 192,  10, 247, 147, 216, 49,  85,
// 249,  85,   2, 179,  58,  23, 21,  65,
// 16, 148, 109, 207, 105, 202, 13, 209,
// 136, 190, 227, 182, 209,  12, 13,  79,
// 139
// ],
// readOnly: false
// }
// ],
// balance: 0,
// revision: 0
// }

pub fn identity_cbor_hex() -> &'static str {
    return "01000000a46269645820881d2d29c075f45d6ca724ce16e327f7e4a6441b131b77c00ae88f0824e21f946762616c616e636500687265766973696f6e006a7075626c69634b65797382a6626964006464617461582102eaf222e32d46b97f56f890bb22c3d65e279b18bda203f30bd2d3eed769a3476264747970650067707572706f73650068726561644f6e6c79f46d73656375726974794c6576656c00a6626964016464617461582103c00af793d83155f95502b33a17154110946dcf69ca0dd188bee3b6d10c0d4f8b64747970650067707572706f73650168726561644f6e6c79f46d73656375726974794c6576656c03";
}

pub fn get_buffer() -> Vec<u8> {
    hex::decode(identity_cbor_hex()).unwrap()
}

mod to_buffer {
    use crate::prelude::Identity;
    use crate::tests::identity::identity_spec::identity_cbor_hex;
    use crate::util::vec::encode_hex;

    #[test]
    pub fn should_serialize_correctly() {
        let identity = Identity::from_buffer(hex::decode(identity_cbor_hex()).unwrap()).unwrap();
        let identity_buffer = identity.to_cbor().unwrap();

        assert_eq!(encode_hex(&identity_buffer), identity_cbor_hex());
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DummyStruct {
    pub id: [u8; 32],
}

mod from_buffer {
    use std::convert::TryFrom;

    use crate::identity::{KeyType, Purpose, SecurityLevel};
    use crate::prelude::Identity;
    use crate::tests::identity::identity_spec::identity_cbor_hex;

    #[test]
    pub fn should_parse_hex_from_js_dpp() {
        let identity_cbor = hex::decode(identity_cbor_hex()).unwrap();

        let identity = Identity::from_buffer(&identity_cbor).unwrap();
        assert_eq!(identity.get_protocol_version(), 1);
        assert_eq!(
            identity.get_id().to_buffer(),
            [
                136, 29, 45, 41, 192, 117, 244, 93, 108, 167, 36, 206, 22, 227, 39, 247, 228, 166,
                68, 27, 19, 27, 119, 192, 10, 232, 143, 8, 36, 226, 31, 148
            ]
        );
        assert_eq!(identity.get_balance(), 0);
        assert_eq!(identity.get_revision(), 0);

        assert_eq!(identity.public_keys.len(), 2);

        let pk_1 = identity.public_keys.first().unwrap();
        let pk_2 = identity.public_keys.get(1).unwrap();

        assert_eq!(pk_1.id, 0);
        assert_eq!(pk_1.key_type, KeyType::try_from(0u8).unwrap());
        assert_eq!(pk_1.purpose, Purpose::try_from(0u8).unwrap());
        assert_eq!(pk_1.security_level, SecurityLevel::try_from(0u8).unwrap());
        assert_eq!(
            pk_1.data,
            vec![
                2, 234, 242, 34, 227, 45, 70, 185, 127, 86, 248, 144, 187, 34, 195, 214, 94, 39,
                155, 24, 189, 162, 3, 243, 11, 210, 211, 238, 215, 105, 163, 71, 98
            ]
        );
        assert_eq!(pk_1.read_only, false);

        assert_eq!(pk_2.id, 1);
        assert_eq!(pk_2.key_type, KeyType::try_from(0u8).unwrap());
        assert_eq!(pk_2.purpose, Purpose::try_from(1u8).unwrap());
        assert_eq!(pk_2.security_level, SecurityLevel::try_from(3u8).unwrap());
        assert_eq!(
            pk_2.data,
            vec![
                3, 192, 10, 247, 147, 216, 49, 85, 249, 85, 2, 179, 58, 23, 21, 65, 16, 148, 109,
                207, 105, 202, 13, 209, 136, 190, 227, 182, 209, 12, 13, 79, 139
            ]
        );
        assert_eq!(pk_2.read_only, false);
    }
}

mod conversions {
    use crate::prelude::Identity;
    use crate::tests::fixtures::{identity_fixture_json, identity_fixture_raw_object};
    use crate::util::string_encoding;
    use crate::util::string_encoding::Encoding;

    #[test]
    fn from_json() {
        let expected_identity_id = string_encoding::decode(
            "3bufpwQjL5qsvuP4fmCKgXJrKG852DDMYfi9J6XKqPAT",
            Encoding::Base58,
        )
        .unwrap();

        let identity_fixture = identity_fixture_json();

        let identity =
            Identity::from_json(identity_fixture).expect("Expected from_json to parse an Object");

        assert_eq!(identity.id.to_buffer().to_vec(), expected_identity_id);
    }

    #[test]
    fn from_raw_object() {
        let identity_fixture = identity_fixture_raw_object();

        let identity = Identity::from_raw_object(identity_fixture)
            .expect("Expected from_raw_object to parse an Object");

        assert_eq!(
            identity.id.to_buffer(),
            [
                198, 23, 40, 120, 58, 93, 0, 165, 27, 49, 4, 117, 107, 204, 67, 46, 164, 216, 230,
                135, 201, 92, 31, 155, 62, 131, 211, 177, 139, 175, 163, 237
            ]
        )
    }
}

mod api {
    use crate::{
        identity::{Purpose, SecurityLevel},
        prelude::IdentityPublicKey,
        tests::fixtures::identity_fixture,
    };

    #[test]
    fn should_get_biggest_public_key_id() {
        let mut identity = identity_fixture();

        let identity_public_key_1 = IdentityPublicKey {
            id: 99,
            key_type: crate::identity::KeyType::ECDSA_SECP256K1,
            data: vec![97_u8, 36],
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            read_only: false,
            disabled_at: None,
            signature: vec![],
        };
        let identity_public_key_2 = IdentityPublicKey {
            id: 50,
            key_type: crate::identity::KeyType::ECDSA_SECP256K1,
            data: vec![97_u8, 36],
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            read_only: false,
            disabled_at: None,
            signature: vec![],
        };

        identity.add_public_keys([identity_public_key_1, identity_public_key_2]);
        assert_eq!(99, identity.get_public_key_max_id());
    }
}
