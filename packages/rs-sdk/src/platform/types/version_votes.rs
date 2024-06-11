//! Helpers for managing platform version votes

use crate::platform::fetch_many::FetchMany;
use crate::{platform::LimitQuery, Error, Sdk};
use async_trait::async_trait;
use dashcore_rpc::dashcore::ProTxHash;
use drive_proof_verifier::types::{MasternodeProtocolVote, MasternodeProtocolVotes};

// Trait needed here to implement functions on foreign type.

/// Helper trait for managing MasternodeProtocolVote objects
#[async_trait]
pub trait MasternodeProtocolVoteEx<K: Ord> {
    /// Fetch masternode votes for version update from the platform.
    ///
    /// ## Parameters
    ///
    /// - `sdk`: An instance of [Sdk].
    /// - `start_protxhash`: [ProTxHash] of the first masternode to fetch votes for.
    /// Use `None` to start from the beginning.
    /// - `limit`: Maximum number of votes to fetch. Defaults to
    /// [DEFAULT_NODES_VOTING_LIMIT](crate::platform::query::DEFAULT_NODES_VOTING_LIMIT)
    ///
    /// ## See also
    ///
    /// - [MasternodeProtocolVote::fetch_many()]
    /// - [MasternodeProtocolVote::fetch_many_with_limit()]
    async fn fetch_votes(
        sdk: &Sdk,
        start_protxhash: Option<ProTxHash>,
        limit: Option<u32>,
    ) -> Result<MasternodeProtocolVotes, Error>;
}

#[async_trait]
impl MasternodeProtocolVoteEx<ProTxHash> for MasternodeProtocolVote {
    async fn fetch_votes(
        sdk: &Sdk,
        start_protxhash: Option<ProTxHash>,
        limit: Option<u32>,
    ) -> Result<MasternodeProtocolVotes, Error> {
        Self::fetch_many(
            sdk,
            LimitQuery {
                query: start_protxhash,
                limit,
                start_info: None,
            },
        )
        .await
    }
}

#[async_trait]
impl MasternodeProtocolVoteEx<ProTxHash> for MasternodeProtocolVotes {
    async fn fetch_votes(
        sdk: &Sdk,
        start_protxhash: Option<ProTxHash>,
        limit: Option<u32>,
    ) -> Result<MasternodeProtocolVotes, Error> {
        MasternodeProtocolVote::fetch_votes(sdk, start_protxhash, limit).await
    }
}
