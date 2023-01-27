use dpp::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use wasm_bindgen::prelude::*;

use crate::console_log;

#[wasm_bindgen(js_name=StateTransitionExecutionContext)]
pub struct StateTransitionExecutionContextWasm(StateTransitionExecutionContext);

impl From<StateTransitionExecutionContext> for StateTransitionExecutionContextWasm {
    fn from(rs: StateTransitionExecutionContext) -> Self {
        StateTransitionExecutionContextWasm(rs)
    }
}

impl From<StateTransitionExecutionContextWasm> for StateTransitionExecutionContext {
    fn from(wa: StateTransitionExecutionContextWasm) -> Self {
        wa.0
    }
}

impl<'a> From<&'a StateTransitionExecutionContextWasm> for StateTransitionExecutionContext {
    fn from(wa: &StateTransitionExecutionContextWasm) -> Self {
        wa.0.clone()
    }
}

impl<'a> From<&'a StateTransitionExecutionContextWasm> for &'a StateTransitionExecutionContext {
    fn from(wa: &'a StateTransitionExecutionContextWasm) -> Self {
        &wa.0
    }
}

impl<'a> From<&'a StateTransitionExecutionContext> for StateTransitionExecutionContextWasm {
    fn from(rs: &'a StateTransitionExecutionContext) -> Self {
        Self(rs.clone())
    }
}

#[wasm_bindgen(js_class=StateTransitionExecutionContext)]
impl StateTransitionExecutionContextWasm {
    #[wasm_bindgen(constructor)]
    pub fn new() -> StateTransitionExecutionContextWasm {
        StateTransitionExecutionContext::default().into()
    }

    #[wasm_bindgen(js_name=enableDryRun)]
    pub fn enable_dry_run(&self) {
        console_log!("enabling the dry run");
        self.0.enable_dry_run();
    }

    #[wasm_bindgen(js_name=disableDryRun)]
    pub fn disable_dry_run(&self) {
        self.0.disable_dry_run();
    }
}
