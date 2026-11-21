//! The whole state of a contest: every contender with its document and tally, read a page at a
//! time.

use crate::platform::fetch_many::FetchMany;
use crate::{Error, Sdk};
use dpp::voting::contender_structs::ContenderWithSerializedDocument;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use drive::config::DEFAULT_QUERY_LIMIT;
use drive::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
};
use drive_proof_verifier::types::Contenders;

impl Sdk {
    /// The state of the contest `vote_poll`: its contenders with their documents and tallies,
    /// the abstain and lock tallies, and the winner once the contest ended.
    ///
    /// Reads the contenders in the order of the ids the contest names them by, in pages of up
    /// to 100 (the most Drive answers to one query) that each start after the last contender
    /// read, until `limit` contenders are read or none are left. The abstain and lock tallies
    /// and the winner come from the first page: Drive returns the tallies only to a query
    /// without a start.
    ///
    /// `limit` is the most contenders returned; `None` returns every contender, and `Some(0)`
    /// an empty state without a request. A contest that ended answers its first page with
    /// every contender of its record, whatever the page asks for: no further page is read, and
    /// its contenders are cut to `limit` here, the first in id order kept; its winner and its
    /// abstain and lock tallies are returned whole. A contest that ends between pages answers
    /// the next page empty, so an empty page after the first is read once more without a
    /// start: if that names a winner, its record is returned in place of the live pages.
    pub async fn fetch_contested_resource_vote_state(
        &self,
        vote_poll: &ContestedDocumentResourceVotePoll,
        limit: Option<u32>,
    ) -> Result<Contenders, Error> {
        let mut state: Option<Contenders> = None;
        let mut start_at = None;
        loop {
            let read = state.as_ref().map_or(0, |state| state.contenders.len());
            let page_limit = match limit {
                Some(limit) => {
                    let left = (limit as usize).saturating_sub(read);
                    if left == 0 {
                        break;
                    }
                    left.min(DEFAULT_QUERY_LIMIT as usize) as u16
                }
                None => DEFAULT_QUERY_LIMIT,
            };
            let page = ContenderWithSerializedDocument::fetch_many(
                self,
                ContestedDocumentVotePollDriveQuery {
                    vote_poll: vote_poll.clone(),
                    result_type:
                        ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
                    allow_include_locked_and_abstaining_vote_tally: true,
                    start_at,
                    limit: Some(page_limit),
                    offset: None,
                },
            )
            .await?;
            // A page with a start selects only the keys after it, and the record of a contest
            // that ended lives at the first key: a contest that ended between requests answers
            // its next page empty. Read it again without a start, where a record that ended
            // comes whole whatever the limit, and take its record over the live pages
            if start_at.is_some() && page.contenders.is_empty() && page.winner.is_none() {
                let record = ContenderWithSerializedDocument::fetch_many(
                    self,
                    ContestedDocumentVotePollDriveQuery {
                        vote_poll: vote_poll.clone(),
                        result_type:
                            ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
                        allow_include_locked_and_abstaining_vote_tally: true,
                        start_at: None,
                        limit: Some(1),
                        offset: None,
                    },
                )
                .await?;
                if record.winner.is_some() {
                    state = Some(record);
                }
                break;
            }
            let page_len = page.contenders.len();
            let last = page.contenders.keys().last().copied();
            // A page naming a winner is the record of a contest that ended, which is whole
            let ended = page.winner.is_some();
            // A record read after live pages (the contest ended between requests) replaces them:
            // it carries the winner and the final tallies, and its contenders are whole
            match state.as_mut() {
                Some(state) if !ended => state.contenders.extend(page.contenders),
                _ => state = Some(page),
            }

            // A short page is the last one
            match last {
                Some(last) if !ended && page_len >= page_limit as usize => {
                    start_at = Some((last.to_buffer(), false));
                }
                _ => break,
            }
        }

        let mut state = state.unwrap_or_default();
        if let Some(limit) = limit {
            while state.contenders.len() > limit as usize {
                state.contenders.pop_last();
            }
        }
        Ok(state)
    }
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use super::*;
    use crate::SdkBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::platform_value::{Identifier, Value};
    use dpp::version::PlatformVersion;
    use dpp::voting::contender_structs::ContenderWithSerializedDocumentV0;
    use dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
    use std::collections::BTreeMap;

    /// A contest that ended answers a page with every contender of its record, however few
    /// the page asked for: the state holds `limit` of them, with the winner and the abstain
    /// and lock tallies of the whole contest.
    #[tokio::test]
    async fn should_read_a_contest_that_ended_in_one_page_and_cut_it_to_the_limit() {
        let mut sdk = SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .build()
            .expect("a mock sdk");
        let vote_poll = ContestedDocumentResourceVotePoll {
            contract_id: Identifier::new([7; 32]),
            document_type_name: "domain".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![
                Value::Text("dash".to_string()),
                Value::Text("quantum".to_string()),
            ],
        };
        let contender = |n: u8, vote_tally: u32| {
            let identity_id = Identifier::new([n; 32]);
            (
                identity_id,
                ContenderWithSerializedDocument::V0(ContenderWithSerializedDocumentV0 {
                    identity_id,
                    serialized_document: None,
                    vote_tally: Some(vote_tally),
                }),
            )
        };
        let winner = Some((
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(Identifier::new([3; 32])),
            BlockInfo::default(),
        ));
        for limit in [1u16, 2] {
            sdk.mock()
                .expect_fetch_many::<_, ContenderWithSerializedDocument, _, Contenders>(
                    ContestedDocumentVotePollDriveQuery {
                        vote_poll: vote_poll.clone(),
                        result_type:
                            ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
                        allow_include_locked_and_abstaining_vote_tally: true,
                        start_at: None,
                        limit: Some(limit),
                        offset: None,
                    },
                    Some(Contenders {
                        winner,
                        contenders: BTreeMap::from([
                            contender(1, 4),
                            contender(2, 5),
                            contender(3, 12),
                        ]),
                        abstain_vote_tally: Some(6),
                        lock_vote_tally: Some(2),
                    }),
                )
                .await
                .expect("registers the page");
        }
        // The whole record of a contest that ended comes with its first page, however many
        // contenders it holds: no page after it is registered, so asking for one would fail
        let record: BTreeMap<_, _> = (1..=150u8).map(|n| contender(n, 1)).collect();
        sdk.mock()
            .expect_fetch_many::<_, ContenderWithSerializedDocument, _, Contenders>(
                ContestedDocumentVotePollDriveQuery {
                    vote_poll: vote_poll.clone(),
                    result_type:
                        ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
                    allow_include_locked_and_abstaining_vote_tally: true,
                    start_at: None,
                    limit: Some(DEFAULT_QUERY_LIMIT),
                    offset: None,
                },
                Some(Contenders {
                    winner,
                    contenders: record,
                    abstain_vote_tally: Some(6),
                    lock_vote_tally: Some(2),
                }),
            )
            .await
            .expect("registers the record");
        let whole = sdk
            .fetch_contested_resource_vote_state(&vote_poll, None)
            .await
            .expect("reads the record in one page");
        assert_eq!(whole.contenders.len(), 150);
        assert_eq!(whole.winner, winner);

        for limit in [1u32, 2] {
            let state = sdk
                .fetch_contested_resource_vote_state(&vote_poll, Some(limit))
                .await
                .expect("reads");
            assert_eq!(
                state.contenders.keys().copied().collect::<Vec<_>>(),
                (1..=limit as u8)
                    .map(|n| Identifier::new([n; 32]))
                    .collect::<Vec<_>>(),
                "limit {limit}"
            );
            assert_eq!(state.winner, winner);
            assert_eq!(state.abstain_vote_tally, Some(6));
            assert_eq!(state.lock_vote_tally, Some(2));
        }
    }

    /// A contest that ends between pages answers the next page, which starts after the last
    /// contender read, empty: the record lives at the first key. The pager reads the contest
    /// again without a start and returns the record, with its winner, instead of the live
    /// contenders read before.
    #[tokio::test]
    async fn should_return_the_record_of_a_contest_that_ended_between_pages() {
        let mut sdk = SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .build()
            .expect("a mock sdk");
        let vote_poll = ContestedDocumentResourceVotePoll {
            contract_id: Identifier::new([7; 32]),
            document_type_name: "domain".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![
                Value::Text("dash".to_string()),
                Value::Text("quantum".to_string()),
            ],
        };
        let contender = |n: u8, vote_tally: u32| {
            let identity_id = Identifier::new([n; 32]);
            (
                identity_id,
                ContenderWithSerializedDocument::V0(ContenderWithSerializedDocumentV0 {
                    identity_id,
                    serialized_document: None,
                    vote_tally: Some(vote_tally),
                }),
            )
        };
        let query =
            |start_at: Option<([u8; 32], bool)>, limit: u16| ContestedDocumentVotePollDriveQuery {
                vote_poll: vote_poll.clone(),
                result_type: ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
                allow_include_locked_and_abstaining_vote_tally: true,
                start_at,
                limit: Some(limit),
                offset: None,
            };

        // A full first page of a live contest, so the pager asks for the next one
        let live: BTreeMap<_, _> = (1..=DEFAULT_QUERY_LIMIT as u8)
            .map(|n| contender(n, 1))
            .collect();
        let last = Identifier::new([DEFAULT_QUERY_LIMIT as u8; 32]);
        sdk.mock()
            .expect_fetch_many::<_, ContenderWithSerializedDocument, _, Contenders>(
                query(None, DEFAULT_QUERY_LIMIT),
                Some(Contenders {
                    winner: None,
                    contenders: live,
                    abstain_vote_tally: Some(1),
                    lock_vote_tally: Some(0),
                }),
            )
            .await
            .expect("registers the first page");
        // The contest ended: the page after the last contender read is empty
        sdk.mock()
            .expect_fetch_many::<_, ContenderWithSerializedDocument, _, Contenders>(
                query(Some((last.to_buffer(), false)), DEFAULT_QUERY_LIMIT),
                Some(Contenders::default()),
            )
            .await
            .expect("registers the empty page");
        // Read without a start, the contest answers with its record
        let winner = Some((
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(Identifier::new([2; 32])),
            BlockInfo::default(),
        ));
        sdk.mock()
            .expect_fetch_many::<_, ContenderWithSerializedDocument, _, Contenders>(
                query(None, 1),
                Some(Contenders {
                    winner,
                    contenders: BTreeMap::from([contender(1, 4), contender(2, 9)]),
                    abstain_vote_tally: Some(6),
                    lock_vote_tally: Some(2),
                }),
            )
            .await
            .expect("registers the record");

        let state = sdk
            .fetch_contested_resource_vote_state(&vote_poll, None)
            .await
            .expect("reads");
        assert_eq!(state.winner, winner);
        assert_eq!(
            state.contenders.keys().copied().collect::<Vec<_>>(),
            vec![Identifier::new([1; 32]), Identifier::new([2; 32])]
        );
        assert_eq!(state.abstain_vote_tally, Some(6));
        assert_eq!(state.lock_vote_tally, Some(2));
    }
}
