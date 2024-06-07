use crate::drive::verify::RootHash;
use dpp::identifier::Identifier;
use grovedb::{Element, GroveDb};

use crate::error::Error;

use crate::drive::votes::paths::{
    RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8, RESOURCE_LOCK_VOTE_TREE_KEY_U8,
};
use crate::error::drive::DriveError;
use crate::query::vote_poll_vote_state_query::{
    ContenderWithSerializedDocument, ContestedDocumentVotePollDriveQueryExecutionResult,
    ContestedDocumentVotePollDriveQueryResultType, ResolvedContestedDocumentVotePollDriveQuery,
};
use dpp::version::PlatformVersion;

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

                for (_, key, vote_tally) in proved_key_values.into_iter() {
                    let Some(vote_tally) = vote_tally else {
                        continue;
                    };
                    let sum_tree_value = vote_tally.into_sum_tree_value()?;
                    if sum_tree_value < 0 || sum_tree_value > u32::MAX as i64 {
                        return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                            "sum tree value for vote tally must be between 0 and u32::Max, received {} from state",
                            sum_tree_value
                        ))));
                    }

                    match key.get(0) {
                        Some(key) if key == &RESOURCE_LOCK_VOTE_TREE_KEY_U8 => {
                            locked_vote_tally = Some(sum_tree_value as u32);
                        }
                        Some(key) => {
                            if key == &RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8 {
                                abstaining_vote_tally = Some(sum_tree_value as u32);
                            }
                        }
                        _ => {
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
                        winner: None,
                        skipped: 0,
                    },
                ))
            }
            ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally => {
                let mut elements_iter = proved_key_values.into_iter();
                let mut contenders = vec![];
                let mut locked_vote_tally: Option<u32> = None;
                let mut abstaining_vote_tally: Option<u32> = None;

                if self.order_ascending {
                    // Handle ascending order
                    while let Some((path, _, element)) = elements_iter.next() {
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
                                            "unexpected key for sum tree value".to_string(),
                                        )));
                                    }
                                }
                            }
                            Element::Item(serialized_document, _) => {
                                // We should find a sum tree paired with this document
                                if let Some((
                                    path_tally,
                                    _,
                                    Some(Element::SumTree(_, sum_tree_value, _)),
                                )) = elements_iter.next()
                                {
                                    if path != path_tally {
                                        return Err(Error::Drive(DriveError::CorruptedDriveState("the two results in a chunk when requesting documents and vote tally should both have the same path".to_string())));
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
                                        serialized_document: Some(serialized_document),
                                        vote_tally: Some(sum_tree_value as u32),
                                    };
                                    contenders.push(contender);
                                } else {
                                    return Err(Error::Drive(DriveError::CorruptedDriveState(
                                        "expected a sum tree element after item element"
                                            .to_string(),
                                    )));
                                }
                            }
                            _ => {
                                return Err(Error::Drive(DriveError::CorruptedDriveState(
                                    "unexpected element type in result".to_string(),
                                )));
                            }
                        }
                    }
                } else {
                    // Handle descending order
                    let mut prev_element: Option<(Vec<Vec<u8>>, i64, Element)> = None;

                    while let Some((path, _, element)) = elements_iter.next() {
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
                                        prev_element =
                                            Some((path.clone(), sum_tree_value, element));
                                    }
                                }
                            }
                            Element::Item(serialized_document, _) => {
                                if let Some((
                                    prev_path,
                                    sum_tree_value,
                                    Element::SumTree(_, _, _),
                                )) = prev_element.take()
                                {
                                    if prev_path != path {
                                        return Err(Error::Drive(DriveError::CorruptedDriveState("the two results in a chunk when requesting documents and vote tally should both have the same path".to_string())));
                                    }

                                    let identity_id = Identifier::from_bytes(identity_bytes)?;
                                    let contender = ContenderWithSerializedDocument {
                                        identity_id,
                                        serialized_document: Some(serialized_document),
                                        vote_tally: Some(sum_tree_value as u32),
                                    };
                                    contenders.push(contender);
                                } else {
                                    return Err(Error::Drive(DriveError::CorruptedDriveState(
                                        "expected a sum tree element before item element"
                                            .to_string(),
                                    )));
                                }
                            }
                            _ => {
                                return Err(Error::Drive(DriveError::CorruptedDriveState(
                                    "unexpected element type in result".to_string(),
                                )));
                            }
                        }
                    }
                }

                Ok((
                    root_hash,
                    ContestedDocumentVotePollDriveQueryExecutionResult {
                        contenders,
                        locked_vote_tally,
                        abstaining_vote_tally,
                        winner: None,
                        skipped: 0,
                    },
                ))
            }
        }
    }
}
