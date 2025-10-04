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
    type Error = JsValue;

    fn try_from(value: JsValue) -> Result<ActionGoalWasm, Self::Error> {
        match value.is_string() {
            true => match value.as_string() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val.to_lowercase().as_str() {
                    "actioncompletion" => Ok(ActionGoalWasm::ActionCompletion),
                    "actionparticipation" => Ok(ActionGoalWasm::ActionParticipation),
                    _ => Err(JsValue::from("unknown action type")),
                },
            },
            false => match value.as_f64() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val as u8 {
                    0 => Ok(ActionGoalWasm::ActionCompletion),
                    1 => Ok(ActionGoalWasm::ActionParticipation),
                    _ => Err(JsValue::from("unknown action type")),
                },
            },
        }
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
