//! Token state transition modules for the Dash Platform SDK.
//!
//! This module provides various token state transitions that allow for managing
//! fungible tokens on the Dash Platform. Each submodule implements a specific
//! type of token operation.

/// Token burning operations for permanently removing tokens from circulation.
pub mod burn;

/// Token claiming operations for acquiring ownership of unclaimed tokens.
pub mod claim;

/// Token configuration updates for modifying token contract settings.
pub mod config_update;

/// Operations for destroying frozen token funds.
pub mod destroy_frozen_funds;

/// Direct token purchasing functionality for buying tokens at set prices.
pub mod direct_purchase;

/// Emergency action operations for critical token management scenarios.
pub mod emergency_action;

/// Token freezing operations to temporarily lock token transfers.
pub mod freeze;

/// Token minting operations for creating new tokens.
pub mod mint;

/// Price setting operations for tokens available for direct purchase.
pub mod set_price_for_direct_purchase;

/// Token transfer operations between identities.
pub mod transfer;

/// Token unfreezing operations to restore transfer capabilities.
pub mod unfreeze;

pub use burn::BurnResult;
pub use claim::ClaimResult;
pub use config_update::ConfigUpdateResult;
pub use destroy_frozen_funds::DestroyFrozenFundsResult;
pub use direct_purchase::DirectPurchaseResult;
pub use emergency_action::EmergencyActionResult;
pub use freeze::FreezeResult;
pub use mint::MintResult;
pub use set_price_for_direct_purchase::SetPriceResult;
pub use transfer::TransferResult;
pub use unfreeze::UnfreezeResult;
