use dpp::errors::AbstractConsensusErrorMock;
use thiserror::Error;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Error, Debug, Clone, Default)]
#[error("Abstract Consensus Error")]
pub struct AbstractConsensusError {}

#[wasm_bindgen]
impl AbstractConsensusError {
    pub fn new() -> AbstractConsensusError {
        AbstractConsensusError {}
    }
}

impl From<AbstractConsensusErrorMock> for AbstractConsensusError {
    fn from(_: AbstractConsensusErrorMock) -> Self {
        unimplemented!()
    }
}
