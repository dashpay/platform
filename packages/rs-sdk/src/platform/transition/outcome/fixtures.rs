//! Transitions for the tests, with a distinct anchor and distinct nullifiers for each pool, so a
//! bundle read in the wrong pool, or with the other bundle's anchor, shows.

use dpp::identifier::Identifier;
use dpp::shielded::SerializedAction;
use dpp::state_transition::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;
use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
use dpp::state_transition::token_purchase_from_shielded_pool_transition::v0::TokenPurchaseFromShieldedPoolTransitionV0;
use dpp::state_transition::token_purchase_from_shielded_pool_transition::TokenPurchaseFromShieldedPoolTransition;
use dpp::state_transition::token_shielded_transfer_with_shielded_fee_transition::v0::TokenShieldedTransferWithShieldedFeeTransitionV0;
use dpp::state_transition::token_shielded_transfer_with_shielded_fee_transition::TokenShieldedTransferWithShieldedFeeTransition;
use dpp::state_transition::token_unshield_with_shielded_fee_transition::v0::TokenUnshieldWithShieldedFeeTransitionV0;
use dpp::state_transition::token_unshield_with_shielded_fee_transition::TokenUnshieldWithShieldedFeeTransition;
use dpp::state_transition::StateTransition;

/// The credit pool anchor a credit-only spend uses.
pub(super) const ANCHOR: [u8; 32] = [0xA1; 32];
/// A credit pool anchor recorded after [`ANCHOR`] and after [`FEE_ANCHOR`].
pub(super) const NEWER_ANCHOR: [u8; 32] = [0xB2; 32];
/// The nullifiers a credit-only spend records.
pub(super) const NULLIFIERS: [[u8; 32]; 2] = [[1; 32], [2; 32]];

/// The token whose pool the token bundles use.
pub(super) const TOKEN_ID: Identifier = Identifier::new([0x70; 32]);
/// The credit pool anchor the fee bundles use.
pub(super) const FEE_ANCHOR: [u8; 32] = [0xFA; 32];
/// The token pool anchor the token bundles use.
pub(super) const TOKEN_ANCHOR: [u8; 32] = [0x7A; 32];
/// The nullifiers the fee bundles record in the credit pool.
pub(super) const FEE_NULLIFIERS: [[u8; 32]; 2] = [[0xF1; 32], [0xF2; 32]];
/// The nullifiers the token bundles record in the token pool.
pub(super) const TOKEN_NULLIFIERS: [[u8; 32]; 2] = [[0x71; 32], [0x72; 32]];

fn action(nullifier: [u8; 32]) -> SerializedAction {
    SerializedAction {
        nullifier,
        rk: [2; 32],
        cmx: [3; 32],
        encrypted_note: vec![4; 216],
        cv_net: [5; 32],
        spend_auth_sig: [6; 64],
    }
}

fn actions(nullifiers: [[u8; 32]; 2]) -> Vec<SerializedAction> {
    nullifiers.into_iter().map(action).collect()
}

/// A credit pool transfer spending [`NULLIFIERS`] against [`ANCHOR`].
pub(super) fn shielded_transfer() -> StateTransition {
    StateTransition::ShieldedTransfer(ShieldedTransferTransition::V0(
        ShieldedTransferTransitionV0 {
            actions: actions(NULLIFIERS),
            value_balance: 1_000,
            anchor: ANCHOR,
            proof: vec![8; 100],
            binding_signature: [9; 64],
        },
    ))
}

/// A token pool transfer whose fee is paid from the credit pool.
pub(super) fn token_transfer() -> StateTransition {
    StateTransition::TokenShieldedTransferWithShieldedFee(
        TokenShieldedTransferWithShieldedFeeTransition::V0(
            TokenShieldedTransferWithShieldedFeeTransitionV0 {
                data_contract_id: Identifier::new([0x60; 32]),
                token_contract_position: 0,
                token_id: TOKEN_ID,
                token_actions: actions(TOKEN_NULLIFIERS),
                token_anchor: TOKEN_ANCHOR,
                token_proof: vec![8; 100],
                token_binding_signature: [9; 64],
                fee_actions: actions(FEE_NULLIFIERS),
                fee_anchor: FEE_ANCHOR,
                fee_proof: vec![8; 100],
                fee_binding_signature: [9; 64],
                credit_amount: 1_000,
            },
        ),
    )
}

/// A token pool unshield whose fee is paid from the credit pool.
pub(super) fn token_unshield() -> StateTransition {
    StateTransition::TokenUnshieldWithShieldedFee(TokenUnshieldWithShieldedFeeTransition::V0(
        TokenUnshieldWithShieldedFeeTransitionV0 {
            data_contract_id: Identifier::new([0x60; 32]),
            token_contract_position: 0,
            token_id: TOKEN_ID,
            recipient_id: Identifier::new([0x61; 32]),
            amount: 5,
            token_actions: actions(TOKEN_NULLIFIERS),
            token_anchor: TOKEN_ANCHOR,
            token_proof: vec![8; 100],
            token_binding_signature: [9; 64],
            fee_actions: actions(FEE_NULLIFIERS),
            fee_anchor: FEE_ANCHOR,
            fee_proof: vec![8; 100],
            fee_binding_signature: [9; 64],
            credit_amount: 1_000,
        },
    ))
}

/// A token purchase paid from the credit pool, minting an outputs-only token bundle.
pub(super) fn token_purchase() -> StateTransition {
    StateTransition::TokenPurchaseFromShieldedPool(TokenPurchaseFromShieldedPoolTransition::V0(
        TokenPurchaseFromShieldedPoolTransitionV0 {
            data_contract_id: Identifier::new([0x60; 32]),
            token_contract_position: 0,
            token_id: TOKEN_ID,
            token_count: 5,
            total_agreed_price: 500,
            token_actions: actions(TOKEN_NULLIFIERS),
            token_anchor: TOKEN_ANCHOR,
            token_proof: vec![8; 100],
            token_binding_signature: [9; 64],
            fee_actions: actions(FEE_NULLIFIERS),
            fee_anchor: FEE_ANCHOR,
            fee_proof: vec![8; 100],
            fee_binding_signature: [9; 64],
            credit_amount: 1_000,
        },
    ))
}
