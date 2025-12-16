use bincode::{Decode, Encode};
use derive_more::From;

#[derive(Debug, Clone, Encode, Decode, From, PartialEq)]
/// Token status
pub struct TokenStatusV0 {
    pub paused: bool,
}

pub trait TokenStatusV0Accessors {
    /// Gets the paused state of the token.
    fn paused(&self) -> bool;

    /// Sets the paused state of the token.
    fn set_paused(&mut self, paused: bool);
}

impl TokenStatusV0Accessors for TokenStatusV0 {
    fn paused(&self) -> bool {
        self.paused
    }

    fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }
}
