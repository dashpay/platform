use crate::tokens::status::v0::TokenStatusV0Accessors;
use crate::tokens::status::TokenStatus;

impl TokenStatusV0Accessors for TokenStatus {
    fn paused(&self) -> bool {
        match self {
            TokenStatus::V0(status) => status.paused,
        }
    }

    fn set_paused(&mut self, frozen: bool) {
        match self {
            TokenStatus::V0(status) => status.set_paused(frozen),
        }
    }
}
