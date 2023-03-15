use std::convert::TryInto;

use dashcore::PrivateKey;
use platform_value::Value;

use crate::tests::fixtures::instant_asset_lock_proof_fixture;
use crate::version;

//3bufpwQjL5qsvuP4fmCKgXJrKG852DDMYfi9J6XKqPAT
//[198, 23, 40, 120, 58, 93, 0, 165, 27, 49, 4, 117, 107, 204,  67, 46, 164, 216, 230, 135, 201, 92, 31, 155, 62, 131, 211, 177, 139, 175, 163, 237]

pub fn identity_topup_transition_fixture(one_time_private_key: Option<PrivateKey>) -> Value {
    let asset_lock_proof = instant_asset_lock_proof_fixture(one_time_private_key);

    Value::from([
        ("protocolVersion", Value::U32(version::LATEST_VERSION)),
        ("type", Value::U8(2)),
        ("assetLockProof", asset_lock_proof.try_into().unwrap()),
        (
            "identityId",
            Value::Identifier([
                198, 23, 40, 120, 58, 93, 0, 165, 27, 49, 4, 117, 107, 204, 67, 46, 164, 216, 230,
                135, 201, 92, 31, 155, 62, 131, 211, 177, 139, 175, 163, 237,
            ]),
        ),
        ("signature", Value::Bytes(vec![0_u8; 65])),
    ])
}
