// use dpp::state_transition::fee::operations::Operation;
// use wasm_bindgen::prelude::*;
//
pub mod errors;
// pub mod state_transition_facade;
pub mod state_transition_factory;

// pub mod fee;
// pub mod validation;
// use crate::state_transition::conversion::create_operation_from_wasm_instance;
// use crate::utils::Inner;
// pub use fee::*;
// pub use validation::*;
//
// pub(crate) mod conversion;
//
// #[wasm_bindgen(js_name=StateTransitionExecutionContext)]
// #[derive(Clone)]
// pub struct StateTransitionExecutionContextWasm(StateTransitionExecutionContext);
//
// impl From<StateTransitionExecutionContext> for StateTransitionExecutionContextWasm {
//     fn from(rs: StateTransitionExecutionContext) -> Self {
//         StateTransitionExecutionContextWasm(rs)
//     }
// }
//
// impl From<StateTransitionExecutionContextWasm> for StateTransitionExecutionContext {
//     fn from(wa: StateTransitionExecutionContextWasm) -> Self {
//         wa.0
//     }
// }
//
// impl<'a> From<&'a StateTransitionExecutionContextWasm> for StateTransitionExecutionContext {
//     fn from(wa: &StateTransitionExecutionContextWasm) -> Self {
//         wa.0.clone()
//     }
// }
//
// impl<'a> From<&'a StateTransitionExecutionContextWasm> for &'a StateTransitionExecutionContext {
//     fn from(wa: &'a StateTransitionExecutionContextWasm) -> Self {
//         &wa.0
//     }
// }
//
// impl<'a> From<&'a StateTransitionExecutionContext> for StateTransitionExecutionContextWasm {
//     fn from(rs: &'a StateTransitionExecutionContext) -> Self {
//         Self(rs.clone())
//     }
// }
//
// impl Default for StateTransitionExecutionContextWasm {
//     fn default() -> Self {
//         Self::new()
//     }
// }
//
// #[wasm_bindgen(js_class=StateTransitionExecutionContext)]
// impl StateTransitionExecutionContextWasm {
//     #[wasm_bindgen(constructor)]
//     pub fn new() -> StateTransitionExecutionContextWasm {
//         StateTransitionExecutionContext::default().into()
//     }
//
//     #[wasm_bindgen(js_name=enableDryRun)]
//     pub fn enable_dry_run(&self) {
//         self.0.enable_dry_run();
//     }
//
//     #[wasm_bindgen(js_name=disableDryRun)]
//     pub fn disable_dry_run(&self) {
//         self.0.disable_dry_run();
//     }
//
//     #[wasm_bindgen(js_name=isDryRun)]
//     pub fn is_dry_run(&self) -> bool {
//         self.0.is_dry_run()
//     }
//
//     #[wasm_bindgen(js_name=addOperation)]
//     pub fn add_operation(&self, operation: JsValue) -> Result<(), JsValue> {
//         let operation = create_operation_from_wasm_instance(&operation)?;
//         self.0.add_operation(operation);
//         Ok(())
//     }
//
//     #[wasm_bindgen(js_name=getDryOperations)]
//     pub fn get_dry_operations(&self) -> Vec<JsValue> {
//         self.0
//             .get_dry_operations()
//             .iter()
//             .map(|operation| match operation {
//                 Operation::PreCalculated(operation) => {
//                     PreCalculatedOperationWasm::from(operation.to_owned()).into()
//                 }
//                 Operation::Read(operation) => ReadOperationWasm::from(operation.to_owned()).into(),
//                 Operation::SignatureVerification(operation) => {
//                     SignatureVerificationOperationWasm::from(operation.to_owned()).into()
//                 }
//             })
//             .collect()
//     }
//
//     #[wasm_bindgen(js_name=getOperations)]
//     pub fn get_operation(&self) -> Vec<JsValue> {
//         self.0
//             .get_operations()
//             .iter()
//             .map(|operation| match operation {
//                 Operation::PreCalculated(operation) => {
//                     PreCalculatedOperationWasm::from(operation.to_owned()).into()
//                 }
//                 Operation::Read(operation) => ReadOperationWasm::from(operation.to_owned()).into(),
//                 Operation::SignatureVerification(operation) => {
//                     SignatureVerificationOperationWasm::from(operation.to_owned()).into()
//                 }
//             })
//             .collect()
//     }
//
//     #[wasm_bindgen(js_name=clearDryOperations)]
//     pub fn clear_dry_run_operations(&self) {
//         self.0.clear_dry_run_operations();
//     }
// }
//
// impl Inner for StateTransitionExecutionContextWasm {
//     type InnerItem = StateTransitionExecutionContext;
//
//     fn into_inner(self) -> Self::InnerItem {
//         self.0
//     }
//
//     fn inner(&self) -> &Self::InnerItem {
//         &self.0
//     }
//
//     fn inner_mut(&mut self) -> &mut Self::InnerItem {
//         &mut self.0
//     }
// }
