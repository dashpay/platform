use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
use crate::state_transition::StateTransitionJsonConvert;

impl StateTransitionJsonConvert for IdentityUpdateTransitionV0 {}


#[cfg(test)]
mod test {
    use crate::tests::{fixtures::identity_fixture, utils::generate_random_identifier_struct};
    use getrandom::getrandom;
    use platform_value::BinaryData;
    use crate::state_transition::{JsonSerializationOptions, StateTransitionJsonConvert};
    use crate::state_transition::identity_update_transition::fields::*;
    use crate::state_transition::identity_update_transition::fields::property_names::*;
    use serde_json::Value as JsonValue;
    use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;

    #[test]
    fn conversion_to_json_object() {
        let public_key = identity_fixture().get_public_keys()[&0].to_owned();
        let mut buffer = [0u8; 33];
        let _ = getrandom(&mut buffer);
        let transition = IdentityUpdateTransitionV0 {
            identity_id: generate_random_identifier_struct(),
            add_public_keys: vec![(&public_key).into()],
            signature: BinaryData::new(buffer.to_vec()),
            ..Default::default()
        };

        let result = transition
            .to_json(JsonSerializationOptions {
                skip_signature: false,
                into_validating_json: false,
            })
            .expect("conversion to json shouldn't fail");
        assert!(matches!(
            result[IDENTITY_ID],
            JsonValue::String(_)
        ));
        assert!(matches!(
            result[SIGNATURE],
            JsonValue::String(_)
        ));
        assert!(matches!(
            result[ADD_PUBLIC_KEYS][0]["data"],
            JsonValue::String(_)
        ));
    }
}