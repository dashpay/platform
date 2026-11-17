//! A stored coin's flags, checked against its creating transaction's row.
//!
//! A store that keeps each coin's `is_confirmed` / `is_instantlocked` /
//! `height` next to the transaction rows can hold the two apart: the
//! transaction row says the transaction is in a block, the coin row still
//! says it is not. A coin is in a block exactly when its creating transaction
//! is, so the transaction row is the evidence; without it the engine restores
//! the coin as unconfirmed and no later event repairs it, because the block
//! was already processed.

use key_wallet::Utxo;

/// What a store's row for a coin's creating transaction says about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoredParentState {
    /// No row, a placeholder row, or a transaction last seen in the mempool.
    Unconfirmed,
    /// The transaction is InstantSend-locked.
    InstantLocked,
    /// The transaction is in the block at `height`.
    InBlock {
        /// Height of the block holding the transaction.
        height: u32,
    },
}

/// Bring a restored coin's flags up to what its creating transaction's row
/// says. Only raises: a coin is never made less final than its own row.
///
/// - Transaction in a block → the coin is confirmed at that block's height
///   (a coin already confirmed keeps its own height unless it has none).
/// - Transaction InstantSend-locked → the coin is InstantSend-locked (a lock
///   is permanent for its transaction).
///
/// A block above `processed_height` (the last block this wallet processed)
/// is not taken: the transaction row is keyed by txid alone, so on a device
/// with several wallets it can come from a wallet that has synced further,
/// and this wallet records the transaction itself when it reaches the block.
pub fn reconcile_with_parent(utxo: &mut Utxo, parent: StoredParentState, processed_height: u32) {
    match parent {
        StoredParentState::Unconfirmed => {}
        StoredParentState::InstantLocked => utxo.is_instantlocked = true,
        StoredParentState::InBlock { height } if height > 0 && height <= processed_height => {
            if !utxo.is_confirmed || utxo.height == 0 {
                utxo.is_confirmed = true;
                utxo.height = height;
            }
        }
        StoredParentState::InBlock { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::{Address, Network, OutPoint, TxOut};

    fn coin(is_confirmed: bool, height: u32) -> Utxo {
        let address = Address::dummy(Network::Testnet, 0);
        let mut utxo = Utxo::new(
            OutPoint::null(),
            TxOut {
                value: 1_000,
                script_pubkey: address.script_pubkey(),
            },
            address,
            height,
            false,
        );
        utxo.is_confirmed = is_confirmed;
        utxo
    }

    #[test]
    fn should_confirm_a_coin_whose_transaction_is_in_a_block() {
        let mut utxo = coin(false, 0);
        reconcile_with_parent(&mut utxo, StoredParentState::InBlock { height: 500 }, 1_000);
        assert!(utxo.is_confirmed);
        assert_eq!(utxo.height, 500);
    }

    #[test]
    fn should_lock_a_coin_whose_transaction_is_instant_locked() {
        let mut utxo = coin(false, 0);
        reconcile_with_parent(&mut utxo, StoredParentState::InstantLocked, 1_000);
        assert!(utxo.is_instantlocked);
        assert!(!utxo.is_confirmed);
    }

    #[test]
    fn should_leave_a_coin_alone_when_its_transaction_is_unconfirmed() {
        let mut utxo = coin(false, 0);
        reconcile_with_parent(&mut utxo, StoredParentState::Unconfirmed, 1_000);
        assert!(!utxo.is_confirmed);
        assert!(!utxo.is_instantlocked);
    }

    #[test]
    fn should_keep_a_confirmed_coins_own_height() {
        let mut utxo = coin(true, 400);
        reconcile_with_parent(&mut utxo, StoredParentState::InBlock { height: 500 }, 1_000);
        assert!(utxo.is_confirmed);
        assert_eq!(utxo.height, 400);
    }

    #[test]
    fn should_give_a_confirmed_coin_without_a_height_its_transactions_height() {
        let mut utxo = coin(true, 0);
        reconcile_with_parent(&mut utxo, StoredParentState::InBlock { height: 500 }, 1_000);
        assert!(utxo.is_confirmed);
        assert_eq!(utxo.height, 500);
    }

    #[test]
    fn should_keep_a_locked_coin_locked_under_an_unconfirmed_transaction() {
        let mut utxo = coin(false, 0);
        utxo.is_instantlocked = true;
        reconcile_with_parent(&mut utxo, StoredParentState::Unconfirmed, 1_000);
        assert!(utxo.is_instantlocked);
    }

    #[test]
    fn should_not_confirm_from_a_block_this_wallet_has_not_processed() {
        let mut utxo = coin(false, 0);
        reconcile_with_parent(&mut utxo, StoredParentState::InBlock { height: 500 }, 499);
        assert!(!utxo.is_confirmed);
        assert_eq!(utxo.height, 0);
    }

    #[test]
    fn should_not_confirm_from_a_block_row_without_a_height() {
        let mut utxo = coin(false, 0);
        reconcile_with_parent(&mut utxo, StoredParentState::InBlock { height: 0 }, 1_000);
        assert!(!utxo.is_confirmed);
    }
}
