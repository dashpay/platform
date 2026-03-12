/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::shielded::shielded_withdrawal::v0::ShieldedWithdrawalTransitionActionV0;
use crate::state_transition_action::shielded::ShieldedActionNote;
use derive_more::From;
use dpp::document::Document;
use dpp::fee::Credits;
use dpp::identity::core_script::CoreScript;
use dpp::withdrawal::Pooling;

/// Shielded withdrawal transition action
#[derive(Debug, Clone, From)]
pub enum ShieldedWithdrawalTransitionAction {
    /// v0
    V0(ShieldedWithdrawalTransitionActionV0),
}

impl ShieldedWithdrawalTransitionAction {
    /// Get withdrawal amount
    pub fn amount(&self) -> Credits {
        match self {
            ShieldedWithdrawalTransitionAction::V0(transition) => transition.amount,
        }
    }
    /// Get notes
    pub fn notes(&self) -> &[ShieldedActionNote] {
        match self {
            ShieldedWithdrawalTransitionAction::V0(transition) => &transition.notes,
        }
    }
    /// Get anchor
    pub fn anchor(&self) -> &[u8; 32] {
        match self {
            ShieldedWithdrawalTransitionAction::V0(transition) => &transition.anchor,
        }
    }
    /// Get core fee per byte
    pub fn core_fee_per_byte(&self) -> u32 {
        match self {
            ShieldedWithdrawalTransitionAction::V0(transition) => transition.core_fee_per_byte,
        }
    }
    /// Get pooling strategy
    pub fn pooling(&self) -> Pooling {
        match self {
            ShieldedWithdrawalTransitionAction::V0(transition) => transition.pooling,
        }
    }
    /// Get output script
    pub fn output_script(&self) -> &CoreScript {
        match self {
            ShieldedWithdrawalTransitionAction::V0(transition) => &transition.output_script,
        }
    }
    /// Fee amount (value_balance - amount), paid to proposers
    pub fn fee_amount(&self) -> Credits {
        match self {
            ShieldedWithdrawalTransitionAction::V0(transition) => transition.fee_amount,
        }
    }
    /// Get prepared withdrawal document
    pub fn prepared_withdrawal_document(&self) -> &Document {
        match self {
            ShieldedWithdrawalTransitionAction::V0(transition) => {
                &transition.prepared_withdrawal_document
            }
        }
    }
    /// Get prepared withdrawal document owned
    pub fn prepared_withdrawal_document_owned(self) -> Document {
        match self {
            ShieldedWithdrawalTransitionAction::V0(transition) => {
                transition.prepared_withdrawal_document
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::document::DocumentV0;
    use dpp::prelude::Identifier;

    fn make_note() -> ShieldedActionNote {
        ShieldedActionNote {
            nullifier: [0x77; 32],
            cmx: [0x88; 32],
            encrypted_note: vec![0xFE, 0xED],
        }
    }

    fn make_document() -> Document {
        Document::V0(DocumentV0 {
            id: Identifier::from([0x11; 32]),
            owner_id: Identifier::from([0x22; 32]),
            properties: Default::default(),
            revision: Some(1),
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        })
    }

    fn make_action() -> ShieldedWithdrawalTransitionAction {
        let v0 = ShieldedWithdrawalTransitionActionV0 {
            amount: 25000,
            notes: vec![make_note(), make_note()],
            anchor: [0x99; 32],
            core_fee_per_byte: 42,
            pooling: Pooling::Never,
            output_script: CoreScript::from_bytes(vec![0x76, 0xA9, 0x14]),
            fee_amount: 500,
            current_total_balance: 300000,
            prepared_withdrawal_document: make_document(),
        };
        ShieldedWithdrawalTransitionAction::from(v0)
    }

    #[test]
    fn test_from_v0() {
        let action = make_action();
        assert!(matches!(
            action,
            ShieldedWithdrawalTransitionAction::V0(_)
        ));
    }

    #[test]
    fn test_amount() {
        let action = make_action();
        assert_eq!(action.amount(), 25000);
    }

    #[test]
    fn test_notes() {
        let action = make_action();
        let notes = action.notes();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].nullifier, [0x77; 32]);
        assert_eq!(notes[1].cmx, [0x88; 32]);
    }

    #[test]
    fn test_anchor() {
        let action = make_action();
        assert_eq!(*action.anchor(), [0x99; 32]);
    }

    #[test]
    fn test_core_fee_per_byte() {
        let action = make_action();
        assert_eq!(action.core_fee_per_byte(), 42);
    }

    #[test]
    fn test_pooling() {
        let action = make_action();
        assert!(matches!(action.pooling(), Pooling::Never));
    }

    #[test]
    fn test_pooling_variants() {
        let mut v0 = ShieldedWithdrawalTransitionActionV0 {
            amount: 0,
            notes: vec![],
            anchor: [0; 32],
            core_fee_per_byte: 0,
            pooling: Pooling::IfAvailable,
            output_script: CoreScript::from_bytes(vec![]),
            fee_amount: 0,
            current_total_balance: 0,
            prepared_withdrawal_document: make_document(),
        };
        let action_if_avail = ShieldedWithdrawalTransitionAction::from(v0.clone());
        assert!(matches!(action_if_avail.pooling(), Pooling::IfAvailable));

        v0.pooling = Pooling::Standard;
        let action_standard = ShieldedWithdrawalTransitionAction::from(v0);
        assert!(matches!(action_standard.pooling(), Pooling::Standard));
    }

    #[test]
    fn test_output_script() {
        let action = make_action();
        let script = action.output_script();
        assert_eq!(script.as_bytes(), &[0x76, 0xA9, 0x14]);
    }

    #[test]
    fn test_fee_amount() {
        let action = make_action();
        assert_eq!(action.fee_amount(), 500);
    }

    #[test]
    fn test_prepared_withdrawal_document_ref() {
        let action = make_action();
        let doc = action.prepared_withdrawal_document();
        match doc {
            Document::V0(v0) => {
                assert_eq!(v0.id, Identifier::from([0x11; 32]));
                assert_eq!(v0.owner_id, Identifier::from([0x22; 32]));
            }
        }
    }

    #[test]
    fn test_prepared_withdrawal_document_owned() {
        let action = make_action();
        let doc = action.prepared_withdrawal_document_owned();
        match doc {
            Document::V0(v0) => {
                assert_eq!(v0.id, Identifier::from([0x11; 32]));
            }
        }
    }

    #[test]
    fn test_clone() {
        let action = make_action();
        let cloned = action.clone();
        assert_eq!(cloned.amount(), 25000);
        assert_eq!(cloned.fee_amount(), 500);
        assert_eq!(cloned.core_fee_per_byte(), 42);
        assert_eq!(cloned.notes().len(), 2);
    }

    #[test]
    fn test_debug() {
        let action = make_action();
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("V0"));
    }
}
