use std::{convert::TryFrom, iter::FromIterator};

use dpp::{
    data_trigger::get_data_triggers_factory::{data_triggers, get_data_triggers},
    document::document_transition::Action,
    prelude::{DataTrigger, Identifier},
};
use itertools::Itertools;
use js_sys::Array;
use wasm_bindgen::prelude::*;

use crate::{
    identifier::IdentifierWrapper,
    utils::{Inner, IntoWasm, WithJsError},
};

use super::DataTriggerWasm;

#[wasm_bindgen(js_name=getDataTriggers)]
pub fn get_data_triggers_wasm(
    js_data_contract_id: &IdentifierWrapper,
    document_type: String,
    transition_action_string: String,
    data_triggers_list: Array,
) -> Result<Array, JsValue> {
    let transition_action = Action::try_from(transition_action_string).with_js_error()?;
    let data_contract_id: Identifier = js_data_contract_id.into();

    let data_triggers: Vec<DataTrigger> = data_triggers_list
        .iter()
        .map(|value| {
            value
                .to_wasm::<DataTriggerWasm>("DataTrigger")
                .map(|v| v.to_owned().into_inner())
        })
        .try_collect()?;

    let data_triggers: Vec<JsValue> = get_data_triggers(
        &data_contract_id,
        &document_type,
        transition_action,
        data_triggers.iter(),
    )
    .with_js_error()?
    .into_iter()
    .map(|dt| DataTriggerWasm::from(dt).into())
    .collect();

    Ok(js_sys::Array::from_iter(data_triggers))
}

#[wasm_bindgen(js_name=getAllDataTriggers)]
pub fn get_all_data_triggers() -> Result<Array, JsValue> {
    let data_triggers: Vec<DataTriggerWasm> = data_triggers()
        .with_js_error()?
        .into_iter()
        .map(DataTriggerWasm::from)
        .collect();
    let array_with_data_triggers = js_sys::Array::new();
    for data_trigger in data_triggers {
        array_with_data_triggers.push(&data_trigger.into());
    }

    Ok(array_with_data_triggers)
}
