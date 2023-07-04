pub mod default_costs;
pub mod epoch;
#[cfg(feature = "fee-distribution")]
pub mod fee_result;

pub use crate::balances::credits::{Credits, SignedCredits};

/// Default original fee multiplier
pub const DEFAULT_ORIGINAL_FEE_MULTIPLIER: f64 = 2.0;
