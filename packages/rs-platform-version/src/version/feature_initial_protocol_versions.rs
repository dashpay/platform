use crate::version::ProtocolVersion;

pub const ADDRESS_FUNDS_INITIAL_PROTOCOL_VERSION: ProtocolVersion = 11;
pub const SHIELDED_POOL_INITIAL_PROTOCOL_VERSION: ProtocolVersion = 12;
/// `ShieldFromIdentity` (identity balance to shielded pool) activates with protocol version 14.
pub const SHIELD_FROM_IDENTITY_INITIAL_PROTOCOL_VERSION: ProtocolVersion = 14;
/// `IdentityTopUpFromShieldedPool` (shielded pool to an existing identity's balance) activates with
/// protocol version 14.
pub const IDENTITY_TOP_UP_FROM_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION: ProtocolVersion = 14;
/// `IdentityKeyLimitsUpdate` (raising the budget or extending the expiry of an authentication
/// key) activates with protocol version 14.
pub const IDENTITY_KEY_LIMITS_UPDATE_INITIAL_PROTOCOL_VERSION: ProtocolVersion = 14;
/// `ContractUserModeration` (banning and suspending identities on a moderated data contract)
/// activates with protocol version 14.
pub const CONTRACT_USER_MODERATION_INITIAL_PROTOCOL_VERSION: ProtocolVersion = 14;

/// The protocol version that introduces document action fees and the `ContractFeeClaim` state
/// transition, which pays out the fee pots they collect in.
pub const CONTRACT_FEE_CLAIM_INITIAL_PROTOCOL_VERSION: ProtocolVersion = 14;

/// The app-connect system contract is written to state by the upgrade to protocol
/// version 14 and registered at genesis from that version on; below it the contract does
/// not exist and lookups must report it absent.
pub const APP_CONNECT_CONTRACT_INITIAL_PROTOCOL_VERSION: ProtocolVersion = 14;
/// Token shielded pools (per-token Orchard pools behind `TokenConfigurationV1::has_shielded_pool`,
/// with the batch token transitions that shield, unshield, transfer, mint, burn, claim and
/// purchase into or out of them) activate with protocol version 15.
pub const TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION: ProtocolVersion = 15;
