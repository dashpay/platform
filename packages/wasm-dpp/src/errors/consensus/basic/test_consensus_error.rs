// use dpp::consensus::ConsensusError;
// // use dpp::errors::consensus::test_consensus_error::TestConsensusError;
// use wasm_bindgen::prelude::*;
//
// #[wasm_bindgen(js_name=TestConsensusError)]
// pub struct TestConsensusErrorWasm {
//     inner: TestConsensusError,
// }
//
// impl From<&TestConsensusError> for TestConsensusErrorWasm {
//     fn from(e: &TestConsensusError) -> Self {
//         Self { inner: e.clone() }
//     }
// }
//
// #[wasm_bindgen(js_class=TestConsensusError)]
// impl TestConsensusErrorWasm {
//     #[wasm_bindgen(js_name=getMessage)]
//     pub fn get_message(&self) -> String {
//         self.inner.message.clone()
//     }
// }
