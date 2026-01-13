// #[global_allocator]
// static ALLOCATOR: talc::Talck<talc::locking::AssumeUnlockable, talc::ClaimOnOom> = unsafe {
//     use core::{mem::MaybeUninit, ptr::addr_of_mut};
//
//     const MEMORY_SIZE: usize = 128 * 1024 * 1024;
//     static mut MEMORY: [MaybeUninit<u8>; MEMORY_SIZE] = [MaybeUninit::uninit(); MEMORY_SIZE];
//     let span = talc::Span::from_array(addr_of_mut!(MEMORY));
//     let oom_handler = { talc::ClaimOnOom::new(span) };
//     talc::Talc::new(oom_handler).lock()
// };

pub mod asset_lock_proof;
pub mod block;
pub mod consensus_error;
pub mod core_script;
pub mod data_contract;
pub mod enums;
pub mod epoch;
pub mod error;
pub mod group;
pub mod identifier;
pub mod identity;
pub mod mock_bls;
pub mod platform_address;
pub mod private_key;
pub mod public_key;
pub mod serialization;
pub mod state_transitions;
pub mod tokens;
pub mod utils;
pub mod voting;

pub use core_script::CoreScriptWasm;
pub use identity::signer::IdentitySignerWasm;
pub use identity::transitions::pooling::PoolingWasm;
pub use private_key::PrivateKeyWasm;

pub use data_contract::{
    ContractBoundsWasm, DataContractCreateTransitionWasm, DataContractUpdateTransitionWasm,
    DataContractWasm, DocumentWasm, tokens_configuration_from_js_value,
};
pub use epoch::*;
pub use group::*;
pub use identity::{
    IdentityCreateTransitionWasm, IdentityCreditTransferWasm,
    IdentityCreditWithdrawalTransitionWasm, IdentityPublicKeyInCreationWasm, IdentityPublicKeyWasm,
    IdentityTopUpTransitionWasm, IdentityUpdateTransitionWasm, IdentityWasm,
    MasternodeVoteTransitionWasm, PartialIdentityWasm,
};
pub use platform_address::{
    FeeStrategyStepWasm, PlatformAddressInputWasm, PlatformAddressOutputWasm,
    PlatformAddressSignerWasm, PlatformAddressWasm, default_fee_strategy, fee_strategy_from_steps,
    fee_strategy_from_steps_or_default, outputs_to_btree_map, outputs_to_optional_btree_map,
};
pub use state_transitions::base::{GroupStateTransitionInfoWasm, StateTransitionWasm};
pub use tokens::*;
pub use voting::{
    ContenderWithSerializedDocumentWasm, ContestedDocumentVotePollWinnerInfoWasm,
    ResourceVoteChoiceWasm, ResourceVoteWasm, VotePollWasm, VoteWasm,
};
