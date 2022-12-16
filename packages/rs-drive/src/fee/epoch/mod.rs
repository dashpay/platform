use crate::fee::credits::{Credits, SignedCredits};
use nohash_hasher::IntMap;

pub mod distribution;

/// Genesis epoch index
pub const GENESIS_EPOCH_INDEX: u16 = 0;

/// Epochs per year
pub const EPOCHS_PER_YEAR: u16 = 20;

/// Years of fees charged for perpetual storage
pub const PERPETUAL_STORAGE_YEARS: u16 = 50;

/// Perpetual storage epochs
pub const PERPETUAL_STORAGE_EPOCHS: u16 = PERPETUAL_STORAGE_YEARS * EPOCHS_PER_YEAR;

pub type EpochIndex = u16;

pub type CreditsPerEpoch = IntMap<EpochIndex, Credits>;
pub type SignedCreditsPerEpoch = IntMap<EpochIndex, SignedCredits>;
