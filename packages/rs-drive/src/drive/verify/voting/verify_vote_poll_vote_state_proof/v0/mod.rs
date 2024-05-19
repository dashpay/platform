use grovedb::{Element, GroveDb};
use dpp::identifier::Identifier;
use crate::drive::verify::RootHash;

use crate::error::Error;

use dpp::version::PlatformVersion;
use crate::error::drive::DriveError;
use crate::query::vote_poll_vote_state_query::{Contender, ContestedDocumentVotePollDriveQueryResultType, ResolvedContestedDocumentVotePollDriveQuery};

impl<'a> ResolvedContestedDocumentVotePollDriveQuery<'a> {
    /// Verifies a proof for a collection of documents.
    ///
    /// This function takes a slice of bytes `proof` containing a serialized proof,
    /// verifies it, and returns a tuple consisting of the root hash and a vector of deserialized documents.
    ///
    /// # Arguments
    ///
    /// * `proof` - A byte slice representing the proof to be verified.
    /// * `drive_version` - The current active drive version
    ///
    /// # Returns
    ///
    /// A `Result` containing:
    /// * A tuple with the root hash and a vector of deserialized `Document`s, if the proof is valid.
    /// * An `Error` variant, in case the proof verification fails or deserialization error occurs.
    ///
    /// # Errors
    ///
    /// This function will return an `Error` variant if:
    /// 1. The proof verification fails.
    /// 2. There is a deserialization error when parsing the serialized document(s) into `Document` struct(s).
    #[inline(always)]
    pub(super) fn verify_vote_poll_vote_state_proof_v0(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<Contender>), Error> {
        let path_query = self.construct_path_query(platform_version)?;
        let (root_hash, proved_key_values) = GroveDb::verify_query(proof, &path_query)?;

        let contenders = match self.result_type {
            ContestedDocumentVotePollDriveQueryResultType::IdentityIdsOnly => {
                proved_key_values.into_iter().map(|(_,identity_id, _)| Ok(Contender {
                    identity_id: Identifier::try_from(identity_id)?,
                    serialized_document: None,
                    vote_tally: None,
                })).collect::<Result<Vec<Contender>, Error>>()
            }
            ContestedDocumentVotePollDriveQueryResultType::Documents => {
                proved_key_values.into_iter().map(|(_, identity_id, document)| {
                    Ok(Contender {
                        identity_id: Identifier::try_from(identity_id)?,
                        serialized_document: document.map(|document| document.into_item_bytes()).transpose()?,
                        vote_tally: None,
                    })
                }).collect::<Result<Vec<Contender>, Error>>()
            }
            ContestedDocumentVotePollDriveQueryResultType::VoteTally => {
                proved_key_values.into_iter().map(|(_, identity_id, vote_tally)| {
                    let sum_tree_value = vote_tally.map(|vote_tally| vote_tally.into_sum_tree_value()).transpose()?.unwrap_or_default();
                    if sum_tree_value < 0 || sum_tree_value > u32::MAX as i64 {
                        return Err(Error::Drive(DriveError::CorruptedDriveState(format!("sum tree value for vote tally must be between 0 and u32::Max, received {} from state", sum_tree_value))));
                    }
                    Ok(Contender {
                        identity_id: Identifier::try_from(identity_id)?,
                        serialized_document: None,
                        vote_tally: Some(sum_tree_value as u32),
                    })
                }).collect::<Result<Vec<Contender>, Error>>()
            }
            ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally => {
                let mut elements_iter = proved_key_values.into_iter();

                let mut contenders = vec![];
                while let (Some((path_doc, _, Some(Element::Item(serialized_document, _)))), Some((path_tally, _, Some(Element::SumTree(_, sum_tree_value, _))))) = (elements_iter.next(), elements_iter.next())  {
                    if path_doc != path_tally {
                        return Err(Error::Drive(DriveError::CorruptedDriveState("the two results in a chunk when requesting documents and vote tally should both have the same path".to_string())));
                    }
                    let Some(identity_bytes) = path_doc.last() else {
                        return Err(Error::Drive(DriveError::CorruptedDriveState("the path must have a last element".to_string())));
                    };

                    let identity_id = Identifier::from_bytes(identity_bytes)?;

                    if sum_tree_value < 0 || sum_tree_value > u32::MAX as i64 {
                        return Err(Error::Drive(DriveError::CorruptedDriveState(format!("sum tree value for vote tally must be between 0 and u32::Max, received {} from state", sum_tree_value))));
                    }

                    let contender = Contender {
                        identity_id,
                        serialized_document: Some(serialized_document),
                        vote_tally: Some(sum_tree_value as u32),
                    };

                    contenders.push(contender)
                }
                Ok(contenders)
            }
        }?;
        
        Ok((root_hash, contenders))
    }
}
