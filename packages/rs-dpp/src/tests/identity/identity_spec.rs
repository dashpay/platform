use crate::util::vec::{decode_hex, hex_to_array};

pub fn get_hex() -> &'static str {
    "01000000a4626964782c336275667077516a4c35717376755034666d434b67584a724b4738353244444d596669394a36584b715041546a7075626c69634b65797382a66269640067707572706f7365006d73656375726974794c6576656c00647479706500646461746198210218ea18f2182218e3182d184618b9187f185618f8189018bb182218c318d6185e1827189b181818bd18a20318f30b18d218d318ee18d7186918a31847186268726561644f6e6c79f4a66269640167707572706f7365016d73656375726974794c6576656c03647479706500646461746198210318c00a18f7189318d81831185518f918550218b3183a17151841101894186d18cf186918ca0d18d1188818be18e318b618d10c0d184f188b68726561644f6e6c79f46762616c616e63650a687265766973696f6e00"
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

mod from_buffer {
    use crate::identifier::Identifier;
    use crate::prelude::Identity;
    use crate::tests::fixtures::identity_fixture_json_base;
    use crate::tests::identity::identity_spec::get_buffer;
    use crate::util::string_encoding::Encoding;

    #[test]
    pub fn should_parse_identity() {
        let identity = Identity::from_buffer(get_buffer()).unwrap();

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
}
