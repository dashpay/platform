use serde_json::Number;
use crate::ProtocolError;
use crate::state_transition::abstract_state_transition::StateTransitionJsonConvert;
use crate::state_transition::data_contract_create_transition::{DataContractCreateTransition};
use crate::state_transition::documents_batch_transition::document_base_transition::JsonValue;
use crate::state_transition::state_transitions::data_contract_create_transition::fields::*;

impl StateTransitionJsonConvert for DataContractCreateTransition {
    fn to_json(&self, skip_signature: bool) -> Result<JsonValue, ProtocolError> {
        match self {
            DataContractCreateTransition::V0(transition) => {
                let mut value = transition.to_json(skip_signature)?;
                let map_value = value.as_object_mut().expect("expected an object");
                map_value.insert(STATE_TRANSITION_PROTOCOL_VERSION.to_string(), JsonValue::Number(Number::from(0)))?;
                Ok(value)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use platform_value::Bytes32;
    use crate::state_transition::abstract_state_transition::StateTransitionJsonConvert;
    use crate::version;
    use crate::state_transition::state_transitions::data_contract_create_transition::fields::*;

    #[test]
    fn should_return_state_transition_in_json_format() {
        let data = crate::state_transition::data_contract_create_transition::test::get_test_data();
        let mut json_object = data
            .state_transition
            .to_json(false)
            .expect("conversion to JSON shouldn't fail");

        assert_eq!(
            version::LATEST_VERSION,
            json_object
                .get_u64(STATE_TRANSITION_PROTOCOL_VERSION)
                .expect("the protocol version should be present") as u32
        );

        assert_eq!(
            0,
            json_object
                .get_u64(TRANSITION_TYPE)
                .expect("the transition type should be present") as u8
        );
        assert_eq!(
            0,
            json_object
                .get_u64(SIGNATURE_PUBLIC_KEY_ID)
                .expect("default public key id should be defined"),
        );
        assert_eq!(
            "",
            json_object
                .remove_into::<String>(SIGNATURE)
                .expect("default string value for signature should be present")
        );

        assert_eq!(
            <Bytes32 as Into<String>>::into(data.created_data_contract.entropy_used),
            json_object
                .remove_into::<String>(ENTROPY)
                .expect("the entropy should be present")
        )
    }
}