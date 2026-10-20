use crate::verify::RootHash;
use dpp::identifier::Identifier;
use dpp::serialization::PlatformDeserializableUntrusted;
use grovedb::{Element, GroveDb};

use crate::error::Error;

use crate::drive::votes::paths::{
    RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32, RESOURCE_LOCK_VOTE_TREE_KEY_U8_32,
    RESOURCE_STORED_INFO_KEY_U8_32,
};
use crate::error::drive::DriveError;
use crate::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQueryExecutionResult,
    ContestedDocumentVotePollDriveQueryResultType, ResolvedContestedDocumentVotePollDriveQuery,
};
use dpp::version::PlatformVersion;
use dpp::voting::contender_structs::{
    ContenderWithSerializedDocument, ContenderWithSerializedDocumentV0,
};
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::{
    ContestedDocumentVotePollStoredInfo, ContestedDocumentVotePollStoredInfoV0Getters,
};

impl ResolvedContestedDocumentVotePollDriveQuery<'_> {
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
        // println!("{:?}", &path_query);
        let (root_hash, proved_key_values) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?;

        match self.result_type {
            ContestedDocumentVotePollDriveQueryResultType::Documents
            | ContestedDocumentVotePollDriveQueryResultType::SingleDocumentByContender(_) => {
                let contenders = proved_key_values
                    .into_iter()
                    .map(|(mut path, _key, document)| {
                        let identity_id =
                            path.pop()
                                .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                                    "the path must have a last element".to_string(),
                                )))?;
                        Ok(ContenderWithSerializedDocumentV0 {
                            identity_id: Identifier::try_from(identity_id)?,
                            serialized_document: document
                                .map(|document| document.into_item_bytes())
                                .transpose()?,
                            vote_tally: None,
                        }
                        .into())
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
                let elements_iter = proved_key_values.into_iter();
                let mut contenders = vec![];
                let mut locked_vote_tally: Option<u32> = None;
                let mut abstaining_vote_tally: Option<u32> = None;
                let mut winner = None;

                // Handle ascending order
                for (path, first_key, element) in elements_iter {
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

                            if identity_bytes.as_slice()
                                == RESOURCE_LOCK_VOTE_TREE_KEY_U8_32.as_slice()
                            {
                                locked_vote_tally = Some(sum_tree_value as u32);
                            } else if identity_bytes.as_slice()
                                == RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32.as_slice()
                            {
                                abstaining_vote_tally = Some(sum_tree_value as u32);
                            } else {
                                contenders.push(
                                    ContenderWithSerializedDocumentV0 {
                                        identity_id: Identifier::try_from(identity_bytes)?,
                                        serialized_document: None,
                                        vote_tally: Some(sum_tree_value as u32),
                                    }
                                    .into(),
                                );
                            }
                        }
                        Element::Item(serialized_item_info, _) => {
                            if first_key.as_slice() == RESOURCE_STORED_INFO_KEY_U8_32 {
                                // this is the stored info, let's check to see if the vote is over
                                let finalized_contested_document_vote_poll_stored_info =
                                    ContestedDocumentVotePollStoredInfo::deserialize_from_bytes_untrusted(
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
                                return Err(Error::Drive(DriveError::CorruptedDriveState(
                                    "the only item that should be returned should be stored info"
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

                            if identity_bytes.as_slice()
                                == RESOURCE_LOCK_VOTE_TREE_KEY_U8_32.as_slice()
                            {
                                locked_vote_tally = Some(sum_tree_value as u32);
                            } else if identity_bytes.as_slice()
                                == RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32.as_slice()
                            {
                                abstaining_vote_tally = Some(sum_tree_value as u32);
                            } else {
                                return Err(Error::Drive(DriveError::CorruptedDriveState(
                                    "unexpected key for sum tree value in verification".to_string(),
                                )));
                            }
                        }
                        Element::Item(serialized_item_info, _) => {
                            if first_key.as_slice() == RESOURCE_STORED_INFO_KEY_U8_32 {
                                // this is the stored info, let's check to see if the vote is over
                                let finalized_contested_document_vote_poll_stored_info =
                                    ContestedDocumentVotePollStoredInfo::deserialize_from_bytes_untrusted(
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
                                    let contender = ContenderWithSerializedDocumentV0 {
                                        identity_id,
                                        serialized_document: Some(serialized_item_info),
                                        vote_tally: Some(sum_tree_value as u32),
                                    }
                                    .into();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed;
    use crate::query::vote_poll_vote_state_query::ContestedDocumentVotePollDriveQueryResultType;
    use crate::util::object_size_info::DataContractResolvedInfo;
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use crate::util::test_helpers::{add_dpns_name_contenders, dpns_name_contender_id};
    use dpp::block::block_info::BlockInfo;
    use dpp::tests::fixtures::get_dpns_data_contract_fixture;
    use dpp::tests::json_document::json_document_to_contract;
    use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
    use std::sync::Arc;

    #[test]
    fn should_prove_and_verify_empty_vote_poll_vote_state_proof_documents() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract(
            "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index.json",
            false,
            platform_version,
        )
        .expect("expected to create a data contract");

        // Insert the DPNS contract so its paths exist in the store
        drive
            .insert_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        let arc_contract = Arc::new(data_contract);

        let query = ResolvedContestedDocumentVotePollDriveQuery {
            vote_poll: ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed {
                contract: DataContractResolvedInfo::ArcDataContract(arc_contract),
                document_type_name: "domain".to_string(),
                index_name: "parentNameAndLabel".to_string(),
                index_values: vec![
                    dpp::platform_value::Value::Text("dash".to_string()),
                    dpp::platform_value::Value::Text("test".to_string()),
                ],
            },
            result_type: ContestedDocumentVotePollDriveQueryResultType::Documents,
            offset: None,
            limit: Some(10),
            start_at: None,
            allow_include_locked_and_abstaining_vote_tally: false,
        };

        let path_query = query
            .construct_path_query(platform_version)
            .expect("expected to construct path query");

        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("expected to get proof");

        let (_, result) = query
            .verify_vote_poll_vote_state_proof(proof.as_slice(), platform_version)
            .expect("expected proof verification to succeed");

        assert!(result.contenders.is_empty());
        assert_eq!(result.locked_vote_tally, None);
        assert_eq!(result.abstaining_vote_tally, None);
        assert_eq!(result.winner, None);
    }

    /// A poll holding the most contenders a contest accepts, its choices in a count tree, is
    /// read page by page as clients read it: each page proved, verified against the state and
    /// equal to the unproved read, the pages together every contender once, in identity order
    #[test]
    fn should_page_proved_vote_states_through_the_most_contenders_a_contest_accepts() {
        let platform_version = PlatformVersion::latest();
        let contenders = platform_version.system_limits.max_contenders_per_contest as u64;
        let page_size = 100;
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let dpns_contract = get_dpns_data_contract_fixture(
            Some(Identifier::from([7; 32])),
            0,
            platform_version.protocol_version,
        )
        .data_contract_owned();
        drive
            .apply_contract(
                &dpns_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply the DPNS contract");
        let vote_poll = add_dpns_name_contenders(
            &drive,
            &dpns_contract,
            "quantum",
            0..contenders,
            |_| 1,
            &BlockInfo::default(),
            platform_version,
        );
        // Every seventh contender gets a vote, and one vote locks
        let votes = (0..contenders)
            .step_by(7)
            .map(|n| ResourceVoteChoice::TowardsIdentity(dpns_name_contender_id(n)))
            .chain([ResourceVoteChoice::Lock]);
        for (voter, choice) in votes.enumerate() {
            let mut pro_tx_hash = [0xFFu8; 32];
            pro_tx_hash[..8].copy_from_slice(&(voter as u64).to_be_bytes());
            drive
                .register_contested_resource_identity_vote(
                    pro_tx_hash,
                    1,
                    vote_poll.clone(),
                    choice,
                    None,
                    &BlockInfo::default(),
                    None,
                    platform_version,
                )
                .expect("expected to register the vote");
        }
        let root_hash = drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("expected the root hash");

        let mut read = vec![];
        let mut start_at = None;
        let mut pages = 0;
        loop {
            let query = ResolvedContestedDocumentVotePollDriveQuery {
                vote_poll: (&vote_poll).into(),
                result_type: ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
                offset: None,
                limit: Some(page_size),
                start_at,
                allow_include_locked_and_abstaining_vote_tally: start_at.is_none(),
            };
            let proof = drive
                .grove_get_proved_path_query(
                    &query
                        .construct_path_query(platform_version)
                        .expect("expected the path query"),
                    None,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected the proof");
            let (proved_root_hash, proved) = query
                .verify_vote_poll_vote_state_proof(proof.as_slice(), platform_version)
                .expect("expected the proof to verify");
            assert_eq!(proved_root_hash, root_hash, "page {pages}");
            let unproved = query
                .execute(&drive, None, &mut vec![], platform_version)
                .expect("expected the unproved read");
            assert_eq!(proved, unproved, "page {pages}");
            if pages == 0 {
                assert_eq!(proved.locked_vote_tally, Some(1));
                assert_eq!(proved.abstaining_vote_tally, Some(0));
            }
            pages += 1;

            let last = proved
                .contenders
                .last()
                .map(|contender| contender.identity_id());
            let full_page = proved.contenders.len() == page_size as usize;
            read.extend(proved.contenders);
            match last {
                Some(last) if full_page => start_at = Some((last.to_buffer(), false)),
                _ => break,
            }
        }

        assert_eq!(pages, contenders as usize / page_size as usize + 1);
        assert_eq!(
            read.iter()
                .map(|contender| contender.identity_id())
                .collect::<Vec<_>>(),
            (0..contenders)
                .map(dpns_name_contender_id)
                .collect::<Vec<_>>()
        );
        for (n, contender) in read.iter().enumerate() {
            assert!(contender.serialized_document().is_some(), "contender {n}");
            assert_eq!(
                contender.vote_tally(),
                Some(u32::from(n % 7 == 0)),
                "contender {n}"
            );
        }
    }
}
