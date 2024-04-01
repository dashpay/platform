use crate::drive::credit_pools::pools_vec_path;
use crate::drive::verify::RootHash;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::fee_pools::epochs::epoch_key_constants::{
    KEY_FEE_MULTIPLIER, KEY_START_BLOCK_CORE_HEIGHT, KEY_START_BLOCK_HEIGHT, KEY_START_TIME,
};
use crate::query::{Query, QueryItem};
use dpp::block::epoch::{EpochIndex, EPOCH_KEY_OFFSET};
use dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0;
use dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dpp::ProtocolError;
use grovedb::{Element, GroveDb, PathQuery, SizedQuery};
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
            SizedQuery::new(query, Some(count * 4), None),
        );

        let (root_hash, elements) = GroveDb::verify_query(proof, &path_query)?;

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

                let first_block_time_element = inner_map.get(&KEY_START_TIME.to_vec())?;

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

                let first_block_height_element = inner_map.get(&KEY_START_BLOCK_HEIGHT.to_vec())?;

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
                    inner_map.get(&KEY_START_BLOCK_CORE_HEIGHT.to_vec())?;

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

                let fee_multiplier_element = inner_map.get(&KEY_FEE_MULTIPLIER.to_vec())?;

                let Some(Element::Item(encoded_multiplier, _)) = fee_multiplier_element else {
                    return Some(Err(Error::Drive(DriveError::UnexpectedElementType(
                        "epochs multiplier must be an item",
                    ))));
                };

                let fee_multiplier_bytes: [u8; 8] =
                    match encoded_multiplier.as_slice().try_into().map_err(|_| {
                        Error::Drive(DriveError::CorruptedSerialization(
                            "fee multiplier must be 8 bytes for a f64".to_string(),
                        ))
                    }) {
                        Ok(value) => value,
                        Err(e) => return Some(Err(e)),
                    };

                let fee_multiplier = f64::from_be_bytes(fee_multiplier_bytes);

                // Construct the ExtendedEpochInfo
                Some(Ok(ExtendedEpochInfoV0 {
                    index: epoch_index,
                    first_block_time,
                    first_block_height,
                    first_core_block_height,
                    fee_multiplier,
                }
                .into()))
            })
            .collect::<Result<Vec<ExtendedEpochInfo>, Error>>()?;

        Ok((root_hash, extended_epoch_infos))
    }
}
