use getrandom::getrandom;
use lazy_static::lazy_static;
use serde_json::Value;

use crate::prelude::Identifier;

fn generate_random_identifier_struct() -> Identifier {
    let mut buffer = [0u8; 32];
    let _ = getrandom(&mut buffer);
    Identifier::from_bytes(&buffer).unwrap()
}
