
use crate::state_transition::state_transition_fee::operations::Operation;


#[derive(Debug, Clone, Default)]
pub struct StateTransitionExecutionContext {
    operations : Vec<Operation>,
}
//
// #[derive(Default, Debug, Clone)]
// struct StateTransitionContextInner {
//     actual_operations: Vec<Operation>,
//     dry_run_operations: Vec<Operation>,
//     is_dry_run: bool,
// }
//
// impl StateTransitionExecutionContext {
//     /// Add [`Operation`] into the execution context
//     pub fn add_operation(&self, operation: Operation) {
//         let mut inner = self.inner.lock().unwrap();
//         if inner.is_dry_run {
//             inner.dry_run_operations.push(operation);
//         } else {
//             inner.actual_operations.push(operation);
//         }
//     }
//
//     /// Add more than one [`Operation`] into the execution context
//     pub fn add_operations(&self, operations: impl IntoIterator<Item = Operation>) {
//         let mut inner = self.inner.lock().unwrap();
//         if inner.is_dry_run {
//             inner.dry_run_operations.extend(operations);
//         } else {
//             inner.actual_operations.extend(operations);
//         }
//     }
//
//     /// Replace all existing operations with a new collection of operations
//     pub fn set_operations(&self, operations: Vec<Operation>) {
//         let mut inner = self.inner.lock().unwrap();
//         inner.actual_operations = operations
//     }
//
//     /// Returns dry run operations
//     pub fn get_dry_operations(&self) -> Vec<Operation> {
//         let inner = self.inner.lock().unwrap();
//         inner.dry_run_operations.to_vec()
//     }
//
//     /// Returns all (actual & dry run) operations
//     pub fn get_operations(&self) -> Vec<Operation> {
//         let inner = self.inner.lock().unwrap();
//         inner
//             .actual_operations
//             .iter()
//             // TODO remove cloning
//             .cloned()
//             // TODO remove cloning
//             .chain(inner.dry_run_operations.iter().cloned())
//             .collect()
//     }
//
//     /// Enable dry run
//     pub fn enable_dry_run(&self) {
//         let mut inner = self.inner.lock().unwrap();
//         inner.is_dry_run = true;
//     }
//
//     /// Enable dry run
//     pub fn with_dry_run(self) -> Self {
//         let mut inner = self.inner.lock().unwrap();
//         inner.is_dry_run = true;
//         drop(inner);
//         self
//     }
//
//     /// Disable dry run
//     pub fn disable_dry_run(&self) {
//         let mut inner = self.inner.lock().unwrap();
//         inner.is_dry_run = false;
//     }
//
//     pub fn clear_dry_run_operations(&self) {
//         let mut inner = self.inner.lock().unwrap();
//         inner.dry_run_operations.clear()
//     }
//
//     pub fn is_dry_run(&self) -> bool {
//         let inner = self.inner.lock().unwrap();
//         inner.is_dry_run
//     }
// }
