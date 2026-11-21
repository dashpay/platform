//! Access paths for the typed, proof-preserving Core history rescan.

pub fn migration() -> String {
    "CREATE INDEX idx_core_utxos_funding ON core_utxos(wallet_id, substr(outpoint, 2, 32));
     CREATE INDEX idx_core_utxos_claimant ON core_utxos(wallet_id, spent_in_txid)
         WHERE spent_in_txid IS NOT NULL;"
        .to_owned()
}
