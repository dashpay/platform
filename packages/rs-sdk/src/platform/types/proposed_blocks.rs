//! Helpers for managing platform proposed block counts per epoch

use crate::platform::{FetchMany, LimitQuery, QueryStartInfo};
use crate::{Error, Sdk};
use async_trait::async_trait;
use dpp::block::epoch::EpochIndex;
use dpp::dashcore_rpc::dashcore::ProTxHash;
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
    /// optionally filtered to a specific epoch, and returning a limited number of results if
    /// specified. If start info is provided, the query will start at that key, and the
    /// `start_included` flag determines whether to include the starting proposer in the results.
    ///
    /// ## Parameters
    ///
    /// * `sdk`: A reference to the `Sdk` instance, which handles the platform interaction.
    /// * `epoch`: An optional [`EpochIndex`] to filter results to a specific epoch.
    /// * `limit`: An optional `u32` representing the maximum number of proposer block counts to retrieve.
    /// * `start_pro_tx_hash`: An optional [`QueryStartInfo`] specifying
    ///    the key to start from and whether to include the starting proposer in the results.
    ///
    /// ## Returns
    ///
    /// A `Result` containing `ProposerBlockCounts`, which is a mapping between proposers and the number of blocks they proposed,
    /// or an `Error` if the operation fails.
    ///
    /// ## See also
    ///
    /// - [`ProposerBlockCounts`](drive_proof_verifier::types::ProposerBlockCounts): The data structure holding the result of this operation.
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
