use crate::prelude::Identifier;

fn generate_random_identifier_struct() -> Identifier {
    let mut buffer = [0u8; 32];
    let _ = getrandom::getrandom(&mut buffer);
    Identifier::from_bytes(&buffer).unwrap()
}
