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
pub mod core;
pub mod data_contract;
pub mod enums;
pub mod epoch;
pub mod error;
pub mod group;
pub mod identifier;
pub mod identity;
pub mod mock_bls;
pub mod platform_address;
pub mod public_key;
pub mod serialization;
pub mod state_transitions;
pub mod tokens;
pub mod utils;
pub mod version;
pub mod voting;

pub use core::core_script::CoreScriptWasm;
pub use core::network::{NetworkLikeJs, NetworkWasm};
pub use core::private_key::PrivateKeyWasm;
pub use core::pro_tx_hash::{
    ProTxHashLikeArrayJs, ProTxHashLikeJs, ProTxHashLikeNullableJs, ProTxHashWasm,
};
pub use identity::signer::IdentitySignerWasm;
pub use identity::transitions::pooling::PoolingWasm;

pub use data_contract::{
    ContractBoundsWasm, DataContractCreateTransitionWasm, DataContractUpdateTransitionWasm,
    DataContractWasm, DocumentWasm, tokens_configuration_from_js_value,
};
pub use epoch::*;
pub use group::*;
pub use identifier::{
    IdentifierLikeArrayJs, IdentifierLikeJs, IdentifierLikeOrUndefinedJs, IdentifierWasm,
};
pub use identity::{
    IdentityCreateTransitionWasm, IdentityCreditTransferWasm,
    IdentityCreditWithdrawalTransitionWasm, IdentityPublicKeyInCreationWasm,
    IdentityPublicKeyOptionsJs, IdentityPublicKeyWasm, IdentityTopUpTransitionWasm,
    IdentityUpdateTransitionWasm, IdentityWasm, MasternodeVoteTransitionWasm, PartialIdentityWasm,
    PublicKeyHashLikeJs, public_key_hash_from_js,
};
pub use platform_address::{
    FeeStrategyStepWasm, PlatformAddressInputWasm, PlatformAddressLikeArrayJs,
    PlatformAddressLikeJs, PlatformAddressOutputWasm, PlatformAddressSignerWasm,
    PlatformAddressWasm, default_fee_strategy, fee_strategy_from_steps,
    fee_strategy_from_steps_or_default, outputs_to_btree_map, outputs_to_optional_btree_map,
};
pub use platform_address::transitions::{
    AddressCreditWithdrawalTransitionWasm, AddressFundingFromAssetLockTransitionWasm,
    AddressFundsTransferTransitionWasm, IdentityCreateFromAddressesTransitionWasm,
    IdentityCreditTransferToAddressesTransitionWasm, IdentityTopUpFromAddressesTransitionWasm,
};
pub use state_transitions::base::{GroupStateTransitionInfoWasm, StateTransitionWasm};
pub use state_transitions::proof_result::{StateTransitionProofResultTypeJs, convert_proof_result};
pub use tokens::*;
pub use version::{PlatformVersionLikeJs, PlatformVersionWasm};
pub use voting::{
    ContenderWithSerializedDocumentWasm, ContestedDocumentVotePollWinnerInfoWasm,
    ResourceVoteChoiceWasm, ResourceVoteWasm, VotePollWasm, VoteWasm,
};
