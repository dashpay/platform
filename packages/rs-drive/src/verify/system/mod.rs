mod verify_elements;
mod verify_epoch_infos;
mod verify_epoch_proposers;
mod verify_total_credits_in_system;
mod verify_upgrade_state;
mod verify_upgrade_vote_status;

// Re-export verify functions as standalone functions
pub use crate::drive::Drive;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::block::epoch::EpochIndex;
use dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dpp::fee::Credits;
use dpp::prelude::CoreBlockHeight;
use dpp::util::deserializer::ProtocolVersion;
use dpp::version::PlatformVersion;
use grovedb::Element;
use nohash_hasher::IntMap;
use std::collections::BTreeMap;

/// Wrapper for Drive::verify_elements
pub fn verify_elements(
    proof: &[u8],
    path: Vec<Vec<u8>>,
    keys: Vec<Vec<u8>>,
    platform_version: &PlatformVersion,
) -> Result<(RootHash, BTreeMap<Vec<u8>, Option<Element>>), Error> {
    Drive::verify_elements(proof, path, keys, platform_version)
}

/// Wrapper for Drive::verify_epoch_infos
pub fn verify_epoch_infos(
    proof: &[u8],
    current_epoch: EpochIndex,
    start_epoch: Option<EpochIndex>,
    count: u16,
    ascending: bool,
    platform_version: &PlatformVersion,
) -> Result<(RootHash, Vec<ExtendedEpochInfo>), Error> {
    Drive::verify_epoch_infos(
        proof,
        current_epoch,
        start_epoch,
        count,
        ascending,
        platform_version,
    )
}

/// Wrapper for Drive::verify_total_credits_in_system
pub fn verify_total_credits_in_system(
    proof: &[u8],
    core_subsidy_halving_interval: u32,
    request_activation_core_height: impl Fn() -> Result<CoreBlockHeight, Error>,
    current_core_height: CoreBlockHeight,
    platform_version: &PlatformVersion,
) -> Result<(RootHash, Credits), Error> {
    Drive::verify_total_credits_in_system(
        proof,
        core_subsidy_halving_interval,
        request_activation_core_height,
        current_core_height,
        platform_version,
    )
}

/// Wrapper for Drive::verify_upgrade_state
pub fn verify_upgrade_state(
    proof: &[u8],
    platform_version: &PlatformVersion,
) -> Result<(RootHash, IntMap<ProtocolVersion, u64>), Error> {
    Drive::verify_upgrade_state(proof, platform_version)
}

/// Wrapper for Drive::verify_upgrade_vote_status
pub fn verify_upgrade_vote_status(
    proof: &[u8],
    start_protx_hash: Option<[u8; 32]>,
    count: u16,
    platform_version: &PlatformVersion,
) -> Result<(RootHash, BTreeMap<[u8; 32], ProtocolVersion>), Error> {
    Drive::verify_upgrade_vote_status(proof, start_protx_hash, count, platform_version)
}
