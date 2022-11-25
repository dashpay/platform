use std::str::FromStr;

use dashcore::PrivateKey;
use serde_json::{json, Value as JsonValue};

use crate::state_transition::StateTransitionType;
use crate::tests::fixtures::instant_asset_lock_proof_fixture;
use crate::version;

//3bufpwQjL5qsvuP4fmCKgXJrKG852DDMYfi9J6XKqPAT
//[198, 23, 40, 120, 58, 93, 0, 165, 27, 49, 4, 117, 107, 204,  67, 46, 164, 216, 230, 135, 201, 92, 31, 155, 62, 131, 211, 177, 139, 175, 163, 237]

pub fn identity_topup_transition_fixture_json(
    one_time_private_key: Option<PrivateKey>,
) -> JsonValue {
    let asset_lock_proof = instant_asset_lock_proof_fixture(one_time_private_key);
    let asset_lock_string = serde_json::ser::to_string(&asset_lock_proof).unwrap();
    let asset_lock_proof_json = JsonValue::from_str(&asset_lock_string).unwrap();

    json!({
        "protocolVersion": version::LATEST_VERSION,
        "type": StateTransitionType::IdentityTopUp,
        "assetLockProof": asset_lock_proof_json,
        "identityId": [198, 23, 40, 120, 58, 93, 0, 165, 27, 49, 4, 117, 107, 204,  67, 46, 164, 216, 230, 135, 201, 92, 31, 155, 62, 131, 211, 177, 139, 175, 163, 237],
        "signature": vec![0_u8; 65]
    })
}
