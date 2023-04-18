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

//! Epoch pools
//!

/// Epoch key constants module
pub mod epoch_key_constants;
pub mod operations_factory;
pub mod paths;

use crate::fee::epoch::EpochIndex;
use serde::{Deserialize, Serialize};

// TODO: I would call it EpochTree because it represent pool,
//  not just Epoch which is more abstract thing that we will probably need in future too

/// Epoch struct
#[derive(Serialize, Deserialize, Default, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Epoch {
    /// Epoch index
    pub index: EpochIndex,
    /// Epoch key
    pub(crate) key: [u8; 2],
}

impl Epoch {
    /// Create new epoch
    pub fn new(index: EpochIndex) -> Self {
        let key = paths::encode_epoch_index_key(index).expect("epoch index is too high");

        Self { index, key }
    }
}
