use crate::version::ProtocolVersion;

pub const ADDRESS_FUNDS_INITIAL_PROTOCOL_VERSION: ProtocolVersion = 11;
pub const SHIELDED_POOL_INITIAL_PROTOCOL_VERSION: ProtocolVersion = 12;
/// `ShieldFromIdentity` (identity balance to shielded pool) activates with protocol version 14.
pub const SHIELD_FROM_IDENTITY_INITIAL_PROTOCOL_VERSION: ProtocolVersion = 14;
/// `IdentityTopUpFromShieldedPool` (shielded pool to an existing identity's balance) activates with
/// protocol version 14.
pub const IDENTITY_TOP_UP_FROM_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION: ProtocolVersion = 14;
/// Token shielded pools (per-token Orchard pools behind `TokenConfigurationV1::has_shielded_pool`,
/// with the `TokenShield`, `TokenUnshield` and `TokenShieldedTransfer` batch transitions) activate
/// with protocol version 14.
pub const TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION: ProtocolVersion = 14;
