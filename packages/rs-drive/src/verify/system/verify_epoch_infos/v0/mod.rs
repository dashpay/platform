use crate::drive::credit_pools::epochs::epoch_key_constants::{
    KEY_FEE_MULTIPLIER, KEY_PROTOCOL_VERSION, KEY_START_BLOCK_CORE_HEIGHT, KEY_START_BLOCK_HEIGHT,
    KEY_START_TIME,
};
use crate::drive::credit_pools::pools_vec_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::query::{Query, QueryItem};
use crate::verify::RootHash;
use dpp::block::epoch::{EpochIndex, EPOCH_KEY_OFFSET};
use dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0;
use dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dpp::ProtocolError;
use grovedb::{Element, GroveDb, PathQuery, SizedQuery};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    /// Verifies that the contract is included in the proof.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof to be verified.
    /// - `current_epoch`: The current epoch index, can be acquired from metadata.
    /// - `start_epoch`: The first epoch index.
    /// - `count`: The amount of epochs to get.
    /// - `ascending`: True if we want to get epochs from oldest to newest.
    /// - `platform_version`: the platform version,
    ///
    /// # Returns
    ///
    /// Returns a `Result` with a tuple of `RootHash` and `Option<DataContract>`. The `Option<DataContract>`
    /// represents the verified contract if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof is corrupted.
    /// - The GroveDb query fails.
    #[inline(always)]
    pub(super) fn verify_epoch_infos_v0(
        proof: &[u8],
        current_epoch: EpochIndex,
        start_epoch: Option<EpochIndex>,
        count: u16,
        ascending: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<ExtendedEpochInfo>), Error> {
        let start_epoch_index = start_epoch.unwrap_or({
            if ascending {
                0
            } else {
                current_epoch
            }
        });
        let index_with_offset = start_epoch_index
            .checked_add(EPOCH_KEY_OFFSET)
            .ok_or(ProtocolError::Overflow("stored epoch index too high"))?;
        let mut subquery = Query::new();
        subquery.insert_keys(vec![
            KEY_START_TIME.to_vec(),
            KEY_START_BLOCK_HEIGHT.to_vec(),
            KEY_START_BLOCK_CORE_HEIGHT.to_vec(),
            KEY_FEE_MULTIPLIER.to_vec(),
            KEY_PROTOCOL_VERSION.to_vec(),
        ]);
        let mut query = if ascending {
            Query::new_single_query_item(QueryItem::RangeFrom(
                index_with_offset.to_be_bytes().to_vec()..,
            ))
        } else {
            Query::new_single_query_item(QueryItem::RangeToInclusive(
                ..=index_with_offset.to_be_bytes().to_vec(),
            ))
        };
        query.left_to_right = ascending;
        query.set_subquery(subquery);
        let path_query = PathQuery::new(
            pools_vec_path(),
            // The multiplier must be equal to requested keys count
            SizedQuery::new(query, Some(count * 5), None),
        );

        let (root_hash, elements) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?;

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

        // Convert the BTreeMap entries to ExtendedEpochInfo
        let extended_epoch_infos = results
            .into_iter()
            .filter_map(|(path, inner_map)| {
                // Extract the epoch index from the path's last component
                // and adjust by subtracting the EPOCH_KEY_OFFSET
                let epoch_index_result: Result<EpochIndex, Error> = path
                    .last()
                    .ok_or(Error::Proof(ProofError::CorruptedProof(
                        "extended epoch info: path can not be empty".to_string(),
                    )))
                    .and_then(|epoch_index_vec| {
                        epoch_index_vec.as_slice().try_into().map_err(|_| {
                            Error::Proof(ProofError::CorruptedProof(
                                "extended epoch info: item has an invalid length".to_string(),
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

                let first_block_time_element = inner_map.get(KEY_START_TIME.as_slice())?;

                let Some(Element::Item(encoded_start_time, _)) = first_block_time_element else {
                    return Some(Err(Error::Drive(DriveError::UnexpectedElementType(
                        "start time must be an item",
                    ))));
                };

                let first_block_time_bytes: [u8; 8] =
                    match encoded_start_time.as_slice().try_into().map_err(|_| {
                        Error::Drive(DriveError::CorruptedSerialization(
                            "extended epoch info: block time must be 8 bytes for a u64".to_string(),
                        ))
                    }) {
                        Ok(value) => value,
                        Err(e) => return Some(Err(e)),
                    };

                let first_block_time = u64::from_be_bytes(first_block_time_bytes);

                let first_block_height_element =
                    inner_map.get(KEY_START_BLOCK_HEIGHT.as_slice())?;

                let Some(Element::Item(encoded_start_block_height, _)) = first_block_height_element
                else {
                    return Some(Err(Error::Drive(DriveError::UnexpectedElementType(
                        "extended epoch info: start time must be an item",
                    ))));
                };

                let first_block_height_bytes: [u8; 8] = match encoded_start_block_height
                    .as_slice()
                    .try_into()
                    .map_err(|_| {
                        Error::Drive(DriveError::CorruptedSerialization(
                            "extended epoch info: block height must be 8 bytes for a u64"
                                .to_string(),
                        ))
                    }) {
                    Ok(value) => value,
                    Err(e) => return Some(Err(e)),
                };

                let first_block_height = u64::from_be_bytes(first_block_height_bytes);

                let first_core_block_height_element =
                    inner_map.get(KEY_START_BLOCK_CORE_HEIGHT.as_slice())?;

                let Some(Element::Item(encoded_start_core_block_height, _)) =
                    first_core_block_height_element
                else {
                    return Some(Err(Error::Drive(DriveError::UnexpectedElementType(
                        "start time must be an item",
                    ))));
                };

                let first_core_block_height_bytes: [u8; 4] = match encoded_start_core_block_height
                    .as_slice()
                    .try_into()
                    .map_err(|_| {
                        Error::Drive(DriveError::CorruptedSerialization(
                            "core block height must be 4 bytes for a u32".to_string(),
                        ))
                    }) {
                    Ok(value) => value,
                    Err(e) => return Some(Err(e)),
                };

                let first_core_block_height = u32::from_be_bytes(first_core_block_height_bytes);

                let fee_multiplier_element = inner_map.get(KEY_FEE_MULTIPLIER.as_slice())?;

                let Some(Element::Item(encoded_multiplier, _)) = fee_multiplier_element else {
                    return Some(Err(Error::Drive(DriveError::UnexpectedElementType(
                        "epochs multiplier must be an item",
                    ))));
                };

                let fee_multiplier_bytes: [u8; 8] =
                    match encoded_multiplier.as_slice().try_into().map_err(|_| {
                        Error::Drive(DriveError::CorruptedSerialization(
                            "fee multiplier must be 8 bytes for a u64".to_string(),
                        ))
                    }) {
                        Ok(value) => value,
                        Err(e) => return Some(Err(e)),
                    };

                let fee_multiplier = u64::from_be_bytes(fee_multiplier_bytes);

                let protocol_version_element = inner_map.get(KEY_PROTOCOL_VERSION.as_slice())?;

                let Some(Element::Item(encoded_protocol_version, _)) = protocol_version_element
                else {
                    return Some(Err(Error::Drive(DriveError::UnexpectedElementType(
                        "protocol version must be an item",
                    ))));
                };

                let protocol_version_bytes: [u8; 4] =
                    match encoded_protocol_version.as_slice().try_into().map_err(|_| {
                        Error::Drive(DriveError::CorruptedSerialization(
                            "protocol version must be 4 bytes for a u32".to_string(),
                        ))
                    }) {
                        Ok(value) => value,
                        Err(e) => return Some(Err(e)),
                    };

                let protocol_version = u32::from_be_bytes(protocol_version_bytes);

                // Construct the ExtendedEpochInfo
                Some(Ok(ExtendedEpochInfoV0 {
                    index: epoch_index,
                    first_block_time,
                    first_block_height,
                    first_core_block_height,
                    fee_multiplier_permille: fee_multiplier,
                    protocol_version,
                }
                .into()))
            })
            .collect::<Result<Vec<ExtendedEpochInfo>, Error>>()?;

        Ok((root_hash, extended_epoch_infos))
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
    use dpp::block::epoch::{Epoch, MAX_EPOCH};

    #[test]
    fn should_prove_and_verify_epoch_infos_ascending() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let transaction = drive.grove.start_transaction();

        // Set up epoch 0 with known data (epoch 0 subtree already exists from initial setup)
        let epoch = Epoch::new(0).unwrap();
        let start_time: u64 = 1_000_000;
        let start_block_height: u64 = 100;
        let start_block_core_height: u32 = 50;
        let fee_multiplier: u64 = 1000;
        let protocol_version: u32 = platform_version.protocol_version;

        let mut batch = GroveDbOpBatch::new();
        epoch.add_init_current_operations(
            fee_multiplier,
            start_block_height,
            start_block_core_height,
            start_time,
            protocol_version,
            &mut batch,
        );
        drive
            .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
            .expect("should apply batch");

        // Set up epoch 1
        let epoch1 = Epoch::new(1).unwrap();
        let start_time1: u64 = 2_000_000;
        let start_block_height1: u64 = 200;
        let start_block_core_height1: u32 = 100;
        let fee_multiplier1: u64 = 2000;

        let mut batch = GroveDbOpBatch::new();
        epoch1
            .add_init_empty_operations(&mut batch)
            .expect("should init empty epoch");
        epoch1.add_init_current_operations(
            fee_multiplier1,
            start_block_height1,
            start_block_core_height1,
            start_time1,
            protocol_version,
            &mut batch,
        );
        drive
            .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
            .expect("should apply batch");

        drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("should commit transaction");

        // Prove starting from epoch 0, ascending, count = 2
        let proof = drive
            .prove_epochs_infos(0, 2, true, None, platform_version)
            .expect("should prove epoch infos");

        let (_root_hash, epoch_infos) = Drive::verify_epoch_infos(
            &proof,
            1,       // current_epoch
            Some(0), // start_epoch
            2,       // count
            true,    // ascending
            platform_version,
        )
        .expect("should verify epoch infos");

        assert_eq!(epoch_infos.len(), 2);

        let info0: &ExtendedEpochInfo = &epoch_infos[0];
        let expected0 = ExtendedEpochInfo::from(ExtendedEpochInfoV0 {
            index: 0,
            first_block_time: start_time,
            first_block_height: start_block_height,
            first_core_block_height: start_block_core_height,
            fee_multiplier_permille: fee_multiplier,
            protocol_version,
        });
        assert_eq!(info0, &expected0);

        let info1: &ExtendedEpochInfo = &epoch_infos[1];
        let expected1 = ExtendedEpochInfo::from(ExtendedEpochInfoV0 {
            index: 1,
            first_block_time: start_time1,
            first_block_height: start_block_height1,
            first_core_block_height: start_block_core_height1,
            fee_multiplier_permille: fee_multiplier1,
            protocol_version,
        });
        assert_eq!(info1, &expected1);
    }

    #[test]
    fn should_prove_and_verify_epoch_infos_descending() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let transaction = drive.grove.start_transaction();

        let epoch = Epoch::new(0).unwrap();
        let mut batch = GroveDbOpBatch::new();
        epoch.add_init_current_operations(
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

        let epoch1 = Epoch::new(1).unwrap();
        let mut batch = GroveDbOpBatch::new();
        epoch1
            .add_init_empty_operations(&mut batch)
            .expect("should init empty epoch");
        epoch1.add_init_current_operations(
            2000,
            200,
            100,
            2_000_000,
            platform_version.protocol_version,
            &mut batch,
        );
        drive
            .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
            .expect("should apply batch");

        drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("should commit transaction");

        // Prove descending from epoch 1
        let proof = drive
            .prove_epochs_infos(1, 2, false, None, platform_version)
            .expect("should prove epoch infos");

        let (_root_hash, epoch_infos) =
            Drive::verify_epoch_infos(&proof, 1, Some(1), 2, false, platform_version)
                .expect("should verify epoch infos descending");

        assert_eq!(epoch_infos.len(), 2);
        // Results are ordered by epoch index (BTreeMap ordering), regardless of query direction
        assert_eq!(
            epoch_infos[0],
            ExtendedEpochInfoV0 {
                index: 0,
                first_block_time: 1_000_000,
                first_block_height: 100,
                first_core_block_height: 50,
                fee_multiplier_permille: 1000,
                protocol_version: platform_version.protocol_version,
            }
            .into()
        );
        assert_eq!(
            epoch_infos[1],
            ExtendedEpochInfoV0 {
                index: 1,
                first_block_time: 2_000_000,
                first_block_height: 200,
                first_core_block_height: 100,
                fee_multiplier_permille: 2000,
                protocol_version: platform_version.protocol_version,
            }
            .into()
        );

        // Descending with a start above the current epoch does NOT skip forward to
        // the newest started epoch: the initial state structure pre-creates empty
        // epoch trees ahead of the current one, and the query limit is consumed by
        // those empty trees, so the proof verifiably contains no epochs at all.
        // This is why "fetch the current epoch" cannot be expressed as a single
        // descending query from MAX_EPOCH — the SDK's `fetch_current` must first
        // learn the current epoch index and use it as an explicit start (as the
        // in-range `Some(1)` queries above do).
        let max_start_proof = drive
            .prove_epochs_infos(MAX_EPOCH, 1, false, None, platform_version)
            .expect("should prove epoch infos from MAX_EPOCH");

        let (_root_hash, far_future_start_infos) = Drive::verify_epoch_infos(
            &max_start_proof,
            1,
            Some(MAX_EPOCH),
            1,
            false,
            platform_version,
        )
        .expect("should verify epoch infos descending from MAX_EPOCH");

        assert_eq!(
            far_future_start_infos.len(),
            0,
            "descending from inside the pre-created empty epoch window must \
             provably return no epochs, not the newest started epoch"
        );
    }
}
