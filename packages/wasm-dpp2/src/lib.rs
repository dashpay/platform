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
pub mod batch;
pub mod consensus_error;
pub mod contract_bounds;
pub mod core_script;
pub mod data_contract;
pub mod data_contract_transitions;
pub mod document;
pub mod encrypted_note;
pub mod enums;
pub mod error;
pub mod group_state_transition_info;
pub mod identifier;
pub mod identity;
pub mod identity_public_key;
pub mod identity_transitions;
pub mod masternode_vote;
pub mod mock_bls;
pub mod partial_identity;
pub mod private_key;
pub mod public_key;
pub mod state_transition;
pub mod token_configuration;
pub mod token_configuration_change_item;
pub mod utils;
