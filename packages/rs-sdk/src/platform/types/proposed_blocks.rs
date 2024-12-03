//! Helpers for managing platform proposed block counts per epoch

use crate::platform::{FetchMany, LimitQuery, QueryStartInfo};
use crate::{Error, Sdk};
use async_trait::async_trait;
use dashcore_rpc::dashcore::ProTxHash;
use dpp::block::epoch::EpochIndex;
use drive_proof_verifier::types::{ProposerBlockCountByRange, ProposerBlockCounts};
// Trait needed here to implement functions on foreign type.

/// A helper trait for fetching block proposal counts for specific proposers.
///
/// This trait defines an asynchronous method to retrieve block counts for proposers within a specified range.
/// It allows fetching a set of proposers and their corresponding block counts, either by setting a limit
/// or starting from a specific proposer hash.
///
/// # Type Parameters
///
/// * `K`: The type of the keys in the map, which must implement the `Ord` trait.
#[async_trait]
pub trait ProposedBlockCountEx<K: Ord> {
    /// Fetches the proposed block counts for proposers within a given range.
    ///
    /// This asynchronous method retrieves the number of blocks proposed by various proposers,
    /// starting from an optional proposer transaction hash (`ProTxHash`) and returning a limited
    /// number of results if specified. If a proposer transaction hash is provided, the query will
    /// start at that hash. The optional boolean flag determines whether to include the proposer
    /// identified by the `ProTxHash` in the results.
    ///
    /// ## Parameters
    ///
    /// * `sdk`: A reference to the `Sdk` instance, which handles the platform interaction.
    /// * `limit`: An optional `u16` representing the maximum number of proposer block counts to retrieve.
    /// * `start_pro_tx_hash`: An optional tuple where the first element is a `ProTxHash` to start
    ///    from, and the second element is a boolean indicating whether to include the starting proposer
    ///    in the results.
    ///
    /// ## Returns
    ///
    /// A `Result` containing `ProposerBlockCounts`, which is a mapping between proposers and the number of blocks they proposed,
    /// or an `Error` if the operation fails.
    ///
    /// ## See also
    ///
    /// - [`ProposerBlockCounts`](crate::ProposerBlockCounts): The data structure holding the result of this operation.

    async fn fetch_proposed_blocks_by_range(
        sdk: &Sdk,
        epoch: Option<EpochIndex>,
        limit: Option<u32>,
        start_pro_tx_hash: Option<QueryStartInfo>,
    ) -> Result<ProposerBlockCounts, Error>;
}

#[async_trait]
impl ProposedBlockCountEx<ProTxHash> for ProposerBlockCounts {
    async fn fetch_proposed_blocks_by_range(
        sdk: &Sdk,
        epoch: Option<EpochIndex>,
        limit: Option<u32>,
        start_pro_tx_hash: Option<QueryStartInfo>,
    ) -> Result<ProposerBlockCounts, Error> {
        ProposerBlockCountByRange::fetch_many(
            sdk,
            LimitQuery {
                query: epoch,
                limit,
                start_info: start_pro_tx_hash,
            },
        )
        .await
    }
}
