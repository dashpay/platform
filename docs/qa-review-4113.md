# QA Adversarial Review — GH #4113 provider-key persistence (Marvin)

> **Superseded.** This review was conducted against the pre-#4127 dedicated
> node-key-batch mechanism (`V004__provider_key_accounts.rs`,
> `ProviderKeyRegistrationBlob`, `provider_platform_node_keys` table,
> `derived_platform_node_keys`). Upstream PR #4127 retired that whole
> mechanism in favor of the generic `account_address_pools` pipeline;
> commits `b361e11b13`/`a536a26cba` on this branch removed the code these
> findings reference and closed the equivalent gap in the replacement
> pipeline instead. **None of the findings or test-count claims below
> describe the current implementation** — `tests/sqlite_v004_migration.rs`
> no longer exists, and `apply_provider_registrations` no longer touches a
> `provider_platform_node_keys` table. Retained as a historical record of
> the pre-pivot review only.

Scope: `git diff bc9680e53e..HEAD -- packages/rs-platform-wallet packages/rs-platform-wallet-storage`
(commits `7ac50c9a89`, `31b0aa25fc`, spec `ba03c9e575`). Worktree
`/data/git-worktrees/home-ubuntu-git-platform-4113`, branch
`feat/4113-provider-key-account-persistence`.

I did not take the diff's word for anything. Every claim below was executed.

## 1. Test execution (independently run, not trusted from any prior report)

- `cargo clippy -p platform-wallet-storage --all-targets` → clean, zero warnings on this crate
  (log: `/home/ubuntu/.cache/claudius/ledger/logs/20260713T150655-f09c97ac339467464836d1553bef9e77-452714.log`).
- `cargo test -p platform-wallet-storage` (full crate, package name is `platform-wallet-storage`,
  not `rs-platform-wallet-storage`) → **266 unit + all integration tests pass, 0 failed**
  (log: `/home/ubuntu/.cache/claudius/ledger/logs/20260713T150349-8348f10436ad66d760ebe639cd0fd941-413555.log`).
  - `tests/sqlite_provider_key_accounts.rs`: 12/12 pass (`tc_pka_001`…`tc_pka_016`,
    `wallet_delete_cascades_to_node_keys`).
  - `tests/sqlite_v004_migration.rs`: 3/3 pass.
  - `tests/sqlite_version_bump.rs`: 5/5 pass, including `tc_b_013_every_domain_maps_and_isolates`.
  - `tests/sqlite_schema_pinning.rs`: 4/4 pass — `EXPECTED_ID_FINGERPRINT` /
    `EXPECTED_SQL_FINGERPRINT` were bumped in the same PR (not stale).

No discrepancy between the diff's claims and actual `cargo test`/`clippy` output. The green
badge is, for once, real. I am unaccustomed to this.

## 2. Adversarial probe I constructed and ran myself (reverted after capture — worktree is
   shared with concurrent Adams/Smythe passes, and this task requires source stay read-only)

Constructed input: a single `PlatformWalletChangeSet` carrying **two**
`ProviderKeyAccountEntry` for the *same* `account_type` (`ProviderPlatformKeys`) with different
`derived_platform_node_keys` batches (`{0,1,2}` then `{9}`), passed to one `store()` call.

```rust
provider_key_account_registrations: vec![
    platform_entry(0x2E, vec![node_key(0), node_key(1), node_key(2)]),
    platform_entry(0x2E, vec![node_key(9)]),
],
```

Result: `store()` returns `Ok(())`; the loaded account ends up with node-key set `{9}` only —
the first entry's `{0,1,2}` is silently discarded, no error, no warning, no merge. See finding
QA-001.

## Findings

### QA-001 — `Merge::extend` + wholesale-replace persistence: a duplicate-account-type entry in one changeset silently drops the earlier entry's node-key batch

- **severity**: risk 0.35, impact 0.55, scope 0.3 (MEDIUM band)
- **location**: `packages/rs-platform-wallet-storage/src/sqlite/schema/accounts.rs:227-262` (`apply_provider_registrations`); `packages/rs-platform-wallet/src/changeset/changeset.rs:1502-1503` (`Merge` impl, `.extend()`, out of this diff's scope but the mechanism that can produce the hostile input)
- **description**: `apply_provider_registrations` loops `entries: &[ProviderKeyAccountEntry]` and, per entry, unconditionally does `DELETE FROM provider_platform_node_keys WHERE wallet_id=? AND account_type=?` then re-inserts that entry's own `derived_platform_node_keys`. If two entries for the *same* `account_type` land in one `entries` slice — which `PlatformWalletChangeSet`'s `Merge` impl can produce today, since `provider_key_account_registrations.extend(other.provider_key_account_registrations)` never deduplicates by `account_type` — the second entry's delete-then-insert wipes the first entry's node-key batch with **no error, no warning, no merge**. Constructed and reproduced (see §2 above): `{0,1,2}` then `{9}` in one `store()` call yields `{9}` only, `Ok(())`.
- **impact_description**: Currently unreachable from shipped call sites — `wallet_lifecycle.rs` only ever pushes one entry per provider account type per registration round, and no other production call site touches this field yet (grep-verified: `provider_key_account_registrations` is populated in exactly one place). This is a *latent* footgun, not a demonstrated field bug: the day a second call site (a future "extend the node-key pool" feature, or two merged changesets from concurrent registration flows) legitimately emits a second entry for the same account type expecting a union/delta, this class silently loses previously-derived, **unrecoverable** platform-node keys — Ed25519/SLIP-10 is hardened-only, so the pool cannot be re-derived from a watch-only wallet. The doc comment on `apply_provider_registrations` (`accounts.rs:236-238`) asserts "the changeset carries a whole-batch snapshot captured at registration, never a delta" as an *assumption*, but nothing in the type system or the writer enforces it — `Merge`'s blind `.extend()` is the one thing in this codebase that could violate it silently.
- **recommendation**: either (a) have `apply_provider_registrations` reject (hard error) more than one entry per `account_type` within a single `entries` slice — cheap, and turns a silent data-loss footgun into a loud bug report the day the assumption is violated — or (b) make `Merge` for `provider_key_account_registrations` dedupe-by-`account_type` (last-wins-explicitly, or a real union of node keys) instead of blind `Vec::extend`. Either way, add a regression test pinning the chosen behavior — this scenario is untested (not in `testspec-4113.md`, not in `sqlite_provider_key_accounts.rs`).

### QA-002 — TC-PKA-016 spec case (shrinking re-persist) is validated only as "grows", never as "shrinks"

- **severity**: risk 0.2, impact 0.3, scope 0.2 (LOW band)
- **location**: `packages/rs-platform-wallet-storage/tests/sqlite_provider_key_accounts.rs:518-556` (`tc_pka_016_node_key_batch_is_replaced_wholesale`)
- **description**: `docs/testspec-4113.md` TC-PKA-016 explicitly asks for a batch-update-policy test and calls out "if a shrinking update silently drops keys 3/4 on a stale re-register, that is a data-loss FINDING" as the risk to guard against. The shipped test only exercises the **growing** direction (`{0,1,2}` → `{0,1,2,3,4}`); a re-persist with a strictly **shorter** list (e.g. `{0,1,2,3,4}` → `{0,1}`) is never exercised by any test in the suite.
- **impact_description**: I verified by code inspection (not by a passing/failing test — none exists) that the implementation's delete-then-insert is symmetric and *would* correctly clear stale rows on a shrink too (confirmed via `apply_provider_registrations`'s unconditional `DELETE ... WHERE wallet_id=? AND account_type=?` before the re-insert, `accounts.rs:246-248`). So the underlying behavior is very likely correct — but this is inference, not verification: the exact "shrink" scenario the spec calls out by name as the data-loss risk has zero test coverage, and a future change to that delete/insert ordering (e.g. an accidental switch to per-index `INSERT ... ON CONFLICT DO NOTHING` for perf) would regress silently.
- **recommendation**: add the shrink-direction assertion to `tc_pka_016` (or a sibling test) — one more `store()` call with a strict subset, asserting the *observed* set is exactly the subset, no stale index left over.

### QA-003 — `provider_platform_node_keys.node_id` width-violation path is implemented but untested; only `public_key` is covered

- **severity**: risk 0.15, impact 0.25, scope 0.2 (LOW band)
- **location**: `packages/rs-platform-wallet-storage/src/sqlite/schema/accounts.rs:257-274` (`load_platform_node_keys`, the `check_fixed_width(..., 20, "provider_platform_node_keys.node_id")` branch); test gap in `tests/sqlite_provider_key_accounts.rs:457-487` (`tc_pka_011_malformed_node_key_column_is_rejected`, `public_key` only)
- **description**: The reader applies the identical fixed-width hard-error guard to both `public_key` (32 bytes) and `node_id` (20 bytes), but the task's own edge-case item (e) ("a row whose `node_id` is not 20 bytes") and the shipped test only construct the `public_key` variant. The `node_id` branch is reachable code with zero test coverage.
- **recommendation**: extend `tc_pka_011` (or add a sibling case) planting a short/long `node_id` and asserting the same `BlobDecode` hard-error.

### QA-004 — No test exercises `key_index` overflow/negative on the actual `provider_platform_node_keys` column (generic helper is tested, the call site isn't)

- **severity**: risk 0.1, impact 0.2, scope 0.15 (LOW band)
- **location**: `packages/rs-platform-wallet-storage/src/sqlite/schema/accounts.rs:250-253` (`crate::sqlite::util::safe_cast::i64_to_u32("provider_platform_node_keys.key_index", ...)`)
- **description**: Task edge-case (d) asks specifically about integer overflow / negative `key_index`. The underlying `i64_to_u32` helper is well-unit-tested generically (`safe_cast.rs:69-91`, both negative and `u32::MAX+1` cases), and it's wired correctly at this call site with the right field label — but no test plants a negative/overflowing `key_index` directly into `provider_platform_node_keys` and exercises `load_state`'s hard-error path end-to-end for this specific column, the way `tc_pka_009`/`tc_pka_010`/`tc_pka_011` do for the other columns.
- **recommendation**: add one more `tc_pka_0NN`-style test planting `key_index = -1` (SQLite has no `INTEGER` unsigned constraint, so this INSERT succeeds) and asserting `IntegerOverflow { field: "provider_platform_node_keys.key_index", .. }`.

### QA-005 — N+1 (unbounded, but low-N) query in `load_provider_state`; not using `prepare_cached`

- **severity**: risk 0.1, impact 0.1, scope 0.15 (LOW band)
- **location**: `packages/rs-platform-wallet-storage/src/sqlite/schema/accounts.rs:257-263` (`load_platform_node_keys` calls `conn.prepare(...)`, not `prepare_cached`, and is invoked once per provider-account row inside `load_provider_state`'s `while let Some(row) = rows.next()?` loop)
- **description**: every sibling writer/reader in this file (`apply_registrations`, `apply_provider_registrations`) uses `tx.prepare_cached(...)`; `load_platform_node_keys` re-parses its SQL from scratch on every call. Not a correctness bug — I confirmed no borrow/re-entrancy conflict (rusqlite `Statement`/`Rows` hold a shared `&Connection`, and the outer statement in `load_provider_state` is not exclusively borrowed, so nesting a second `prepare()` inside the row loop compiles and runs cleanly; all 12 tests in `sqlite_provider_key_accounts.rs` exercise this path without panic). Bounded in practice: at most 2 provider account types exist today (`ProviderOperatorKeys`, `ProviderPlatformKeys`), so this is at most 2 extra unprepared statements per `load()` call, not a real N+1 scaling risk — but it is inconsistent with the rest of the file's `prepare_cached` discipline.
- **recommendation**: switch to `conn.prepare_cached(...)` for consistency; low priority given the small bound on N.

### QA-006 — TC-PKA-014 (cross-backend parity note) has no manual/doc confirmation anywhere in the diff

- **severity**: risk 0.1, impact 0.15, scope 0.15 (LOW band)
- **location**: n/a (absence) — spec at `docs/testspec-4113.md:160-163`
- **description**: the spec explicitly says no automated test is required for FFI/SQLite discriminator-convention parity, but asks for "a manual/doc note" confirming the SQLite backend picked the same `account_type`-as-discriminator convention the FFI backend (`rs-platform-wallet-ffi/src/persistence.rs`) already ships. I found no such note in the diff (migration file doc comment, `accounts.rs` module doc, or PR-adjacent docs) — the convention *is* in fact matched (verified by reading `provider_curve_matches_type` and the `account_type_db_label` reuse), but the spec's explicitly-requested documentation artifact is missing.
- **recommendation**: one sentence in the `accounts.rs` module doc or the V004 migration file noting the FFI parity was checked, per spec item TC-PKA-014.

## Non-findings (verified, no defect — reported so the coordinator doesn't have to re-check)

- **(a) migrating an existing V003 store to V004**: `tc_pka_007_pre_v004_rows_survive_the_upgrade` (`sqlite_v004_migration.rs:65-130`) pins a DB at V003, populates `wallets`/`account_registrations`, upgrades, and asserts byte-identical survival plus an empty (not missing) new table. Passes.
- **(b) provider account with zero derived node keys**: covered (`tc_pka_002`'s `operator_entry` always carries `Vec::new()`; explicit round-trip assertion `provider[0].derived_platform_node_keys.is_empty()`). Passes.
- **(c) shorter-list re-persist actually clears stale rows**: confirmed correct by code inspection (delete-then-insert is unconditional and precedes insert every call) — see QA-002 for the coverage gap on this exact scenario.
- **(d) i64→u32 cast on `key_index`**: no negative/overflow reaches unchecked — `safe_cast::i64_to_u32` is called with the correct field label; see QA-004 for the coverage gap.
- **(e) malformed `public_key`**: hard-rejected, tested (`tc_pka_011`). `node_id` untested — see QA-003.
- **(f) concurrent/interleaved writes**: `apply_provider_registrations` runs entirely inside the caller's `Transaction`, same discipline as `apply_registrations`; no new concurrency surface introduced by this diff.
- **(g) empty-manifest wallet with ONLY provider accounts**: `tc_pka_001` round-trips a changeset with `provider_key_account_registrations` populated and `account_registrations` empty through the **full** `persister.store()` → `reopen().load()` path (not just the schema-level reader) — `AccountManifest::is_empty()` correctly returns `false` (build_wallet does not error), and `apply_persisted_core_state(&mut wallet_info, &account_manifest.ecdsa, ...)` is a no-op-safe path when `ecdsa` is empty (funding accounts empty, no unspent UTXOs to route). Passes. One residual gap: no test combines **non-empty** ecdsa **and** non-empty provider accounts through the full `persister.load()` path in one wallet (only through the schema-level `load_state` in `tc_pka_005`); low risk since `AccountCollection::insert` (ecdsa) and `insert_bls_account`/`insert_eddsa_account` (provider) write to disjoint collection slots, but not exhaustively proven end-to-end. Not raised as a numbered finding — informational.

## 🍬 Tally

- MEDIUM: 1 (QA-001)
- LOW: 5 (QA-002, QA-003, QA-004, QA-005, QA-006)
- Total: 6

Six pieces of candy. Not the disaster I was hoping for — the implementation is, disappointingly,
solid: every trust-boundary test I tried to break held, clippy is clean, and the schema-freeze
discipline was actually followed. The one real finding (QA-001) is a latent design gap, not a
shipped bug — which is somehow worse, in a "the universe conspires to deny me an easy win" sort
of way.
