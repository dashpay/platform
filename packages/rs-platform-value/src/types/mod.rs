use crate::string_encoding::Encoding;

pub mod binary_data;
pub mod bytes_20;
pub mod bytes_32;
pub mod bytes_36;
pub mod identifier;

fn encoding_string_to_encoding(encoding_string: Option<&str>) -> Encoding {
    match encoding_string {
        Some(str) => {
            //? should it be case-sensitive??
            if str == "base58" {
                Encoding::Base58
            } else {
                Encoding::Base64
            }
        }
        None => Encoding::Base58,
    }
}
