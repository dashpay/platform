/// Module for verifying address funds branch chunk queries
pub mod verify_address_funds_branch_query;
/// Module for verifying address funds trunk chunk queries
pub mod verify_address_funds_trunk_query;
/// Module for verifying address info (balance and nonce) for a single address
pub mod verify_address_info;
/// Module for verifying address infos (balances and nonces) for multiple addresses
pub mod verify_addresses_infos;
/// Module for verifying compacted address balance changes
pub mod verify_compacted_address_balance_changes;
/// Module for verifying recent address balance changes
pub mod verify_recent_address_balance_changes;

/// The most blocks one recent address balance changes query answers with.
/// The query handler proves at most this many and the proof verifier checks
/// a proof against the same limit, so a proved page with fewer blocks is the
/// end of the recent zone at its height.
pub const RECENT_ADDRESS_BALANCE_CHANGES_QUERY_LIMIT: u16 = 100;

/// The most compacted ranges one compacted address balance changes query
/// answers with (kept small to bound the proof size). The query handler
/// proves at most this many and the proof verifier checks a proof against
/// the same limit, so a proved page with fewer ranges is the end of the
/// compacted zone at its height.
pub const COMPACTED_ADDRESS_BALANCE_CHANGES_QUERY_LIMIT: u16 = 25;
