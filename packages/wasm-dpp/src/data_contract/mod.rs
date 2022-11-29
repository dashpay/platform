#![allow(clippy::module_inception)]
pub use data_contract::*;

mod data_contract;
pub mod errors;

mod factory;
pub use factory::*;
