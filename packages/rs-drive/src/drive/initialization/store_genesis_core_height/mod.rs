//! Drive Initialization

use crate::drive::Drive;
use crate::error::Error;

use crate::drive::system::misc_path_vec;
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::util::batch::GroveDbOpBatch;
use dpp::prelude::CoreBlockHeight;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};
use integer_encoding::VarInt;

/// Genesis Core Height Key
#[cfg(any(feature = "server", feature = "verify"))]
pub const GENESIS_CORE_HEIGHT_KEY: &[u8; 1] = b"C";

impl Drive {
    /// Stores the genesis core height in groveDB
    pub fn store_genesis_core_height(
        &self,
        genesis_core_height: CoreBlockHeight,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let drive_version = &platform_version.drive;
        let mut batch = GroveDbOpBatch::new();

        // In Misc
        batch.add_insert(
            misc_path_vec(),
            GENESIS_CORE_HEIGHT_KEY.to_vec(),
            Element::Item(genesis_core_height.encode_var_vec(), None),
        );

        self.grove_apply_batch(batch, false, transaction, drive_version)?;

        Ok(())
    }
}
