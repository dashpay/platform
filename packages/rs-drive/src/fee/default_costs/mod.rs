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

//! Fee costs
//!
//! Fee costs for Drive (GroveDB) operations
//!

use crate::fee::epoch::EpochIndex;
use crate::fee_pools::epochs::Epoch;
use lazy_static::lazy_static;
use std::collections::HashMap;

/// A Known Cost Item is an item that changes costs depending on the Epoch
#[derive(Eq, PartialEq, Copy, Clone, Hash)]
pub enum KnownCostItem {
    /// The storage cost used when writing bytes
    StorageDiskUsageCreditPerByte,
    /// The processing cost used when writing bytes
    StorageProcessingCreditPerByte,
    /// The processing cost used when loading bytes from storage
    StorageLoadCreditPerByte,
    /// The processing cost used when loading bytes not from storage
    NonStorageLoadCreditPerByte,
    /// The cost used when performing a disk seek
    StorageSeekCost,
    // The following are set costs of routine operations
    /// The cost for fetching an identity balance
    FetchIdentityBalanceProcessingCost,
    /// The cost for fetching an identity key
    FetchSingleIdentityKeyProcessingCost,
}

const EPOCH_COST_UPDATE_VERSIONS: [u16; 1] = [0];

lazy_static! {
    static ref EPOCH_COSTS: HashMap<EpochIndex, HashMap<KnownCostItem, u64>> = [(
        0,
        [
            (KnownCostItem::StorageDiskUsageCreditPerByte, 27000),
            (KnownCostItem::StorageProcessingCreditPerByte, 400),
            (KnownCostItem::StorageLoadCreditPerByte, 400),
            (KnownCostItem::NonStorageLoadCreditPerByte, 30),
            (KnownCostItem::StorageSeekCost, 4000),
            (KnownCostItem::FetchIdentityBalanceProcessingCost, 10000),
            (KnownCostItem::FetchSingleIdentityKeyProcessingCost, 10000),
        ]
        .into_iter()
        .collect()
    ),]
    .into_iter()
    .collect();
}

impl Epoch {
    //todo: should just have a static lookup table
    /// Get the closest epoch in the past that has a cost table
    /// This is where the base costs last changed
    fn get_closest_epoch_index_cost_update_version(&self) -> EpochIndex {
        match EPOCH_COST_UPDATE_VERSIONS.binary_search(&self.index) {
            Ok(_) => self.index,
            Err(pos) => EPOCH_COST_UPDATE_VERSIONS[pos - 1],
        }
    }

    /// Get the cost for the known cost item
    pub fn cost_for_known_cost_item(&self, cost_item: KnownCostItem) -> u64 {
        let epoch = self.get_closest_epoch_index_cost_update_version();
        let specific_epoch_costs = EPOCH_COSTS.get(&epoch).unwrap();
        *specific_epoch_costs.get(&cost_item).unwrap()
    }
}
