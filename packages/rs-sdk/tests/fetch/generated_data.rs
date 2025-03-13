use dash_sdk::platform::Identifier;
use dpp::tokens::calculate_token_id;
use std::sync::LazyLock;

/// Existing document ID
///
// TODO: this is copy-paste from drive-abci `packages/rs-sdk/tests/fetch/main.rs` where it's private,
// consider defining it in `data-contracts` crate
pub const DPNS_DASH_TLD_DOCUMENT_ID: [u8; 32] = [
    215, 242, 197, 63, 70, 169, 23, 171, 110, 91, 57, 162, 215, 188, 38, 11, 100, 146, 137, 69, 55,
    68, 209, 224, 212, 242, 106, 141, 142, 255, 55, 207,
];

/// Data contract with groups and tokens created by init chain for testing
/// See `/packages/rs-drive-abci/src/execution/platform_events/initialization/create_genesis_state/mod.rs#L49`
pub const DATA_CONTRACT_ID: Identifier = Identifier::new([3; 32]);
/// Identity used in the data contract above created by init chain for testing
/// See `/packages/rs-drive-abci/src/execution/platform_events/initialization/create_genesis_state/mod.rs#L49`
pub const IDENTITY_ID_1: Identifier = Identifier::new([1; 32]);
/// Second identity used in the data contract above created by init chain for testing
/// See `/packages/rs-drive-abci/src/execution/platform_events/initialization/create_genesis_state/mod.rs#L49`
pub const IDENTITY_ID_2: Identifier = Identifier::new([2; 32]);
/// Third identity used in the data contract above created by init chain for testing
/// See `/packages/rs-drive-abci/src/execution/platform_events/initialization/create_genesis_state/mod.rs#L49`
pub const IDENTITY_ID_3: Identifier = Identifier::new([3; 32]);
/// Token ID that doesn't exist
pub const UNKNOWN_TOKEN_ID: Identifier = Identifier::new([1; 32]);
/// Identity ID that doesn't exist
pub const UNKNOWN_IDENTITY_ID: Identifier = Identifier::new([255; 32]);
/// Group action ID that burns some tokens of the data contract above by the first identity
/// This group action is created by init chain for testing
/// See `/packages/rs-drive-abci/src/execution/platform_events/initialization/create_genesis_state/mod.rs#L49`
pub const GROUP_ACTION_ID: Identifier = Identifier::new([32; 32]);
/// The first token ID from the data contract above created by init chain for testing
/// See `/packages/rs-drive-abci/src/execution/platform_events/initialization/create_genesis_state/mod.rs#L49`
pub static TOKEN_ID_0: LazyLock<Identifier> =
    LazyLock::new(|| Identifier::new(calculate_token_id(&DATA_CONTRACT_ID.to_buffer(), 0)));
/// The second token ID from the data contract above created by init chain for testing
/// See `/packages/rs-drive-abci/src/execution/platform_events/initialization/create_genesis_state/mod.rs#L49`
pub static TOKEN_ID_1: LazyLock<Identifier> =
    LazyLock::new(|| Identifier::new(calculate_token_id(&DATA_CONTRACT_ID.to_buffer(), 1)));
/// The third token ID from the data contract above created by init chain for testing
/// See `/packages/rs-drive-abci/src/execution/platform_events/initialization/create_genesis_state/mod.rs#L49`
pub static TOKEN_ID_2: LazyLock<Identifier> =
    LazyLock::new(|| Identifier::new(calculate_token_id(&DATA_CONTRACT_ID.to_buffer(), 2)));
