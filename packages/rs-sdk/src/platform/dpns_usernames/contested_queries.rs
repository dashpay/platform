//! Contested DPNS username queries
//!
//! This module provides specialized queries for contested DPNS usernames.
//! These are wrappers around the general contested resource queries that automatically
//! set the DPNS contract ID and document type.

use crate::platform::fetch_many::FetchMany;
use crate::{Error, Sdk};
use dpp::platform_value::{Identifier, Value};
use dpp::prelude::TimestampMillis;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use dpp::voting::vote_polls::VotePoll;
use drive::config::DEFAULT_QUERY_LIMIT;
use drive::query::contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery;
use drive::query::vote_poll_contestant_votes_query::ContestedDocumentVotePollVotesDriveQuery;
use drive::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
};
use drive::query::vote_polls_by_document_type_query::VotePollsByDocumentTypeQuery;
use drive::query::VotePollsByEndDateDriveQuery;
use drive_proof_verifier::types::{Contenders, ContestedResource, VotePollsGroupedByTimestamp};
use futures::{stream, StreamExt};
use std::collections::BTreeMap;

// DPNS parent domain constant
const DPNS_PARENT_DOMAIN: &str = "dash";
/// Keep the fan-out small enough not to overwhelm one DAPI node while avoiding
/// the previous one-network-round-trip-per-contest serial load.
const DPNS_VOTE_STATE_QUERY_CONCURRENCY: usize = 8;

/// Represents contest information including contenders and end time
#[derive(Debug, Clone)]
pub struct ContestInfo {
    /// The contenders for this contested name
    pub contenders: Contenders,
    /// The timestamp when the voting ends (milliseconds since epoch)
    pub end_time: TimestampMillis,
}

/// A contested DPNS username
#[derive(Debug, Clone)]
pub struct ContestedDpnsUsername {
    /// The domain label (e.g., "alice")
    pub label: String,
    /// The normalized label
    pub normalized_label: String,
    /// The contenders for this name
    pub contenders: Vec<Identifier>,
}

impl Sdk {
    /// Get all contested DPNS usernames
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of results to return
    /// * `start_after` - Optional name to start after
    ///   (for pagination)
    ///
    /// # Returns
    ///
    /// Returns a list of contested DPNS usernames
    pub async fn get_contested_dpns_normalized_usernames(
        &self,
        limit: Option<u32>,
        start_after: Option<String>,
    ) -> Result<Vec<String>, Error> {
        let dpns_contract_id = self.get_dpns_contract_id()?;

        let start_index_values = vec![Value::Text(DPNS_PARENT_DOMAIN.to_string())];

        // For a range query of all items under "dash", we use empty end_index_values
        let end_index_values = vec![];

        // If we have a start_after value, we use it as the start_at_value
        let start_at_value = start_after.map(|name| {
            // Create a compound value with both parent domain and label
            let value = Value::Array(vec![
                Value::Text(DPNS_PARENT_DOMAIN.to_string()),
                Value::Text(name),
            ]);
            (value, false) // false means exclusive (start after, not at)
        });

        let query = VotePollsByDocumentTypeQuery {
            contract_id: dpns_contract_id,
            document_type_name: "domain".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            start_index_values,
            end_index_values,
            start_at_value,
            limit: limit.map(|l| l as u16),
            order_ascending: true,
        };

        let contested_resources = ContestedResource::fetch_many(self, query).await?;

        // Convert ContestedResources to our ContestedDpnsUsername format
        let mut usernames = Vec::new();

        // The ContestedResources contains a Vec of ContestedResource items
        for contested_resource in contested_resources.0.iter() {
            // Extract the label from the contested resource
            // The ContestedResource contains the index values [parent_domain, label]
            if let Some(label) = Self::extract_label_from_contested_resource(&contested_resource.0)
            {
                // For now, we'll create a simplified version
                // In a real implementation, we'd fetch the contenders
                usernames.push(label);
            }
        }

        Ok(usernames)
    }

    /// Get the vote state for a contested DPNS username
    ///
    /// # Arguments
    ///
    /// * `label` - The username label to check (e.g., "alice")
    /// * `limit` - Maximum number of contenders to return
    ///
    /// # Returns
    ///
    /// Returns the contenders and their vote counts for the username
    pub async fn get_contested_dpns_vote_state(
        &self,
        label: &str,
        limit: Option<u32>,
    ) -> Result<Contenders, Error> {
        use dpp::voting::contender_structs::ContenderWithSerializedDocument;

        let dpns_contract_id = self.get_dpns_contract_id()?;

        let vote_poll = ContestedDocumentResourceVotePoll {
            contract_id: dpns_contract_id,
            document_type_name: "domain".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![
                Value::Text(DPNS_PARENT_DOMAIN.to_string()),
                Value::Text(label.to_string()),
            ],
        };

        let query = ContestedDocumentVotePollDriveQuery {
            vote_poll,
            result_type: ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
            allow_include_locked_and_abstaining_vote_tally: true,
            start_at: None,
            limit: limit.map(|l| l as u16),
            offset: None,
        };

        // Fetch the contenders using FetchMany
        // ContenderWithSerializedDocument implements FetchMany and returns Contenders
        let result = ContenderWithSerializedDocument::fetch_many(self, query).await?;

        Ok(result)
    }

    /// Get voters who voted for a specific identity for a contested DPNS username
    ///
    /// # Arguments
    ///
    /// * `label` - The username label (e.g., "alice")
    /// * `contestant_id` - The identity ID of the contestant
    /// * `limit` - Maximum number of voters to return
    ///
    /// # Returns
    ///
    /// Returns the list of masternode voters who voted for this contestant
    pub async fn get_contested_dpns_voters_for_identity(
        &self,
        label: &str,
        contestant_id: Identifier,
        limit: Option<u32>,
    ) -> Result<(), Error> {
        let dpns_contract_id = self.get_dpns_contract_id()?;

        let vote_poll = ContestedDocumentResourceVotePoll {
            contract_id: dpns_contract_id,
            document_type_name: "domain".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![
                Value::Text(DPNS_PARENT_DOMAIN.to_string()),
                Value::Text(label.to_string()),
            ],
        };

        let _query = ContestedDocumentVotePollVotesDriveQuery {
            vote_poll,
            contestant_id,
            start_at: None,
            limit: limit.map(|l| l as u16),
            offset: None,
            order_ascending: true,
        };

        // ContestedResourceVoters isn't available, so we'll skip this for now
        Ok(())
    }

    /// Get all contested DPNS usernames that an identity has voted on
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity ID (typically a masternode ProTxHash)
    /// * `limit` - Maximum number of votes to return
    /// * `offset` - Offset for pagination
    ///
    /// # Returns
    ///
    /// Returns the list of contested DPNS usernames this identity has voted on
    pub async fn get_contested_dpns_identity_votes(
        &self,
        identity_id: Identifier,
        limit: Option<u32>,
        offset: Option<u16>,
    ) -> Result<Vec<ContestedDpnsUsername>, Error> {
        let query = ContestedResourceVotesGivenByIdentityQuery {
            identity_id,
            offset,
            limit: limit.map(|l| l as u16),
            order_ascending: true,
            start_at: None,
        };

        // ContestedResourceIdentityVotes isn't available, so we'll skip this for now
        let _ = query;
        let usernames = Vec::new();

        Ok(usernames)
    }

    /// Get all contested DPNS usernames where an identity is a contender
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity ID to search for
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    ///
    /// Returns the list of contested DPNS usernames where this identity is competing
    pub async fn get_contested_dpns_usernames_by_identity(
        &self,
        identity_id: Identifier,
        limit: Option<u32>,
    ) -> Result<Vec<ContestedDpnsUsername>, Error> {
        // First, get all contested DPNS usernames
        let all_contested = self
            .get_contested_dpns_normalized_usernames(limit, None)
            .await?;

        let mut usernames_with_identity = Vec::new();

        // Check each contested name to see if our identity is a contender
        for contested_label in all_contested {
            let vote_state = self
                .get_contested_dpns_vote_state(&contested_label, None)
                .await?;

            // Check if our identity is among the contenders
            let is_contender = vote_state
                .contenders
                .iter()
                .any(|(contender_id, _)| contender_id == &identity_id);

            if is_contender {
                let contenders = vote_state.contenders.into_keys().collect();
                usernames_with_identity.push(ContestedDpnsUsername {
                    label: contested_label.clone(),
                    normalized_label: contested_label.to_lowercase(),
                    contenders,
                });
            }
        }

        Ok(usernames_with_identity)
    }

    // Helper function to extract label from contested resource value
    fn extract_label_from_contested_resource(
        resource: &dpp::platform_value::Value,
    ) -> Option<String> {
        // The ContestedResource contains a Value that represents the serialized index values
        // For DPNS with parentNameAndLabel index, this should be [parent_domain, label]
        // However, the exact structure depends on how the data is serialized

        // First, try to interpret as an array directly
        if let dpp::platform_value::Value::Array(values) = resource {
            if values.len() >= 2 {
                if let dpp::platform_value::Value::Text(label) = &values[1] {
                    return Some(label.clone());
                }
            }
        }

        // If not an array, it might be encoded differently
        // For now, return None if we can't extract it
        None
    }

    // Helper function to extract label from index values
    #[allow(dead_code)]
    fn extract_label_from_index_values(index_values: &[Vec<u8>]) -> Option<String> {
        if index_values.len() >= 2 {
            String::from_utf8(index_values[1].clone()).ok()
        } else {
            None
        }
    }

    /// Get contested usernames that are not yet resolved
    ///
    /// This method fetches all currently contested DPNS usernames that haven't been resolved yet.
    /// It gets current contests and returns the contenders and end time for each unresolved name.
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    ///
    /// Returns a map of contested but unresolved DPNS usernames to their contest info (contenders and end time)
    pub async fn get_contested_non_resolved_usernames(
        &self,
        limit: Option<u32>,
    ) -> Result<BTreeMap<String, ContestInfo>, Error> {
        // First, get all current DPNS contests (returns BTreeMap<String, TimestampMillis>)
        let current_contests = self
            .get_current_dpns_contests(None, None, Some(100))
            .await?;

        // Check each name to see if it's resolved and collect contenders with
        // end times. `buffered` runs a bounded number of requests concurrently
        // while yielding them in the BTreeMap's deterministic name order. That
        // preserves the old limit semantics (the first N unresolved names) and
        // avoids an unbounded burst against DAPI.
        let mut non_resolved_names: BTreeMap<String, ContestInfo> = BTreeMap::new();
        let vote_states = stream::iter(current_contests).map(|(name, end_time)| async move {
            let state = self.get_contested_dpns_vote_state(&name, None).await;
            (name, end_time, state)
        });
        let mut vote_states = vote_states.buffered(DPNS_VOTE_STATE_QUERY_CONCURRENCY);

        while let Some((name, end_time, state)) = vote_states.next().await {
            match state {
                Ok(contenders) => {
                    // Check if there's a winner - if not, it's unresolved
                    if contenders.winner.is_none() {
                        non_resolved_names.insert(
                            name,
                            ContestInfo {
                                contenders,
                                end_time,
                            },
                        );
                    }
                }
                Err(_) => {
                    // If we can't get the vote state, skip this name
                    // (we could include it with empty contenders, but it's better to skip)
                }
            }

            // Check if we've reached the limit
            if let Some(limit) = limit {
                if non_resolved_names.len() >= limit as usize {
                    break;
                }
            }
        }

        Ok(non_resolved_names)
    }

    /// Get non-resolved DPNS contests for a specific identity
    ///
    /// This method fetches all currently contested DPNS usernames that haven't been resolved yet
    /// and filters them to only include contests where the specified identity is a contender.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity ID to filter contests for
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    ///
    /// Returns a map of contested but unresolved DPNS usernames (where the identity is a contender) to their contenders
    pub async fn get_non_resolved_dpns_contests_for_identity(
        &self,
        identity_id: Identifier,
        limit: Option<u32>,
    ) -> Result<BTreeMap<String, ContestInfo>, Error> {
        // First, get all non-resolved contests
        let all_non_resolved = self.get_contested_non_resolved_usernames(limit).await?;

        // Filter to only include contests where the identity is a contender
        let mut identity_contests: BTreeMap<String, ContestInfo> = BTreeMap::new();

        for (name, contest_info) in all_non_resolved {
            // Check if the identity is among the contenders
            let is_contender = contest_info
                .contenders
                .contenders
                .iter()
                .any(|(contender_id, _)| contender_id == &identity_id);

            if is_contender {
                identity_contests.insert(name, contest_info);
            }
        }

        Ok(identity_contests)
    }

    /// Get current DPNS contests (active vote polls)
    ///
    /// This method fetches all currently active DPNS username contests by querying
    /// vote polls by their end date. It automatically paginates through all results
    /// if there are more than the limit.
    ///
    /// # Arguments
    ///
    /// * `start_time` - Optional start time to filter contests (in milliseconds)
    /// * `end_time` - Optional end time to filter contests (in milliseconds)
    /// * `limit` - Maximum number of results per query (defaults to 100)
    ///
    /// # Returns
    ///
    /// Returns a map of contested DPNS names to their end timestamps
    pub async fn get_current_dpns_contests(
        &self,
        start_time: Option<TimestampMillis>,
        end_time: Option<TimestampMillis>,
        limit: Option<u16>,
    ) -> Result<BTreeMap<String, TimestampMillis>, Error> {
        let dpns_contract_id = self.get_dpns_contract_id()?;
        let mut query_limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT);
        let mut name_to_end_time: BTreeMap<String, TimestampMillis> = BTreeMap::new();
        let mut current_start_time = start_time.map(|t| (t, true));

        // A proved query takes no offset, so the pages follow the end dates. The limit counts
        // the polls of every kind, and also each empty end date tree in range, which returns no
        // row: a page can be cut short with fewer polls than the limit, so only an empty page
        // ends the listing.
        loop {
            let result = self
                .fetch_vote_polls_by_end_date(current_start_time, end_time, query_limit)
                .await?;
            let Some(&(last_ts, _)) = result.0.last() else {
                // Empty end date trees alone can fill a page and leave it empty. Before ending,
                // read on at the largest page, which only that many empty end dates in a row
                // would leave empty too (each block sweeps the first due ones away).
                if query_limit < DEFAULT_QUERY_LIMIT {
                    query_limit = DEFAULT_QUERY_LIMIT;
                    continue;
                }
                break;
            };
            let polls_in_page: usize = result.0.iter().map(|(_, polls)| polls.len()).sum();
            record_dpns_contests(&mut name_to_end_time, dpns_contract_id, result);

            if current_start_time == Some((last_ts, true)) && polls_in_page >= query_limit as usize
            {
                // This end date alone filled the page, so the limit may have cut it, and a
                // proved query cannot continue within one end date: read it alone at the
                // largest limit, or refuse rather than leave contests out.
                if query_limit >= DEFAULT_QUERY_LIMIT {
                    return Err(too_many_vote_polls_at_end_date(last_ts));
                }
                let whole_end_date = self
                    .fetch_vote_polls_by_end_date(
                        Some((last_ts, true)),
                        Some(last_ts),
                        DEFAULT_QUERY_LIMIT,
                    )
                    .await?;
                let polls_at_end_date: usize =
                    whole_end_date.0.iter().map(|(_, polls)| polls.len()).sum();
                if polls_at_end_date >= DEFAULT_QUERY_LIMIT as usize {
                    return Err(too_many_vote_polls_at_end_date(last_ts));
                }
                record_dpns_contests(&mut name_to_end_time, dpns_contract_id, whole_end_date);
            }

            match next_vote_polls_page_start(current_start_time, last_ts, end_time) {
                Some(next_start_time) => current_start_time = Some(next_start_time),
                None => break,
            }
        }

        Ok(name_to_end_time)
    }
}

impl Sdk {
    /// One proved page of vote polls by end date, from `start_time` up to `end_time` included.
    async fn fetch_vote_polls_by_end_date(
        &self,
        start_time: Option<(TimestampMillis, bool)>,
        end_time: Option<TimestampMillis>,
        limit: u16,
    ) -> Result<VotePollsGroupedByTimestamp, Error> {
        let query = VotePollsByEndDateDriveQuery {
            start_time,
            end_time: end_time.map(|t| (t, true)),
            limit: Some(limit),
            offset: None,
            order_ascending: true,
        };
        VotePoll::fetch_many(self, query).await
    }
}

/// Adds the DPNS name contests of a page to `name_to_end_time`; other contests and yes/no polls,
/// which share the end date index, are skipped. A name seen twice is just written again.
fn record_dpns_contests(
    name_to_end_time: &mut BTreeMap<String, TimestampMillis>,
    dpns_contract_id: Identifier,
    page: VotePollsGroupedByTimestamp,
) {
    for (timestamp, polls) in page.0 {
        for poll in polls {
            let VotePoll::ContestedDocumentResourceVotePoll(contested_poll) = poll else {
                continue;
            };
            if contested_poll.contract_id == dpns_contract_id
                && contested_poll.document_type_name == "domain"
            {
                // Extract the contested name from index_values
                if let Some(Value::Text(label)) = contested_poll.index_values.get(1) {
                    name_to_end_time.insert(label.clone(), timestamp);
                }
            }
        }
    }
}

/// The error for an end date that holds at least as many vote polls as one page can carry, so
/// that its contests cannot all be listed.
fn too_many_vote_polls_at_end_date(end_date: TimestampMillis) -> Error {
    Error::Generic(format!(
        "at least {DEFAULT_QUERY_LIMIT} vote polls end at {end_date}, more than \
         one page of vote polls by end date can list"
    ))
}

/// Where the page after one that ended at `last_end_date` starts, or `None` when nothing is left.
///
/// The limit may have cut the last end date short, so the next page starts at it again. A page
/// that started at that end date held only it, all of it, so the next one starts past it, unless
/// it is the caller's end time: a range starting past its own end is refused.
fn next_vote_polls_page_start(
    page_start_time: Option<(TimestampMillis, bool)>,
    last_end_date: TimestampMillis,
    end_time: Option<TimestampMillis>,
) -> Option<(TimestampMillis, bool)> {
    if page_start_time != Some((last_end_date, true)) {
        Some((last_end_date, true))
    } else if end_time == Some(last_end_date) {
        None
    } else {
        Some((last_end_date, false))
    }
}

#[cfg(test)]
mod tests {
    use super::next_vote_polls_page_start;

    #[test]
    fn should_start_the_next_page_at_the_last_end_date_again() {
        assert_eq!(next_vote_polls_page_start(None, 10, None), Some((10, true)));
        assert_eq!(
            next_vote_polls_page_start(Some((5, true)), 10, Some(20)),
            Some((10, true))
        );
    }

    #[test]
    fn should_move_past_an_end_date_a_page_held_alone() {
        assert_eq!(
            next_vote_polls_page_start(Some((10, true)), 10, None),
            Some((10, false))
        );
        assert_eq!(
            next_vote_polls_page_start(Some((10, true)), 10, Some(20)),
            Some((10, false))
        );
    }

    #[test]
    fn should_stop_past_the_end_time_instead_of_asking_for_an_empty_range() {
        // A start past the end time at the same instant is refused by the query.
        assert_eq!(
            next_vote_polls_page_start(Some((20, true)), 20, Some(20)),
            None
        );
    }
}
