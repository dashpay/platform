//! Rewrite persisted domain labels so they name their live SQL tables.
//!
//! Taking `MAX(seq)` on collision preserves the monotonic cache-invalidation invariant.

pub fn migration() -> String {
    "\
INSERT INTO meta_data_versions (wallet_id, domain, seq)
SELECT wallet_id, 'wallets', seq FROM meta_data_versions WHERE domain = 'wallet_metadata'
ON CONFLICT(wallet_id, domain) DO UPDATE
  SET seq = MAX(meta_data_versions.seq, excluded.seq);
DELETE FROM meta_data_versions WHERE domain = 'wallet_metadata';

INSERT INTO meta_data_versions (wallet_id, domain, seq)
SELECT wallet_id, 'core_address_pool', seq FROM meta_data_versions WHERE domain = 'account_address_pools'
ON CONFLICT(wallet_id, domain) DO UPDATE
  SET seq = MAX(meta_data_versions.seq, excluded.seq);
DELETE FROM meta_data_versions WHERE domain = 'account_address_pools';
"
    .to_string()
}
