use crate::drive::votes::paths::VotePollPaths;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use crate::query::{GroveError, Query};
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use grovedb::query_result_type::{QueryResultElements, QueryResultType};
use grovedb::{Element, PathQuery, SizedQuery, TransactionArg};
use platform_version::version::PlatformVersion;
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed;
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::resolve::ContestedDocumentResourceVotePollResolver;

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
}

/// Represents a contender in the contested document vote poll.
///
/// This struct holds the identity ID of the contender, the serialized document,
/// and the vote tally.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct Contender {
    /// The identity ID of the contender.
    pub identity_id: Identifier,
    /// The serialized document associated with the contender.
    pub serialized_document: Option<Vec<u8>>,
    /// The vote tally for the contender.
    pub vote_tally: Option<u32>,
}

/// Represents the result of executing a contested document vote poll drive query.
///
/// This struct holds the list of contenders and the number of skipped items
/// when an offset is given.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct ContestedDocumentVotePollDriveQueryExecutionResult {
    /// The list of contenders returned by the query.
    pub contenders: Vec<Contender>,
    /// The number of skipped items when an offset is given.
    pub skipped: u16,
}

impl ContestedDocumentVotePollDriveQuery {
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
        } = self;
        Ok(ResolvedContestedDocumentVotePollDriveQuery {
            vote_poll: vote_poll.resolve_allow_borrowed(drive, transaction, platform_version)?,
            result_type: *result_type,
            offset: *offset,
            limit: *limit,
            start_at: *start_at,
            order_ascending: *order_ascending,
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
                let contenders = match self.result_type {
                    ContestedDocumentVotePollDriveQueryResultType::IdentityIdsOnly => {
                        query_result_elements.to_keys().into_iter().map(|identity_id| Ok(Contender {
                            identity_id: Identifier::try_from(identity_id)?,
                            serialized_document: None,
                            vote_tally: None,
                        })).collect::<Result<Vec<Contender>, Error>>()
                    }
                    ContestedDocumentVotePollDriveQueryResultType::Documents => {
                        query_result_elements.to_key_elements().into_iter().map(|(identity_id, document)| {
                            Ok(Contender {
                                identity_id: Identifier::try_from(identity_id)?,
                                serialized_document: Some(document.into_item_bytes()?),
                                vote_tally: None,
                            })
                        }).collect::<Result<Vec<Contender>, Error>>()
                    }
                    ContestedDocumentVotePollDriveQueryResultType::VoteTally => {
                        query_result_elements.to_key_elements().into_iter().map(|(identity_id, vote_tally)| {
                            let sum_tree_value = vote_tally.into_sum_tree_value()?;
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
                        let mut elements_iter = query_result_elements.to_path_key_elements().into_iter();

                        let mut contenders = vec![];
                        while let (Some((path_doc, _, Element::Item(serialized_document, _))), Some((path_tally, _, Element::SumTree(_, sum_tree_value, _)))) = (elements_iter.next(), elements_iter.next())  {
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

                Ok(ContestedDocumentVotePollDriveQueryExecutionResult {
                    contenders,
                    skipped,
                })
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
}

impl<'a> ResolvedContestedDocumentVotePollDriveQuery<'a> {
    /// Operations to construct a path query.
    pub fn construct_path_query(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        let path = self.vote_poll.contenders_path(platform_version)?;

        let mut query = Query::new_with_direction(self.order_ascending);

        // this is a range on all elements
        match &self.start_at {
            None => {
                query.insert_all();
            }
            Some((starts_at_key_bytes, start_at_included)) => {
                let starts_at_key = starts_at_key_bytes.to_vec();
                match self.order_ascending {
                    true => match start_at_included {
                        true => query.insert_range_from(starts_at_key..),
                        false => query.insert_range_after(starts_at_key..),
                    },
                    false => match start_at_included {
                        true => query.insert_range_to_inclusive(..=starts_at_key),
                        false => query.insert_range_to(..starts_at_key),
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
