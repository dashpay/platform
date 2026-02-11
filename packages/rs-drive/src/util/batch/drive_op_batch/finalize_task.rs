use crate::drive::shielded::paths::{
    shielded_anchors_credit_pool_path, shielded_credit_pool_path, SHIELDED_COMMITMENTS_KEY,
    SHIELDED_PARAMS_KEY,
};
use crate::drive::Drive;
use crate::error::Error;
use dpp::prelude::Identifier;
use dpp::shielded::ShieldedPoolParams;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};

#[derive(Clone, Debug)]
pub enum DriveOperationFinalizeTask {
    RemoveDataContractFromCache {
        contract_id: Identifier,
    },
    /// Record the current commitment tree root hash as a new anchor
    RecordShieldedAnchor,
}

/// Enable callbacks for drive operations that will be called after successful execution
pub trait DriveOperationFinalizationTasks {
    /// Returns a finalize tasks that will be called after successful execution of the drive operation
    fn finalization_tasks(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<DriveOperationFinalizeTask>>, Error>; // Since we have it only for one operation implemeneted we don't want the extra calls and empty vectors
}

impl DriveOperationFinalizeTask {
    pub fn execute(
        self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match self {
            DriveOperationFinalizeTask::RemoveDataContractFromCache { contract_id } => {
                drive.cache.data_contracts.remove(contract_id.to_buffer());
                Ok(())
            }
            DriveOperationFinalizeTask::RecordShieldedAnchor => {
                let grove_version = &platform_version.drive.grove_version;
                let pool_path = shielded_credit_pool_path();

                // Read current checkpoint_id_counter from ShieldedPoolParams
                let params_element = drive
                    .grove
                    .get(
                        &pool_path,
                        &[SHIELDED_PARAMS_KEY],
                        transaction,
                        grove_version,
                    )
                    .unwrap()
                    .map_err(Error::from)?;

                let params_bytes = match &params_element {
                    Element::Item(data, _) => data.as_slice(),
                    _ => {
                        return Err(Error::Drive(
                            crate::error::drive::DriveError::CorruptedElementType(
                                "shielded pool params should be an Item",
                            ),
                        ))
                    }
                };

                let (params, _): (ShieldedPoolParams, _) =
                    bincode::decode_from_slice(params_bytes, bincode::config::standard()).map_err(
                        |e| {
                            Error::Drive(crate::error::drive::DriveError::CorruptedSerialization(
                                format!("could not decode shielded pool params: {e}"),
                            ))
                        },
                    )?;

                let new_checkpoint_id = params.checkpoint_id_counter + 1;

                // Create a checkpoint in the commitment tree
                drive
                    .grove
                    .commitment_tree_checkpoint(
                        &pool_path,
                        &[SHIELDED_COMMITMENTS_KEY],
                        new_checkpoint_id,
                        transaction,
                        grove_version,
                    )
                    .unwrap()
                    .map_err(Error::from)?;

                // Get the current root hash (anchor) of the commitment tree
                let root_hash = drive
                    .grove
                    .commitment_tree_root_hash(
                        &pool_path,
                        &[SHIELDED_COMMITMENTS_KEY],
                        transaction,
                        grove_version,
                    )
                    .unwrap()
                    .map_err(Error::from)?;

                // Insert the root hash as an empty Item into the anchors tree
                drive
                    .grove
                    .insert(
                        &shielded_anchors_credit_pool_path(),
                        &root_hash,
                        Element::Item(vec![], None),
                        None,
                        transaction,
                        grove_version,
                    )
                    .unwrap()
                    .map_err(Error::from)?;

                // Update ShieldedPoolParams with new checkpoint_id_counter
                let new_params = ShieldedPoolParams {
                    checkpoint_id_counter: new_checkpoint_id,
                };
                let encoded_params =
                    bincode::encode_to_vec(&new_params, bincode::config::standard()).map_err(
                        |e| {
                            Error::Drive(crate::error::drive::DriveError::CorruptedSerialization(
                                format!("could not encode shielded pool params: {e}"),
                            ))
                        },
                    )?;

                drive
                    .grove
                    .insert(
                        &pool_path,
                        &[SHIELDED_PARAMS_KEY],
                        Element::new_item(encoded_params),
                        None,
                        transaction,
                        grove_version,
                    )
                    .unwrap()
                    .map_err(Error::from)?;

                Ok(())
            }
        }
    }
}
