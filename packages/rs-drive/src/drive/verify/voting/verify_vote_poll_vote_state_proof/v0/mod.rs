use crate::drive::verify::RootHash;
use dpp::identifier::Identifier;
use dpp::serialization::PlatformDeserializable;
use grovedb::{Element, GroveDb};

use crate::error::Error;

use crate::drive::votes::paths::{
    RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8, RESOURCE_LOCK_VOTE_TREE_KEY_U8, RESOURCE_STORED_INFO_KEY_U8,
};
use crate::error::drive::DriveError;
use crate::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQueryExecutionResult,
    ContestedDocumentVotePollDriveQueryResultType, ResolvedContestedDocumentVotePollDriveQuery,
};
use dpp::version::PlatformVersion;
use dpp::voting::contender_structs::ContenderWithSerializedDocument;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::{
    ContestedDocumentVotePollStoredInfo, ContestedDocumentVotePollStoredInfoV0Getters,
};

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
    ) -> Result<(RootHash, ContestedDocumentVotePollDriveQueryExecutionResult), Error> {
        let path_query = self.construct_path_query(platform_version)?;
        let (root_hash, proved_key_values) = GroveDb::verify_query(proof, &path_query)?;

        match self.result_type {
            ContestedDocumentVotePollDriveQueryResultType::IdentityIdsOnly => {
                let contenders = proved_key_values
                    .into_iter()
                    .map(|(_, identity_id, _)| {
                        Ok(ContenderWithSerializedDocument {
                            identity_id: Identifier::try_from(identity_id)?,
                            serialized_document: None,
                            vote_tally: None,
                        })
                    })
                    .collect::<Result<Vec<ContenderWithSerializedDocument>, Error>>()?;

                Ok((
                    root_hash,
                    ContestedDocumentVotePollDriveQueryExecutionResult {
                        contenders,
                        locked_vote_tally: None,
                        abstaining_vote_tally: None,
                        winner: None,
                        skipped: 0,
                    },
                ))
            }
            ContestedDocumentVotePollDriveQueryResultType::Documents => {
                let contenders = proved_key_values
                    .into_iter()
                    .map(|(_, identity_id, document)| {
                        Ok(ContenderWithSerializedDocument {
                            identity_id: Identifier::try_from(identity_id)?,
                            serialized_document: document
                                .map(|document| document.into_item_bytes())
                                .transpose()?,
                            vote_tally: None,
                        })
                    })
                    .collect::<Result<Vec<ContenderWithSerializedDocument>, Error>>()?;

                Ok((
                    root_hash,
                    ContestedDocumentVotePollDriveQueryExecutionResult {
                        contenders,
                        locked_vote_tally: None,
                        abstaining_vote_tally: None,
                        winner: None,
                        skipped: 0,
                    },
                ))
            }
            ContestedDocumentVotePollDriveQueryResultType::VoteTally => {
                let mut contenders = Vec::new();
                let mut locked_vote_tally: Option<u32> = None;
                let mut abstaining_vote_tally: Option<u32> = None;
                let mut winner = None;

                for (_, key, vote_tally) in proved_key_values.into_iter() {
                    let Some(vote_tally) = vote_tally else {
                        continue;
                    };

                    match key.get(0) {
                        Some(key) if key == &RESOURCE_LOCK_VOTE_TREE_KEY_U8 => {
                            let sum_tree_value = vote_tally.into_sum_tree_value()?;
                            if sum_tree_value < 0 || sum_tree_value > u32::MAX as i64 {
                                return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                                    "sum tree value for vote tally must be between 0 and u32::Max, received {} from state",
                                    sum_tree_value
                                ))));
                            }
                            locked_vote_tally = Some(sum_tree_value as u32);
                        }
                        Some(key) if key == &RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8 => {
                            let sum_tree_value = vote_tally.into_sum_tree_value()?;
                            if sum_tree_value < 0 || sum_tree_value > u32::MAX as i64 {
                                return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                                    "sum tree value for vote tally must be between 0 and u32::Max, received {} from state",
                                    sum_tree_value
                                ))));
                            }
                            abstaining_vote_tally = Some(sum_tree_value as u32);
                        }
                        Some(key) if key == &RESOURCE_STORED_INFO_KEY_U8 => {
                            // The vote is actually over, let's deserialize the info
                            let finalized_contested_document_vote_poll_stored_info =
                                ContestedDocumentVotePollStoredInfo::deserialize_from_bytes(
                                    &vote_tally.into_item_bytes()?,
                                )?;
                            if finalized_contested_document_vote_poll_stored_info
                                .vote_poll_status()
                                .awarded_or_locked()
                            {
                                locked_vote_tally = Some(
                                    finalized_contested_document_vote_poll_stored_info
                                        .last_locked_votes()
                                        .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                                            "we should have last locked votes".to_string(),
                                        )))?,
                                );
                                abstaining_vote_tally = Some(
                                    finalized_contested_document_vote_poll_stored_info
                                        .last_abstain_votes()
                                        .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                                            "we should have last abstain votes".to_string(),
                                        )))?,
                                );
                                winner = Some((
                                    finalized_contested_document_vote_poll_stored_info.winner(),
                                    finalized_contested_document_vote_poll_stored_info
                                        .last_finalization_block()
                                        .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                                            "we should have a last finalization block".to_string(),
                                        )))?,
                                ));
                                contenders = finalized_contested_document_vote_poll_stored_info
                                    .contender_votes_in_vec_of_contender_with_serialized_document()
                                    .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                                        "we should have a last contender votes".to_string(),
                                    )))?;
                            }
                        }
                        _ => {
                            let sum_tree_value = vote_tally.into_sum_tree_value()?;
                            if sum_tree_value < 0 || sum_tree_value > u32::MAX as i64 {
                                return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                                    "sum tree value for vote tally must be between 0 and u32::Max, received {} from state",
                                    sum_tree_value
                                ))));
                            }
                            let identity_id = Identifier::try_from(key)?;
                            contenders.push(ContenderWithSerializedDocument {
                                identity_id,
                                serialized_document: None,
                                vote_tally: Some(sum_tree_value as u32),
                            });
                        }
                    }
                }
                Ok((
                    root_hash,
                    ContestedDocumentVotePollDriveQueryExecutionResult {
                        contenders,
                        locked_vote_tally,
                        abstaining_vote_tally,
                        winner,
                        skipped: 0,
                    },
                ))
            }
            ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally => {
                let mut elements_iter = proved_key_values.into_iter();
                let mut contenders = vec![];
                let mut locked_vote_tally: Option<u32> = None;
                let mut abstaining_vote_tally: Option<u32> = None;
                let mut winner = None;

                // Handle ascending order
                while let Some((path, first_key, element)) = elements_iter.next() {
                    let Some(element) = element else {
                        continue;
                    };
                    let Some(identity_bytes) = path.last() else {
                        return Err(Error::Drive(DriveError::CorruptedDriveState(
                            "the path must have a last element".to_string(),
                        )));
                    };

                    match element {
                        Element::SumTree(_, sum_tree_value, _) => {
                            if sum_tree_value < 0 || sum_tree_value > u32::MAX as i64 {
                                return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                                        "sum tree value for vote tally must be between 0 and u32::Max, received {} from state",
                                        sum_tree_value
                                    ))));
                            }

                            match identity_bytes.get(0) {
                                Some(key) if key == &RESOURCE_LOCK_VOTE_TREE_KEY_U8 => {
                                    locked_vote_tally = Some(sum_tree_value as u32);
                                }
                                Some(key) if key == &RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8 => {
                                    abstaining_vote_tally = Some(sum_tree_value as u32);
                                }
                                _ => {
                                    return Err(Error::Drive(DriveError::CorruptedDriveState(
                                        "unexpected key for sum tree value in verification"
                                            .to_string(),
                                    )));
                                }
                            }
                        }
                        Element::Item(serialized_item_info, _) => {
                            if first_key.len() == 1
                                && first_key.first() == Some(&RESOURCE_STORED_INFO_KEY_U8)
                            {
                                // this is the stored info, let's check to see if the vote is over
                                let finalized_contested_document_vote_poll_stored_info =
                                    ContestedDocumentVotePollStoredInfo::deserialize_from_bytes(
                                        &serialized_item_info,
                                    )?;
                                if finalized_contested_document_vote_poll_stored_info
                                    .vote_poll_status()
                                    .awarded_or_locked()
                                {
                                    locked_vote_tally = Some(
                                        finalized_contested_document_vote_poll_stored_info
                                            .last_locked_votes()
                                            .ok_or(Error::Drive(
                                                DriveError::CorruptedDriveState(
                                                    "we should have last locked votes".to_string(),
                                                ),
                                            ))?,
                                    );
                                    abstaining_vote_tally = Some(
                                        finalized_contested_document_vote_poll_stored_info
                                            .last_abstain_votes()
                                            .ok_or(Error::Drive(
                                                DriveError::CorruptedDriveState(
                                                    "we should have last abstain votes".to_string(),
                                                ),
                                            ))?,
                                    );
                                    winner = Some((
                                        finalized_contested_document_vote_poll_stored_info.winner(),
                                        finalized_contested_document_vote_poll_stored_info
                                            .last_finalization_block()
                                            .ok_or(Error::Drive(
                                                DriveError::CorruptedDriveState(
                                                    "we should have a last finalization block"
                                                        .to_string(),
                                                ),
                                            ))?,
                                    ));
                                    contenders = finalized_contested_document_vote_poll_stored_info
                                            .contender_votes_in_vec_of_contender_with_serialized_document().ok_or(Error::Drive(DriveError::CorruptedDriveState(
                                            "we should have a last contender votes".to_string(),
                                        )))?;
                                }
                            } else {
                                // We should find a sum tree paired with this document
                                if let Some((
                                    path_tally,
                                    _,
                                    Some(Element::SumTree(_, sum_tree_value, _)),
                                )) = elements_iter.next()
                                {
                                    if path != path_tally {
                                        return Err(Error::Drive(DriveError::CorruptedDriveState("the two results in a chunk when requesting documents and vote tally should both have the same path when in item verifying vote vote state proof".to_string())));
                                    }

                                    if sum_tree_value < 0 || sum_tree_value > u32::MAX as i64 {
                                        return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                                                "sum tree value for vote tally must be between 0 and u32::Max, received {} from state",
                                                sum_tree_value
                                            ))));
                                    }

                                    let identity_id = Identifier::from_bytes(identity_bytes)?;
                                    let contender = ContenderWithSerializedDocument {
                                        identity_id,
                                        serialized_document: Some(serialized_item_info),
                                        vote_tally: Some(sum_tree_value as u32),
                                    };
                                    contenders.push(contender);
                                } else {
                                    return Err(Error::Drive(DriveError::CorruptedDriveState(
                                        "we should have a sum item after a normal item".to_string(),
                                    )));
                                }
                            }
                        }
                        _ => {
                            return Err(Error::Drive(DriveError::CorruptedDriveState(
                                "unexpected element type in result".to_string(),
                            )));
                        }
                    }
                }

                Ok((
                    root_hash,
                    ContestedDocumentVotePollDriveQueryExecutionResult {
                        contenders,
                        locked_vote_tally,
                        abstaining_vote_tally,
                        winner,
                        skipped: 0,
                    },
                ))
            }
        }
    }
}
