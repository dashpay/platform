use crate::drive::votes::paths::vote_end_date_queries_tree_path_vec;
use crate::drive::Drive;
#[cfg(feature = "server")]
use crate::error::drive::DriveError;
use crate::error::Error;
#[cfg(feature = "server")]
use crate::fees::op::LowLevelDriveOperation;
#[cfg(feature = "server")]
use crate::query::GroveError;
use crate::query::Query;
use crate::util::common::encode::{decode_u64, encode_u64};
#[cfg(feature = "server")]
use dpp::block::block_info::BlockInfo;
#[cfg(feature = "server")]
use dpp::fee::Credits;
use dpp::prelude::{TimestampIncluded, TimestampMillis};
#[cfg(feature = "server")]
use dpp::serialization::PlatformDeserializable;
#[cfg(feature = "server")]
use dpp::voting::vote_polls::VotePoll;
#[cfg(feature = "server")]
use grovedb::query_result_type::{QueryResultElements, QueryResultType};
#[cfg(feature = "server")]
use grovedb::TransactionArg;
use grovedb::{PathQuery, SizedQuery};
#[cfg(feature = "server")]
use platform_version::version::PlatformVersion;
#[cfg(feature = "server")]
use std::collections::BTreeMap;

/// Vote Poll Drive Query struct
#[derive(Debug, PartialEq, Clone)]
pub struct VotePollsByEndDateDriveQuery {
    /// What is the start time we are asking for
    pub start_time: Option<(TimestampMillis, TimestampIncluded)>,
    /// What vote poll are we asking for?
    pub end_time: Option<(TimestampMillis, TimestampIncluded)>,
    /// Limit
    pub limit: Option<u16>,
    /// Offset
    pub offset: Option<u16>,
    /// Ascending
    pub order_ascending: bool,
}

impl VotePollsByEndDateDriveQuery {
    /// Get the path query for an abci query that gets vote polls until an end time
    pub fn path_query_for_end_time_included(end_time: TimestampMillis, limit: u16) -> PathQuery {
        let path = vote_end_date_queries_tree_path_vec();

        let mut query = Query::new_with_direction(true);

        let encoded_time = encode_u64(end_time);

        query.insert_range_to_inclusive(..=encoded_time);

        let mut sub_query = Query::new();

        sub_query.insert_all();

        query.default_subquery_branch.subquery = Some(sub_query.into());

        PathQuery {
            path,
            query: SizedQuery {
                query,
                limit: Some(limit),
                offset: None,
            },
        }
    }

    /// Get the path query for an abci query that gets vote polls at an the end time
    pub fn path_query_for_single_end_time(end_time: TimestampMillis, limit: u16) -> PathQuery {
        let path = vote_end_date_queries_tree_path_vec();

        let mut query = Query::new_with_direction(true);

        let encoded_time = encode_u64(end_time);

        query.insert_key(encoded_time);

        let mut sub_query = Query::new();

        sub_query.insert_all();

        query.default_subquery_branch.subquery = Some(sub_query.into());

        PathQuery {
            path,
            query: SizedQuery {
                query,
                limit: Some(limit),
                offset: None,
            },
        }
    }

    #[cfg(feature = "server")]
    /// Executes a special query with no proof to get contested document resource vote polls.
    /// This is meant for platform abci to get votes that have finished
    pub fn execute_no_proof_for_specialized_end_time_query(
        end_time: TimestampMillis,
        limit: u16,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<TimestampMillis, Vec<VotePoll>>, Error> {
        let path_query = Self::path_query_for_end_time_included(end_time, limit);
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
            | Err(Error::GroveDB(GroveError::PathParentLayerNotFound(_))) => Ok(BTreeMap::new()),
            Err(e) => Err(e),
            Ok((query_result_elements, _)) => {
                let vote_polls_by_end_date = query_result_elements
                    .to_path_key_elements()
                    .into_iter()
                    .map(|(path, _, element)| {
                        let Some(last_path_component) = path.last() else {
                            return Err(Error::Drive(DriveError::CorruptedDriveState(
                                "we should always have a path not be null".to_string(),
                            )));
                        };
                        let timestamp = decode_u64(last_path_component).map_err(Error::from)?;
                        let contested_document_resource_vote_poll_bytes =
                            element.into_item_bytes().map_err(Error::from)?;
                        let vote_poll = VotePoll::deserialize_from_bytes(
                            &contested_document_resource_vote_poll_bytes,
                        )?;
                        Ok((timestamp, vote_poll))
                    })
                    .collect::<Result<Vec<_>, Error>>()?
                    .into_iter()
                    .fold(
                        BTreeMap::new(),
                        |mut acc: BTreeMap<u64, Vec<VotePoll>>, (timestamp, vote_poll)| {
                            acc.entry(timestamp).or_default().push(vote_poll);
                            acc
                        },
                    );
                Ok(vote_polls_by_end_date)
            }
        }
    }

    #[cfg(feature = "server")]
    /// Executes a special query with no proof to get contested document resource vote polls.
    /// This is meant for platform abci to get votes that have finished
    pub fn execute_no_proof_for_specialized_end_time_query_only_check_end_time(
        end_time: TimestampMillis,
        limit: u16,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<VotePoll>, Error> {
        let path_query = Self::path_query_for_single_end_time(end_time, limit);
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
            | Err(Error::GroveDB(GroveError::PathParentLayerNotFound(_))) => Ok(vec![]),
            Err(e) => Err(e),
            Ok((query_result_elements, _)) => {
                // Process the query result elements and collect VotePolls
                let vote_polls = query_result_elements
                    .to_path_key_elements()
                    .into_iter()
                    .map(|(_, _, element)| {
                        // Extract the bytes from the element
                        let vote_poll_bytes = element.into_item_bytes().map_err(Error::from)?;
                        // Deserialize the bytes into a VotePoll
                        let vote_poll = VotePoll::deserialize_from_bytes(&vote_poll_bytes)?;
                        Ok(vote_poll)
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                Ok(vote_polls)
            }
        }
    }

    /// Operations to construct a path query.
    pub fn construct_path_query(&self) -> PathQuery {
        let path = vote_end_date_queries_tree_path_vec();

        let mut query = Query::new_with_direction(self.order_ascending);

        // this is a range on all elements
        match &(self.start_time, self.end_time) {
            (None, None) => {
                query.insert_all();
            }
            (Some((starts_at_key_bytes, start_at_included)), None) => {
                let starts_at_key = encode_u64(*starts_at_key_bytes);
                match start_at_included {
                    true => query.insert_range_from(starts_at_key..),
                    false => query.insert_range_after(starts_at_key..),
                }
            }
            (None, Some((ends_at_key_bytes, ends_at_included))) => {
                let ends_at_key = encode_u64(*ends_at_key_bytes);
                match ends_at_included {
                    true => query.insert_range_to_inclusive(..=ends_at_key),
                    false => query.insert_range_to(..ends_at_key),
                }
            }
            (
                Some((starts_at_key_bytes, start_at_included)),
                Some((ends_at_key_bytes, ends_at_included)),
            ) => {
                let starts_at_key = encode_u64(*starts_at_key_bytes);
                let ends_at_key = encode_u64(*ends_at_key_bytes);
                match (start_at_included, ends_at_included) {
                    (true, true) => query.insert_range_inclusive(starts_at_key..=ends_at_key),
                    (true, false) => query.insert_range(starts_at_key..ends_at_key),
                    (false, true) => {
                        query.insert_range_after_to_inclusive(starts_at_key..=ends_at_key)
                    }
                    (false, false) => query.insert_range_after_to(starts_at_key..ends_at_key),
                }
            }
        }

        let mut sub_query = Query::new();

        sub_query.insert_all();

        query.default_subquery_branch.subquery = Some(sub_query.into());

        PathQuery {
            path,
            query: SizedQuery {
                query,
                limit: self.limit,
                offset: None,
            },
        }
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
                None,
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
        let path_query = self.construct_path_query();
        drive.grove_get_proved_path_query(
            &path_query,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }
    #[cfg(feature = "server")]
    /// Executes a query with no proof and returns the items, skipped items, and fee.
    pub fn execute_no_proof_with_cost(
        &self,
        drive: &Drive,
        block_info: Option<BlockInfo>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(BTreeMap<TimestampMillis, Vec<VotePoll>>, Credits), Error> {
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
                None,
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
    ) -> Result<BTreeMap<TimestampMillis, Vec<VotePoll>>, Error> {
        let path_query = self.construct_path_query();
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
            | Err(Error::GroveDB(GroveError::PathParentLayerNotFound(_))) => Ok(BTreeMap::new()),
            Err(e) => Err(e),
            Ok((query_result_elements, _)) => {
                let vote_polls_by_end_date = query_result_elements
                    .to_path_key_elements()
                    .into_iter()
                    .map(|(path, _, element)| {
                        let Some(last_path_component) = path.last() else {
                            return Err(Error::Drive(DriveError::CorruptedDriveState(
                                "we should always have a path not be null".to_string(),
                            )));
                        };
                        let timestamp = decode_u64(last_path_component).map_err(Error::from)?;
                        let contested_document_resource_vote_poll_bytes =
                            element.into_item_bytes().map_err(Error::from)?;
                        let vote_poll = VotePoll::deserialize_from_bytes(
                            &contested_document_resource_vote_poll_bytes,
                        )?;
                        Ok((timestamp, vote_poll))
                    })
                    .collect::<Result<Vec<_>, Error>>()?
                    .into_iter()
                    .fold(
                        BTreeMap::new(),
                        |mut acc: BTreeMap<u64, Vec<VotePoll>>, (timestamp, vote_poll)| {
                            acc.entry(timestamp).or_default().push(vote_poll);
                            acc
                        },
                    );
                Ok(vote_polls_by_end_date)
            }
        }
    }

    #[cfg(feature = "server")]
    /// Executes an internal query with no proof and returns the values and skipped items.
    pub fn execute_no_proof_keep_serialized(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<TimestampMillis, Vec<Vec<u8>>>, Error> {
        let path_query = self.construct_path_query();
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
            | Err(Error::GroveDB(GroveError::PathParentLayerNotFound(_))) => Ok(BTreeMap::new()),
            Err(e) => Err(e),
            Ok((query_result_elements, _)) => {
                let vote_polls_by_end_date = query_result_elements
                    .to_path_key_elements()
                    .into_iter()
                    .map(|(path, _, element)| {
                        let Some(last_path_component) = path.last() else {
                            return Err(Error::Drive(DriveError::CorruptedDriveState(
                                "we should always have a path not be null".to_string(),
                            )));
                        };
                        let timestamp = decode_u64(last_path_component).map_err(Error::from)?;
                        let contested_document_resource_vote_poll_bytes =
                            element.into_item_bytes().map_err(Error::from)?;
                        Ok((timestamp, contested_document_resource_vote_poll_bytes))
                    })
                    .collect::<Result<Vec<_>, Error>>()?
                    .into_iter()
                    .fold(
                        BTreeMap::new(),
                        |mut acc: BTreeMap<u64, Vec<Vec<u8>>>,
                         (timestamp, vote_poll_serialized)| {
                            acc.entry(timestamp).or_default().push(vote_poll_serialized);
                            acc
                        },
                    );
                Ok(vote_polls_by_end_date)
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
    ) -> Result<QueryResultElements, Error> {
        let path_query = self.construct_path_query();
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
                Ok(QueryResultElements::new())
            }
            _ => {
                let (data, _) = query_result?;
                {
                    Ok(data)
                }
            }
        }
    }
}
