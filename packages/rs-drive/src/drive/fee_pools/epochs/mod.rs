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

//! Epochs Mod File.
//!

use crate::drive::fee_pools::pools_path;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee_pools::epochs::Epoch;
use grovedb::TransactionArg;

/// Constants module
pub mod constants;
/// Credit distribution pools module
pub mod credit_distribution_pools;
/// Proposers module
pub mod proposers;
/// Start block module
pub mod start_block;
/// Start time module
pub mod start_time;

impl Drive {
    /// Checks if an Epoch tree exists. Returns a bool.
    pub fn is_epoch_tree_exists(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
    ) -> Result<bool, Error> {
        self.grove
            .has_raw(pools_path(), &epoch_tree.key, transaction)
            .unwrap()
            .map_err(Error::GroveDB)
    }
}

#[cfg(test)]
mod tests {
    use crate::common::helpers::setup::{setup_drive, setup_drive_with_initial_state_structure};

    use crate::drive::batch::GroveDbOpBatch;
    use crate::error;
    use crate::fee_pools::epochs::epoch_key_constants;
    use crate::fee_pools::epochs::Epoch;

    mod is_epoch_tree_exists {
        use crate::common::helpers::setup::setup_drive_with_initial_state_structure;
        use crate::drive::fee_pools::epochs::constants::{
            GENESIS_EPOCH_INDEX, PERPETUAL_STORAGE_EPOCHS,
        };
        use crate::fee_pools::epochs::Epoch;

        #[test]
        fn test_return_true_if_tree_exists() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch_tree = Epoch::new(GENESIS_EPOCH_INDEX);

            let is_exist = drive
                .is_epoch_tree_exists(&epoch_tree, Some(&transaction))
                .expect("should check epoch tree existence");

            assert!(is_exist);
        }

        #[test]
        fn test_return_false_if_tree_doesnt_exist() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch_tree = Epoch::new(PERPETUAL_STORAGE_EPOCHS + 1);

            let is_exist = drive
                .is_epoch_tree_exists(&epoch_tree, Some(&transaction))
                .expect("should check epoch tree existence");

            assert!(!is_exist);
        }
    }
}
