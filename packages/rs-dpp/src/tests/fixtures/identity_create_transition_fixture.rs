use std::convert::TryInto;
use std::str::FromStr;

use dashcore::PrivateKey;
use platform_value::Value;

use crate::identity::{KeyType, Purpose, SecurityLevel};
use crate::tests::fixtures::instant_asset_lock_proof_fixture;
use platform_value::string_encoding::{decode, Encoding};
use crate::version;

//3bufpwQjL5qsvuP4fmCKgXJrKG852DDMYfi9J6XKqPAT
//[198, 23, 40, 120, 58, 93, 0, 165, 27, 49, 4, 117, 107, 204,  67, 46, 164, 216, 230, 135, 201, 92, 31, 155, 62, 131, 211, 177, 139, 175, 163, 237]

pub fn identity_create_transition_fixture_json(
    one_time_private_key: Option<PrivateKey>,
) -> Value {
    let asset_lock_proof = instant_asset_lock_proof_fixture(one_time_private_key);
    let read_only = false;
    let signature = vec![0_u8; 65];

    let public_keys =     vec![Value::from([("id", Value::U32(version::LATEST_VERSION)),
                                       ("type", Value::U8(2)),
                                       ("data", Value::Bytes(decode("AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di", Encoding::Base64).unwrap())),
                                       ("purpose": Value::U8(Purpose::AUTHENTICATION)),
        ("securityLevel": Value::U8(SecurityLevel::MASTER)),
        ("readOnly": Value::Bool(read_only)),
        ("signature": Value::Bytes(signature))])];

    Value::from([("protocolVersion", Value::U32(version::LATEST_VERSION)),
                  ("type", Value::U8(2)),
                  ("assetLockProof", asset_lock_proof.try_into().unwrap()),
                  ("publicKeys": Value::Array(public_keys)),
        ("signature": Value::Bytes(signature))])
}
