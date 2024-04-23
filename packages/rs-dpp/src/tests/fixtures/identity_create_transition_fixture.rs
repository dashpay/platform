use dashcore::PrivateKey;
use platform_value::BinaryData;
use platform_value::{platform_value, Value};

use crate::identity::{KeyType, Purpose, SecurityLevel};
use crate::tests::fixtures::raw_instant_asset_lock_proof_fixture;
use crate::version;
use platform_value::string_encoding::{decode, Encoding};

//3bufpwQjL5qsvuP4fmCKgXJrKG852DDMYfi9J6XKqPAT
//[198, 23, 40, 120, 58, 93, 0, 165, 27, 49, 4, 117, 107, 204,  67, 46, 164, 216, 230, 135, 201, 92, 31, 155, 62, 131, 211, 177, 139, 175, 163, 237]

pub fn identity_create_transition_fixture(one_time_private_key: Option<PrivateKey>) -> Value {
    let asset_lock_proof = raw_instant_asset_lock_proof_fixture(one_time_private_key, None);

    platform_value!({
        "protocolVersion": version::LATEST_VERSION,
        "type": 2u8,
        "assetLockProof": asset_lock_proof,
        "publicKeys": [
            {
                "id": 0u32,
                "type": KeyType::ECDSA_SECP256K1 as u8,
                "data": BinaryData::new(decode("AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di", Encoding::Base64).unwrap()),
                "purpose": Purpose::AUTHENTICATION as u8,
                "securityLevel": SecurityLevel::MASTER as u8,
                "readOnly": false,
                "signature": BinaryData::new(vec![0_u8; 65])
            },
        ],
        "signature": BinaryData::new(vec![0_u8; 65])
    })
}
