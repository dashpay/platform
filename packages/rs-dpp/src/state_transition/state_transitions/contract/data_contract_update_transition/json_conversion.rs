use crate::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use crate::state_transition::state_transitions::data_contract_update_transition::fields::*;
use crate::state_transition::{
    JsonStateTransitionSerializationOptions, StateTransitionJsonConvert,
};
use crate::ProtocolError;
use serde_json::Number;
use serde_json::Value as JsonValue;

impl StateTransitionJsonConvert<'_> for DataContractUpdateTransition {
    fn to_json(
        &self,
        options: JsonStateTransitionSerializationOptions,
    ) -> Result<JsonValue, ProtocolError> {
        match self {
            DataContractUpdateTransition::V0(transition) => {
                let mut value = transition.to_json(options)?;
                let map_value = value.as_object_mut().expect("expected an object");
                map_value.insert(
                    STATE_TRANSITION_PROTOCOL_VERSION.to_string(),
                    JsonValue::Number(Number::from(0)),
                );
                Ok(value)
            }
        }
    }
}
#[cfg(test)]
mod test {
    use super::*;
    use crate::state_transition::data_contract_update_transition::DataContractUpdateTransition;
    use crate::state_transition::JsonStateTransitionSerializationOptions;
    use crate::tests::fixtures::get_data_contract_fixture;
    use platform_version::version::PlatformVersion;
    use platform_version::TryIntoPlatformVersioned;

    fn get_test_data() -> DataContractUpdateTransition {
        let platform_version = PlatformVersion::first();
        let data_contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();

        (data_contract, 1)
            .try_into_platform_versioned(platform_version)
            .expect("expected to get transition")
    }

    #[test]
    fn should_return_state_transition_in_json_format() {
        let json_object = get_test_data()
            .to_json(JsonStateTransitionSerializationOptions {
                skip_signature: false,
                into_validating_json: false,
            })
            .expect("conversion to JSON shouldn't fail");

        assert_eq!(
            Some(0),
            json_object
                .get(STATE_TRANSITION_PROTOCOL_VERSION)
                .and_then(JsonValue::as_u64)
                .map(|v| v as u32),
            "the protocol version should be present",
        );
        assert_eq!(
            None,
            json_object
                .get(TRANSITION_TYPE)
                .and_then(JsonValue::as_u64)
                .map(|v| v as u8),
            "the transition type is not serialized in non-validating JSON",
        );
        assert_eq!(
            Some(0),
            json_object
                .get(SIGNATURE_PUBLIC_KEY_ID)
                .and_then(JsonValue::as_u64),
            "default public key id should be defined",
        );
        assert_eq!(
            Some(""),
            json_object.get(SIGNATURE).and_then(JsonValue::as_str),
            "default string value for signature should be present",
        );
    }
}
