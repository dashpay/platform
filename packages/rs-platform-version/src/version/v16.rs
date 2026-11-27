use crate::version::drive_versions::v11::DRIVE_VERSION_V11;
use crate::version::protocol_version::PlatformVersion;
use crate::version::system_data_contract_versions::v5::SYSTEM_DATA_CONTRACT_VERSIONS_V5;
use crate::version::v15::PLATFORM_V15;
use crate::version::ProtocolVersion;

pub const PROTOCOL_VERSION_16: ProtocolVersion = 16;

/// Introduced as the activation gate for the consensus changes of the 5.2 line.
///
/// Every table this version does not name is protocol version 15's (`..PLATFORM_V15`), so a
/// change made in place to an unreleased protocol version 15 table reaches this version too.
/// Each change that needs this gate lands in its own follow-up, adds a numbered note here and
/// names the table it bumps in `PLATFORM_V16` with a `// changed:` comment.
///
/// v16 hosts these consensus changes:
///
/// 1. **Token marketplace state**: the token marketplace starts with this version
///    (`TOKEN_MARKETPLACE_INITIAL_PROTOCOL_VERSION`). This version admits tradeable tokens
///    (`marketplaceRules.tradeMode` `Tradeable`) and creates the escrow trees and the token
///    market contract that listings and offers will use; the transitions that place and fill
///    orders come in follow-ups. Protocol v15 and earlier hold none of it (v15 note 17).
///    - A token configuration, or a configuration update, that makes a token tradeable is
///      admitted; before this version it is refused unpaid as not active.
///    - `create_initial_state_structure` 5 (`DRIVE_VERSION_V11`) creates the two escrow trees at
///      genesis, and the upgrade to this version (`transition_to_version_16`) inserts them
///      through the same helper, `Drive::insert_token_order_escrow_trees`.
///    - `calculate_total_tokens_balance` 2 counts the escrowed tokens in the token
///      conservation check, and the escrow methods (`order_escrow`) are active
///      (`DRIVE_TOKEN_METHOD_VERSIONS_V3`).
///    - The escrow keys sit deeper in their layers than the keys beside them, so the cost
///      estimators of the balances, the token balances, the prefunded specialized balances and
///      the contract fee pots keep a deeper description of those layers (`DRIVE_VERSION_V11`).
///    - The token market contract v1 (`token_market: 1`, `SYSTEM_DATA_CONTRACT_VERSIONS_V5`) is
///      registered at genesis and stored without storage flags by the upgrade. Its `ask` and
///      `bid` types hold the open orders.
///
/// 2. **ML_DSA_65 identity keys**: a key type whose signatures stay secure against a quantum
///    computer, the ML-DSA-65 scheme of FIPS 204, exists from this version
///    (`ML_DSA_65_KEY_INITIAL_PROTOCOL_VERSION`). It names no table of its own: the key type is
///    refused while a transition is decoded, so every method reads it only from this version on.
///    - `KeyType::ML_DSA_65` is 5. Its data is the 1,952 byte public key, stored whole, and its
///      signature is 3,309 bytes over the signable bytes themselves, with the empty context
///      string. Like an ECDSA_SECP256K1 or a BLS12_381 key it belongs to one identity and is
///      found by its hash.
///    - A transition registering one (`IdentityCreate`, `IdentityUpdate`,
///      `IdentityCreateFromAddresses`, `IdentityCreateFromShieldedPool`) is not active before
///      this version and is refused unpaid there (`StateTransitionNotActiveError`, 10603), as
///      software that cannot decode the key type refuses it.
///    - Key structure validation 1 refuses key data that is not 1,952 bytes
///      (`InvalidIdentityPublicKeyDataError`, 10511). A proof of possession that does not verify
///      is `BasicMLDSAError` (20019), and a transition signature that does not verify is
///      `InvalidStateTransitionSignatureError` (20002), as for the other key types.
///    - Checking one signature costs `verify_signature_ml_dsa_65`, 100,000 credits, a new entry
///      of the signature fee table that no earlier version reads.
pub const PLATFORM_V16: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_16,
    drive: DRIVE_VERSION_V11, // changed: create_initial_state_structure 5 creates the token order escrow trees; the token methods of the marketplace and the deeper escrow layer estimates (note 1)
    system_data_contracts: SYSTEM_DATA_CONTRACT_VERSIONS_V5, // changed: the token market contract v1 (note 1)
    ..PLATFORM_V15
};
