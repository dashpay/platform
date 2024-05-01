mod v0;

#[allow(unused_imports)] // Removing causes build failures; yet clippy insists it's unused
pub(in crate::identity::state_transition::asset_lock_proof::instant) use v0::validate_instant_asset_lock_proof_structure_v0;
