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

#[cfg(any(feature = "full", feature = "verify"))]
use crate::fee::credits::Credits;
#[cfg(feature = "full")]
use crate::fee::credits::SignedCredits;
#[cfg(any(feature = "full", feature = "verify"))]
use nohash_hasher::IntMap;

#[cfg(feature = "full")]
pub mod distribution;

#[cfg(any(feature = "full", feature = "verify"))]
/// Epoch index type
pub type EpochIndex = u16;

#[cfg(feature = "full")]
/// Genesis epoch index
pub const GENESIS_EPOCH_INDEX: EpochIndex = 0;

#[cfg(feature = "full")]
/// Epochs per year
pub const EPOCHS_PER_YEAR: u16 = 20;

#[cfg(feature = "full")]
/// Years of fees charged for perpetual storage
pub const PERPETUAL_STORAGE_YEARS: u16 = 50;

#[cfg(feature = "full")]
/// Perpetual storage epochs
pub const PERPETUAL_STORAGE_EPOCHS: u16 = PERPETUAL_STORAGE_YEARS * EPOCHS_PER_YEAR;

#[cfg(any(feature = "full", feature = "verify"))]
/// Credits per epoch map
pub type CreditsPerEpoch = IntMap<EpochIndex, Credits>;
#[cfg(feature = "full")]
/// Signed credits per epoch map
pub type SignedCreditsPerEpoch = IntMap<EpochIndex, SignedCredits>;
