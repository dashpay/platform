use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::proposer_block_count_query::ProposerQueryType;
use crate::verify::RootHash;
use dpp::block::epoch::{Epoch, EpochIndex};
use grovedb::{Element, GroveDb};
use platform_version::version::PlatformVersion;

impl Drive {
    /// Verifies the proposers for a given epoch using the provided proof.
    ///
    /// This function checks if the proposers for the specified epoch are correctly included
    /// in the provided proof by querying GroveDB and returns the root hash along with the
    /// list of proposers and their corresponding block counts.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the Merkle proof used to verify the data.
    /// - `epoch_index`: The index of the epoch for which the proposers are being verified.
    /// - `limit`: The maximum number of proposers to retrieve.
    /// - `platform_version`: The current version of the platform to ensure compatibility during verification.
    ///
    /// # Returns
    ///
    /// A `Result` containing a tuple:
    ///
    /// - `RootHash`: The calculated root hash from the verified proof.
    /// - `I`: An iterator over the proposers, each paired with their block count. The type `I`
    ///   is expected to be constructed using the `FromIterator` trait.
    ///
    /// # Errors
    ///
    /// This function returns an error if:
    ///
    /// - The proof is invalid or corrupted.
    /// - The GroveDB query fails.
    /// - A proposer’s block count cannot be parsed as a `u64`.
    /// - An unexpected element type is encountered during verification.
    ///
    /// ```
    #[inline(always)]
    pub(super) fn verify_epoch_proposers_v0<I, P, E>(
        proof: &[u8],
        epoch_index: EpochIndex,
        proposer_query_type: ProposerQueryType,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, I), Error>
    where
        I: FromIterator<(P, u64)>,
        P: TryFrom<Vec<u8>, Error = E>,
    {
        let epoch = Epoch::new(epoch_index)?;

        let path_query = proposer_query_type.into_path_query(&epoch);

        let (root_hash, elements) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?;

        let proposers = elements
            .into_iter()
            .map(|(_, pro_tx_hash, element)| {
                let Some(Element::Item(encoded_block_count, _)) = element else {
                    return Err(Error::Drive(DriveError::UnexpectedElementType(
                        "epochs proposer block count must be an item",
                    )));
                };

                let block_count = u64::from_be_bytes(
                    encoded_block_count.as_slice().try_into().map_err(|_| {
                        Error::Drive(DriveError::CorruptedSerialization(String::from(
                            "epochs proposer block count must be u64",
                        )))
                    })?,
                );

                Ok((
                    pro_tx_hash.try_into().map_err(|_| {
                        Error::Drive(DriveError::CorruptedSerialization(
                            "item has an invalid length".to_string(),
                        ))
                    })?,
                    block_count,
                ))
            })
            .collect::<Result<I, _>>()?;

        Ok((root_hash, proposers))
    }
}
