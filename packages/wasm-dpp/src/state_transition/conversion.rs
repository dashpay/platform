use crate::errors::consensus_error::from_consensus_error;
use crate::utils::generic_of_js_val;
use crate::{
    DataContractCreateTransitionWasm, DataContractUpdateTransitionWasm,
    DocumentsBatchTransitionWasm, IdentityCreateTransitionWasm, IdentityTopUpTransitionWasm,
    IdentityUpdateTransitionWasm,
};
use dpp::consensus::basic::state_transition::InvalidStateTransitionTypeError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::state_transition::{StateTransition, StateTransitionType};
use std::convert::TryInto;
use wasm_bindgen::__rt::Ref;
use wasm_bindgen::{JsCast, JsError, JsValue};

pub fn create_state_transition_from_wasm_instance(
    js_value: &JsValue,
) -> Result<StateTransition, JsValue> {
    let get_type_value = js_sys::Reflect::get(js_value, &JsValue::from_str("getType"))
        .map_err(|_| JsError::new("No 'getType' property present in state transition"))?;

    let get_type_function: &js_sys::Function = get_type_value
        .dyn_ref::<js_sys::Function>()
        .ok_or_else(|| JsError::new("'getType' is not a function"))?;

    let raw_state_transition_type = get_type_function
        .call0(js_value)?
        .as_f64()
        .ok_or(JsError::new("Error calling 'getType' function"))?
        as u8;

    let state_transition_type: StateTransitionType =
        raw_state_transition_type.try_into().map_err(|_| {
            from_consensus_error(ConsensusError::BasicError(Box::new(
                BasicError::InvalidStateTransitionTypeError(InvalidStateTransitionTypeError::new(
                    raw_state_transition_type,
                )),
            )))
        })?;

    match state_transition_type {
        StateTransitionType::DataContractCreate => {
            let st: Ref<DataContractCreateTransitionWasm> =
                generic_of_js_val::<DataContractCreateTransitionWasm>(
                    js_value,
                    "DataContractCreateTransition",
                )?;

            Ok(StateTransition::DataContractCreate(st.clone().into()))
        }
        StateTransitionType::DocumentsBatch => {
            let st: Ref<DocumentsBatchTransitionWasm> = generic_of_js_val::<
                DocumentsBatchTransitionWasm,
            >(
                js_value, "DocumentsBatchTransition"
            )?;

            Ok(StateTransition::DocumentsBatch(st.clone().into()))
        }
        StateTransitionType::IdentityCreate => {
            let st: Ref<IdentityCreateTransitionWasm> = generic_of_js_val::<
                IdentityCreateTransitionWasm,
            >(
                js_value, "IdentityCreateTransition"
            )?;

            Ok(StateTransition::IdentityCreate(st.clone().into()))
        }
        StateTransitionType::IdentityTopUp => {
            let st: Ref<IdentityTopUpTransitionWasm> = generic_of_js_val::<
                IdentityTopUpTransitionWasm,
            >(
                js_value, "IdentityTopUpTransition"
            )?;

            Ok(StateTransition::IdentityTopUp(st.clone().into()))
        }
        StateTransitionType::DataContractUpdate => {
            let st: Ref<DataContractUpdateTransitionWasm> =
                generic_of_js_val::<DataContractUpdateTransitionWasm>(
                    js_value,
                    "DataContractUpdateTransition",
                )?;

            Ok(StateTransition::DataContractUpdate(st.clone().into()))
        }
        StateTransitionType::IdentityUpdate => {
            let st: Ref<IdentityUpdateTransitionWasm> = generic_of_js_val::<
                IdentityUpdateTransitionWasm,
            >(
                js_value, "IdentityUpdateTransition"
            )?;

            Ok(StateTransition::IdentityUpdate(st.clone().into()))
        }
        _ => Err(JsError::new("Unsupported state transition type").into()),
    }
}
