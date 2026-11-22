//! A state transition's status from its hash alone, proved where the network can prove it.
//!
//! [`Sdk::state_transition_status_by_hash`] asks a node what it saw of the transition
//! (`getRecentStateTransitionStatus`, unproved). When the node says the transition is in a block,
//! it then reads that block with its Tenderdash block proof (`getBlocks`) and looks for the
//! transition in it. Only a transition found there is reported proved: the block a validator
//! quorum committed at that height holds it, with the result shown.
//!
//! A hash is all a caller needs, which is what a write's result usually leaves at hand. A
//! transition still pending or unknown to the node cannot be proved from its hash: the proven
//! [`TransitionOutcome`](super::outcome::TransitionOutcome) reads the transition's inputs, so it
//! needs the whole transition.

use crate::platform::blocks::{Block, BlockTransaction, Blocks, BlocksQuery};
use crate::platform::transition::node_status::{
    NodeReportedError, NodeTransitionStatus, NodeTransitionView,
};
use crate::{Error, Sdk};
use rs_dapi_client::RequestSettings;

/// What the network shows of a state transition known by its hash.
///
/// Only [`TransitionStatusByHash::InBlock`] is proved. The other answers are one node's word and
/// must not drive any decision about funds or privacy; see
/// [`NodeTransitionStatus`](super::node_status::NodeTransitionStatus).
#[derive(Debug, Clone, PartialEq)]
pub enum TransitionStatusByHash {
    /// Proved: the block a validator quorum committed at `block_height` holds the transition,
    /// with the result in `transaction` (code 0 if it executed, otherwise the code of the error
    /// it paid for).
    InBlock {
        /// The height of the block.
        block_height: u64,
        /// The hash of the block.
        block_hash: [u8; 32],
        /// The block time, in milliseconds since the Unix epoch.
        block_time_ms: u64,
        /// The transition, with the result of its execution.
        transaction: Box<BlockTransaction>,
    },
    /// The node says the transition is in the block at `height`, and the block the network
    /// served for that height did not come back, so nothing proves it. Unproved; asking again
    /// later, or another node, may prove it.
    CommittedNotProved {
        /// The height of the block the node says holds it.
        height: u64,
        /// The error the node reports it failed with, unproved.
        error: Option<NodeReportedError>,
        /// The node's latest block height when it answered, unproved.
        node_height: Option<u64>,
    },
    /// The node says the transition is in the block at `height`, and the proved block at that
    /// height does not hold it: the node's claim is false.
    CommittedRefuted {
        /// The height the node named.
        height: u64,
        /// The node's latest block height when it answered, unproved.
        node_height: Option<u64>,
    },
    /// The node says the transition is in its mempool. Unproved.
    Pending {
        /// The node's latest block height when it answered, unproved.
        node_height: Option<u64>,
    },
    /// The node knows nothing of the transition. Unproved: it may be in another node's mempool,
    /// have been dropped, or never have reached this node.
    Unknown {
        /// The node's latest block height when it answered, unproved.
        node_height: Option<u64>,
    },
}

impl TransitionStatusByHash {
    /// Whether the answer is proved by a block a validator quorum signed.
    pub fn is_proved(&self) -> bool {
        matches!(self, TransitionStatusByHash::InBlock { .. })
    }
}

impl Sdk {
    /// The status of the state transition with this hash: proved in a block when the node says
    /// it is committed and the block the network serves at that height, proved, holds it;
    /// otherwise what the node says, unproved. See [`TransitionStatusByHash`].
    ///
    /// Needs an SDK that verifies proofs, and a network at protocol version 15 or later, which
    /// serves both queries. For shielded and private document transitions, ask only the node
    /// the transition was broadcast to (see
    /// [`node_status`](super::node_status) for why).
    pub async fn state_transition_status_by_hash(
        &self,
        state_transition_hash: [u8; 32],
        settings: Option<RequestSettings>,
    ) -> Result<TransitionStatusByHash, Error> {
        if !self.prove() {
            return Err(Error::Config(
                "a proved transition status needs an SDK that verifies proofs".to_string(),
            ));
        }
        let view = self
            .recent_state_transition_status(state_transition_hash, settings)
            .await?;
        let block = match view.status {
            NodeTransitionStatus::Committed { height, .. } => {
                let Blocks(blocks) = self
                    .fetch_blocks_with_settings(
                        BlocksQuery {
                            start_height: height,
                            count: 1,
                        },
                        settings.unwrap_or_default(),
                    )
                    .await?;
                blocks.into_iter().find(|block| block.height == height)
            }
            NodeTransitionStatus::Pending | NodeTransitionStatus::Unknown => None,
        };
        Ok(status_from(view, block, &state_transition_hash))
    }
}

/// Judge a node's view against the proved block at the height it named, if one came back.
fn status_from(
    view: NodeTransitionView,
    block: Option<Block>,
    state_transition_hash: &[u8; 32],
) -> TransitionStatusByHash {
    let node_height = view.node_height;
    match view.status {
        NodeTransitionStatus::Committed { height, error } => match block {
            Some(block) => match block
                .transactions
                .into_iter()
                .find(|transaction| &transaction.hash == state_transition_hash)
            {
                Some(transaction) => TransitionStatusByHash::InBlock {
                    block_height: block.height,
                    block_hash: block.hash,
                    block_time_ms: block.time_ms,
                    transaction: Box::new(transaction),
                },
                None => TransitionStatusByHash::CommittedRefuted {
                    height,
                    node_height,
                },
            },
            None => TransitionStatusByHash::CommittedNotProved {
                height,
                error,
                node_height,
            },
        },
        NodeTransitionStatus::Pending => TransitionStatusByHash::Pending { node_height },
        NodeTransitionStatus::Unknown => TransitionStatusByHash::Unknown { node_height },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
    use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
    use dpp::state_transition::StateTransition;

    const HASH: [u8; 32] = [7; 32];

    fn transaction(hash: [u8; 32], code: u32) -> BlockTransaction {
        BlockTransaction {
            state_transition: StateTransition::IdentityCreditTransfer(
                IdentityCreditTransferTransition::V0(IdentityCreditTransferTransitionV0::default()),
            ),
            hash,
            code,
            result_data: vec![],
            gas_used: 0,
        }
    }

    fn block(height: u64, transactions: Vec<BlockTransaction>) -> Block {
        Block {
            height,
            hash: [9; 32],
            time_ms: 1_000,
            core_chain_locked_height: 1,
            app_hash: [0; 32],
            proposer_pro_tx_hash: [0; 32],
            transactions,
        }
    }

    fn committed(height: u64) -> NodeTransitionView {
        NodeTransitionView {
            status: NodeTransitionStatus::Committed {
                height,
                error: None,
            },
            node_height: Some(height + 2),
        }
    }

    #[test]
    fn should_prove_a_transition_the_named_block_holds() {
        let status = status_from(
            committed(5),
            Some(block(
                5,
                vec![transaction([1; 32], 0), transaction(HASH, 40100)],
            )),
            &HASH,
        );
        assert_eq!(
            status,
            TransitionStatusByHash::InBlock {
                block_height: 5,
                block_hash: [9; 32],
                block_time_ms: 1_000,
                transaction: Box::new(transaction(HASH, 40100)),
            }
        );
        assert!(status.is_proved());
    }

    #[test]
    fn should_refute_a_committed_claim_the_proved_block_does_not_hold() {
        let status = status_from(
            committed(5),
            Some(block(5, vec![transaction([1; 32], 0)])),
            &HASH,
        );
        assert_eq!(
            status,
            TransitionStatusByHash::CommittedRefuted {
                height: 5,
                node_height: Some(7),
            }
        );
        assert!(!status.is_proved());
    }

    #[test]
    fn should_leave_a_committed_claim_unproved_when_no_block_came_back() {
        let status = status_from(committed(5), None, &HASH);
        assert_eq!(
            status,
            TransitionStatusByHash::CommittedNotProved {
                height: 5,
                error: None,
                node_height: Some(7),
            }
        );
    }

    #[test]
    fn should_pass_pending_and_unknown_through_unproved() {
        for (status, expected) in [
            (
                NodeTransitionStatus::Pending,
                TransitionStatusByHash::Pending {
                    node_height: Some(3),
                },
            ),
            (
                NodeTransitionStatus::Unknown,
                TransitionStatusByHash::Unknown {
                    node_height: Some(3),
                },
            ),
        ] {
            let view = NodeTransitionView {
                status,
                node_height: Some(3),
            };
            assert_eq!(status_from(view, None, &HASH), expected);
        }
    }

    #[tokio::test]
    async fn should_refuse_an_sdk_that_does_not_verify_proofs() {
        let sdk = crate::SdkBuilder::new_mock()
            .with_proofs(false)
            .build()
            .expect("build a mock SDK");
        let error = sdk
            .state_transition_status_by_hash(HASH, None)
            .await
            .expect_err("proofs are off");
        assert!(matches!(error, Error::Config(_)), "got {error:?}");
    }
}
