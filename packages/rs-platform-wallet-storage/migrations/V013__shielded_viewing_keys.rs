//! Persist Orchard full viewing keys by wallet and shielded account.

pub fn migration() -> String {
    "\
CREATE TABLE shielded_viewing_keys (
    wallet_id BLOB NOT NULL,
    account_index INTEGER NOT NULL CHECK (account_index BETWEEN 0 AND 4294967295),
    viewing_key BLOB NOT NULL,
    PRIMARY KEY (wallet_id, account_index),
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
);
"
    .to_string()
}
