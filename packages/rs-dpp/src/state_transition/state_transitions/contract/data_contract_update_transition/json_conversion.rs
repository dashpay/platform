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
//
// #[cfg(test)]
// mod test {
//     use crate::state_transition::data_contract_update_transition::STATE_TRANSITION_PROTOCOL_VERSION;
//     use crate::state_transition::JsonStateTransitionSerializationOptions;
//
//     #[test]
//     fn should_return_state_transition_in_json_format() {
//         let data = get_test_data();
//         let mut json_object = data
//             .state_transition
//             .to_json(JsonStateTransitionSerializationOptions {
//                 skip_signature: false,
//                 into_validating_json: false,
//             })
//             .expect("conversion to JSON shouldn't fail");
//
//         assert_eq!(
//             0,
//             json_object
//                 .get_u64(STATE_TRANSITION_PROTOCOL_VERSION)
//                 .expect("the protocol version should be present") as u32
//         );
//
//         assert_eq!(
//             4,
//             json_object
//                 .get_u64(TRANSITION_TYPE)
//                 .expect("the transition type should be present") as u8
//         );
//         assert_eq!(
//             0,
//             json_object
//                 .get_u64(SIGNATURE_PUBLIC_KEY_ID)
//                 .expect("default public key id should be defined"),
//         );
//         assert_eq!(
//             "",
//             json_object
//                 .remove_into::<String>(SIGNATURE)
//                 .expect("default string value for signature should be present")
//         );
//     }
// }
