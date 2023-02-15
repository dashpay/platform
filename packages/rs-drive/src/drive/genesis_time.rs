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

//! Genesis Time.
//!
//! This module defines functions relevant to the chain's genesis time.
//!

use crate::drive::Drive;
use crate::error::Error;
use crate::fee::epoch::GENESIS_EPOCH_INDEX;
use crate::fee_pools::epochs::Epoch;
use grovedb::TransactionArg;

impl Drive {
    /// Returns the genesis time. Checks cache first, then storage.
    pub fn get_genesis_time(&self, transaction: TransactionArg) -> Result<Option<u64>, Error> {
        // let's first check the cache
        let cache = self.cache.borrow();

        if cache.genesis_time_ms.is_some() {
            return Ok(cache.genesis_time_ms);
        };

        drop(cache);

        let epoch = Epoch::new(GENESIS_EPOCH_INDEX);

        match self.get_epoch_start_time(&epoch, transaction) {
            Ok(genesis_time_ms) => {
                let mut cache = self.cache.borrow_mut();

                cache.genesis_time_ms = Some(genesis_time_ms);

                Ok(Some(genesis_time_ms))
            }
            Err(Error::GroveDB(
                grovedb::Error::PathParentLayerNotFound(_) | grovedb::Error::PathKeyNotFound(_),
            )) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Sets genesis time
    pub fn set_genesis_time(&self, genesis_time_ms: u64) {
        self.cache.borrow_mut().genesis_time_ms = Some(genesis_time_ms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::common::helpers::setup::{setup_drive, setup_drive_with_initial_state_structure};

    mod get_genesis_time {
        use super::*;

        use crate::drive::batch::GroveDbOpBatch;

        #[test]
        fn should_return_none_if_cache_is_empty_and_start_time_is_not_persisted() {
            let drive = setup_drive(None);

            let result = drive
                .get_genesis_time(None)
                .expect("should get empty genesis time");

            assert!(result.is_none());
        }

        #[test]
        fn should_return_some_if_cache_is_set() {
            let drive = setup_drive(None);

            let mut cache = drive.cache.borrow_mut();

            let genesis_time_ms = 100;

            cache.genesis_time_ms = Some(genesis_time_ms);

            drop(cache);

            let result = drive
                .get_genesis_time(None)
                .expect("should get empty genesis time");

            assert!(matches!(result, Some(g) if g == genesis_time_ms));
        }

        #[test]
        fn should_return_some_if_genesis_time_is_persisted() {
            let drive = setup_drive_with_initial_state_structure();

            let genesis_time_ms = 100;

            let epoch = Epoch::new(GENESIS_EPOCH_INDEX);

            let mut batch = GroveDbOpBatch::new();
            let mut drive_operations = Vec::new();

            epoch
                .add_init_empty_operations(&mut batch)
                .expect("should add init operations");

            epoch.add_init_current_operations(0.0, 1, genesis_time_ms, &mut batch);

            drive
                .apply_batch_grovedb_operations(None, None, batch, &mut drive_operations)
                .expect("should apply batch");

            let result = drive
                .get_genesis_time(None)
                .expect("should get empty genesis time");

            assert!(matches!(result, Some(g) if g == genesis_time_ms));
        }
    }

    mod set_genesis_time {
        use super::*;

        #[test]
        fn should_set_genesis_time_to_cache() {
            let drive = setup_drive(None);

            let genesis_time_ms: u64 = 100;

            drive.set_genesis_time(genesis_time_ms);

            let cache = drive.cache.borrow();

            assert!(matches!(cache.genesis_time_ms, Some(g) if g == genesis_time_ms));
        }
    }
}
