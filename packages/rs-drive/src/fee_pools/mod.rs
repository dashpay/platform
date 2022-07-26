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

pub mod epochs;
pub mod epochs_root_tree_key_constants;

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

pub fn update_storage_fee_distribution_pool_operation(storage_fee: u64) -> GroveDbOp {
    GroveDbOp {
        path: pools_vec_path(),
        key: KEY_STORAGE_FEE_POOL.to_vec(),
        op: Insert {
            element: Element::new_item(storage_fee.to_be_bytes().to_vec()),
        },
    }
}

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
