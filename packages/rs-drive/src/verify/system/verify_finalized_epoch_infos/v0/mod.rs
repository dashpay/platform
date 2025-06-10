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
