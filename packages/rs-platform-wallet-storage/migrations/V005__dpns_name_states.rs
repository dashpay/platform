//! Add the `dpns_name_states` table (DPNS username marketplace).
//!
//! One row per tracked DPNS `domain` document belonging to (or recently
//! departed from) a wallet identity, carrying the sale state (`$price`)
//! the label-only `dpns_names` list on the identity blob cannot: document
//! id, listed price, ownership status, and the document's own timestamps.
//! Written by the marketplace sync pass and the set-price / delist /
//! purchase / transfer orchestration ops.
//!
//! All fields map to explicit columns (the entry is all-primitive), so no
//! opaque blob is needed — the row reconstructs directly. `counterparty_id`
//! carries the buyer/recipient for `sold` / `transferred` rows and is NULL
//! for `owned` rows (the status enum's payload flattened into a column).

pub fn migration() -> String {
    "CREATE TABLE dpns_name_states (
        wallet_id BLOB NOT NULL,
        document_id BLOB NOT NULL,
        identity_id BLOB NOT NULL,
        label TEXT NOT NULL,
        normalized_label TEXT NOT NULL,
        normalized_parent_domain TEXT NOT NULL,
        price INTEGER CHECK (price IS NULL OR price >= 0),
        status TEXT NOT NULL CHECK (status IN ('owned', 'sold', 'transferred')),
        counterparty_id BLOB,
        created_at_ms INTEGER,
        updated_at_ms INTEGER,
        transferred_at_ms INTEGER,
        last_synced_at_ms INTEGER NOT NULL,
        PRIMARY KEY (wallet_id, document_id),
        CHECK ((status = 'owned') = (counterparty_id IS NULL)),
        FOREIGN KEY (wallet_id) REFERENCES wallet_metadata(wallet_id) ON DELETE CASCADE
    );"
    .to_string()
}
