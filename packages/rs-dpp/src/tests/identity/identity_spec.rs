use crate::util::vec::{decode_hex, hex_to_array};
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
    //"01000000a4626964782c336275667077516a4c35717376755034666d434b67584a724b4738353244444d596669394a36584b715041546a7075626c69634b65797382a66269640067707572706f7365006d73656375726974794c6576656c00647479706500646461746198210218ea18f2182218e3182d184618b9187f185618f8189018bb182218c318d6185e1827189b181818bd18a20318f30b18d218d318ee18d7186918a31847186268726561644f6e6c79f4a66269640167707572706f7365016d73656375726974794c6576656c03647479706500646461746198210318c00a18f7189318d81831185518f918550218b3183a17151841101894186d18cf186918ca0d18d1188818be18e318b618d10c0d184f188b68726561644f6e6c79f46762616c616e63650a687265766973696f6e00"
}

pub fn get_buffer() -> Vec<u8> {
    hex::decode(identity_cbor_hex()).unwrap()
}

mod to_buffer {
    use crate::prelude::Identity;
    use crate::tests::fixtures::{identity_fixture_json, identity_fixture_json_base};
    use crate::tests::identity::identity_spec::identity_cbor_hex;
    use crate::util::vec::encode_hex;

    #[test]
    pub fn should_serialize_correctly() {
        let identity = Identity::from_buffer(hex::decode(identity_cbor_hex()).unwrap()).unwrap();
        let identity_buffer = identity.to_cbor().unwrap();

        println!("{:?}", encode_hex(&identity_buffer));
        assert_eq!(encode_hex(&identity_buffer), identity_cbor_hex());
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DummyStruct {
    pub id: [u8; 32],
}

mod from_buffer {
    use std::collections::BTreeMap;
    use std::convert::TryFrom;
    use crate::identifier::Identifier;
    use crate::prelude::Identity;
    use crate::tests::fixtures::identity_fixture_json_base;
    use crate::tests::identity::identity_spec::{get_buffer, identity_cbor_hex, DummyStruct};
    use crate::util::string_encoding::Encoding;
    use crate::util::vec::{decode_hex, encode_hex};
    use ciborium::value::Value;
    use crate::identity::{KeyType, Purpose, SecurityLevel};

    #[test]
    pub fn should_parse_identity() {
        let identity = Identity::from_buffer(get_buffer()).unwrap();
        println!("{:?}", identity);

        assert_eq!(
            identity.id,
            Identifier::from_string(
                "3bufpwQjL5qsvuP4fmCKgXJrKG852DDMYfi9J6XKqPAT",
                Encoding::Base58
            )
            .unwrap()
        );
        assert_eq!(identity.balance, 10);
        assert_eq!(identity.revision, 0);
        assert_eq!(identity.public_keys.len(), 2);
    }

    #[test]
    pub fn should_parse_hex_from_js_dpp() {
        let identity_cbor = hex::decode(identity_cbor_hex()).unwrap();

        let identity = Identity::from_buffer(&identity_cbor).unwrap();
        assert_eq!(identity.get_protocol_version(), 1);
        assert_eq!(identity.get_id().to_buffer(), [
            136,  29,  45,  41, 192, 117, 244,  93,
            108, 167,  36, 206,  22, 227,  39, 247,
            228, 166,  68,  27,  19,  27, 119, 192,
            10, 232, 143,   8,  36, 226,  31, 148
        ]);
        assert_eq!(identity.get_balance(), 0);
        assert_eq!(identity.get_revision(), 0);

        assert_eq!(identity.public_keys.len(), 2);

        let pk_1 = identity.public_keys.first().unwrap();
        let pk_2 = identity.public_keys.get(1).unwrap();

        assert_eq!(pk_1.id, 0);
        assert_eq!(pk_1.key_type, KeyType::try_from(0u8).unwrap());
        assert_eq!(pk_1.purpose, Purpose::try_from(0u8).unwrap());
        assert_eq!(pk_1.security_level, SecurityLevel::try_from(0u8).unwrap());
        assert_eq!(pk_1.data, vec![
            2, 234, 242,  34, 227,  45,  70, 185,
            127,  86, 248, 144, 187,  34, 195, 214,
            94,  39, 155,  24, 189, 162,   3, 243,
            11, 210, 211, 238, 215, 105, 163,  71,
            98
        ]);
        assert_eq!(pk_1.read_only, false);

        assert_eq!(pk_2.id, 1);
        assert_eq!(pk_2.key_type, KeyType::try_from(0u8).unwrap());
        assert_eq!(pk_2.purpose, Purpose::try_from(1u8).unwrap());
        assert_eq!(pk_2.security_level, SecurityLevel::try_from(3u8).unwrap());
        assert_eq!(pk_2.data, vec![
            3, 192,  10, 247, 147, 216, 49,  85,
            249,  85,   2, 179,  58,  23, 21,  65,
            16, 148, 109, 207, 105, 202, 13, 209,
            136, 190, 227, 182, 209,  12, 13,  79,
            139
        ]);
        assert_eq!(pk_2.read_only, false);

        println!("{:?}", identity);
    }
}
