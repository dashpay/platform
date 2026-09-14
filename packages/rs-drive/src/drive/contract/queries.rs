use crate::drive::contract::paths::{
    contract_keeping_history_root_path_vec, contract_root_path_vec, CONTRACT_VERSION_KEY,
};
use crate::drive::contract::{paths, MAX_CONTRACT_HISTORY_FETCH_LIMIT};
use crate::drive::{Drive, RootTree};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::{Query, QueryItem};
use crate::util::common::encode::encode_u64;
use grovedb::{PathQuery, SizedQuery};
use platform_version::version::PlatformVersion;
use std::collections::BTreeSet;
use std::ops::RangeFull;

impl Drive {
    /// Builds the query over the contracts root that selects contract ids in ascending
    /// order from an optional cursor.
    ///
    /// * `None` selects every contract (the first page).
    /// * `Some((id, true))` starts at `id` (inclusive, `startAt`).
    /// * `Some((id, false))` starts after `id` (exclusive, `startAfter`).
    pub(crate) fn contracts_range_query(start_at: Option<([u8; 32], bool)>) -> Query {
        let mut query = Query::new();
        match start_at {
            None => query.insert_item(QueryItem::RangeFull(RangeFull)),
            Some((start_at_id, true)) => {
                query.insert_item(QueryItem::RangeFrom(start_at_id.to_vec()..))
            }
            Some((start_at_id, false)) => {
                query.insert_item(QueryItem::RangeAfter(start_at_id.to_vec()..))
            }
        }
        query
    }

    /// Creates the path query that proves one page of the contract enumeration
    /// (`getDataContractsByRange`) together with the serialized contracts.
    ///
    /// The query ranges over the contract ids under the contracts root and descends the
    /// two-key subquery path `0 / 0`. A contract that does not keep history stores the
    /// serialized contract as an item at the first key; GroveDB proves and verifies that
    /// item as a result row without descending further. A contract that keeps history
    /// stores its history subtree at the first key and a reference to the latest revision
    /// at the second key; the proof dereferences it. One path query therefore proves a page
    /// regardless of how each contract is stored.
    ///
    /// Only for proving and verifying. The trusted read path must not use it: a non-proof
    /// path query opens the subtree at the extended path blindly and would skip the
    /// contracts stored as items. `fetch_contracts` reads with a single-level subquery and
    /// resolves the history-keeping contracts afterwards instead.
    ///
    /// # Arguments
    ///
    /// * `start_at` - Optional cursor, see [`Drive::contracts_range_query`].
    /// * `limit` - Maximum number of contracts in the page.
    pub fn fetch_contracts_by_range_query(
        start_at: Option<([u8; 32], bool)>,
        limit: u16,
    ) -> PathQuery {
        let mut query = Self::contracts_range_query(start_at);
        query.set_subquery_path(vec![vec![0], vec![0]]);
        PathQuery::new(
            vec![Into::<&[u8; 1]>::into(RootTree::DataContractDocuments).to_vec()],
            SizedQuery::new(query, Some(limit), None),
        )
    }

    /// Creates the path query for one page of contract ids (`getDataContractsByRange`
    /// with `ids_only`).
    ///
    /// It ranges over the contract ids under the contracts root without descending, so
    /// every result row is a contract subtree keyed by its id. Shared by the trusted read
    /// (`fetch_contract_ids`), the prover and the verifier.
    ///
    /// # Arguments
    ///
    /// * `start_at` - Optional cursor, see [`Drive::contracts_range_query`].
    /// * `limit` - Maximum number of contract ids in the page.
    pub fn fetch_contract_ids_by_range_query(
        start_at: Option<([u8; 32], bool)>,
        limit: u16,
    ) -> PathQuery {
        PathQuery::new(
            vec![Into::<&[u8; 1]>::into(RootTree::DataContractDocuments).to_vec()],
            SizedQuery::new(Self::contracts_range_query(start_at), Some(limit), None),
        )
    }

    /// Creates the path query that proves the version items of the given contracts
    /// (`getDataContractsLatestVersions` without the contracts), from protocol version 14.
    ///
    /// It selects the requested contract ids under the contracts root and descends the
    /// subquery key `2`, the four-byte version item every contract carries beside its
    /// serialized form or history subtree. A requested id no contract has is proved absent.
    /// Duplicate ids are folded, so the limit is the number of distinct ids; the prover and
    /// the verifier reject more distinct ids than a query limit can hold, so the saturation
    /// here is never reached.
    ///
    /// Shared by the prover and the verifier, which must rebuild the exact query.
    ///
    /// # Arguments
    ///
    /// * `contract_ids` - The contract ids whose versions to prove, at least one.
    pub fn fetch_contracts_versions_query(contract_ids: &[[u8; 32]]) -> PathQuery {
        let distinct_ids: BTreeSet<&[u8; 32]> = contract_ids.iter().collect();
        let mut query = Query::new();
        query.insert_keys(distinct_ids.iter().map(|key| key.to_vec()).collect());
        query.set_subquery_key(vec![CONTRACT_VERSION_KEY]);
        PathQuery::new(
            vec![Into::<&[u8; 1]>::into(RootTree::DataContractDocuments).to_vec()],
            SizedQuery::new(
                query,
                Some(u16::try_from(distinct_ids.len()).unwrap_or(u16::MAX)),
                None,
            ),
        )
    }

    /// Creates a path query for a specified contract.
    ///
    /// This function takes a contract ID and creates a path query for fetching the contract data.
    ///
    /// Note it can only be used for simple queries that are not merged with other queries.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - A contract ID as a 32-byte array. The contract ID is used to
    ///   create the path query.
    /// * `with_limit` - Should be set to false when we are going to merge the path queries.
    ///
    /// # Returns
    ///
    /// * `PathQuery` - A `PathQuery` object representing the query for fetching the contract data.
    pub fn fetch_contract_query(contract_id: [u8; 32], with_limit: bool) -> PathQuery {
        let contract_path = contract_root_path_vec(contract_id.as_slice());
        let mut query = PathQuery::new_single_key(contract_path, vec![0]);

        if with_limit {
            // TODO: remove this limit once `verify_query_with_absence_proof` supports queries without limits
            query.query.limit = Some(1);
        }
        query
    }

    /// Creates a path query for a specified contract.
    ///
    /// This function takes a contract ID and creates a path query for fetching the contract data.
    ///
    /// Note it can only be used for simple queries that are not merged with other queries.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - A contract ID as a 32-byte array. The contract ID is used to
    ///   create the path query.
    /// * `with_limit` - Should be set to false when we are going to merge the path queries.
    ///
    /// # Returns
    ///
    /// * `PathQuery` - A `PathQuery` object representing the query for fetching the contract data.
    pub fn fetch_contract_with_history_latest_query(
        contract_id: [u8; 32],
        with_limit: bool,
    ) -> PathQuery {
        let contract_path = contract_keeping_history_root_path_vec(contract_id.as_slice());
        let mut query = PathQuery::new_single_key(contract_path, vec![0]);

        if with_limit {
            // TODO: remove this limit once `verify_query_with_absence_proof` supports queries without limits
            query.query.limit = Some(1);
        }

        query
    }

    /// Creates a merged path query for multiple contracts.
    ///
    /// This function takes a slice of contract IDs and creates a merged path query for fetching
    /// the data of all specified contracts.
    ///
    /// # Arguments
    ///
    /// * `contract_ids` - A slice of contract IDs as 32-byte arrays. The contract IDs are used to
    ///   create the path queries.
    ///
    /// # Returns
    ///
    /// * `Result<PathQuery, Error>` - If successful, returns a `PathQuery` object representing the
    ///   merged query for fetching the contracts' data. If an error occurs during the merging of
    ///   path queries, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the merging of path queries fails.
    pub fn fetch_non_historical_contracts_query(contract_ids: &[[u8; 32]]) -> PathQuery {
        let mut query = Query::new();
        query.insert_keys(contract_ids.iter().map(|key| key.to_vec()).collect());
        query.set_subquery_key(vec![0]);
        PathQuery::new(
            vec![Into::<&[u8; 1]>::into(RootTree::DataContractDocuments).to_vec()],
            SizedQuery::new(query, Some(contract_ids.len() as u16), None),
        )
    }

    /// Creates a merged path query for multiple contracts.
    ///
    /// This function takes a slice of contract IDs and creates a merged path query for fetching
    /// the data of all specified contracts.
    ///
    /// # Arguments
    ///
    /// * `non_historical_contract_ids` - A slice of contract IDs as 32-byte arrays. The contract IDs are used to
    ///   create the path queries.
    /// * `historical_contract_ids` - A slice of contract IDs as 32-byte arrays. The contract IDs are used to
    ///   create the path queries.
    ///
    /// # Returns
    ///
    /// * `Result<PathQuery, Error>` - If successful, returns a `PathQuery` object representing the
    ///   merged query for fetching the contracts' data. If an error occurs during the merging of
    ///   path queries, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the merging of path queries fails.
    pub fn fetch_contracts_query(
        non_historical_contract_ids: &[[u8; 32]],
        historical_contract_ids: &[[u8; 32]],
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        if non_historical_contract_ids.is_empty() {
            return Ok(Self::fetch_historical_contracts_query(
                historical_contract_ids,
            ));
        }
        if historical_contract_ids.is_empty() {
            return Ok(Self::fetch_non_historical_contracts_query(
                non_historical_contract_ids,
            ));
        }
        let mut contracts_query =
            Self::fetch_non_historical_contracts_query(non_historical_contract_ids);
        contracts_query.query.limit = None;
        let mut historical_contracts_query =
            Self::fetch_historical_contracts_query(historical_contract_ids);
        historical_contracts_query.query.limit = None;
        PathQuery::merge(
            vec![&contracts_query, &historical_contracts_query],
            &platform_version.drive.grove_version,
        )
        .map_err(Error::from)
    }

    /// Creates a merged path query for multiple historical contracts.
    ///
    /// This function takes a slice of contract IDs and creates a merged path query for fetching
    /// the data of all specified contracts.
    ///
    /// # Arguments
    ///
    /// * `historical_contract_ids` - A slice of contract IDs as 32-byte arrays. The contract IDs are used to
    ///   create the path queries.
    ///
    /// # Returns
    ///
    /// * `Result<PathQuery, Error>` - If successful, returns a `PathQuery` object representing the
    ///   merged query for fetching the contracts' data. If an error occurs during the merging of
    ///   path queries, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the merging of path queries fails.
    pub fn fetch_historical_contracts_query(historical_contract_ids: &[[u8; 32]]) -> PathQuery {
        let mut query = Query::new();
        query.insert_keys(
            historical_contract_ids
                .iter()
                .map(|key| key.to_vec())
                .collect(),
        );
        query.set_subquery_path(vec![vec![0], vec![0]]);
        PathQuery::new(
            vec![Into::<&[u8; 1]>::into(RootTree::DataContractDocuments).to_vec()],
            SizedQuery::new(query, Some(historical_contract_ids.len() as u16), None),
        )
    }

    /// Creates a path query for historical entries of a specified contract.
    ///
    /// This function takes a slice of contract IDs and creates a path query for fetching
    /// the historical data of the specified contract.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - A contract ID as a 32-byte array. The contract ID is used to
    ///   create the path query.
    ///
    /// # Returns
    ///
    /// * `Result<PathQuery, Error>` - If successful, returns a `PathQuery` object representing the
    ///   merged query for fetching the contracts' data. If limit are outside of the
    ///   allowed range, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the limit is out of the allowed range.
    pub fn fetch_contract_history_query(
        contract_id: [u8; 32],
        start_at_ms: u64,
        limit: Option<u16>,
        offset: Option<u16>,
    ) -> Result<PathQuery, Error> {
        let limit = limit.unwrap_or(MAX_CONTRACT_HISTORY_FETCH_LIMIT);
        if !(1..=MAX_CONTRACT_HISTORY_FETCH_LIMIT).contains(&limit) {
            return Err(Error::Drive(DriveError::InvalidContractHistoryFetchLimit(
                limit,
            )));
        }

        let query = Query::new_single_query_item_with_direction(
            QueryItem::RangeAfter(std::ops::RangeFrom {
                start: encode_u64(start_at_ms),
            }),
            false,
        );

        Ok(PathQuery::new(
            paths::contract_keeping_history_root_path_vec(&contract_id),
            SizedQuery::new(query, Some(limit), offset),
        ))
    }
}
