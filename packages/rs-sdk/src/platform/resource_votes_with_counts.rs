//! The votes of an identity on contested resources, each with how many times the identity has
//! voted on its vote poll.
//!
//! A masternode may change its vote on a vote poll a limited number of times
//! (`votes_allowed_per_masternode`). Platform keeps, next to the masternode's current vote, how
//! many times it has voted on that poll, and proves both together. Fetching
//! [`ResourceVote`](dpp::voting::votes::resource_vote::ResourceVote)s returns the votes alone;
//! fetching [`ResourceVoteWithCount`]s returns the counts too, from the same request and with
//! the same proof verification:
//!
//! ```ignore
//! let votes: ResourceVotesWithCountsByIdentity =
//!     ResourceVoteWithCount::fetch_many(&sdk, masternode_pro_tx_hash).await?;
//! match votes.get(&vote_poll.unique_id()?) {
//!     Some(Some(vote)) => { /* voted `vote.vote_count` times, now for `vote.resource_vote` */ }
//!     _ => { /* no vote of the masternode on this poll among the fetched ones */ }
//! }
//! ```
//!
//! Every query [`ResourceVote`](dpp::voting::votes::resource_vote::ResourceVote) is fetched
//! with is supported, among them
//! [`ContestedResourceVotesGivenByIdentityQuery`](drive::query::contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery)
//! for reading the votes page by page. One request answers one identity and as many vote polls
//! as a page holds. A page proves absence too: a vote poll id inside the range of ids the page
//! covers and without an entry is a poll the identity has not voted on.

use crate::platform::{FetchMany, Identifier};
use dapi_grpc::platform::v0::GetContestedResourceIdentityVotesRequest;
pub use drive_proof_verifier::types::{ResourceVoteWithCount, ResourceVotesWithCountsByIdentity};

/// Fetch the votes of some identity on contested resources, each with the number of times the
/// identity has voted on its vote poll.
///
/// ## Supported query types
///
/// * [`ContestedResourceVotesGivenByIdentityQuery`](drive::query::contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery)
/// * [`ProTxHash`](dpp::dashcore::ProTxHash)
impl FetchMany<Identifier, ResourceVotesWithCountsByIdentity> for ResourceVoteWithCount {
    type Query = GetContestedResourceIdentityVotesRequest;
    type Request = GetContestedResourceIdentityVotesRequest;
}
