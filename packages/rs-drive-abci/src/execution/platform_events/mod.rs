/// Storage of the ephemeral state
pub(in crate::execution) mod store_ephemeral_state;
/// Initialization
/// Init chain is called from abci
pub(crate) mod initialization;
/// Update masternode identities
pub(in crate::execution) mod update_masternode_identities;
/// Functions related to IdentityCreditWithdrawalTransaction
pub(in crate::execution) mod identity_credit_withdrawal;
/// Fee pools module
pub(in crate::execution) mod fee_pool_inwards_distribution;
/// Block fee processing
pub(in crate::execution) mod block_fee_processing;
/// Fee pool outwards distribution towards proposers
pub(in crate::execution) mod fee_pool_outwards_distribution;
/// Core subsidy
pub(in crate::execution) mod core_subsidy;
/// Validator set update
pub(in crate::execution) mod validator_set_update;
/// Protocol upgrade events
pub(in crate::execution) mod protocol_upgrade;
/// State transition processing
pub(in crate::execution) mod state_transition_processing;
/// Update from core such as a masternode list update or quorums being updated
pub(in crate::execution)mod core_based_updates;
