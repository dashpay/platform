//! Add `core_instant_locks.verified`: 1 on a lock row written by a build
//! that stores only verified InstantSend locks, 0 otherwise.
//!
//! A lock row makes its transaction final on load: its coins come back
//! InstantSend-locked and spendable. Rows written before this column existed
//! have unknown provenance (the wallet may have been handed a lock that was
//! never verified), so they read 0 and no longer count; such a transaction
//! loads as unconfirmed until its lock is announced again or it is mined.
//! Writing 1 is sound only while the SPV client hands the wallet nothing but
//! verified locks.
//!
//! Applied by `migrations::v023` only when the column is missing. A database
//! some build already gave this exact column keeps it and its values.

pub fn migration() -> String {
    "ALTER TABLE core_instant_locks
         ADD COLUMN verified INTEGER NOT NULL DEFAULT 0 CHECK (verified IN (0, 1));"
        .to_string()
}
