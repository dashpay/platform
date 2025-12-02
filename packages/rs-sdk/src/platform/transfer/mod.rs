//! Transfer orchestrator module.
//!
//! Transfer helpers live in dedicated files to keep responsibilities focused.
//! This module only wires them together so callers interact through a single entry point.

pub mod credit_transfer;
mod identity;
mod types;

pub use credit_transfer::{CreditTransfer, CreditTransferBuilder};
pub use types::{TransferInput, TransferOutput};
