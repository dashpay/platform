# Asset-lock proof blob rehydration fix — design

Issue: [dashpay/platform#4133](https://github.com/dashpay/platform/issues/4133)
Crate: `packages/rs-platform-wallet-storage` (with a secondary touch in `packages/rs-platform-wallet`)
Status: design (implementation is a separate change)

## 1. Summary

An `AssetLockEntry` carrying `proof: Some(AssetLockProof)` can be written to the
`asset_locks.lifecycle_blob` column but never read back. Every wallet holding
such a row fails rehydration permanently, because the shared `_blob` codec routes
the value through `bincode::serde`, whose deserializer cannot service the
`deserialize_any` request that an internally-tagged serde enum requires.

The failure class is already known and already solved once in this crate. The fix
extends that existing, tested pattern to the one place it was omitted, and adds a
guard so the omission cannot recur silently.

## 2. Root cause — confirmed against current code

`AssetLockProof` (`packages/rs-dpp/src/identity/state_transition/asset_lock_proof/mod.rs:37-43`):

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Encode, Decode)]
#[serde(tag = "$type", rename_all = "camelCase")]
pub enum AssetLockProof {
    Instant(#[bincode(with_serde)] InstantAssetLockProof),
    Chain(#[bincode(with_serde)] ChainAssetLockProof),
}
```

Two independent codecs coexist on this type:

- **serde** — internally tagged (`#[serde(tag = "$type")]`). Its hand-written
  `Deserialize` (via `RawAssetLockProof`, same file) must buffer the input as a
  self-describing map to find the `$type` discriminant → it calls
  `Deserializer::deserialize_any`.
- **native bincode** — the outer `Encode`/`Decode` derive writes the variant
  discriminant itself; only the *inner* fields defer to serde
  (`#[bincode(with_serde)]`), and those inner values (`InstantAssetLockProof`,
  `ChainAssetLockProof`) are plain **structs** — `deserialize_struct`, never
  `deserialize_any`.

The shared codec (`packages/rs-platform-wallet-storage/src/sqlite/schema/blob.rs:85-117`):

```rust
pub fn encode<T: PersistableBlob>(value: &T) -> Result<Vec<u8>, WalletStorageError> {
    Ok(bincode::serde::encode_to_vec(value, bounded_config())?)   // serde bridge
}
pub fn decode<T: DeserializeOwned>(blob: &[u8]) -> Result<T, WalletStorageError> {
    ... bincode::serde::decode_from_slice(blob, bounded_config()) ...
}
```

`AssetLockEntry` (`packages/rs-platform-wallet/src/changeset/changeset.rs:933`) embeds
`pub proof: Option<AssetLockProof>` and reaches the codec via
`impl_persistable_blob!(AssetLockEntry)` in
`packages/rs-platform-wallet-storage/src/sqlite/schema/asset_locks.rs:25`.

**Why encode succeeds, decode fails.** Serialization never needs lookahead, so
`bincode::serde::encode_to_vec` writes the internally-tagged form without
complaint. On read, `bincode::serde`'s `Deserializer` returns
`SerdeDecodeError::AnyNotSupported` the instant the internally-tagged
`Deserialize` asks for `deserialize_any`, surfaced as
`WalletStorageError::BincodeDecode { source: Serde(AnyNotSupported) }`. The row is
write-once, read-never.

Note that `Option<AssetLockProof>` only trips this when the value is `Some`: a
`None` encodes as a single `0x00` tag and decodes without ever touching the inner
enum. This is precisely why the defect escaped the existing tests — every
`asset_locks.rs` test fixture uses `proof: None`.

The crate is already aware of this exact hazard. `WalletStorageError::BincodeEncode`
(`error.rs:142-149`) documents it verbatim: *"the value's serde representation
needs `deserialize_any`-style dispatch — see dpp's `IdentityPublicKey` workaround."*

## 3. The precedent — this was solved once already

`identity_keys.rs:4-7` module docs:

> `IdentityKeyEntry.public_key`'s `#[serde(tag = ...)]` enum is rejected by
> bincode-serde (needs `deserialize_any`), so `IdentityKeyWire` pre-encodes the
> key with bincode's native `Encode`/`Decode` and rides the surrounding fields on
> the serde encoder, keeping one blob per row.

`IdentityPublicKey` is `#[serde(tag = "$formatVersion")]`
(`packages/rs-dpp/src/identity/identity_public_key/mod.rs:56`) — the identical
shape as `AssetLockProof`. `IdentityKeyWire` (`identity_keys.rs:24-72`) solves it
by carrying the offending field as a natively pre-encoded `Vec<u8>`
(`public_key_bincode`) and letting the outer serde blob path handle the rest. It
guards trailing bytes on the inner decode and is covered by round-trip tests.

`asset_locks.rs` simply never received the same treatment. That is the whole bug.

## 4. Audit — every `impl_persistable_blob!` call site

Method: enumerated all `impl_persistable_blob!` sites; for each blob type checked
the full field graph for `deserialize_any`-requiring serde shapes
(`#[serde(tag)]` internally-tagged, `#[serde(untagged)]`, `#[serde(flatten)]`, or a
manual `Deserialize` calling `deserialize_any`). A repository scan for those three
attributes across `rs-platform-wallet`'s own blob types returned **empty**; a scan
for manual `deserialize_any` in the reachable asset-lock/wallet graph returned
**empty**. The only such shapes reach the codec transitively through embedded DPP
types.

| Blob type | `deserialize_any` shape reachable? | Status |
|---|---|---|
| `dashcore::OutPoint` | No (struct) | Safe |
| `Vec<u32>` | No (seq) | Safe |
| `ContactRequest` | No | Safe |
| `IdentityEntry` | No — keys live in the sibling `identity_keys` changeset; embeds only plain structs/enums (`IdentityStatus`, `DashPayProfile`, `PaymentEntry`, `DpnsNameInfo`, externally-tagged) | Safe |
| `IdentityKeyWire` | `IdentityPublicKey` is internally tagged **but pre-encoded natively** | Mitigated (the precedent) |
| `PendingContactCrypto` | No — `PendingContactCryptoOp` is externally tagged | Safe |
| `AccountRegistrationEntry` | No — `AccountType` externally tagged; `ExtendedPubKey` is a struct | Safe |
| `DashPayProfile`, `PaymentEntry` | No (plain profile/payment fields) | Safe |
| `TransactionRecord`, `dashcore::InstantLock` | No (structs) | Safe |
| **`AssetLockEntry`** | **`Option<AssetLockProof>`, `#[serde(tag = "$type")]`** | **Broken — the reported defect** |

**Conclusion:** `AssetLockEntry` is the sole currently-broken type. No other blob
type shares the latent break today. One future risk worth recording:
`DataContractConfig` is `#[serde(tag = "$formatVersion")]`
(`wallet/identity/network/contract.rs:705`) and would break the same way *if* it
ever reaches a `_blob` column — it does not today. The recurrence guard in §9
covers this class going forward.

The `funding_type` field is not a hazard: its adapter
(`changeset/serde_adapters.rs`, `asset_lock_funding_type`) encodes a stable `u8`
tag, no `deserialize_any`.

## 5. Chosen approach

**Mirror `IdentityKeyWire`: introduce `AssetLockEntryWire` in
`asset_locks.rs` that carries `proof` as a natively pre-encoded
`Option<Vec<u8>>`, and route the rest through the existing serde blob path.**

`AssetLockProof` already derives native bincode `Encode`/`Decode`, and its inner
variants are structs, so `bincode::encode_to_vec` / `decode_from_slice` on the
proof round-trips without serde lookahead — identical mechanics to the proven
`IdentityKeyWire::{from_entry,into_entry}` on `IdentityPublicKey`.

Rationale:

- **Reuses an established, tested pattern** in the same file's sibling module —
  lowest cognitive and review cost.
- **Localised to the storage crate.** No change to `rs-dpp` or the
  `platform-wallet` changeset; the changeset serde/replication surface is
  untouched.
- **Honours the stated constraints.** Format stays bincode; schema evolution
  stays gated by the refinery migration version (§7). No per-blob revision tag.
- **Compat is nearly free.** The only altered field is `proof`; a pre-fix
  `proof: None` row is byte-identical under the new wire type (§7), so the common
  case rehydrates with no migration.

**Marker-trait question (asked explicitly):** no new marker trait is warranted.
`AssetLockEntryWire` is `Serialize + Deserialize + Sealed` via the existing
`impl_persistable_blob!`, exactly like `IdentityKeyWire`. The native encoding is an
implementation detail inside `from_entry`/`into_entry`. Adding a parallel
`PersistableBlobNative` trait was considered and rejected: it could not actually
*prevent* the bug, because the offending shape is a transitive **field** of the
blob type, not the blob type itself — a `T: Serialize` type with a hidden
internally-tagged field would still compile and still fail only at decode time.
The transitive case is caught instead by a round-trip test (§9), which is both
necessary and sufficient; the extra trait machinery buys no additional guarantee.

## 6. Rejected alternatives

- **Whole-blob native codec (`blob::encode_native<T: Encode>`), making
  `AssetLockEntry` itself `Encode`/`Decode`.** Requires native bincode derives on
  every field type — `dashcore::Transaction`, `OutPoint`, `AssetLockFundingType`,
  `AssetLockStatus` — several of them upstream in `platform-wallet`/`dashcore`,
  and touches the changeset serialization surface used for replication. Larger
  blast radius, no in-crate precedent, and it changes the on-disk layout of
  *every* asset-lock field (not just `proof`), forfeiting the free `None`-row
  compatibility. Rejected.
- **Switch the blob codec to a self-describing format (CBOR / MessagePack).**
  Would decode internally-tagged enums directly, but violates the "bincode, no
  per-blob revision" constraint and rewrites the on-disk format for *all* blob
  columns, breaking every existing row. Disproportionate. Rejected.
- **Graceful skip-on-decode-error in the read path.** Contradicts the crate's
  explicit invariant that a row failing to decode is a hard error (corruption is
  never silently dropped; `asset_locks.rs:99-149`), and would mask genuine
  corruption. The compat need is met by a targeted one-time migration (§7)
  instead. Rejected.

## 7. Exact changes

### Primary — `packages/rs-platform-wallet-storage/src/sqlite/schema/asset_locks.rs`

1. Define the on-disk wire type, **preserving the exact field order and serde
   encodings of fields 1–7** so pre-fix `None` rows stay readable:

   ```rust
   #[derive(Serialize, Deserialize)]
   struct AssetLockEntryWire {
       out_point: OutPoint,
       transaction: Transaction,
       account_index: u32,
       #[serde(with = "platform_wallet::changeset::serde_adapters::asset_lock_funding_type")]
       funding_type: AssetLockFundingType,
       identity_index: u32,
       amount_duffs: u64,
       status: AssetLockStatus,
       proof: Option<Vec<u8>>,      // natively bincode-encoded AssetLockProof
   }
   ```
   (`serde_adapters` is `pub`; reuse the same adapter so `funding_type` bytes are
   unchanged.)

2. Replace `impl_persistable_blob!(AssetLockEntry)` → `impl_persistable_blob!(AssetLockEntryWire)`.

3. `from_entry(&AssetLockEntry) -> Result<AssetLockEntryWire, WalletStorageError>`
   — `proof.as_ref().map(|p| bincode::encode_to_vec(p, blob::bounded_config())).transpose()?`.

4. `into_entry(self) -> Result<AssetLockEntry, WalletStorageError>` — for
   `Some(bytes)`, `bincode::decode_from_slice::<AssetLockProof, _>(&bytes,
   blob::bounded_config())`, and **reject trailing bytes**
   (`consumed != bytes.len()` → `WalletStorageError::blob_decode(...)`),
   mirroring `IdentityKeyWire::into_entry` (`identity_keys.rs:53-62`).

5. `apply` (line 46): `let lifecycle_blob = blob::encode(&AssetLockEntryWire::from_entry(entry)?)?;`

6. `decode_row` (line 114): `let wire: AssetLockEntryWire = blob::decode(blob_bytes)?; let entry = wire.into_entry()?;` — the existing typed-column/blob cross-checks and the `status`-column consistency check stay unchanged.

No change to `blob.rs` (reuse the already-`pub(crate)` `bounded_config()`), and none
to `rs-dpp` (the native derive on `AssetLockProof` is already present).

### Compatibility with rows already on disk

- **Pre-fix `proof: None` rows → decode transparently.** Fields 1–7 encode
  identically under `AssetLockEntry` and `AssetLockEntryWire`; `None` is `0x00` in
  both. No migration needed; a compat test pins this (§8).
- **Pre-fix `proof: Some(...)` rows → unrecoverable, by construction.** They were
  produced by serializing an internally-tagged enum through a **non-self-describing**
  format; the field names/structure serde needs on the way back were never written,
  so no decoder — serde or bespoke — can reliably reconstruct them. These rows are
  already unreadable today (they hard-fail the whole wallet load), so nothing is
  regressed; the value is only that *new* `Some` rows become readable.

  Recommendation: ship a one-time refinery migration
  `packages/rs-platform-wallet-storage/migrations/V004__drop_undecodable_asset_locks.rs`
  that deletes the pre-fix proof-bearing rows so a previously-bricked wallet loads
  cleanly:

  ```sql
  DELETE FROM asset_locks WHERE status IN ('is_locked', 'chain_locked');
  ```

  `max_supported_version()` lifts from 3 to 4 automatically (it is derived from the
  embedded list — `migrations.rs:41`). Scope is limited to the two statuses that (a)
  can carry a `Some` proof and (b) feed the production `load_unconsumed` path;
  `consumed` rows carry a proof but are already SQL-filtered out of rehydration and
  are decode-attempted only by test-only readers, so they need not be deleted.
  Deleting is safe: the asset-lock lifecycle is a chain-derived cache, not a source
  of truth — a dropped unconsumed lock re-derives from Core on the next SPV sync.
  This is deterministic, SQL-only, and keys on the `status` column rather than on
  attempting to decode the opaque blob.

  Rationale over a self-healing read path: it preserves the hard-fail-on-corruption
  invariant for *genuine* corruption while making the one-time format cleanup an
  auditable, single-event migration.

### Secondary — `AlreadyOpen` masking (`packages/rs-platform-wallet/src/manager/`)

`PlatformWalletManager::new` (`manager/mod.rs:110-196`) spawns
`spawn_wallet_event_adapter(...)` holding `Arc::clone(&persister)` and stores its
`JoinHandle` in `event_adapter_join`, **before** the fallible
`load_from_persistor` (`manager/load.rs:32`) runs. Dropping a manager does not
cancel `event_adapter_cancel` (a dropped `CancellationToken` is not a cancelled
one) and merely detaches the `JoinHandle`; the still-running adapter keeps its
`Arc<SqlitePersister>` alive, so `SqlitePersister::drop → release_open_path`
(`persister.rs:100-106`) never runs, and a same-process retry hits
`WalletStorageError::AlreadyOpen` (`persister.rs:87-98`) instead of the real load
error.

Where the cleanup belongs — in the manager, which owns the task lifecycle:

- **Preferred (structural):** defer the event-adapter spawn out of `new` into the
  existing post-registration `start()` step. Other subsystems already follow
  "not auto-started — call `start` after wallets are registered"
  (`manager/mod.rs:52,72`); the adapter is not needed to service a load. A failed
  load then leaves nothing retaining the persister.
- **Backstop (covers all drop paths):** add `impl Drop for PlatformWalletManager`
  that calls `event_adapter_cancel.cancel()` and `abort()`s the join handle.
- **Deterministic teardown for the construct→load orchestration:** an
  `async fn shutdown(&self)` (cancel token + `await` the join handle) invoked on
  the `load_from_persistor` error path before the caller drops/retries, so the
  `Arc<P>` is provably released before re-open.

### Tertiary — error typing / logging (observability)

`load_from_persistor` maps the persister error with
`format!("Failed to load persisted client state: {}", e)` into
`PlatformWalletError::WalletCreation(String)` (`manager/load.rs:40-45`) — `Display`
only, so `BincodeDecode { source: Serde(AnyNotSupported) }` collapses to "bincode
decode error" and the root cause is lost. Add a typed
`PlatformWalletError::PersisterLoad(#[source] …)` variant preserving the chain, and
log the persister error with `{:?}` (Debug), not `{}`. Consistent with
`rust-best-practices` error handling and the project rule against user-facing
`String` error fields. In scope if the change budget allows; otherwise a fast
follow-up.

## 8. Test plan

Follow the existing `tests/` style (`sqlite_asset_locks_filter.rs`,
`sqlite_second_open_guard.rs`, `sqlite_v003_migration.rs`).

1. **Repro (RED first).** Unit test in `asset_locks.rs`: build an `AssetLockEntry`
   with `proof: Some(AssetLockProof::Chain(...))` and one with
   `proof: Some(AssetLockProof::Instant(...))`, `apply` then `load_unconsumed` /
   `load_state`; assert the entry (proof included) round-trips. Confirm it FAILS on
   the current code with `BincodeDecode`/`AnyNotSupported` before the fix, GREEN
   after. This is the test the pre-fix suite lacked.
2. **Wire trailing-byte guard.** `into_entry` rejects a `proof` payload with a
   valid `AssetLockProof` prefix plus trailing garbage → `BlobDecode` (mirrors
   `into_entry_rejects_trailing_bytes_in_public_key_bincode`).
3. **`None`-row compat.** Pin that an `AssetLockEntry`/wire value with
   `proof: None` encodes byte-identically across the old and new shapes (assert the
   two encodings are equal), so pre-fix `None` rows still decode.
4. **Migration** (`tests/sqlite_asset_lock_v004_migration.rs`): seed a DB at the
   pre-fix schema with a proof-bearing (`is_locked`) row in the old serde format;
   run migrations; assert `load_unconsumed` succeeds and the undecodable row is
   gone.
5. **Cross-column integrity preserved.** Re-confirm the existing
   `load_state_rejects_status_column_mismatch` still holds through the wire type.
6. **Secondary bug** (in `rs-platform-wallet`, style of `sqlite_second_open_guard.rs`):
   construct a manager, force `load_from_persistor` to fail, tear down, and assert a
   subsequent `SqlitePersister::open()` on the same path succeeds (no `AlreadyOpen`).

## 9. Recurrence guard

The bug survived because `impl_persistable_blob!` admits any `T: Serialize` and the
one round-trip that would have caught it was never written. Two low-cost guards:

- **Blob round-trip coverage test** (`tests/sqlite_blob_roundtrip_coverage.rs`, or
  extend `sqlite_persist_roundtrip.rs`): encode+decode a representative **non-empty**
  value of every `_blob` type, exercising the enum-bearing variants explicitly
  (`AssetLockProof::{Instant,Chain}`). This catches the transitive
  internally-tagged-field case a marker trait cannot.
- **Advisory note in `blob.rs`**: a short doc comment stating that any type whose
  serde shape (or a transitive field's) needs `deserialize_any` —
  internally-tagged, `untagged`, or `flatten` — must pre-encode that field with
  native bincode via the wire-type pattern, citing `IdentityKeyWire` and
  `AssetLockEntryWire`.
