use crate::util::vec::{decode_hex, hex_to_array};
use serde::{Deserialize, Serialize};

pub fn get_hex() -> &'static str {
    return "01000000a46269645820648d10ec3a16a37d2e62e8481820dbc2a853834625b065c036e3f998389e6a296762616c616e636500687265766973696f6e006a7075626c69634b65797382a6626964006464617461582102eaf222e32d46b97f56f890bb22c3d65e279b18bda203f30bd2d3eed769a3476264747970650067707572706f73650068726561644f6e6c79f46d73656375726974794c6576656c00a6626964016464617461582103c00af793d83155f95502b33a17154110946dcf69ca0dd188bee3b6d10c0d4f8b64747970650067707572706f73650168726561644f6e6c79f46d73656375726974794c6576656c03";
    //"01000000a4626964782c336275667077516a4c35717376755034666d434b67584a724b4738353244444d596669394a36584b715041546a7075626c69634b65797382a66269640067707572706f7365006d73656375726974794c6576656c00647479706500646461746198210218ea18f2182218e3182d184618b9187f185618f8189018bb182218c318d6185e1827189b181818bd18a20318f30b18d218d318ee18d7186918a31847186268726561644f6e6c79f4a66269640167707572706f7365016d73656375726974794c6576656c03647479706500646461746198210318c00a18f7189318d81831185518f918550218b3183a17151841101894186d18cf186918ca0d18d1188818be18e318b618d10c0d184f188b68726561644f6e6c79f46762616c616e63650a687265766973696f6e00"
}

pub fn get_buffer() -> Vec<u8> {
    decode_hex(get_hex()).unwrap()
}

mod to_buffer {
    use crate::prelude::Identity;
    use crate::tests::fixtures::{identity_fixture_json, identity_fixture_json_base};
    use crate::tests::identity::identity_spec::get_hex;
    use crate::util::vec::encode_hex;

    #[test]
    pub fn should_serialize_correctly() {
        let identity_fixture = identity_fixture_json_base();
        let identity = Identity::from_raw_object(identity_fixture).unwrap();
        let identity_buffer = identity.to_buffer().unwrap();

        println!("{:?}", encode_hex(&identity_buffer));
        assert_eq!(encode_hex(&identity_buffer), get_hex());
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DummyStruct {
    pub id: [u8; 32],
}

mod from_buffer {
    use std::collections::BTreeMap;
    //use serde_json::Value;
    use crate::identifier::Identifier;
    use crate::prelude::Identity;
    use crate::tests::fixtures::identity_fixture_json_base;
    use crate::tests::identity::identity_spec::{get_buffer, get_hex, DummyStruct};
    use crate::util::string_encoding::Encoding;
    use crate::util::vec::{decode_hex, encode_hex};
    use ciborium::value::Value;

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
        let value: [u8; 3] = [1, 2, 3];
        let mut buffer: Vec<u8> = Vec::new();
        ciborium::ser::into_writer(&value, &mut buffer).unwrap();

        println!("{:?}", encode_hex(&buffer));

        let bytes: &[u8] = &buffer;
        let de: Vec<u8> = ciborium::de::from_reader(bytes).unwrap();
        println!("{:?}", de);

        let dummy = DummyStruct {
            id: [
                221, 104, 105, 181, 51, 31, 19, 224, 98, 66, 73, 127, 224, 47, 250, 48, 202, 185,
                191, 198, 54, 246, 17, 109, 41, 56, 83, 33, 49, 175, 4, 31,
            ],
        };
        let mut buffer: Vec<u8> = Vec::new();
        ciborium::ser::into_writer(&dummy.id, &mut buffer).unwrap();
        println!("Dummy: {:?}", encode_hex(&buffer));

        let dummy_he = "a1626964982018dd1868186918b51833181f1318e0186218421849187f18e0182f18fa183018ca18b918bf18c6183618f611186d1829183818531821183118af04181f";
        let real_hex = "5820dd6869b5331f13e06242497fe02ffa30cab9bfc636f6116d2938532131af041f";

        //println!("Start");
        let real_hex = "01000000a46269645820648d10ec3a16a37d2e62e8481820dbc2a853834625b065c036e3f998389e6a296762616c616e636500687265766973696f6e006a7075626c69634b65797382a6626964006464617461582102eaf222e32d46b97f56f890bb22c3d65e279b18bda203f30bd2d3eed769a3476264747970650067707572706f73650068726561644f6e6c79f46d73656375726974794c6576656c00a6626964016464617461582103c00af793d83155f95502b33a17154110946dcf69ca0dd188bee3b6d10c0d4f8b64747970650067707572706f73650168726561644f6e6c79f46d73656375726974794c6576656c03";
        let vec = hex::decode(real_hex).unwrap();
        let (version, read_identity_cbor) = vec.split_at(4);
        println!("kek");
        let identity: BTreeMap<String, Value> =
            ciborium::de::from_reader(read_identity_cbor).unwrap();
        println!("kek 2");
        //println!("{:?}", identity);

        // let identity_js_hex = "01000000a46269645820648d10ec3a16a37d2e62e8481820dbc2a853834625b065c036e3f998389e6a296762616c616e636500687265766973696f6e006a7075626c69634b65797382a6626964006464617461582102eaf222e32d46b97f56f890bb22c3d65e279b18bda203f30bd2d3eed769a3476264747970650067707572706f73650068726561644f6e6c79f46d73656375726974794c6576656c00a6626964016464617461582103c00af793d83155f95502b33a17154110946dcf69ca0dd188bee3b6d10c0d4f8b64747970650067707572706f73650168726561644f6e6c79f46d73656375726974794c6576656c03";
        // let identity_buffer = decode_hex(identity_js_hex).unwrap();
        //
        println!("kek 3");
        let identity = Identity::from_buffer(&vec).unwrap();
        println!("{:?}", identity);
    }
}
