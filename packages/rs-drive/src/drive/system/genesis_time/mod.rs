//! Genesis Time.
//!
//! This module defines functions relevant to the chain's genesis time.
//!

use crate::drive::Drive;
use crate::error::Error;
use dpp::block::epoch::Epoch;
use dpp::fee::epoch::GENESIS_EPOCH_INDEX;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Returns the genesis time. Checks cache first, then storage.
    pub fn get_genesis_time(
        &self,
        transaction: TransactionArg,
    ) -> Result<Option<TimestampMillis>, Error> {
        // let's first check the cache
        let genesis_time_ms = self.cache.genesis_time_ms.read();

        let platform_version = PlatformVersion::latest();

        if genesis_time_ms.is_some() {
            return Ok(*genesis_time_ms);
        };

        drop(genesis_time_ms);

        let epoch = Epoch::new(GENESIS_EPOCH_INDEX)
            .expect("expected to be able to create epoch with genesis time");

        self.get_epoch_start_time(&epoch, transaction, platform_version)
    }

    /// Sets genesis time
    pub fn set_genesis_time(&self, genesis_time_ms: u64) {
        let mut genesis_time_ms_cache = self.cache.genesis_time_ms.write();
        *genesis_time_ms_cache = Some(genesis_time_ms);
    }
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use super::*;

    use crate::util::test_helpers::setup::{setup_drive, setup_drive_with_initial_state_structure};

    mod get_genesis_time {
        use super::*;
        use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;

        use crate::drive::credit_pools::epochs::operations_factory::EpochOperations;
        use crate::util::batch::GroveDbOpBatch;

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

            let mut genesis_time_ms_cache = drive.cache.genesis_time_ms.write();

            let genesis_time_ms = 100;

            *genesis_time_ms_cache = Some(genesis_time_ms);

            drop(genesis_time_ms_cache);

            let result = drive
                .get_genesis_time(None)
                .expect("should get empty genesis time");

            assert!(matches!(result, Some(g) if g == genesis_time_ms));
        }

        #[test]
        fn should_return_some_if_genesis_time_is_persisted() {
            let drive = setup_drive_with_initial_state_structure(None);

            let genesis_time_ms = 100;

            let epoch = Epoch::new(GENESIS_EPOCH_INDEX).unwrap();

            let platform_version = PlatformVersion::latest();

            let mut batch = GroveDbOpBatch::new();
            let mut drive_operations = Vec::new();

            epoch
                .add_init_empty_operations(&mut batch)
                .expect("should add init operations");

            epoch.add_init_current_operations(
                0,
                1,
                1,
                genesis_time_ms,
                platform_version.protocol_version,
                &mut batch,
            );

            drive
                .apply_batch_grovedb_operations(
                    None,
                    None,
                    batch,
                    &mut drive_operations,
                    &platform_version.drive,
                )
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

            let genesis_time_ms_cache = drive.cache.genesis_time_ms.read();

            assert!(matches!(*genesis_time_ms_cache, Some(g) if g == genesis_time_ms));
        }
    }
}
