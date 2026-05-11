//! Defensive `BEFORE UPDATE` triggers (SEC-003 from the Phase-2.8
//! triage report).
//!
//! V001 emulates `INSERT` parent-existence checks and `AFTER DELETE`
//! cascade via triggers. It does NOT install `BEFORE UPDATE` triggers
//! on the parent's primary-key column or on the composite-FK column of
//! child tables. The persister's own write path never updates those
//! columns, but if a future migration accidentally introduces such an
//! UPDATE, the result is silent orphaning of child rows.
//!
//! This migration installs `BEFORE UPDATE OF wallet_id` triggers on
//! `wallet_metadata` and `BEFORE UPDATE OF identity_id` triggers on
//! `identity_keys` and `dashpay_profiles`. Each raises
//! `RAISE(ABORT, 'FOREIGN KEY constraint failed')` — the same idiom
//! V001 uses for the parent-existence check, so downstream string
//! matching stays stable.
//!
//! V001 remains untouched (append-only migration policy).

pub fn migration() -> String {
    let mut sql = String::new();
    sql.push_str(
        "CREATE TRIGGER IF NOT EXISTS reject_wallet_metadata_id_update \
         BEFORE UPDATE OF wallet_id ON wallet_metadata \
         FOR EACH ROW \
         WHEN NEW.wallet_id IS NOT OLD.wallet_id \
         BEGIN \
            SELECT RAISE(ABORT, 'FOREIGN KEY constraint failed'); \
         END;\n",
    );
    sql.push_str(
        "CREATE TRIGGER IF NOT EXISTS reject_identity_keys_identity_id_update \
         BEFORE UPDATE OF identity_id ON identity_keys \
         FOR EACH ROW \
         WHEN NEW.identity_id IS NOT OLD.identity_id \
         BEGIN \
            SELECT RAISE(ABORT, 'FOREIGN KEY constraint failed'); \
         END;\n",
    );
    sql.push_str(
        "CREATE TRIGGER IF NOT EXISTS reject_dashpay_profiles_identity_id_update \
         BEFORE UPDATE OF identity_id ON dashpay_profiles \
         FOR EACH ROW \
         WHEN NEW.identity_id IS NOT OLD.identity_id \
         BEGIN \
            SELECT RAISE(ABORT, 'FOREIGN KEY constraint failed'); \
         END;\n",
    );
    sql
}
