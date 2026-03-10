/// Shield transition action
pub mod shield;
/// Shield from asset lock transition action
pub mod shield_from_asset_lock;
/// Shielded transfer transition action
pub mod shielded_transfer;
/// Shielded withdrawal transition action
pub mod shielded_withdrawal;
/// Unshield transition action
pub mod unshield;

use dpp::shielded::SerializedAction;

/// One note from an Orchard action: the three per-action fields that travel together.
#[derive(Debug, Clone)]
pub struct ShieldedActionNote {
    /// Nullifier (needed for Rho derivation in trial decryption)
    pub nullifier: [u8; 32],
    /// Note commitment (cmx value)
    pub cmx: [u8; 32],
    /// Encrypted note ciphertext
    pub encrypted_note: Vec<u8>,
}

impl From<&SerializedAction> for ShieldedActionNote {
    fn from(action: &SerializedAction) -> Self {
        ShieldedActionNote {
            nullifier: action.nullifier,
            cmx: action.cmx,
            encrypted_note: action.encrypted_note.clone(),
        }
    }
}
