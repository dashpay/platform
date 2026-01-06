use crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
use crate::state_transition::state_transitions::address_funds_transfer_transition::fields::*;
use crate::state_transition::{
    JsonStateTransitionSerializationOptions, StateTransitionJsonConvert,
};
use crate::ProtocolError;
use serde_json::Number;
use serde_json::Value as JsonValue;

impl StateTransitionJsonConvert<'_> for AddressFundsTransferTransition {
    fn to_json(
        &self,
        options: JsonStateTransitionSerializationOptions,
    ) -> Result<JsonValue, ProtocolError> {
        match self {
            AddressFundsTransferTransition::V0(transition) => {
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
