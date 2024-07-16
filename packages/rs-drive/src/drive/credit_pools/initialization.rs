use crate::drive::credit_pools::epochs::operations_factory::EpochOperations;
use crate::drive::credit_pools::operations;
use crate::drive::Drive;
use crate::error::Error;
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::util::batch::GroveDbOpBatch;
use dpp::block::epoch::Epoch;
use dpp::fee::epoch::{perpetual_storage_epochs, GENESIS_EPOCH_INDEX};
use dpp::util::deserializer::ProtocolVersion;

impl Drive {
    #[cfg(feature = "server")]
    /// Adds the operations to groveDB op batch to create the fee pool trees
    pub fn add_create_fee_pool_trees_operations(
        batch: &mut GroveDbOpBatch,
        epochs_per_era: u16,
        protocol_version: ProtocolVersion,
    ) -> Result<(), Error> {
        // Init storage credit pool
        batch.push(operations::update_storage_fee_distribution_pool_operation(
            0,
        )?);

        // Init next epoch to pay
        batch.push(operations::update_unpaid_epoch_index_operation(
            GENESIS_EPOCH_INDEX,
        ));

        operations::add_create_pending_epoch_refunds_tree_operations(batch);

        // We need to insert 50 era worth of epochs,
        // with 40 epochs per era that's 2000 epochs
        // however this is configurable
        for i in GENESIS_EPOCH_INDEX..perpetual_storage_epochs(epochs_per_era) {
            let epoch = Epoch::new(i)?;
            epoch.add_init_empty_operations(batch)?;
        }

        let genesis_epoch = Epoch::new(GENESIS_EPOCH_INDEX)?;

        // Initial protocol version for genesis epoch
        batch.push(genesis_epoch.update_protocol_version_operation(protocol_version));

        Ok(())
    }
}
