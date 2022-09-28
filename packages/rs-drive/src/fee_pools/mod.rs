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

//! Fee Pool Epochs Mod File
//!

use crate::drive::batch::GroveDbOpBatch;
use crate::drive::fee_pools::epochs::constants::{GENESIS_EPOCH_INDEX, PERPETUAL_STORAGE_EPOCHS};
use crate::drive::fee_pools::pools_vec_path;
use crate::fee_pools::epochs::Epoch;
use crate::fee_pools::epochs_root_tree_key_constants::{
    KEY_STORAGE_FEE_POOL, KEY_UNPAID_EPOCH_INDEX,
};
use grovedb::batch::GroveDbOp;
use grovedb::batch::Op::Insert;
use grovedb::Element;

/// Epochs module
pub mod epochs;
/// Epochs root tree key constants module
pub mod epochs_root_tree_key_constants;

/// Adds the operations to groveDB op batch to create the fee pool trees
pub fn add_create_fee_pool_trees_operations(batch: &mut GroveDbOpBatch) {
    // Init storage credit pool
    batch.push(update_storage_fee_distribution_pool_operation(0));

    // Init next epoch to pay
    batch.push(update_unpaid_epoch_index_operation(GENESIS_EPOCH_INDEX));

    // We need to insert 50 years worth of epochs,
    // with 20 epochs per year that's 1000 epochs
    for i in GENESIS_EPOCH_INDEX..PERPETUAL_STORAGE_EPOCHS {
        let epoch = Epoch::new(i);
        epoch.add_init_empty_operations(batch);
    }
}

/// Updates the storage fee distribution pool with a new storage fee
pub fn update_storage_fee_distribution_pool_operation(storage_fee: u64) -> GroveDbOp {
    GroveDbOp {
        path: pools_vec_path(),
        key: KEY_STORAGE_FEE_POOL.to_vec(),
        op: Insert {
            element: Element::new_item(storage_fee.to_be_bytes().to_vec()),
        },
    }
}

/// Updates the unpaid epoch index
pub fn update_unpaid_epoch_index_operation(epoch_index: u16) -> GroveDbOp {
    GroveDbOp {
        path: pools_vec_path(),
        key: KEY_UNPAID_EPOCH_INDEX.to_vec(),
        op: Insert {
            element: Element::new_item(epoch_index.to_be_bytes().to_vec()),
        },
    }
}

// TODD: Find tests
