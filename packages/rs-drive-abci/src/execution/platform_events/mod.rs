/// Methods occurring at the finalization of a block
pub(in crate::execution) mod block_end;
/// Block fee processing
pub(in crate::execution) mod block_fee_processing;
/// Events happening what starting to process a block
pub(in crate::execution) mod block_start;
/// Update from core such as a masternode list update or quorums being updated
pub(in crate::execution) mod core_based_updates;
/// Verify the chain lock
pub(in crate::execution) mod core_chain_lock;
/// Instant lock methods
pub(in crate::execution) mod core_instant_send_lock;
/// Core subsidy
pub(in crate::execution) mod core_subsidy;
/// Epoch based methods
pub(in crate::execution) mod epoch;
/// Fee pools module
pub(in crate::execution) mod fee_pool_inwards_distribution;
/// Fee pool outwards distribution towards proposers
pub(in crate::execution) mod fee_pool_outwards_distribution;
/// Initialization
pub(in crate::execution) mod initialization;
/// Protocol upgrade events
pub(in crate::execution) mod protocol_upgrade;
/// State transition processing
pub(in crate::execution) mod state_transition_processing;
/// Voting
pub(in crate::execution) mod voting;
/// Withdrawal methods
pub(in crate::execution) mod withdrawals;
