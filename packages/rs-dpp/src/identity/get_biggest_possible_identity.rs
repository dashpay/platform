use getrandom::getrandom;
use lazy_static::lazy_static;
use serde_json::Value;

use crate::prelude::Identifier;

lazy_static! {
    static ref IDENTITY_CREATE_TRANSITION_SCHEMA: Value = serde_json::from_str(include_str!(
        "../schema/identity/stateTransition/identityCreate.json"
    ))
    .unwrap();
}

fn generate_random_identifier_struct() -> Identifier {
    let mut buffer = [0u8; 32];
    let _ = getrandom(&mut buffer);
    Identifier::from_bytes(&buffer).unwrap()
}
