use std::{convert::TryFrom, sync::Arc};

use dpp::{
    identity::KeyType,
    prelude::Identifier,
    state_transition::{
        state_transition_execution_context::StateTransitionExecutionContext, StateTransition,
        StateTransitionConvert, StateTransitionIdentitySigned, StateTransitionLike,
    },
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::{
    bls_adapter::BlsAdapter, buffer::Buffer, errors::from_dpp_err, identifier::IdentifierWrapper,
    with_js_error,
};

pub mod errors;
pub mod state_transition_factory;

pub mod validation;
pub use validation::*;

pub(crate) mod conversion;

#[wasm_bindgen(js_name=StateTransitionExecutionContext)]
#[derive(Clone)]
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

impl Default for StateTransitionExecutionContextWasm {
    fn default() -> Self {
        Self::new()
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
        self.0.enable_dry_run();
    }

    #[wasm_bindgen(js_name=disableDryRun)]
    pub fn disable_dry_run(&self) {
        self.0.disable_dry_run();
    }
}

impl Inner for StateTransitionExecutionContextWasm {
    type InnerItem = StateTransitionExecutionContext;

    fn into_inner(self) -> Self::InnerItem {
        self.0
    }

    fn inner(&self) -> &Self::InnerItem {
        &self.0
    }

    fn inner_mut(&mut self) -> &mut Self::InnerItem {
        &mut self.0
    }
}
