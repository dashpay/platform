use getrandom::getrandom;
use itertools::Itertools;
use lazy_static::lazy_static;
use serde_json::Value;

use crate::prelude::Identifier;

use super::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};

lazy_static! {
    static ref IDENTITY_CREATE_TRANSITION_SCHEMA: Value = serde_json::from_str(include_str!(
        "../schema/identity/stateTransition/identityCreate.json"
    ))
    .unwrap();
}

// TODO change into `const` after stabilizing the `Option::unwrap()`
pub fn get_biggest_possible_identity() -> Identity {
    let max_items = IDENTITY_CREATE_TRANSITION_SCHEMA["properties"]["publicKeys"]["maxItems"]
        .as_u64()
        .expect("the property max_items must exist");

    let public_keys = (0..max_items)
        .into_iter()
        .map(|i| {
            let security_level = if i == 0 {
                SecurityLevel::MASTER
            } else {
                SecurityLevel::HIGH
            };

            IdentityPublicKey {
                id: i,
                key_type: KeyType::BLS12_381,
                purpose: Purpose::AUTHENTICATION,
                security_level,
                read_only: false,
                data: vec![255u8; 48],

                //? is that correct?
                disabled_at: None,
                signature: vec![],
            }
        })
        .collect_vec();

    Identity {
        id: generate_random_identifier_struct(),
        protocol_version: 1,
        public_keys,
        balance: u64::MAX,
        revision: u64::MAX,

        asset_lock_proof: None,
        metadata: None,
    }
}

fn generate_random_identifier_struct() -> Identifier {
    let mut buffer = [0u8; 32];
    let _ = getrandom(&mut buffer);
    Identifier::from_bytes(&buffer).unwrap()
}
