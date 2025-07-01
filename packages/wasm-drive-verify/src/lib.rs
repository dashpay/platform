#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

//! # wasm-drive-verify
//!
//! WebAssembly bindings for Drive verification functions.
//!
//! This crate provides JavaScript/TypeScript bindings for verifying proofs from the Dash Platform.
//! It's organized into modules for different verification categories, allowing for optimal bundle
//! sizes through tree-shaking when using ES modules.
//!
//! ## Modules
//!
//! - **identity** - Verify identities, balances, keys, and related data
//! - **document** - Verify documents and document queries
//! - **contract** - Verify data contracts and contract history
//! - **tokens** - Verify token balances, info, and statuses
//! - **governance** - Verify voting polls, groups, and system state
//! - **transitions** - Verify state transition execution
//!
//! ## Usage
//!
//! ### ES Modules (Recommended)
//!
//! Import only what you need for optimal bundle size:
//!
//! ```javascript
//! import { verifyFullIdentityByIdentityId } from 'wasm-drive-verify/identity';
//!
//! const result = await verifyFullIdentityByIdentityId(proof, identityId, platformVersion);
//! ```
//!
//! ### Identifier Encoding
//!
//! All identifiers (identity IDs, contract IDs, document IDs, etc.) are returned as base58-encoded
//! strings for consistency and compatibility with the Dash ecosystem.

// Core utilities module (always available)
mod utils;
pub use utils::serialization::*;

// Native Rust API (for use by other Rust/WASM projects)
pub mod native;

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

#[cfg(any(feature = "governance", feature = "full"))]
mod group;

#[cfg(any(feature = "governance", feature = "full"))]
mod voting;

#[cfg(any(feature = "governance", feature = "full"))]
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
    pub use crate::group::*;
    pub use crate::system::*;
    pub use crate::voting::*;
}

#[cfg(any(feature = "transitions", feature = "full"))]
pub mod transition_verification {
    pub use crate::state_transition::*;
}

use wasm_bindgen::prelude::*;

#[cfg(all(target_arch = "wasm32", not(test)))]
#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}
