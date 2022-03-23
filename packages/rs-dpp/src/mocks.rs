use thiserror::Error;

#[derive(Debug, Clone)]
pub struct DocumentTransition {
    pub action: String,
}

#[derive(Error, Debug, Clone)]
pub enum ConsensusError {}
