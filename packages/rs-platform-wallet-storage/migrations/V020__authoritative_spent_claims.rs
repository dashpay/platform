//! Keep engine claims separate from UI/history sweep stamps.

pub fn migration() -> String {
    "ALTER TABLE wallets ADD COLUMN spent_claims_complete INTEGER NOT NULL DEFAULT 0 CHECK (spent_claims_complete IN (0, 1));
    CREATE TABLE core_spent_claims (
        wallet_id BLOB NOT NULL,
        outpoint BLOB NOT NULL,
        claimant BLOB CHECK (claimant IS NULL OR length(claimant) = 32),
        PRIMARY KEY (wallet_id, outpoint),
        FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
    );".to_owned()
}
