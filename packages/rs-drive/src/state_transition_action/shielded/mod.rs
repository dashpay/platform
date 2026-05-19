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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shielded_action_note_fields() {
        let note = ShieldedActionNote {
            nullifier: [0xAA; 32],
            cmx: [0xBB; 32],
            encrypted_note: vec![1, 2, 3, 4],
        };
        assert_eq!(note.nullifier, [0xAA; 32]);
        assert_eq!(note.cmx, [0xBB; 32]);
        assert_eq!(note.encrypted_note, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_from_serialized_action() {
        let action = SerializedAction {
            nullifier: [0x11; 32],
            rk: [0x00; 32],
            cmx: [0x22; 32],
            encrypted_note: vec![0xAB, 0xCD],
            cv_net: [0x00; 32],
            spend_auth_sig: [0x00; 64],
        };
        let note = ShieldedActionNote::from(&action);
        assert_eq!(note.nullifier, [0x11; 32]);
        assert_eq!(note.cmx, [0x22; 32]);
        assert_eq!(note.encrypted_note, vec![0xAB, 0xCD]);
    }

    #[test]
    fn test_from_serialized_action_empty_encrypted_note() {
        let action = SerializedAction {
            nullifier: [0xFF; 32],
            rk: [0x00; 32],
            cmx: [0x00; 32],
            encrypted_note: vec![],
            cv_net: [0x00; 32],
            spend_auth_sig: [0x00; 64],
        };
        let note = ShieldedActionNote::from(&action);
        assert_eq!(note.nullifier, [0xFF; 32]);
        assert!(note.encrypted_note.is_empty());
    }

    #[test]
    fn test_clone() {
        let note = ShieldedActionNote {
            nullifier: [0xAA; 32],
            cmx: [0xBB; 32],
            encrypted_note: vec![5, 6, 7],
        };
        let cloned = note.clone();
        assert_eq!(cloned.nullifier, note.nullifier);
        assert_eq!(cloned.cmx, note.cmx);
        assert_eq!(cloned.encrypted_note, note.encrypted_note);
    }

    #[test]
    fn test_debug() {
        let note = ShieldedActionNote {
            nullifier: [0x00; 32],
            cmx: [0x00; 32],
            encrypted_note: vec![],
        };
        let debug_str = format!("{:?}", note);
        assert!(debug_str.contains("ShieldedActionNote"));
    }
}
