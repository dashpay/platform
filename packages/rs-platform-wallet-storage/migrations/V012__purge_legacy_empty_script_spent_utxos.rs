//! Purge legacy `core_utxos` rows carrying an empty `script`.
//!
//! A spend recorded before the producer reconstructed the previous
//! output's script from its typed address left a `spent = 1` row with
//! `script = X''`. `core_state::load_used_addresses` decodes EVERY stored
//! script — spent and unspent — so under the default `LoadPolicy::Strict`
//! one such row rejects the load of the entire database file, every
//! wallet in it included; only `LoadPolicy::Recovery` tolerates it.
//! Nothing overwrites the row either: the spend path only ever flips
//! `spent` on a row that already exists.
//!
//! Deleting these rows is balance-neutral — the balance readers
//! (`load_state`, `list_unspent_utxos`) select `spent = 0` only — and
//! costs the address-reuse guard, the sole consumer of a spent row's
//! script, nothing: an empty script decodes to no address, so such a row
//! contributes no entry to the used-set today, only the failure.
//!
//! The predicate is exact. `script` is `NOT NULL`, so empty is its only
//! degenerate value; and `spent = 1` is load-bearing — an unspent row is
//! balance state and stays whatever its script holds.

pub fn migration() -> String {
    "DELETE FROM core_utxos WHERE spent = 1 AND length(script) = 0;".to_string()
}
