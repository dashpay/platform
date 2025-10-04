use dpp::group::action_taker::ActionGoal;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "ActionGoalWASM")]
#[allow(non_camel_case_types)]
#[derive(Default, Clone)]
pub enum ActionGoalWASM {
    #[default]
    ActionCompletion = 0,
    ActionParticipation = 1,
}

impl From<ActionGoalWASM> for ActionGoal {
    fn from(action_goal: ActionGoalWASM) -> Self {
        match action_goal {
            ActionGoalWASM::ActionCompletion => ActionGoal::ActionCompletion,
            ActionGoalWASM::ActionParticipation => ActionGoal::ActionParticipation,
        }
    }
}

impl From<ActionGoal> for ActionGoalWASM {
    fn from(action_goal: ActionGoal) -> Self {
        match action_goal {
            ActionGoal::ActionCompletion => ActionGoalWASM::ActionCompletion,
            ActionGoal::ActionParticipation => ActionGoalWASM::ActionParticipation,
        }
    }
}

impl TryFrom<JsValue> for ActionGoalWASM {
    type Error = JsValue;

    fn try_from(value: JsValue) -> Result<ActionGoalWASM, Self::Error> {
        match value.is_string() {
            true => match value.as_string() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val.to_lowercase().as_str() {
                    "actioncompletion" => Ok(ActionGoalWASM::ActionCompletion),
                    "actionparticipation" => Ok(ActionGoalWASM::ActionParticipation),
                    _ => Err(JsValue::from("unknown action type")),
                },
            },
            false => match value.as_f64() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val as u8 {
                    0 => Ok(ActionGoalWASM::ActionCompletion),
                    1 => Ok(ActionGoalWASM::ActionParticipation),
                    _ => Err(JsValue::from("unknown action type")),
                },
            },
        }
    }
}

impl From<ActionGoalWASM> for String {
    fn from(action_goal: ActionGoalWASM) -> Self {
        match action_goal {
            ActionGoalWASM::ActionCompletion => String::from("ActionCompletion"),
            ActionGoalWASM::ActionParticipation => String::from("ActionParticipation"),
        }
    }
}
