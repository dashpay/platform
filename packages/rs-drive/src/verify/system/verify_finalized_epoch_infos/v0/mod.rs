use crate::drive::credit_pools::epochs::epoch_key_constants::KEY_FINISHED_EPOCH_INFO;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::proof::ProofError;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::block::epoch::{EpochIndex, EPOCH_KEY_OFFSET};
use dpp::block::finalized_epoch_info::FinalizedEpochInfo;
use dpp::serialization::PlatformDeserializable;
use grovedb::{Element, GroveDb};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    /// Verifies finalized epoch information for a given range of epochs.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof to be verified.
    /// - `start_epoch_index`: The starting epoch index for the query.
    /// - `start_epoch_index_included`: If `true`, the epoch at `start_epoch_index` is included.
    /// - `end_epoch_index`: The ending epoch index for the query.
    /// - `end_epoch_index_included`: If `true`, the epoch at `end_epoch_index` is included.
    /// - `platform_version`: The platform version to use for method dispatch.
    ///
    /// # Returns
    ///
    /// Returns a `Result` with a tuple of `RootHash` and `Vec<(EpochIndex, FinalizedEpochInfo)>`.
    /// The vector contains verified finalized epoch information.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof is corrupted.
    /// - The GroveDb query fails.
    /// - An epoch index plus the offset overflows.
    #[inline(always)]
    pub(super) fn verify_finalized_epoch_infos_v0(
        proof: &[u8],
        start_epoch_index: u16,
        start_epoch_index_included: bool,
        end_epoch_index: u16,
        end_epoch_index_included: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<(EpochIndex, FinalizedEpochInfo)>), Error> {
        let Some(path_query) = Drive::finalized_epoch_infos_query(
            start_epoch_index,
            start_epoch_index_included,
            end_epoch_index,
            end_epoch_index_included,
        )?
        else {
            return Err(Error::Query(QuerySyntaxError::NoQueryItems(
                "the end epoch index is the start epoch index and they are not included",
            )));
        };

        // Use verify_subset_query because the proof may contain extra lower layers
        // for sibling subtrees at the root level (e.g., shielded pool subtrees)
        let (root_hash, elements) = GroveDb::verify_subset_query(
            proof,
            &path_query,
            &platform_version.drive.grove_version,
        )?;

        let results = elements.into_iter().fold(
            BTreeMap::<_, BTreeMap<_, _>>::new(),
            |mut acc, result_item| {
                let (path, key, element) = result_item;
                if path.len() == 2 {
                    acc.entry(path).or_default().insert(key, element);
                }
                acc
            },
        );

        // Convert the BTreeMap entries to (EpochIndex, FinalizedEpochInfo)
        let finalized_epoch_infos = results
            .into_iter()
            .filter_map(|(path, inner_map)| {
                // Extract the epoch index from the path's last component
                // and adjust by subtracting the EPOCH_KEY_OFFSET
                let epoch_index_result: Result<EpochIndex, Error> = path
                    .last()
                    .ok_or(Error::Proof(ProofError::CorruptedProof(
                        "finalized epoch info: path can not be empty".to_string(),
                    )))
                    .and_then(|epoch_index_vec| {
                        epoch_index_vec.as_slice().try_into().map_err(|_| {
                            Error::Proof(ProofError::CorruptedProof(
                                "finalized epoch info: item has an invalid length".to_string(),
                            ))
                        })
                    })
                    .and_then(|epoch_index_bytes| {
                        EpochIndex::from_be_bytes(epoch_index_bytes)
                            .checked_sub(EPOCH_KEY_OFFSET)
                            .ok_or(Error::Proof(ProofError::CorruptedProof(
                                "epoch bytes on disk too small, should be over epoch key offset"
                                    .to_string(),
                            )))
                    });

                let epoch_index = match epoch_index_result {
                    Ok(value) => value,
                    Err(e) => return Some(Err(e)),
                };

                // Get the finalized epoch info element
                let finalized_epoch_info_element =
                    inner_map.get(KEY_FINISHED_EPOCH_INFO.as_slice())?;

                let Some(Element::Item(item_bytes, _)) = finalized_epoch_info_element else {
                    return Some(Err(Error::Drive(DriveError::UnexpectedElementType(
                        "finalized epoch info must be an item",
                    ))));
                };

                // Deserialize the FinalizedEpochInfo
                match FinalizedEpochInfo::deserialize_from_bytes(item_bytes) {
                    Ok(epoch_info) => Some(Ok((epoch_index, epoch_info))),
                    Err(e) => Some(Err(e.into())),
                }
            })
            .collect::<Result<Vec<(EpochIndex, FinalizedEpochInfo)>, Error>>()?;

        Ok((root_hash, finalized_epoch_infos))
    }
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::credit_pools::epochs::operations_factory::EpochOperations;
    use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
    use crate::util::batch::GroveDbOpBatch;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::epoch::Epoch;
    use dpp::block::finalized_epoch_info::v0::FinalizedEpochInfoV0;

    #[test]
    fn should_prove_and_verify_finalized_epoch_infos() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let transaction = drive.grove.start_transaction();

        // Initialize epoch 0 with current operations (subtree already exists)
        let epoch0 = Epoch::new(0).unwrap();
        let mut batch = GroveDbOpBatch::new();
        epoch0.add_init_current_operations(
            1000,
            100,
            50,
            1_000_000,
            platform_version.protocol_version,
            &mut batch,
        );
        drive
            .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
            .expect("should apply batch");

        // Add finalized epoch info for epoch 0
        let finalized_info_0 = FinalizedEpochInfo::V0(FinalizedEpochInfoV0 {
            first_block_time: 1_000_000,
            first_block_height: 100,
            total_blocks_in_epoch: 500,
            first_core_block_height: 50,
            next_epoch_start_core_block_height: 100,
            total_processing_fees: 10_000,
            total_distributed_storage_fees: 5_000,
            total_created_storage_fees: 6_000,
            core_block_rewards: 100_000,
            block_proposers: std::collections::BTreeMap::new(),
            fee_multiplier_permille: 1000,
            protocol_version: platform_version.protocol_version,
        });

        let op = drive
            .add_epoch_final_info_operation(&epoch0, finalized_info_0.clone(), platform_version)
            .expect("should create finalized epoch info operation");
        drive
            .grove_apply_operation(op, false, Some(&transaction), &platform_version.drive)
            .expect("should apply finalized epoch info");

        // Initialize epoch 1 and add finalized info
        let epoch1 = Epoch::new(1).unwrap();
        let mut batch = GroveDbOpBatch::new();
        epoch1
            .add_init_empty_operations(&mut batch)
            .expect("should init empty epoch");
        epoch1.add_init_current_operations(
            2000,
            600,
            100,
            2_000_000,
            platform_version.protocol_version,
            &mut batch,
        );
        drive
            .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
            .expect("should apply batch");

        let finalized_info_1 = FinalizedEpochInfo::V0(FinalizedEpochInfoV0 {
            first_block_time: 2_000_000,
            first_block_height: 600,
            total_blocks_in_epoch: 400,
            first_core_block_height: 100,
            next_epoch_start_core_block_height: 150,
            total_processing_fees: 20_000,
            total_distributed_storage_fees: 10_000,
            total_created_storage_fees: 12_000,
            core_block_rewards: 200_000,
            block_proposers: std::collections::BTreeMap::new(),
            fee_multiplier_permille: 2000,
            protocol_version: platform_version.protocol_version,
        });

        let op = drive
            .add_epoch_final_info_operation(&epoch1, finalized_info_1.clone(), platform_version)
            .expect("should create finalized epoch info operation");
        drive
            .grove_apply_operation(op, false, Some(&transaction), &platform_version.drive)
            .expect("should apply finalized epoch info");

        drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("should commit transaction");

        // Prove finalized epoch infos for epochs 0..=1
        let proof = drive
            .prove_finalized_epoch_infos(0, true, 1, true, None, platform_version)
            .expect("should prove finalized epoch infos");

        let (_root_hash, verified_infos) =
            Drive::verify_finalized_epoch_infos(&proof, 0, true, 1, true, platform_version)
                .expect("should verify finalized epoch infos");

        assert_eq!(verified_infos.len(), 2);
        assert_eq!(verified_infos[0].0, 0);
        assert_eq!(verified_infos[0].1, finalized_info_0);
        assert_eq!(verified_infos[1].0, 1);
        assert_eq!(verified_infos[1].1, finalized_info_1);
    }

    #[test]
    fn should_prove_and_verify_single_finalized_epoch_info() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let transaction = drive.grove.start_transaction();

        let epoch0 = Epoch::new(0).unwrap();
        let mut batch = GroveDbOpBatch::new();
        epoch0.add_init_current_operations(
            1000,
            100,
            50,
            1_000_000,
            platform_version.protocol_version,
            &mut batch,
        );
        drive
            .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
            .expect("should apply batch");

        let finalized_info = FinalizedEpochInfo::V0(FinalizedEpochInfoV0 {
            first_block_time: 1_000_000,
            first_block_height: 100,
            total_blocks_in_epoch: 500,
            first_core_block_height: 50,
            next_epoch_start_core_block_height: 100,
            total_processing_fees: 10_000,
            total_distributed_storage_fees: 5_000,
            total_created_storage_fees: 6_000,
            core_block_rewards: 100_000,
            block_proposers: std::collections::BTreeMap::new(),
            fee_multiplier_permille: 1000,
            protocol_version: platform_version.protocol_version,
        });

        let op = drive
            .add_epoch_final_info_operation(&epoch0, finalized_info.clone(), platform_version)
            .expect("should create finalized epoch info operation");
        drive
            .grove_apply_operation(op, false, Some(&transaction), &platform_version.drive)
            .expect("should apply finalized epoch info");

        drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("should commit transaction");

        let proof = drive
            .prove_finalized_epoch_infos(0, true, 0, true, None, platform_version)
            .expect("should prove finalized epoch infos");

        let (_root_hash, verified_infos) =
            Drive::verify_finalized_epoch_infos(&proof, 0, true, 0, true, platform_version)
                .expect("should verify finalized epoch infos");

        assert_eq!(verified_infos.len(), 1);
        assert_eq!(verified_infos[0].0, 0);
        assert_eq!(verified_infos[0].1, finalized_info);
    }
}
