use crate::error::WasmDppError;
use dpp::group::action_taker::ActionGoal;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "ActionGoal")]
#[allow(non_camel_case_types)]
#[derive(Default, Clone)]
pub enum ActionGoalWasm {
    #[default]
    ActionCompletion = 0,
    ActionParticipation = 1,
}

impl From<ActionGoalWasm> for ActionGoal {
    fn from(action_goal: ActionGoalWasm) -> Self {
        match action_goal {
            ActionGoalWasm::ActionCompletion => ActionGoal::ActionCompletion,
            ActionGoalWasm::ActionParticipation => ActionGoal::ActionParticipation,
        }
    }
}

impl From<ActionGoal> for ActionGoalWasm {
    fn from(action_goal: ActionGoal) -> Self {
        match action_goal {
            ActionGoal::ActionCompletion => ActionGoalWasm::ActionCompletion,
            ActionGoal::ActionParticipation => ActionGoalWasm::ActionParticipation,
        }
    }
}

impl TryFrom<JsValue> for ActionGoalWasm {
    type Error = WasmDppError;

    fn try_from(value: JsValue) -> Result<ActionGoalWasm, Self::Error> {
        if let Some(enum_val) = value.as_string() {
            return match enum_val.to_lowercase().as_str() {
                "actioncompletion" => Ok(ActionGoalWasm::ActionCompletion),
                "actionparticipation" => Ok(ActionGoalWasm::ActionParticipation),
                _ => Err(WasmDppError::invalid_argument("unknown action type")),
            };
        }

        if let Some(enum_val) = value.as_f64() {
            return match enum_val as u8 {
                0 => Ok(ActionGoalWasm::ActionCompletion),
                1 => Ok(ActionGoalWasm::ActionParticipation),
                _ => Err(WasmDppError::invalid_argument("unknown action type")),
            };
        }

        Err(WasmDppError::invalid_argument(
            "cannot read value from action goal enum",
        ))
    }
}

impl From<ActionGoalWasm> for String {
    fn from(action_goal: ActionGoalWasm) -> Self {
        match action_goal {
            ActionGoalWasm::ActionCompletion => String::from("ActionCompletion"),
            ActionGoalWasm::ActionParticipation => String::from("ActionParticipation"),
        }
    }
}
