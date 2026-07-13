# Test Case Specification — GH #4113: persist `provider_key_account_registrations`

Scope: `packages/rs-platform-wallet-storage` (SQLite persister). Fixes the
`let _ = provider_key_account_registrations;` drop in
`src/sqlite/schema/versions.rs:112` so `ProviderKeyAccountEntry` rows (BLS
operator-key / EdDSA platform-node-key accounts) survive `store()` →
`load()`, on par with `AccountRegistrationEntry` via `account_registrations`
(`src/sqlite/schema/accounts.rs`).

This is a **specification only** — no test code is written here. IDs use
prefix `TC-PKA-` (Provider Key Account), continuing the crate's `TC-<area>-<seq>`
convention (cf. `TC-B-0NN` in `tests/sqlite_version_bump.rs`,
`tests/sqlite_schema_pinning.rs`).

## Reference material (not requirements, orientation only)

- Changeset types: `packages/rs-platform-wallet/src/changeset/changeset.rs:1075-1144`
  (`ProviderKeyExtendedPubKey`, `ProviderPlatformNodePubKey`, `ProviderKeyAccountEntry`).
- Drop site: `packages/rs-platform-wallet-storage/src/sqlite/schema/versions.rs:106-112`,
  comment explicitly cites issue #4113.
- Sibling pattern to mirror: `packages/rs-platform-wallet-storage/src/sqlite/schema/accounts.rs`
  (`apply_registrations`, `load_state`, fail-hard column/blob cross-check, sealed
  `PersistableBlob` blob encoding).
- FFI reference implementation (already ships the BLS/EdDSA split, informs
  cross-backend parity): `packages/rs-platform-wallet-ffi/src/persistence.rs`
  — `build_account_specs_for_callback` (~2453-2529) encodes
  `ProviderKeyExtendedPubKey::Bls`/`EdDSA` into the same `account_xpub_bytes`
  slot `AccountRegistrationEntry` uses; the restore side
  (~3041-3092) discriminates the decode **by `account_type`**
  (`ProviderOperatorKeys` → BLS, `ProviderPlatformKeys` → EdDSA), not by an
  extra tag byte inside the blob. The SQLite fix should use the same
  discriminator: the existing `account_type` column/label already carries
  this information (`account_type_db_label`, `accounts.rs:243-269`, labels
  `"provider_operator"` / `"provider_platform"` already reserved in
  `ACCOUNT_TYPE_LABELS`).
- Trust-boundary pattern to replicate: `accounts.rs` tests
  `load_state_rejects_account_type_column_mismatch`,
  `load_state_rejects_account_index_column_mismatch` (lines 358-432).
- No-secret-material invariant tests to replicate/extend:
  `tests/secrets_scan.rs` (substring scan over `src/sqlite/schema/` +
  `migrations/`) and the sealed `PersistableBlob` trait
  (`src/sqlite/schema/blob.rs:14-33`).
- Schema-freeze golden fingerprints to update:
  `tests/sqlite_schema_pinning.rs` (`EXPECTED_ID_FINGERPRINT`,
  `EXPECTED_SQL_FINGERPRINT`, `tc_b_040_*`).
- Domain/version-bump wiring: `src/sqlite/schema/versions.rs` (`Domain` enum,
  `Domain::ALL`, `touched_domains`) and its coverage test
  `tc_b_013_every_domain_maps_and_isolates` in `tests/sqlite_version_bump.rs`
  (lines 283-303) — a new domain must appear in both.
- Apply-side wiring point: `src/sqlite/persister.rs:1168-1227`
  (`apply_changeset_to_tx`, currently calls
  `schema::accounts::apply_registrations` at line 1177 — the new provider-key
  writer belongs alongside it) and the read side at `persister.rs:891-944`
  (`load()`, currently calls `schema::accounts::load_state` at line 933).
- `bls`/`eddsa` are default-on features of `rs-platform-wallet`
  (`packages/rs-platform-wallet/Cargo.toml:114-116`), pulled in transitively
  by `rs-platform-wallet-storage`'s `platform-wallet` dependency (no
  `default-features = false`), so `ProviderKeyAccountEntry` variants are
  reachable in the storage crate's default test build without extra feature
  flags.

## Preconditions common to all cases

- In-memory SQLite connection migrated via
  `crate::sqlite::migrations::run` (mirrors `accounts.rs::migrated_conn()`),
  or a `SqlitePersister` opened against a tempdir (mirrors
  `tests/sqlite_version_bump.rs::fresh_persister`).
  Prior to the fix, `store()` never writes provider-key data at all —
  every round-trip case below is expected to **fail against current
  `main`/branch HEAD** and pass only once the fix lands.
- A `wallets` row exists for the test `wallet_id` (FK requirement, per
  existing test setup pattern).
- Test fixture BLS/EdDSA xpubs: deterministic seed → `Wallet` →
  `ProviderOperatorKeys`/`ProviderPlatformKeys` account derivation, mirroring
  `packages/rs-platform-wallet/src/wallet/provider_key_at_index.rs` test
  helpers, rather than hand-rolled byte literals (BLS/EdDSA extended keys
  have no widely-available test-vector analogous to `test_xpub()`'s BIP-32
  mainnet vector).

---

### TC-PKA-001 — Round-trip: BLS operator + EdDSA platform accounts survive store→load
**Requirement**: item 1 (round-trip), GH #4113 primary fix.
**Preconditions**: fresh persister, wallet registered.
**Steps**:
1. Build a `PlatformWalletChangeSet` with `provider_key_account_registrations` containing two entries: one `ProviderOperatorKeys`/`Bls(xpub_a)` with empty `derived_platform_node_keys`, one `ProviderPlatformKeys`/`EdDSA(xpub_b)` with a non-empty `derived_platform_node_keys` (see TC-PKA-004 for the multi-key case; here a single key suffices).
2. `persister.store(wallet_id, cs)`.
3. Drop the in-memory state; re-open/re-query via a public read API for provider-key accounts (whatever the fix names it, e.g. `schema::provider_accounts::load_state`, or via `PlatformWalletPersistence::load()` → `ClientStartState`/account manifest, per how the fix wires it into `persister.rs:933`).
**Expected**:
- Exactly 2 accounts returned for the wallet.
- The `ProviderOperatorKeys` entry decodes as `ProviderKeyExtendedPubKey::Bls` with xpub bytes equal to `xpub_a`.
- The `ProviderPlatformKeys` entry decodes as `ProviderKeyExtendedPubKey::EdDSA` with xpub bytes equal to `xpub_b`, and its `derived_platform_node_keys` contains the one seeded key with correct `index`, `public_key`, `node_id`.
- No unrelated wallet/account rows affected.

### TC-PKA-002 — Discriminated encoding: BLS decodes as BLS, not EdDSA
**Requirement**: item 2.
**Steps**: Store only a `ProviderOperatorKeys`/`Bls` entry. Load it back.
**Expected**: decoded variant is `ProviderKeyExtendedPubKey::Bls(_)`; asserting `matches!(entry.extended_public_key, ProviderKeyExtendedPubKey::EdDSA(_))` is false. Byte-for-byte xpub equality against the input BLS key.

### TC-PKA-003 — Discriminated encoding: EdDSA decodes as EdDSA, cross-type mismatch is rejected
**Requirement**: item 2, mirrors `load_state_rejects_account_type_column_mismatch` (`accounts.rs:358-393`).
**Steps**:
1. Store a valid `ProviderPlatformKeys`/`EdDSA` entry normally; confirm round-trip (positive half, same shape as TC-PKA-002 but for EdDSA).
2. Negative half: hand-craft a row (direct SQL insert, bypassing the writer) whose `account_type` column/label says `"provider_operator"` but whose blob encodes an EdDSA key (or vice versa) — the cross-type-confusion scenario the task calls out explicitly.
**Expected**: (1) passes as in TC-PKA-002 mirrored for EdDSA. (2) load hard-errors (a new `WalletStorageError` variant, e.g. `ProviderKeyAccountEntryMismatch`, matching the existing `AccountRegistrationEntryMismatch` naming convention) — it must NOT silently decode the wrong curve's bytes as if they were the other curve (a `Bls`-shaped blob decoded as `ExtendedEd25519PubKey` would either bincode-error or, worse, "succeed" on garbage — either way this must not reach the caller as an `Ok` provider account).

### TC-PKA-004 — Multiple platform-node keys per account: order and identity preserved
**Requirement**: item 3 (one-to-many).
**Steps**: Store one `ProviderPlatformKeys` entry with `derived_platform_node_keys` = 5 entries with distinct, non-sequential `index` values (e.g. 0, 1, 2, 5, 9) and distinct `public_key`/`node_id` bytes. Load back.
**Expected**:
- Exactly 5 rows returned for this account, no fewer, no more (catches silent dedup or truncation).
- Each returned key's `(index, public_key, node_id)` triplet matches its source exactly — assert on the full struct, not just count.
- Order: either the loader returns them sorted by `index` (deterministic contract, matching `load_state`'s `ORDER BY` discipline elsewhere) or, if insertion order is the contract instead, that must be verified explicitly — do not accept "same set" without confirming which ordering guarantee the implementation documents, and add a regression test pinning that choice.

### TC-PKA-005 — Empty case: no provider-key accounts round-trips cleanly
**Requirement**: item 4.
**Steps**: Store a changeset with `provider_key_account_registrations: vec![]` (and otherwise-populated unrelated fields, e.g. a standard `account_registrations` entry, to prove independence). Load back.
**Expected**: zero provider-key-account rows for the wallet; zero rows in the new child table (platform-node keys); the unrelated `account_registrations` entry is unaffected (no cross-talk). No panics, no spurious `WalletStorageError`.

### TC-PKA-006 — Migration is additive: schema-freeze fingerprints updated deliberately
**Requirement**: item 5.
**Steps**: After the fix lands (new migration file / new tables/columns), run `tests/sqlite_schema_pinning.rs::tc_b_040_identity_fingerprint_pinned` and `tc_b_040_sql_fingerprint_pinned`.
**Expected**: both `EXPECTED_ID_FINGERPRINT` and `EXPECTED_SQL_FINGERPRINT` constants are updated in the same PR to match the new migration set (a new migration file changes the identity fingerprint; any DDL in an existing file, which should NOT happen per additive-only design, would also change the SQL fingerprint). Flag as a FINDING if the PR ships with stale golden constants (test would fail) or, worse, if it edits an *existing* migration file's DDL in place instead of adding a new `V004__*.rs` (violates the "migration is additive" requirement and the refinery one-directional-migration discipline documented in `migrations.rs:74-79`).

### TC-PKA-007 — Existing V001-V003 data unaffected by the new migration
**Requirement**: item 5.
**Steps**: Build a DB pinned at the pre-fix version (e.g. via `refinery::Runner::set_target` per `migrations.rs::runner()` test helper), populate `wallets`, `account_registrations`, `core_address_pool`, etc. with representative rows, then run the new migration to bring the DB current.
**Expected**: all pre-existing rows in all pre-existing tables are byte-identical after migration (no `ALTER TABLE ... DROP/RENAME` touches them); `account_registrations.load_state` and other existing readers still return the same data they did pre-migration.

### TC-PKA-008 — `touched_domains` / `Domain` wiring: new domain isolates correctly
**Requirement**: item 5 (schema correctness) + the crate's R8 forgotten-domain guard.
**Steps**: Extend `Domain` with a new variant (e.g. `ProviderKeyAccounts`), add it to `Domain::ALL`, and extend `tc_b_013_every_domain_maps_and_isolates` (`tests/sqlite_version_bump.rs:283-303`) coverage — a `PlatformWalletChangeSet` carrying only `provider_key_account_registrations` must touch exactly the new domain and no other.
**Expected**: `touched_domains` returns `[Domain::ProviderKeyAccounts]` for such a changeset; `versions::bump_domain` fires for it inside the same flush tx (mirror `tc_b_011_bump_rides_the_flush`); a partial-failure mid-flush rolls back both the provider-key rows and the version bump together (mirror `tc_b_012_partial_failure_rolls_back_data_and_bump`). Flag as a FINDING if the destructure in `touched_domains` is left with a silent `let _ = provider_key_account_registrations;` (i.e., the bug this issue reports, merely moved rather than fixed) or if the field is dropped from the exhaustive destructure entirely (compile error is fine; silently ignoring it again is not).

### TC-PKA-009 — Trust boundary: corrupted blob hard-errors `load()`, no silent skip
**Requirement**: item 6, mirrors `load_state_rejects_account_type_column_mismatch`/`load_state_rejects_account_index_column_mismatch` (`accounts.rs:358-432`).
**Steps**: Direct-SQL-insert a provider-key row whose blob bytes are truncated/garbage (not valid bincode for either `ExtendedBLSPubKey` or `ExtendedEd25519PubKey`). Call the load path.
**Expected**: the whole `load()` call returns `Err(WalletStorageError::...)` (a decode-failure variant) — it must NOT skip just that row and return the rest of the wallet's accounts, and must NOT return an empty `Vec` as if no provider accounts existed (the fail-hard contract already enforced for `account_registrations::load_state`).

### TC-PKA-010 — Trust boundary: oversized blob is rejected before allocation
**Requirement**: item 6, mirrors `blob::check_size`/`BLOB_SIZE_LIMIT_BYTES` (16 MiB cap, `blob.rs:39-58`).
**Steps**: Direct-SQL-insert a provider-key row whose `account_xpub_bytes` (or new child-table blob column, if one exists) exceeds `BLOB_SIZE_LIMIT_BYTES`. Load.
**Expected**: `WalletStorageError::BlobTooLarge { len_bytes, limit_bytes }`, and per the existing pattern the size check must run against `length(<col>)` *before* materializing the full `Vec<u8>` (verify via a huge declared length that would OOM if naively slurped — SQLite's `length()` is O(1) so this is testable without actually allocating gigabytes; a targeted unit test on the size-check helper alone, analogous to `blob::check_size`'s existing coverage, is acceptable in lieu of an end-to-end multi-GB blob).

### TC-PKA-011 — Trust boundary: typed-column vs decoded-blob cross-check on the new table(s)
**Requirement**: item 6.
**Steps**: If the fix stores typed columns alongside the blob for the new table(s) (as `account_registrations` does for `account_type`/`account_index`/`key_class`/dashpay ids), craft a row where a typed column disagrees with the decoded blob (e.g. child-table `index` column says 3 but the decoded `ProviderPlatformNodePubKey.index` says 5).
**Expected**: hard error, not silent acceptance of either value — same discipline as `accounts.rs::load_state`'s cross-check block (lines 182-203). If the implementation carries no redundant typed columns for the child table (only a blob), this test case is N/A but that design choice must be recorded as a residual risk (a corrupted blob could shift `index` without any column to catch it against) — call this out as a FINDING if unaddressed.

### TC-PKA-012 — Invariant: no private-key material reachable via the new schema surface
**Requirement**: item 7, mirrors `tests/secrets_scan.rs` (substring scan) and the sealed `PersistableBlob` trait (`blob.rs:14-33`).
**Steps**: Confirm the new schema/writer code (`src/sqlite/schema/<new module>.rs` and any new migration file) is inside the scan roots (`src/sqlite/schema/`, `migrations/`) already covered by `tests/secrets_scan.rs::no_secret_substrings_in_schema_or_migrations`; run it. Separately, confirm any newly-`impl_persistable_blob!`-sealed type (e.g. `ProviderKeyExtendedPubKey` or a wrapper) only ever carries `Bls(ExtendedBLSPubKey)`/`EdDSA(ExtendedEd25519PubKey)` — both public-key types upstream — never a private scalar type.
**Expected**: `secrets_scan.rs` passes with zero offenders on the new files without needing a new `ALLOWLIST` entry (if one is needed, it must carry an explicit "PUBLIC material only"-style justification comment, per the existing convention). The sealed-trait `impl` for the new blob type is a reviewable, explicit line (not a blanket `impl<T: Serialize>`), so a private-key-bearing type could not silently opt in. Flag as a FINDING if the new module needs an `ALLOWLIST` entry added to suppress a genuine hit rather than because the token appears only in a "PUBLIC material only"-style disclaimer comment.

### TC-PKA-013 — Invariant: derived_platform_node_keys carries no private scalar
**Requirement**: item 7.
**Steps**: Inspect (structural, not runtime-executable as a Rust test, but specify as a documentation/type-review checklist item) that the persisted child-table row for `ProviderPlatformNodePubKey` carries exactly `{index: u32, public_key: [u8;32], node_id: [u8;20]}` and nothing else — no `private_key`/`scalar`/`secret_key` field ever added to the row shape, matching the upstream doc comment ("Only the public parts are carried — the private scalar stays resolver-gated per index", `changeset.rs:1096-1097`).
**Expected**: type-level guarantee (the persisted struct simply has no field for it) plus the secrets-scan (TC-PKA-012) as the running regression guard against a future column addition reintroducing it.

### TC-PKA-014 — Cross-backend parity (informational / manual)
**Requirement**: item 8.
**Steps**: Compare, for the same seed and account, the account rebuilt by `SqlitePersister::load()` post-fix against the account the FFI backend already rebuilds via `build_account_specs_for_callback`/`account_type_from_spec` (`rs-platform-wallet-ffi/src/persistence.rs:2453-2529`, `3041-3092`): same xpub bytes, same discriminator convention (`account_type` label decides BLS vs EdDSA, not an in-blob tag byte), same `derived_platform_node_keys` set.
**Expected**: no automated cross-crate test is required (the two backends don't share a persistence format contractually), but a manual/doc note should confirm the SQLite backend picked the *same* discriminator convention the FFI backend already ships (see Reference material above) — divergence here would be a real inconsistency (two different "correct" encodings for the same logical data) worth flagging even though it's not a functional regression on its own.

### TC-PKA-015 — Idempotent re-persist does not duplicate rows
**Requirement**: not explicitly listed by the task but directly analogous to `accounts.rs::idempotent_repersist_does_not_duplicate` (lines 549-572); `ProviderOperatorKeys`/`ProviderPlatformKeys` are index-less (always account index 0 per `accounts.rs:284-287`), so `(wallet_id, account_type)` is the natural key and a naive `INSERT` without an `ON CONFLICT` upsert will duplicate on every `store()` call touching provider keys.
**Steps**: Call `store()` twice with an identical `ProviderKeyAccountEntry` for the same account type.
**Expected**: exactly one row (account + its node-key set) after both calls, not two.

### TC-PKA-016 — Re-persist with an updated `derived_platform_node_keys` batch
**Requirement**: gap-filling for item 3; not explicit in the task, but the one-to-many child table needs a defined update policy that TC-PKA-004/015 alone don't pin down.
**Steps**: Store a `ProviderPlatformKeys` entry with node keys `{0,1,2}`. Store again for the same account with node keys `{0,1,2,3,4}` (a superset, matching how the pool is only ever *extended*, never shrunk, per the "pre-generated fixed batch" design intent).
**Expected**: whatever policy the implementation chooses (replace-whole-set vs. append-only upsert-by-index), the *observable* result after the second `store()` must be exactly `{0,1,2,3,4}` with no duplicate index rows and no key silently dropped. Pin this down as an explicit assertion — do not leave it implicit. If the implementation instead only supports strict replacement and a shrinking update silently drops keys 3/4 on a stale re-register, that is a data-loss FINDING (this account's whole point is "wallet can never re-derive keys without the seed" — losing previously-known indices here is exactly the bug class #4113 exists to prevent).

---

## Summary table

| ID | Area | Requirement item |
|---|---|---|
| TC-PKA-001 | Round-trip (BLS+EdDSA, mixed) | 1 |
| TC-PKA-002 | Discriminated encoding (BLS) | 2 |
| TC-PKA-003 | Discriminated encoding (EdDSA) + cross-type reject | 2 |
| TC-PKA-004 | One-to-many node keys, order/identity | 3 |
| TC-PKA-005 | Empty-case round-trip | 4 |
| TC-PKA-006 | Migration fingerprint discipline | 5 |
| TC-PKA-007 | Pre-existing data unaffected | 5 |
| TC-PKA-008 | `Domain`/`touched_domains` wiring | 5 |
| TC-PKA-009 | Fail-hard on corrupt blob | 6 |
| TC-PKA-010 | Fail-hard on oversized blob | 6 |
| TC-PKA-011 | Typed-column/blob cross-check | 6 |
| TC-PKA-012 | No-secret-material scan | 7 |
| TC-PKA-013 | Node-key struct shape invariant | 7 |
| TC-PKA-014 | Cross-backend parity (manual) | 8 |
| TC-PKA-015 | Idempotent re-persist | (gap) |
| TC-PKA-016 | Node-key batch update policy | (gap) |
