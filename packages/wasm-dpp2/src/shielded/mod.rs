pub mod address_witness;
pub mod identity_create_from_shielded_pool_transition;
pub mod identity_top_up_from_shielded_pool_transition;
pub mod orchard_action;
pub mod shield_from_asset_lock_transition;
pub mod shield_from_identity_transition;
pub mod shield_transition;
pub mod shielded_transfer_transition;
pub mod shielded_withdrawal_transition;
pub mod token_purchase_from_shielded_pool_transition;
pub mod token_shielded_transfer_with_shielded_fee_transition;
pub mod token_unshield_with_shielded_fee_transition;
pub mod unshield_transition;

pub use address_witness::{AddressWitnessWasm, input_witnesses_from_js_options};
pub use identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransitionWasm;
pub use identity_top_up_from_shielded_pool_transition::IdentityTopUpFromShieldedPoolTransitionWasm;
pub use orchard_action::{SerializedOrchardActionWasm, actions_from_js_options};
pub use shield_from_asset_lock_transition::ShieldFromAssetLockTransitionWasm;
pub use shield_from_identity_transition::ShieldFromIdentityTransitionWasm;
pub use shield_transition::ShieldTransitionWasm;
pub use shielded_transfer_transition::ShieldedTransferTransitionWasm;
pub use shielded_withdrawal_transition::ShieldedWithdrawalTransitionWasm;
pub use token_purchase_from_shielded_pool_transition::TokenPurchaseFromShieldedPoolTransitionWasm;
pub use token_shielded_transfer_with_shielded_fee_transition::TokenShieldedTransferWithShieldedFeeTransitionWasm;
pub use token_unshield_with_shielded_fee_transition::TokenUnshieldWithShieldedFeeTransitionWasm;
pub use unshield_transition::UnshieldTransitionWasm;

use crate::error::WasmDppResult;
use crate::utils::try_vec_to_fixed_bytes;
use wasm_bindgen::prelude::wasm_bindgen;

/// Compute the platform sighash from an Orchard bundle commitment and extra data.
///
/// `sighash = SHA-256("DashPlatformSighash" || bundleCommitment || extraData)`
///
/// `extraData` is the exact preimage the transition's own layout defines; consensus rebuilds
/// it and compares, so a byte out of place rejects an otherwise valid bundle. Integers are
/// little endian.
///
/// - Credit pool `ShieldedTransfer`: empty.
/// - Credit pool transitions that only create notes — shield, shield from identity and shield
///   from asset lock: `kind tag (1 byte) || owner (32 bytes)`, with the tag `0x84`, `0x85` and
///   `0x86` in that order. The owner is what funds the bundle: for shield, the SHA-256 of its
///   input addresses, each as `type byte (0x00 P2PKH, 0x01 P2SH) || 20-byte hash` (not the
///   bech32m type byte), sorted by type byte then hash, nonces and amounts excluded; for shield
///   from identity, the identity id; for shield from asset lock, the double SHA-256 of the
///   locked 36-byte outpoint. Before protocol version 14 shield and shield from asset lock bind
///   nothing, so build for the network's protocol version. Such a bundle carries no anchor of
///   its own, so this data is what ties it to the transition that carries it.
/// - `Unshield`: `outputAddress || amount (u64)`.
/// - Shielded withdrawal: `outputScript || amount (u64) || coreFeePerByte (u32) ||
///   pooling (1 byte)`.
/// - Token pool transitions that only create notes — token shield, and mint, claim or direct
///   purchase into the pool: `kind tag (1 byte) || tokenId (32 bytes) || ownerId (32 bytes)`,
///   with the tag `0x80`, `0x81`, `0x82` and `0x83` in that order. `ownerId` is the batch
///   owner; for a group action mint it is the proposer, and every other signer submits the
///   proposer's bundle unchanged. Empty is rejected: such a bundle carries no anchor, every
///   token pool shares the empty-tree anchor it proves against, and nothing else in the bundle
///   says which pool, which kind or which owner it was built for.
/// - Other token pool transitions bind their own fields; see the `extra_sighash_data`
///   builders in `dpp::shielded`.
#[wasm_bindgen(js_name = computePlatformSighash)]
pub fn compute_platform_sighash_wasm(
    bundle_commitment: Vec<u8>,
    extra_data: &[u8],
) -> WasmDppResult<Vec<u8>> {
    let commitment: [u8; 32] = try_vec_to_fixed_bytes(bundle_commitment, "bundleCommitment")?;
    let result = dpp::shielded::compute_platform_sighash(&commitment, extra_data);
    Ok(result.to_vec())
}
