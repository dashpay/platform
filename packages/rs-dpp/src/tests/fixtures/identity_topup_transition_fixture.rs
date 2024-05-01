use crate::state_transition::StateTransitionType;
use dashcore::PrivateKey;
use platform_value::{platform_value, BinaryData, Identifier, Value};

use crate::tests::fixtures::raw_instant_asset_lock_proof_fixture;
use crate::version;

//3bufpwQjL5qsvuP4fmCKgXJrKG852DDMYfi9J6XKqPAT
//[198, 23, 40, 120, 58, 93, 0, 165, 27, 49, 4, 117, 107, 204,  67, 46, 164, 216, 230, 135, 201, 92, 31, 155, 62, 131, 211, 177, 139, 175, 163, 237]

pub fn identity_topup_transition_fixture(one_time_private_key: Option<PrivateKey>) -> Value {
    let asset_lock_proof = raw_instant_asset_lock_proof_fixture(one_time_private_key, None);
    platform_value!({
        "protocolVersion": version::LATEST_VERSION,
        "type": StateTransitionType::IdentityTopUp as u8,
        "assetLockProof": asset_lock_proof,
        "identityId": Identifier::new([198, 23, 40, 120, 58, 93, 0, 165, 27, 49, 4, 117, 107, 204,  67, 46, 164, 216, 230, 135, 201, 92, 31, 155, 62, 131, 211, 177, 139, 175, 163, 237]),
        "signature": BinaryData::new(vec![0_u8; 65])
    })
}
