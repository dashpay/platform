use crate::string_encoding::Encoding;

pub(crate) mod binary_data;
pub(crate) mod bytes_20;
pub(crate) mod bytes_32;
pub(crate) mod bytes_36;
pub(crate) mod identifier;

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
