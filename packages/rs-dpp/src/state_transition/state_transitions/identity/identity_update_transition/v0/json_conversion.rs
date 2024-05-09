use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
use crate::state_transition::StateTransitionJsonConvert;

impl<'a> StateTransitionJsonConvert<'a> for IdentityUpdateTransitionV0 {}

#[cfg(test)]
mod test {
    use crate::identity::accessors::IdentityGettersV0;
    use crate::state_transition::identity_update_transition::fields::property_names::*;
    use crate::state_transition::identity_update_transition::fields::*;
    use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
    use crate::state_transition::identity_update_transition::IdentityUpdateTransition;
    use crate::state_transition::{
        JsonStateTransitionSerializationOptions, StateTransitionJsonConvert,
    };
    use crate::tests::fixtures::identity_v0_fixture;
    use crate::tests::utils::generate_random_identifier_struct;
    use platform_value::BinaryData;
    use serde_json::Value as JsonValue;

    #[test]
    fn conversion_to_json_object() {
        let public_key = identity_v0_fixture().public_keys()[&0].to_owned();
        let buffer = [0u8; 33];
        let transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: generate_random_identifier_struct(),
            revision: 0,
            nonce: 1,
            add_public_keys: vec![public_key.into()],
            disable_public_keys: vec![],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: BinaryData::new(buffer.to_vec()),
        }
        .into();

        let result = transition
            .to_json(JsonStateTransitionSerializationOptions {
                skip_signature: false,
                into_validating_json: false,
            })
            .expect("conversion to json shouldn't fail");
        assert!(matches!(result[IDENTITY_ID], JsonValue::String(_)));
        assert!(matches!(result[SIGNATURE], JsonValue::String(_)));
        assert!(matches!(
            result[ADD_PUBLIC_KEYS][0]["data"],
            JsonValue::String(_)
        ));
    }
}
