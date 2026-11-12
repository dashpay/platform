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
    /// an empty state without a request.
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
            let page_len = page.contenders.len();
            let last = page.contenders.keys().last().copied();
            match state.as_mut() {
                None => state = Some(page),
                Some(state) => state.contenders.extend(page.contenders),
            }

            // A short page is the last one
            match last {
                Some(last) if page_len >= page_limit as usize => {
                    start_at = Some((last.to_buffer(), false));
                }
                _ => break,
            }
        }

        Ok(state.unwrap_or_default())
    }
}
