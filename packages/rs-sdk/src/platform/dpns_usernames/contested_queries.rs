//! Contested DPNS username queries
//!
//! This module provides specialized queries for contested DPNS usernames.
//! These are wrappers around the general contested resource queries that automatically
//! set the DPNS contract ID and document type.

use crate::platform::fetch_many::FetchMany;
use crate::{Error, Sdk};
use dpp::platform_value::{Identifier, Value};
use dpp::prelude::TimestampMillis;
use dpp::voting::contender_structs::ContenderWithSerializedDocument;
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
    /// Reads the names in ascending order, in pages of up to 100 (the most Drive answers to one
    /// query), until `limit` names are read or none are left.
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of results to return; `None` returns every name
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
        let mut usernames = Vec::new();
        let mut start_after = start_after;

        loop {
            let page_limit = match limit {
                Some(limit) => {
                    let left = (limit as usize).saturating_sub(usernames.len());
                    if left == 0 {
                        break;
                    }
                    left.min(DEFAULT_QUERY_LIMIT as usize) as u16
                }
                None => DEFAULT_QUERY_LIMIT,
            };

            // With the parent domain fixed, the query ranges over the next property of the
            // index, the label: Drive answers each contest with its label alone, and starts
            // after a label, not after a [parent domain, label] pair
            let query = VotePollsByDocumentTypeQuery {
                contract_id: dpns_contract_id,
                document_type_name: "domain".to_string(),
                index_name: "parentNameAndLabel".to_string(),
                start_index_values: vec![Value::Text(DPNS_PARENT_DOMAIN.to_string())],
                end_index_values: vec![],
                // false means exclusive (start after, not at)
                start_at_value: start_after.take().map(|name| (Value::Text(name), false)),
                limit: Some(page_limit),
                order_ascending: true,
            };

            let contested_resources = ContestedResource::fetch_many(self, query).await?;
            let read = contested_resources.0.len();
            for contested_resource in contested_resources.0 {
                usernames.push(Self::extract_label_from_contested_resource(
                    contested_resource,
                )?);
            }

            // A short page is the last one
            if read < page_limit as usize {
                break;
            }
            start_after = usernames.last().cloned();
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

        // Check each contested name to see if our identity is a contender. `buffered` runs a
        // bounded number of contests at once and yields them in name order
        let mut contests = stream::iter(all_contested)
            .map(|contested_label| async move {
                let contenders = self
                    .get_all_contested_dpns_contenders(&contested_label)
                    .await;
                (contested_label, contenders)
            })
            .buffered(DPNS_VOTE_STATE_QUERY_CONCURRENCY);
        while let Some((contested_label, contenders)) = contests.next().await {
            let contenders = contenders?;

            if contenders.contains(&identity_id) {
                usernames_with_identity.push(ContestedDpnsUsername {
                    normalized_label: contested_label.to_lowercase(),
                    label: contested_label,
                    contenders,
                });
            }
        }

        Ok(usernames_with_identity)
    }

    /// Every contender of the contest for `label`, in identity order. A contest holds up to
    /// `max_contenders_per_contest` (1,000) contenders and Drive answers at most 100 per query,
    /// so they are read in pages of 100 that each start after the last contender read; every
    /// request names its count, so its proof is verified against the page Drive proved.
    async fn get_all_contested_dpns_contenders(
        &self,
        label: &str,
    ) -> Result<Vec<Identifier>, Error> {
        let vote_poll = ContestedDocumentResourceVotePoll {
            contract_id: self.get_dpns_contract_id()?,
            document_type_name: "domain".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![
                Value::Text(DPNS_PARENT_DOMAIN.to_string()),
                Value::Text(label.to_string()),
            ],
        };

        let mut contenders: Vec<Identifier> = Vec::new();
        let mut start_at = None;
        loop {
            let page = ContenderWithSerializedDocument::fetch_many(
                self,
                ContestedDocumentVotePollDriveQuery {
                    vote_poll: vote_poll.clone(),
                    result_type: ContestedDocumentVotePollDriveQueryResultType::VoteTally,
                    offset: None,
                    limit: Some(DEFAULT_QUERY_LIMIT),
                    start_at,
                    allow_include_locked_and_abstaining_vote_tally: false,
                },
            )
            .await?;
            let read = page.contenders.len();
            contenders.extend(page.contenders.into_keys());

            // A short page is the last one
            match contenders.last() {
                Some(last) if read >= DEFAULT_QUERY_LIMIT as usize => {
                    start_at = Some((last.to_buffer(), false));
                }
                _ => break,
            }
        }

        Ok(contenders)
    }

    /// The label of a contested resource of the `parentNameAndLabel` index queried with the
    /// parent domain fixed: Drive answers with the value of the label property alone, as text.
    /// Anything else means the query and Drive disagree on the index, so it is an error rather
    /// than a name left out.
    fn extract_label_from_contested_resource(resource: ContestedResource) -> Result<String, Error> {
        match resource.0 {
            Value::Text(label) => Ok(label),
            other => Err(Error::InvalidProvedResponse(format!(
                "expected a contested DPNS name to be text, got {other:?}"
            ))),
        }
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
        let query_limit = limit.unwrap_or(100);
        let mut name_to_end_time: BTreeMap<String, TimestampMillis> = BTreeMap::new();
        let mut current_start_time = start_time.map(|t| (t, true));

        loop {
            let query = VotePollsByEndDateDriveQuery {
                start_time: current_start_time,
                end_time: end_time.map(|t| (t, true)),
                limit: Some(query_limit),
                offset: None,
                order_ascending: true,
            };

            // Execute the query
            let result: VotePollsGroupedByTimestamp = VotePoll::fetch_many(self, query).await?;

            // Check if we got any results
            if result.0.is_empty() {
                break;
            }

            let mut last_timestamp = None;
            let mut polls_in_last_group = 0;

            // Process each timestamp group
            for (timestamp, polls) in result.0 {
                let mut dpns_polls_count = 0;

                for VotePoll::ContestedDocumentResourceVotePoll(contested_poll) in polls {
                    if contested_poll.contract_id == dpns_contract_id
                        && contested_poll.document_type_name == "domain"
                    {
                        // Extract the contested name from index_values
                        if contested_poll.index_values.len() >= 2 {
                            if let Value::Text(label) = &contested_poll.index_values[1] {
                                name_to_end_time.insert(label.clone(), timestamp);
                                dpns_polls_count += 1;
                            }
                        }
                    }
                }

                if dpns_polls_count > 0 {
                    last_timestamp = Some(timestamp);
                    polls_in_last_group = dpns_polls_count;
                }
            }

            // Check if we should continue pagination
            // If we got less than the limit, we've reached the end
            if polls_in_last_group < query_limit as usize {
                break;
            }

            // Set up for next query - use the last timestamp as the new start
            // with false (not included) to avoid duplicates
            if let Some(last_ts) = last_timestamp {
                current_start_time = Some((last_ts, false));
            } else {
                break;
            }
        }

        Ok(name_to_end_time)
    }
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use super::*;
    use crate::SdkBuilder;
    use dpp::version::PlatformVersion;
    use dpp::voting::contender_structs::ContenderWithSerializedDocumentV0;
    use drive_proof_verifier::types::ContestedResources;

    fn mock_sdk() -> Sdk {
        SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .build()
            .expect("expected a mock sdk")
    }

    /// Contested names sort as their numbers do
    fn name(n: usize) -> String {
        format!("name{n:03}")
    }

    fn names(from: usize, to: usize) -> Vec<String> {
        (from..to).map(name).collect()
    }

    /// The query reading the contested names after `start_after`, built here rather than with
    /// the code under test, so a request shaped differently matches no expectation
    fn names_page_query(
        sdk: &Sdk,
        start_after: Option<&str>,
        limit: u16,
    ) -> VotePollsByDocumentTypeQuery {
        VotePollsByDocumentTypeQuery {
            contract_id: sdk
                .get_dpns_contract_id()
                .expect("expected the dpns contract id"),
            document_type_name: "domain".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            start_index_values: vec![Value::Text("dash".to_string())],
            end_index_values: vec![],
            start_at_value: start_after.map(|name| (Value::Text(name.to_string()), false)),
            limit: Some(limit),
            order_ascending: true,
        }
    }

    /// Drive's answer to a names query: each contest as its label alone
    fn names_page(labels: &[String]) -> ContestedResources {
        labels
            .iter()
            .map(|label| ContestedResource(Value::Text(label.clone())))
            .collect()
    }

    async fn expect_names_page(
        sdk: &mut Sdk,
        start_after: Option<&str>,
        limit: u16,
        labels: &[String],
    ) {
        let query = names_page_query(sdk, start_after, limit);
        sdk.mock()
            .expect_fetch_many::<Identifier, ContestedResource, _, ContestedResources>(
                query,
                Some(names_page(labels)),
            )
            .await
            .expect("expected to register the page");
    }

    #[tokio::test]
    async fn should_read_every_contested_name_once_across_pages() {
        let mut sdk = mock_sdk();
        expect_names_page(&mut sdk, None, 100, &names(0, 100)).await;
        expect_names_page(&mut sdk, Some(&name(99)), 100, &names(100, 200)).await;
        expect_names_page(&mut sdk, Some(&name(199)), 100, &names(200, 250)).await;

        let usernames = sdk
            .get_contested_dpns_normalized_usernames(None, None)
            .await
            .expect("expected every page to answer");

        assert_eq!(usernames, names(0, 250));
    }

    #[tokio::test]
    async fn should_stop_at_an_empty_page_after_a_full_one() {
        let mut sdk = mock_sdk();
        expect_names_page(&mut sdk, None, 100, &names(0, 100)).await;
        expect_names_page(&mut sdk, Some(&name(99)), 100, &[]).await;

        let usernames = sdk
            .get_contested_dpns_normalized_usernames(None, None)
            .await
            .expect("expected both pages to answer");

        assert_eq!(usernames, names(0, 100));
    }

    #[tokio::test]
    async fn should_read_a_limit_over_100_in_pages_of_at_most_100() {
        let mut sdk = mock_sdk();
        expect_names_page(&mut sdk, None, 100, &names(0, 100)).await;
        expect_names_page(&mut sdk, Some(&name(99)), 50, &names(100, 150)).await;

        let usernames = sdk
            .get_contested_dpns_normalized_usernames(Some(150), None)
            .await
            .expect("expected both pages to answer");

        assert_eq!(usernames, names(0, 150));
    }

    #[tokio::test]
    async fn should_start_after_the_given_name() {
        let mut sdk = mock_sdk();
        expect_names_page(&mut sdk, Some(&name(41)), 5, &names(42, 47)).await;

        let usernames = sdk
            .get_contested_dpns_normalized_usernames(Some(5), Some(name(41)))
            .await
            .expect("expected the page to answer");

        assert_eq!(usernames, names(42, 47));
    }

    #[test]
    fn should_extract_a_label_answered_as_text() {
        let label = Sdk::extract_label_from_contested_resource(ContestedResource(Value::Text(
            "alice".to_string(),
        )))
        .expect("expected a text label");

        assert_eq!(label, "alice");
    }

    #[test]
    fn should_refuse_a_contested_resource_that_is_not_text() {
        let error =
            Sdk::extract_label_from_contested_resource(ContestedResource(Value::Array(vec![
                Value::Text("dash".to_string()),
                Value::Text("alice".to_string()),
            ])))
            .expect_err("expected an array to be refused");

        assert!(matches!(error, Error::InvalidProvedResponse(_)));
    }

    /// Identity `n`: identities sort as their numbers do
    fn contender(n: u16) -> Identifier {
        let mut id = [0u8; 32];
        id[30..].copy_from_slice(&n.to_be_bytes());
        Identifier::new(id)
    }

    fn contenders(from: u16, to: u16) -> Vec<Identifier> {
        (from..to).map(contender).collect()
    }

    /// The page of contenders of the contest for `label` after `start_after`, built here rather
    /// than with the code under test, so a request shaped differently (one without a count, or
    /// a cursor that is not the last contender, exclusive) matches no expectation
    async fn expect_contenders_page(
        sdk: &mut Sdk,
        label: &str,
        start_after: Option<Identifier>,
        page: &[Identifier],
    ) {
        let query = ContestedDocumentVotePollDriveQuery {
            vote_poll: ContestedDocumentResourceVotePoll {
                contract_id: sdk
                    .get_dpns_contract_id()
                    .expect("expected the dpns contract id"),
                document_type_name: "domain".to_string(),
                index_name: "parentNameAndLabel".to_string(),
                index_values: vec![
                    Value::Text("dash".to_string()),
                    Value::Text(label.to_string()),
                ],
            },
            result_type: ContestedDocumentVotePollDriveQueryResultType::VoteTally,
            allow_include_locked_and_abstaining_vote_tally: false,
            start_at: start_after.map(|id| (id.to_buffer(), false)),
            limit: Some(100),
            offset: None,
        };
        let page = Contenders::from_iter(page.iter().map(|&identity_id| {
            (
                identity_id,
                Some(ContenderWithSerializedDocument::V0(
                    ContenderWithSerializedDocumentV0 {
                        identity_id,
                        serialized_document: None,
                        vote_tally: Some(0),
                    },
                )),
            )
        }));
        sdk.mock()
            .expect_fetch_many::<_, ContenderWithSerializedDocument, _, Contenders>(
                query,
                Some(page),
            )
            .await
            .expect("expected to register the page of contenders");
    }

    #[tokio::test]
    async fn should_list_the_contested_names_an_identity_contends_for() {
        let identity = contender(1);
        let other = contender(2);
        let mut sdk = mock_sdk();
        expect_names_page(&mut sdk, None, 100, &names(0, 3)).await;
        expect_contenders_page(&mut sdk, &name(0), None, &[other]).await;
        expect_contenders_page(&mut sdk, &name(1), None, &[identity, other]).await;
        expect_contenders_page(&mut sdk, &name(2), None, &[identity]).await;

        let usernames = sdk
            .get_contested_dpns_usernames_by_identity(identity, None)
            .await
            .expect("expected every query to answer");

        let labels: Vec<&str> = usernames.iter().map(|u| u.label.as_str()).collect();
        assert_eq!(labels, [name(1), name(2)]);
        assert_eq!(usernames[0].contenders, [identity, other]);
        assert_eq!(usernames[1].contenders, [identity]);
    }

    /// A contest holds up to 1,000 contenders and Drive answers 100 per query: an identity past
    /// the first 100 is found, and the name lists every contender
    #[tokio::test]
    async fn should_find_an_identity_past_the_first_100_contenders() {
        let identity = contender(150);
        let mut sdk = mock_sdk();
        expect_names_page(&mut sdk, None, 100, &names(0, 2)).await;
        // 160 contenders, the identity on the second page
        expect_contenders_page(&mut sdk, &name(0), None, &contenders(0, 100)).await;
        expect_contenders_page(
            &mut sdk,
            &name(0),
            Some(contender(99)),
            &contenders(100, 160),
        )
        .await;
        // Exactly one full page, without the identity
        expect_contenders_page(&mut sdk, &name(1), None, &contenders(200, 300)).await;
        expect_contenders_page(&mut sdk, &name(1), Some(contender(299)), &[]).await;

        let usernames = sdk
            .get_contested_dpns_usernames_by_identity(identity, None)
            .await
            .expect("expected every page to answer");

        assert_eq!(usernames.len(), 1);
        assert_eq!(usernames[0].label, name(0));
        assert_eq!(usernames[0].contenders, contenders(0, 160));
    }
}
