use wasm_bindgen::prelude::*;

// Core utilities module (always available)
mod utils;
pub use utils::serialization::*;

// Conditional compilation for modules
#[cfg(any(feature = "identity", feature = "full"))]
mod identity;

#[cfg(any(feature = "document", feature = "full"))]
mod document;

#[cfg(any(feature = "document", feature = "full"))]
mod single_document;

#[cfg(any(feature = "contract", feature = "full"))]
mod contract;

#[cfg(any(feature = "tokens", feature = "full"))]
mod tokens;

#[cfg(any(feature = "group", feature = "governance", feature = "full"))]
mod group;

#[cfg(any(feature = "voting", feature = "governance", feature = "full"))]
mod voting;

#[cfg(any(feature = "system", feature = "governance", feature = "full"))]
mod system;

#[cfg(any(feature = "transitions", feature = "full"))]
mod state_transition;

// Namespaced exports for tree-shaking
#[cfg(any(feature = "identity", feature = "full"))]
pub mod identity_verification {
    pub use crate::identity::*;
}

#[cfg(any(feature = "document", feature = "full"))]
pub mod document_verification {
    pub use crate::document::*;
    pub use crate::single_document::*;
}

#[cfg(any(feature = "contract", feature = "full"))]
pub mod contract_verification {
    pub use crate::contract::*;
}

#[cfg(any(feature = "tokens", feature = "full"))]
pub mod token_verification {
    pub use crate::tokens::*;
}

#[cfg(any(feature = "governance", feature = "full"))]
pub mod governance_verification {
    #[cfg(any(feature = "group", feature = "governance", feature = "full"))]
    pub use crate::group::*;

    #[cfg(any(feature = "voting", feature = "governance", feature = "full"))]
    pub use crate::voting::*;

    #[cfg(any(feature = "system", feature = "governance", feature = "full"))]
    pub use crate::system::*;
}

#[cfg(any(feature = "transitions", feature = "full"))]
pub mod transition_verification {
    pub use crate::state_transition::*;
}

#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}
