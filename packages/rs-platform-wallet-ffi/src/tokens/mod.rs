//! Token state-transition FFI surface.
//!
//! Each entry point here is a thin C-ABI shell over a
//! `TokenWallet::*_external_signer` method in `platform-wallet`. The
//! orchestration (contract fetch, identity / signing-key resolution,
//! state-transition broadcast + proof verification) lives Rust-side;
//! the FFI just marshals arguments and translates errors.
//!
//! Why these go through `platform-wallet-ffi` and not `rs-sdk-ffi`:
//! the platform-wallet runtime (`runtime::block_on_worker`) configures
//! an 8 MB worker stack — proof verification on the broadcast response
//! recurses through GroveDB deeply enough to crash the rs-sdk-ffi
//! tokio default (~512 KB on iOS). See `data_contract.rs` for the
//! same precedent applied to contract creation.

mod balances_json;
pub mod burn;
pub mod claim;
pub mod destroy_frozen_funds;
pub mod freeze;
pub mod group_info;
pub mod group_queries;
pub mod mint;
pub mod pause;
pub mod purchase;
pub mod resume;
pub mod set_price;
pub mod transfer;
pub mod unfreeze;
pub mod update_config;

pub use burn::*;
pub use claim::*;
pub use destroy_frozen_funds::*;
pub use freeze::*;
pub use group_queries::*;
pub use mint::*;
pub use pause::*;
pub use purchase::*;
pub use resume::*;
pub use set_price::*;
pub use transfer::*;
pub use unfreeze::*;
pub use update_config::*;
