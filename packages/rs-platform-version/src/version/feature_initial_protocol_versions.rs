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
/// purchase into or out of them) activate with protocol version 14.
pub const TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION: ProtocolVersion = 14;
/// The moderation charters system contract is written to state by the upgrade to protocol
/// version 14 and registered at genesis from that version on; below it the contract does not
/// exist and lookups must report it absent.
pub const MODERATION_CHARTERS_CONTRACT_INITIAL_PROTOCOL_VERSION: ProtocolVersion = 14;
/// The token marketplace's state exists from protocol version 15: the escrow trees and the token
/// market system contract are created at genesis from that version on and by the upgrade to it;
/// below it the contract does not exist and lookups must report it absent. No token can use that
/// state yet: see `TOKEN_TRADE_MODE_TRADEABLE_INITIAL_PROTOCOL_VERSION`.
pub const TOKEN_MARKETPLACE_INITIAL_PROTOCOL_VERSION: ProtocolVersion = 15;
/// A tradeable token (`marketplaceRules.tradeMode` `Tradeable`, whose holders sell it to each
/// other through escrowed listings and offers) is admitted from protocol version 16, the version
/// that brings the transitions placing and filling those orders. Below it a token configuration
/// or a configuration update that makes a token tradeable is refused without a fee. That covers
/// protocol version 15, whose software decodes the mode: it has no way to trade a token, and the
/// stored form of the mode is not final until the order transitions ship, so no contract may
/// store it before then.
pub const TOKEN_TRADE_MODE_TRADEABLE_INITIAL_PROTOCOL_VERSION: ProtocolVersion = 16;
/// Yes/no masternode vote polls activate with protocol version 15: a masternode vote naming one
/// (a `YesNoVote`, or a resource vote on a `YesNoVotePoll`) is not active before it, every
/// yes/no method slot is `None` in the tables before it, and the vote decisions trees are
/// created at genesis from it (and by the upgrade to it).
pub const YES_NO_VOTE_POLL_INITIAL_PROTOCOL_VERSION: ProtocolVersion = 15;
/// The decision proposals system contract is written to state by the upgrade to protocol
/// version 15 and registered at genesis from that version on; below it the contract does not
/// exist and lookups must report it absent. Creating one of its proposals opens a yes/no vote
/// poll.
pub const DECISION_PROPOSALS_CONTRACT_INITIAL_PROTOCOL_VERSION: ProtocolVersion = 15;
/// Private document entries activate with protocol version 15: a
/// `PrivateDocumentCreateFromShieldedPool`, which appends entries to a private document type's
/// store paying from the credit shielded pool, is not active before it, as software that
/// predates it cannot decode one.
pub const PRIVATE_DOCUMENT_CREATE_FROM_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION: ProtocolVersion = 15;
