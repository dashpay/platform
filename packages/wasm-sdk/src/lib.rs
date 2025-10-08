use wasm_bindgen::prelude::wasm_bindgen;

pub mod context_provider;
pub mod dpns;
pub mod error;
pub mod logging;
pub mod queries;
pub mod sdk;
pub mod state_transitions;
pub mod wallet;

// Re-export commonly used items
pub use dpns::*;
pub use error::{WasmSdkError, WasmSdkErrorKind};
pub use queries::*;
pub use state_transitions::identity as state_transition_identity;
pub use wallet::*;
pub use wasm_dpp2::*;

// pub use wasm_dpp2::asset_lock_proof::{
//     chain::ChainAssetLockProofWasm, instant::InstantAssetLockProofWasm, outpoint::OutPointWasm,
//     AssetLockProofWasm,
// };
// pub use wasm_dpp2::consensus_error::ConsensusErrorWasm;
// pub use wasm_dpp2::core_script::CoreScriptWasm;
// pub use wasm_dpp2::data_contract::{
//     ContractBoundsWasm, DataContractCreateTransitionWasm, DataContractUpdateTransitionWasm,
//     DataContractWasm, DocumentWasm,
// };
// pub use wasm_dpp2::enums::{
//     batch::batch_enum::BatchTypeWasm,
//     batch::gas_fees_paid_by::GasFeesPaidByWasm,
//     contested::vote_state_result_type::VoteStateResultTypeWasm,
//     keys::{key_type::KeyTypeWasm, purpose::PurposeWasm, security_level::SecurityLevelWasm},
//     lock_types::AssetLockProofTypeWasm,
//     network::NetworkWasm,
//     platform::PlatformVersionWasm,
//     token::{
//         action_goal::ActionGoalWasm, distribution_type::TokenDistributionTypeWasm,
//         emergency_action::TokenEmergencyActionWasm,
//     },
//     withdrawal::PoolingWasm,
// };
// pub use wasm_dpp2::error::{WasmDppError, WasmDppErrorKind};
// pub use wasm_dpp2::identifier::IdentifierWasm;
// pub use wasm_dpp2::identity::{
//     IdentityCreateTransitionWasm, IdentityCreditTransferWasm,
//     IdentityCreditWithdrawalTransitionWasm, IdentityPublicKeyInCreationWasm, IdentityPublicKeyWasm,
//     IdentityTopUpTransitionWasm, IdentityUpdateTransitionWasm, IdentityWasm,
//     MasternodeVoteTransitionWasm, PartialIdentityWasm, ResourceVoteChoiceWasm, VotePollWasm,
//     VoteWasm,
// };
// pub use wasm_dpp2::private_key::PrivateKeyWasm;
// pub use wasm_dpp2::public_key::PublicKeyWasm;
// pub use wasm_dpp2::state_transitions::{
//     base::{GroupStateTransitionInfoWasm, StateTransitionWasm},
//     batch::{batched_transition::BatchedTransitionWasm, BatchTransitionWasm},
// };
// pub use wasm_dpp2::tokens::configuration::action_taker::ActionTakerWasm;
// pub use wasm_dpp2::tokens::configuration::change_control_rules::ChangeControlRulesWasm;
// pub use wasm_dpp2::tokens::{
//     AuthorizedActionTakersWasm, GroupWasm, PrivateEncryptedNoteWasm, SharedEncryptedNoteWasm,
//     TokenConfigurationChangeItemWasm, TokenConfigurationLocalizationWasm, TokenConfigurationWasm,
// };

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen(start)]
pub async fn start() -> Result<(), WasmSdkError> {
    console_error_panic_hook::set_once();

    Ok(())
}
