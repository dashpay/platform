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
