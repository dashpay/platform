use crate::identifier::Identifier;
use crate::util::string_encoding::Encoding;

#[test]
pub fn from_string() {
    let string_id = "EDCuAy8AXqAh56eFRkKRKb79SC35csP3W9VPe1UMaz87";
    let buffer_id: [u8; 32] = [
        196,  72,  90, 197, 121,  84, 125, 235,
        40,  10,  91, 100, 170,  33,  98,  93,
        203, 213, 159,  81, 146, 213, 142,  95,
        74, 193,  29, 134,  81,  15, 181, 104
    ];

    // Testing from_string
    let identifier = Identifier::from_string(string_id, Encoding::Base58).unwrap();

    // Testing to_string
    let identifier2 = Identifier::from_string(&identifier.to_string(Encoding::Base58), Encoding::Base58).unwrap();

    // Testing to_buffer
    assert_eq!(identifier.to_buffer(), identifier2.to_buffer());
    assert_eq!(identifier.to_buffer(), buffer_id);
}

#[test]
pub fn from_string_fails_for_strings_encoding_more_than_32_bytes() {
    let res = Identifier::from_string("tprv8ZgxMBicQKsPest8KoX5aRksibjZwu1nYwqxc3VWzn2vkV5BjQtX87oYPz9CfwSwXv2oHq4MnjiZArA1kYGFZbrGnP7Vvcd55zpByJPPitv", Encoding::Base58);

    match res {
        Err(err) => assert_eq!(err.message, "Identifier must be 32 bytes long"),
        Ok(_) => panic!("Expected from_string to return error")
    }
}