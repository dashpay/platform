// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Epochs
//!
//! Fee distribution is based on epochs. One epoch is about 18 days
//!

use nohash_hasher::IntMap;

#[cfg(feature = "fee-distribution")]
pub mod distribution;

/// Epoch index type
use crate::block::epoch::EpochIndex;
use crate::fee::{Credits, SignedCredits};

/// Genesis epoch index
pub const GENESIS_EPOCH_INDEX: EpochIndex = 0;

/// Eras of fees charged for perpetual storage
/// An Era is set to 1 year on Mainnet
pub const PERPETUAL_STORAGE_ERAS: u16 = 50;

pub const DEFAULT_EPOCHS_PER_ERA: u16 = 40;

pub const fn perpetual_storage_epochs(epochs_per_era: u16) -> u16 {
    epochs_per_era * PERPETUAL_STORAGE_ERAS
}

/// Credits per epoch map
pub type CreditsPerEpoch = IntMap<EpochIndex, Credits>;

/// Bytes removed per epoch map
pub type BytesPerEpoch = IntMap<EpochIndex, u32>;

/// Signed credits per epoch map
pub type SignedCreditsPerEpoch = IntMap<EpochIndex, SignedCredits>;
