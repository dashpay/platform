use crate::drive::votes::paths::{
    VotePollPaths, RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8, RESOURCE_LOCK_VOTE_TREE_KEY_U8,
    RESOURCE_STORED_INFO_KEY_U8,
};
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::resolve::ContestedDocumentResourceVotePollResolver;
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
#[cfg(feature = "server")]
use crate::fee::op::LowLevelDriveOperation;
#[cfg(feature = "server")]
use crate::query::GroveError;
#[cfg(feature = "server")]
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use dpp::serialization::PlatformDeserializable;
use dpp::voting::contender_structs::{
    ContenderWithSerializedDocument, FinalizedContenderWithSerializedDocument,
};
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::{
    ContestedDocumentVotePollStoredInfo, ContestedDocumentVotePollStoredInfoV0Getters,
};
use dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
#[cfg(feature = "server")]
use grovedb::query_result_type::{QueryResultElements, QueryResultType};
#[cfg(feature = "server")]
use grovedb::{Element, TransactionArg};
use grovedb::{PathQuery, Query, QueryItem, SizedQuery};
use platform_version::version::PlatformVersion;
#[cfg(feature = "verify")]
use std::sync::Arc;

/// Represents the types of results that can be obtained from a contested document vote poll query.
///
/// This enum defines the various types of results that can be returned when querying the drive
/// for contested document vote poll information.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ContestedDocumentVotePollDriveQueryResultType {
    /// Only the identity IDs are returned in the query result.
    IdentityIdsOnly,
    /// The documents associated with the vote poll are returned in the query result.
    Documents,
    /// The vote tally results are returned in the query result.
    VoteTally,
    /// Both the documents and the vote tally results are returned in the query result.
    DocumentsAndVoteTally,
}

impl ContestedDocumentVotePollDriveQueryResultType {
    /// Helper method to say if this result type should return vote tally
    pub fn has_vote_tally(&self) -> bool {
        match self {
            ContestedDocumentVotePollDriveQueryResultType::IdentityIdsOnly => false,
            ContestedDocumentVotePollDriveQueryResultType::Documents => false,
            ContestedDocumentVotePollDriveQueryResultType::VoteTally => true,
            ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally => true,
        }
    }

    /// Helper method to say if this result type should return documents
    pub fn has_documents(&self) -> bool {
        match self {
            ContestedDocumentVotePollDriveQueryResultType::IdentityIdsOnly => false,
            ContestedDocumentVotePollDriveQueryResultType::Documents => true,
            ContestedDocumentVotePollDriveQueryResultType::VoteTally => false,
            ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally => true,
        }
    }
}

impl TryFrom<i32> for ContestedDocumentVotePollDriveQueryResultType {
    type Error = Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ContestedDocumentVotePollDriveQueryResultType::IdentityIdsOnly),
            1 => Ok(ContestedDocumentVotePollDriveQueryResultType::Documents),
            2 => Ok(ContestedDocumentVotePollDriveQueryResultType::VoteTally),
            3 => Ok(ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally),
            n => Err(Error::Query(QuerySyntaxError::Unsupported(format!(
                "unsupported contested document vote poll drive query result type {}, only 0, 1, 2 and 3 are supported",
                n
            )))),
        }
    }
}

/// Vote Poll Drive Query struct
#[derive(Debug, PartialEq, Clone)]
pub struct ContestedDocumentVotePollDriveQuery {
    /// What vote poll are we asking for?
    pub vote_poll: ContestedDocumentResourceVotePoll,
    /// What result type are we interested in
    pub result_type: ContestedDocumentVotePollDriveQueryResultType,
    /// Offset
    pub offset: Option<u16>,
    /// Limit
    pub limit: Option<u16>,
    /// Start at identity id
    pub start_at: Option<([u8; 32], bool)>,
    /// Ascending
    pub order_ascending: bool,
    /// Include locked and abstaining vote tally
    /// This is not automatic, it will just be at the beginning if the order is ascending
    /// If the order is descending, we will get a value if we finish the query
    pub allow_include_locked_and_abstaining_vote_tally: bool,
}

/// Represents the result of executing a contested document vote poll drive query.
///
/// This struct holds the list of contenders and the number of skipped items
/// when an offset is given.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct ContestedDocumentVotePollDriveQueryExecutionResult {
    /// The list of contenders returned by the query.
    pub contenders: Vec<ContenderWithSerializedDocument>,
    /// Locked tally
    pub locked_vote_tally: Option<u32>,
    /// Abstaining tally
    pub abstaining_vote_tally: Option<u32>,
    /// Finalization info
    pub winner: Option<(ContestedDocumentVotePollWinnerInfo, BlockInfo)>,
    /// The number of skipped items when an offset is given.
    pub skipped: u16,
}

/// Represents the result of executing a contested document vote poll drive query.
///
/// This struct holds the list of contenders and the number of skipped items
/// when an offset is given.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct FinalizedContestedDocumentVotePollDriveQueryExecutionResult {
    /// The list of contenders returned by the query.
    pub contenders: Vec<FinalizedContenderWithSerializedDocument>,
    /// Locked tally
    pub locked_vote_tally: u32,
    /// Abstaining tally
    pub abstaining_vote_tally: u32,
}

impl TryFrom<ContestedDocumentVotePollDriveQueryExecutionResult>
    for FinalizedContestedDocumentVotePollDriveQueryExecutionResult
{
    type Error = Error;

    fn try_from(
        value: ContestedDocumentVotePollDriveQueryExecutionResult,
    ) -> Result<Self, Self::Error> {
        let ContestedDocumentVotePollDriveQueryExecutionResult {
            contenders,
            locked_vote_tally,
            abstaining_vote_tally,
            ..
        } = value;

        let finalized_contenders = contenders
            .into_iter()
            .map(|contender| {
                let finalized: FinalizedContenderWithSerializedDocument = contender.try_into()?;
                Ok(finalized)
            })
            .collect::<Result<Vec<_>, Error>>()?;

        Ok(
            FinalizedContestedDocumentVotePollDriveQueryExecutionResult {
                contenders: finalized_contenders,
                locked_vote_tally: locked_vote_tally.ok_or(Error::Drive(
                    DriveError::CorruptedCodeExecution("expected a locked tally"),
                ))?,
                abstaining_vote_tally: abstaining_vote_tally.ok_or(Error::Drive(
                    DriveError::CorruptedCodeExecution("expected an abstaining tally"),
                ))?,
            },
        )
    }
}

impl ContestedDocumentVotePollDriveQuery {
    #[cfg(feature = "server")]
    /// Resolves the contested document vote poll drive query.
    ///
    /// This method processes the query by interacting with the drive, using the provided
    /// transaction and platform version to ensure consistency and compatibility.
    ///
    /// # Parameters
    ///
    /// * `drive`: A reference to the `Drive` object used for database interactions.
    /// * `transaction`: The transaction argument used to ensure consistency during the resolve operation.
    /// * `platform_version`: The platform version to ensure compatibility.
    ///
    /// # Returns
    ///
    /// * `Ok(ResolvedContestedDocumentVotePollDriveQuery)` - The resolved query information.
    /// * `Err(Error)` - An error if the resolution process fails.
    ///
    /// # Errors
    ///
    /// This method returns an `Error` variant if there is an issue resolving the query.
    /// The specific error depends on the underlying problem encountered during resolution.
    pub fn resolve(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ResolvedContestedDocumentVotePollDriveQuery, Error> {
        let ContestedDocumentVotePollDriveQuery {
            vote_poll,
            result_type,
            offset,
            limit,
            start_at,
            order_ascending,
            allow_include_locked_and_abstaining_vote_tally,
        } = self;
        Ok(ResolvedContestedDocumentVotePollDriveQuery {
            vote_poll: vote_poll.resolve_allow_borrowed(drive, transaction, platform_version)?,
            result_type: *result_type,
            offset: *offset,
            limit: *limit,
            start_at: *start_at,
            order_ascending: *order_ascending,
            allow_include_locked_and_abstaining_vote_tally:
                *allow_include_locked_and_abstaining_vote_tally,
        })
    }

    #[cfg(feature = "verify")]
    /// Resolves with a known contract provider
    pub fn resolve_with_known_contracts_provider<'a>(
        &self,
        known_contracts_provider_fn: &impl Fn(&Identifier) -> Result<Option<Arc<DataContract>>, Error>,
    ) -> Result<ResolvedContestedDocumentVotePollDriveQuery<'a>, Error> {
        let ContestedDocumentVotePollDriveQuery {
            vote_poll,
            result_type,
            offset,
            limit,
            start_at,
            order_ascending,
            allow_include_locked_and_abstaining_vote_tally,
        } = self;
        Ok(ResolvedContestedDocumentVotePollDriveQuery {
            vote_poll: vote_poll
                .resolve_with_known_contracts_provider(known_contracts_provider_fn)?,
            result_type: *result_type,
            offset: *offset,
            limit: *limit,
            start_at: *start_at,
            order_ascending: *order_ascending,
            allow_include_locked_and_abstaining_vote_tally:
                *allow_include_locked_and_abstaining_vote_tally,
        })
    }

    #[cfg(any(feature = "verify", feature = "server"))]
    /// Resolves with a provided borrowed contract
    pub fn resolve_with_provided_borrowed_contract<'a>(
        &self,
        data_contract: &'a DataContract,
    ) -> Result<ResolvedContestedDocumentVotePollDriveQuery<'a>, Error> {
        let ContestedDocumentVotePollDriveQuery {
            vote_poll,
            result_type,
            offset,
            limit,
            start_at,
            order_ascending,
            allow_include_locked_and_abstaining_vote_tally,
        } = self;
        Ok(ResolvedContestedDocumentVotePollDriveQuery {
            vote_poll: vote_poll.resolve_with_provided_borrowed_contract(data_contract)?,
            result_type: *result_type,
            offset: *offset,
            limit: *limit,
            start_at: *start_at,
            order_ascending: *order_ascending,
            allow_include_locked_and_abstaining_vote_tally:
                *allow_include_locked_and_abstaining_vote_tally,
        })
    }

    #[cfg(feature = "server")]
    /// Executes a query with proof and returns the items and fee.
    pub fn execute_with_proof(
        self,
        drive: &Drive,
        block_info: Option<BlockInfo>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<u8>, u64), Error> {
        let mut drive_operations = vec![];
        let items = self.execute_with_proof_internal(
            drive,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let cost = if let Some(block_info) = block_info {
            let fee_result = Drive::calculate_fee(
                None,
                Some(drive_operations),
                &block_info.epoch,
                drive.config.epochs_per_era,
                platform_version,
            )?;
            fee_result.processing_fee
        } else {
            0
        };
        Ok((items, cost))
    }

    #[cfg(feature = "server")]
    /// Executes an internal query with proof and returns the items.
    pub(crate) fn execute_with_proof_internal(
        self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let resolved = self.resolve(drive, transaction, platform_version)?;
        let path_query = resolved.construct_path_query(platform_version)?;
        drive.grove_get_proved_path_query(
            &path_query,
            false,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }
    //
    // #[cfg(all(feature = "server", feature = "verify"))]
    // /// Executes a query with proof and returns the root hash, items, and fee.
    // pub fn execute_with_proof_only_get_elements(
    //     self,
    //     drive: &Drive,
    //     block_info: Option<BlockInfo>,
    //     transaction: TransactionArg,
    //     platform_version: &PlatformVersion,
    // ) -> Result<(RootHash, Vec<Vec<u8>>, u64), Error> {
    //     let mut drive_operations = vec![];
    //     let (root_hash, items) = self.execute_with_proof_only_get_elements_internal(
    //         drive,
    //         transaction,
    //         &mut drive_operations,
    //         platform_version,
    //     )?;
    //     let cost = if let Some(block_info) = block_info {
    //         let fee_result = Drive::calculate_fee(
    //             None,
    //             Some(drive_operations),
    //             &block_info.epoch,
    //             drive.config.epochs_per_era,
    //             platform_version,
    //         )?;
    //         fee_result.processing_fee
    //     } else {
    //         0
    //     };
    //     Ok((root_hash, items, cost))
    // }
    //
    // #[cfg(all(feature = "server", feature = "verify"))]
    // /// Executes an internal query with proof and returns the root hash and values.
    // pub(crate) fn execute_with_proof_only_get_elements_internal(
    //     self,
    //     drive: &Drive,
    //     transaction: TransactionArg,
    //     drive_operations: &mut Vec<LowLevelDriveOperation>,
    //     platform_version: &PlatformVersion,
    // ) -> Result<(RootHash, Vec<Vec<u8>>), Error> {
    //     let resolved = self.resolve(drive, transaction, platform_version)?;
    //     let path_query = resolved.construct_path_query(platform_version)?;
    //     let proof = drive.grove_get_proved_path_query(
    //         &path_query,
    //         self.start_at.is_some(),
    //         transaction,
    //         drive_operations,
    //         &platform_version.drive,
    //     )?;
    //     self.verify_proof_keep_serialized(proof.as_slice(), platform_version)
    // }

    #[cfg(feature = "server")]
    /// Executes a query with no proof and returns the items, skipped items, and fee.
    pub fn execute_no_proof_with_cost(
        &self,
        drive: &Drive,
        block_info: Option<BlockInfo>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(ContestedDocumentVotePollDriveQueryExecutionResult, u64), Error> {
        let mut drive_operations = vec![];
        let result =
            self.execute_no_proof(drive, transaction, &mut drive_operations, platform_version)?;
        let cost = if let Some(block_info) = block_info {
            let fee_result = Drive::calculate_fee(
                None,
                Some(drive_operations),
                &block_info.epoch,
                drive.config.epochs_per_era,
                platform_version,
            )?;
            fee_result.processing_fee
        } else {
            0
        };
        Ok((result, cost))
    }

    #[cfg(feature = "server")]
    /// Executes an internal query with no proof and returns the values and skipped items.
    pub fn execute_no_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<ContestedDocumentVotePollDriveQueryExecutionResult, Error> {
        let resolved = self.resolve(drive, transaction, platform_version)?;
        let path_query = resolved.construct_path_query(platform_version)?;
        let order_ascending = resolved.order_ascending;
        let query_result = drive.grove_get_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryPathKeyElementTrioResultType,
            drive_operations,
            &platform_version.drive,
        );
        match query_result {
            Err(Error::GroveDB(GroveError::PathKeyNotFound(_)))
            | Err(Error::GroveDB(GroveError::PathNotFound(_)))
            | Err(Error::GroveDB(GroveError::PathParentLayerNotFound(_))) => {
                Ok(ContestedDocumentVotePollDriveQueryExecutionResult::default())
            }
            Err(e) => Err(e),
            Ok((query_result_elements, skipped)) => {
                match self.result_type {
                    ContestedDocumentVotePollDriveQueryResultType::IdentityIdsOnly => {
                        // with identities only we don't need to work about lock and abstaining tree
                        let contenders = query_result_elements
                            .to_keys()
                            .into_iter()
                            .map(|identity_id| {
                                Ok(ContenderWithSerializedDocument {
                                    identity_id: Identifier::try_from(identity_id)?,
                                    serialized_document: None,
                                    vote_tally: None,
                                })
                            })
                            .collect::<Result<Vec<ContenderWithSerializedDocument>, Error>>()?;

                        Ok(ContestedDocumentVotePollDriveQueryExecutionResult {
                            contenders,
                            locked_vote_tally: None,
                            abstaining_vote_tally: None,
                            winner: None,
                            skipped,
                        })
                    }
                    ContestedDocumentVotePollDriveQueryResultType::Documents => {
                        // with documents only we don't need to work about lock and abstaining tree
                        let contenders = query_result_elements
                            .to_path_key_elements()
                            .into_iter()
                            .map(|(mut path, _, document)| {
                                let identity_id = path.pop().ok_or(Error::Drive(
                                    DriveError::CorruptedDriveState(
                                        "the path must have a last element".to_string(),
                                    ),
                                ))?;
                                Ok(ContenderWithSerializedDocument {
                                    identity_id: Identifier::try_from(identity_id)?,
                                    serialized_document: Some(document.into_item_bytes()?),
                                    vote_tally: None,
                                })
                            })
                            .collect::<Result<Vec<ContenderWithSerializedDocument>, Error>>()?;

                        Ok(ContestedDocumentVotePollDriveQueryExecutionResult {
                            contenders,
                            locked_vote_tally: None,
                            abstaining_vote_tally: None,
                            winner: None,
                            skipped,
                        })
                    }
                    ContestedDocumentVotePollDriveQueryResultType::VoteTally => {
                        let mut contenders = Vec::new();
                        let mut locked_vote_tally: Option<u32> = None;
                        let mut abstaining_vote_tally: Option<u32> = None;
                        let mut winner = None;

                        for (key, vote_tally) in query_result_elements.to_key_elements().into_iter()
                        {
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
                                    let finalized_contested_document_vote_poll_stored_info = ContestedDocumentVotePollStoredInfo::deserialize_from_bytes(&vote_tally.into_item_bytes()?)?;
                                    if finalized_contested_document_vote_poll_stored_info
                                        .vote_poll_status()
                                        .awarded_or_locked()
                                    {
                                        locked_vote_tally = Some(
                                            finalized_contested_document_vote_poll_stored_info
                                                .last_locked_votes()
                                                .ok_or(Error::Drive(
                                                    DriveError::CorruptedDriveState(
                                                        "we should have last locked votes"
                                                            .to_string(),
                                                    ),
                                                ))?,
                                        );
                                        abstaining_vote_tally = Some(
                                            finalized_contested_document_vote_poll_stored_info
                                                .last_abstain_votes()
                                                .ok_or(Error::Drive(
                                                    DriveError::CorruptedDriveState(
                                                        "we should have last abstain votes"
                                                            .to_string(),
                                                    ),
                                                ))?,
                                        );
                                        winner = Some((
                                            finalized_contested_document_vote_poll_stored_info
                                                .winner(),
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
                        Ok(ContestedDocumentVotePollDriveQueryExecutionResult {
                            contenders,
                            locked_vote_tally,
                            abstaining_vote_tally,
                            winner,
                            skipped,
                        })
                    }
                    ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally => {
                        let mut elements_iter =
                            query_result_elements.to_path_key_elements().into_iter();
                        let mut contenders = vec![];
                        let mut locked_vote_tally: Option<u32> = None;
                        let mut abstaining_vote_tally: Option<u32> = None;
                        let mut winner = None;

                        if order_ascending {
                            // Handle ascending order
                            while let Some((path, _, element)) = elements_iter.next() {
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
                                            Some(key)
                                                if key == &RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8 =>
                                            {
                                                abstaining_vote_tally = Some(sum_tree_value as u32);
                                            }
                                            _ => {
                                                return Err(Error::Drive(
                                                    DriveError::CorruptedDriveState(
                                                        "unexpected key for sum tree value"
                                                            .to_string(),
                                                    ),
                                                ));
                                            }
                                        }
                                    }
                                    Element::Item(serialized_item_info, _) => {
                                        // We should find a sum tree paired with this document
                                        if let Some((
                                            path_tally,
                                            _,
                                            Element::SumTree(_, sum_tree_value, _),
                                        )) = elements_iter.next()
                                        {
                                            if path != path_tally {
                                                return Err(Error::Drive(DriveError::CorruptedDriveState("the two results in a chunk when requesting documents and vote tally should both have the same path".to_string())));
                                            }

                                            if sum_tree_value < 0
                                                || sum_tree_value > u32::MAX as i64
                                            {
                                                return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                                                    "sum tree value for vote tally must be between 0 and u32::Max, received {} from state",
                                                    sum_tree_value
                                                ))));
                                            }

                                            let identity_id =
                                                Identifier::from_bytes(identity_bytes)?;
                                            let contender = ContenderWithSerializedDocument {
                                                identity_id,
                                                serialized_document: Some(serialized_item_info),
                                                vote_tally: Some(sum_tree_value as u32),
                                            };
                                            contenders.push(contender);
                                        } else {
                                            // The vote is actually over, let's deserialize the info
                                            let finalized_contested_document_vote_poll_stored_info = ContestedDocumentVotePollStoredInfo::deserialize_from_bytes(&serialized_item_info)?;
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
                                                abstaining_vote_tally =
                                                    Some(finalized_contested_document_vote_poll_stored_info
                                                        .last_abstain_votes().ok_or(Error::Drive(DriveError::CorruptedDriveState(
                                                        "we should have last abstain votes".to_string(),
                                                    )))?);
                                                winner = Some((
                                                    finalized_contested_document_vote_poll_stored_info.winner(),
                                                    finalized_contested_document_vote_poll_stored_info
                                                        .last_finalization_block().ok_or(Error::Drive(DriveError::CorruptedDriveState(
                                                        "we should have a last finalization block".to_string(),
                                                    )))?,
                                                ));
                                                contenders = finalized_contested_document_vote_poll_stored_info
                                                    .contender_votes_in_vec_of_contender_with_serialized_document().ok_or(Error::Drive(DriveError::CorruptedDriveState(
                                                    "we should have a last contender votes".to_string(),
                                                )))?;
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
                        } else {
                            // Handle descending order
                            let mut prev_element: Option<(Vec<Vec<u8>>, i64, Element)> = None;

                            while let Some((path, _, element)) = elements_iter.next() {
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
                                            Some(key)
                                                if key == &RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8 =>
                                            {
                                                abstaining_vote_tally = Some(sum_tree_value as u32);
                                            }
                                            _ => {
                                                prev_element =
                                                    Some((path.clone(), sum_tree_value, element));
                                            }
                                        }
                                    }
                                    Element::Item(serialized_item_info, _) => {
                                        if let Some((
                                            prev_path,
                                            sum_tree_value,
                                            Element::SumTree(_, _, _),
                                        )) = prev_element.take()
                                        {
                                            if prev_path != path {
                                                return Err(Error::Drive(DriveError::CorruptedDriveState("the two results in a chunk when requesting documents and vote tally should both have the same path".to_string())));
                                            }

                                            let identity_id =
                                                Identifier::from_bytes(identity_bytes)?;
                                            let contender = ContenderWithSerializedDocument {
                                                identity_id,
                                                serialized_document: Some(serialized_item_info),
                                                vote_tally: Some(sum_tree_value as u32),
                                            };
                                            contenders.push(contender);
                                        } else {
                                            // The vote is actually over, let's deserialize the info
                                            let finalized_contested_document_vote_poll_stored_info = ContestedDocumentVotePollStoredInfo::deserialize_from_bytes(&serialized_item_info)?;
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
                                                abstaining_vote_tally =
                                                    Some(finalized_contested_document_vote_poll_stored_info
                                                        .last_abstain_votes().ok_or(Error::Drive(DriveError::CorruptedDriveState(
                                                        "we should have last abstain votes".to_string(),
                                                    )))?);
                                                winner = Some((
                                                    finalized_contested_document_vote_poll_stored_info.winner(),
                                                    finalized_contested_document_vote_poll_stored_info
                                                        .last_finalization_block().ok_or(Error::Drive(DriveError::CorruptedDriveState(
                                                        "we should have a last finalization block".to_string(),
                                                    )))?,
                                                ));
                                                contenders = finalized_contested_document_vote_poll_stored_info
                                                    .contender_votes_in_vec_of_contender_with_serialized_document().ok_or(Error::Drive(DriveError::CorruptedDriveState(
                                                    "we should have a last contender votes".to_string(),
                                                )))?;
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
                        }

                        Ok(ContestedDocumentVotePollDriveQueryExecutionResult {
                            contenders,
                            locked_vote_tally,
                            abstaining_vote_tally,
                            winner,
                            skipped,
                        })
                    }
                }
            }
        }
    }

    #[cfg(feature = "server")]
    #[allow(unused)]
    /// Executes an internal query with no proof and returns the values and skipped items.
    pub(crate) fn execute_no_proof_internal(
        &self,
        drive: &Drive,
        result_type: QueryResultType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(QueryResultElements, u16), Error> {
        let resolved = self.resolve(drive, transaction, platform_version)?;
        let path_query = resolved.construct_path_query(platform_version)?;
        let query_result = drive.grove_get_path_query(
            &path_query,
            transaction,
            result_type,
            drive_operations,
            &platform_version.drive,
        );
        match query_result {
            Err(Error::GroveDB(GroveError::PathKeyNotFound(_)))
            | Err(Error::GroveDB(GroveError::PathNotFound(_)))
            | Err(Error::GroveDB(GroveError::PathParentLayerNotFound(_))) => {
                Ok((QueryResultElements::new(), 0))
            }
            _ => {
                let (data, skipped) = query_result?;
                {
                    Ok((data, skipped))
                }
            }
        }
    }
}

/// Vote Poll Drive Query struct
#[derive(Debug, PartialEq, Clone)]
pub struct ResolvedContestedDocumentVotePollDriveQuery<'a> {
    /// What vote poll are we asking for?
    pub vote_poll: ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed<'a>,
    /// What result type are we interested in
    pub result_type: ContestedDocumentVotePollDriveQueryResultType,
    /// Offset
    pub offset: Option<u16>,
    /// Limit
    pub limit: Option<u16>,
    /// Start at identity id, the bool is if it is also included
    pub start_at: Option<([u8; 32], bool)>,
    /// Ascending
    pub order_ascending: bool,
    /// Include locked and abstaining vote tally
    pub allow_include_locked_and_abstaining_vote_tally: bool,
}

impl<'a> ResolvedContestedDocumentVotePollDriveQuery<'a> {
    /// Operations to construct a path query.
    pub fn construct_path_query(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        let path = self.vote_poll.contenders_path(platform_version)?;

        let mut query = Query::new_with_direction(self.order_ascending);

        let allow_include_locked_and_abstaining_vote_tally = self
            .allow_include_locked_and_abstaining_vote_tally
            && self.result_type.has_vote_tally();

        // this is a range on all elements
        match &self.start_at {
            None => {
                if allow_include_locked_and_abstaining_vote_tally {
                    query.insert_all()
                } else {
                    query.insert_range_after(vec![RESOURCE_LOCK_VOTE_TREE_KEY_U8]..)
                }
            }
            Some((starts_at_key_bytes, start_at_included)) => {
                let starts_at_key = starts_at_key_bytes.to_vec();
                match self.order_ascending {
                    true => match start_at_included {
                        true => query.insert_range_from(starts_at_key..),
                        false => query.insert_range_after(starts_at_key..),
                    },
                    false => match start_at_included {
                        true => {
                            if allow_include_locked_and_abstaining_vote_tally {
                                query.insert_range_to_inclusive(..=starts_at_key)
                            } else {
                                query.insert_range_after_to_inclusive(
                                    vec![RESOURCE_LOCK_VOTE_TREE_KEY_U8]..=starts_at_key,
                                )
                            }
                        }
                        false => {
                            if allow_include_locked_and_abstaining_vote_tally {
                                query.insert_range_to(..starts_at_key)
                            } else {
                                query.insert_range_after_to(
                                    vec![RESOURCE_LOCK_VOTE_TREE_KEY_U8]..starts_at_key,
                                )
                            }
                        }
                    },
                }
            }
        }

        let (subquery_path, subquery) = match self.result_type {
            ContestedDocumentVotePollDriveQueryResultType::Documents => (Some(vec![vec![0]]), None),
            ContestedDocumentVotePollDriveQueryResultType::VoteTally => (Some(vec![vec![1]]), None),
            ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally => {
                let mut query = Query::new();
                query.insert_keys(vec![vec![0], vec![1]]);
                (None, Some(query.into()))
            }
            ContestedDocumentVotePollDriveQueryResultType::IdentityIdsOnly => (None, None),
        };

        query.default_subquery_branch.subquery_path = subquery_path;
        query.default_subquery_branch.subquery = subquery;

        if allow_include_locked_and_abstaining_vote_tally {
            query.add_conditional_subquery(
                QueryItem::Key(vec![RESOURCE_LOCK_VOTE_TREE_KEY_U8]),
                Some(vec![vec![1]]),
                None,
            );
            query.add_conditional_subquery(
                QueryItem::Key(vec![RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8]),
                Some(vec![vec![1]]),
                None,
            );
        }

        Ok(PathQuery {
            path,
            query: SizedQuery {
                query,
                limit: self.limit,
                offset: self.offset,
            },
        })
    }
}
