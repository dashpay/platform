# Core 23 split-host masternode persistence (`MasternodeStateV1`)

## Problem

Core 23 (`DEPLOYMENT_V24`) decouples a masternode's **platform** endpoints from its
**core** service address: the nested `addresses` object (`MasternodeAddresses`) carries
`platform_p2p` / `platform_https` as `host:port` strings whose host may differ from the
core `service` IP. The live validator path already honors this — `ValidatorV0.node_ip`
is taken from `DMNState::platform_p2p_address().0` (the platform host), not `service.ip()`.

But the **persisted** masternode state (`MasternodeStateV0`) stores only the resolved
**ports** (`platform_p2p_port`, `platform_http_port: Option<u32>`); it has no host field.
The reverse conversion `From<MasternodeStateV0> for DMNState` therefore reconstructs the
nested `addresses` using `service.ip()` as the host.

Consequence: after a normal restart (which **loads the persisted masternode list from disk
and applies incremental Core diffs** — only `is_init_chain` re-fetches the full list), any
Core 23 HPMN whose platform host differs from its core service IP is reconstructed with the
**wrong host**. A validator rebuilt from that state (`new_validator_if_masternode_in_state`)
or refreshed (`validator_refresh_from_state`) advertises `node_id@service_ip:port` to
Tenderdash until the next Core `addresses` diff for that node overwrites it.

Today this is **latent** (mainnet/testnet have `DEPLOYMENT_V24` `NEVER_ACTIVE`, so platform
and core hosts are collocated), but the live path already ships and tests the distinct-host
case, so persistence is inconsistent with the live path's claimed support.

## Chosen approach

Persist the platform host by introducing a **new versioned `MasternodeStateV1`** that adds
the host, gated to **protocol version 12 only**.

- Keep `MasternodeStateV0` / `MasternodeV0` **byte-identical** (a regression test
  deserializes real testnet-v0 and devnet-v8 persisted state — backward compat is required).
- Add `MasternodeStateV1` = `MasternodeStateV0` + `platform_host: Option<String>` (the single
  platform host paired with both platform ports — matches `ValidatorV0`'s single-`node_ip`
  model; `platform_p2p` and `platform_https` resolve to the same host in the validator path).
- Add `MasternodeV1` (= `MasternodeV0` with `state: MasternodeStateV1`) and a new
  `Masternode::V1(MasternodeV1)` enum variant.
- **Write gate:** `drive_abci.structs.masternode` switches `0 → 1` **only for protocol v12**.
  The structure-versions are currently a single shared const (`DRIVE_ABCI_STRUCTURE_VERSIONS_V1`,
  `masternode: 0`) referenced by v1–v12. Add a second const
  `DRIVE_ABCI_STRUCTURE_VERSIONS_V2` (`masternode: 1`, otherwise identical) and point **only
  `v12.rs`** at it. v1–v11 keep V1 (write `Masternode::V0`, byte-identical, zero change).
- **Read:** `Masternode`'s `Decode` handles whichever variant tag is present (V0 or V1)
  regardless of gate. All `match self { Masternode::V0(..) }` sites gain a `V1` arm. The
  6 top-level accessors + `From<Masternode> for MasternodeListItem` are the only match sites.

### Conversions (all four required — the outer `MasternodeV1 ↔ MasternodeListItem` pair is the actual save/load entry point, not the inner `DMNState` pair)

- `From<MasternodeListItem> for MasternodeV1`: mirrors `From<MasternodeListItem> for MasternodeV0`
  (`v0/mod.rs:48`); `state: value.state.into()` lands in the `DMNState → MasternodeStateV1`
  impl. This is what the write gate's `1 => Ok(Self::V1(value.into()))` arm calls.
- `From<MasternodeV1> for MasternodeListItem`: mirrors `v0/mod.rs:72`; the read path
  `From<Masternode> for MasternodeListItem`'s `V1(v1) => v1.into()` arm calls it.
- `From<DMNState> for MasternodeStateV1`: extract ports via the existing accessors **and**
  `platform_host = platform_p2p_address().map(|(host, _)| host)` (the platform host; `None`
  when no platform p2p address resolves — Core 22 / non-HPMN).
- `From<MasternodeStateV1> for DMNState`: reconstruct `addresses` **only when a platform port
  is present** — carry over the V0 guard `(platform_p2p_port.is_some() || platform_http_port.is_some()).then(...)`
  so a host-only/no-ports state never builds an empty `addresses`. Pair each stored port with
  `platform_host` when present, falling back to `service.ip()` when absent (Core 22 / legacy
  round-trips, preserving today's behavior, and reproducing exactly the host the live path
  would advertise for such a node). Leave `legacy_*` ports `None` (same rationale as V0: a
  later `addresses: Some(None)` clear must actually drop the endpoint).
- `MasternodeV1` needs a **hand-written `Debug` impl** copied from `MasternodeV0` (`v0/mod.rs:34`)
  — V0's Debug is manual (renders `ProTxHash` via `.to_string()`), not derived. A plain
  `#[derive(Debug)]` would compile but diverge. Derives otherwise carry over: `MasternodeV1`
  → `Clone, PartialEq, Encode, Decode`; `MasternodeStateV1` → `Clone, PartialEq, Eq, Debug, Encode, Decode`.

### Read path is tag-driven — must stay version-agnostic

The `structs.masternode` gate is consulted at **exactly one** site: the write conversion
`Masternode::try_from_platform_versioned(MasternodeListItem)`. The read side (`From<Masternode>
for MasternodeListItem` + the 6 accessors) is a plain variant `match` with **no** version check
— do NOT add one. This is the same write-gated/read-all design already shipped for
`PlatformStateForSaving` (`platform_state/mod.rs:233` reads all variants; only the write
consults its gate). It also makes a protocol-rollback safe: a v12-binary node that rolls its
*protocol* back to v11 still decodes a previously-written V1 blob (the shared enum knows the
tag). Only a genuine *binary* downgrade to a pre-V1 build fails — and it fails **loud at
startup** (`fetch_platform_state` propagates the decode error → `CorruptedCachedState`), never
a silent misdecode.

### Why tail-appending a field to `MasternodeStateV0` is NOT an option

Considered and rejected: adding `Option<String>` as a trailing field to `MasternodeStateV0`
to avoid `MasternodeV1`. bincode is non-self-describing and fixed-layout, and masternodes are
stored in a `BTreeMap` (length-prefixed sequential entries). Decoding an old blob would read
the new `Option` discriminant byte from the **next** map entry's bytes — silent misalignment,
not a clean `None`. The versioned-enum (new variant tag) is the only safe path.

### Why this is consensus-safe (verified)

- The serialized platform state is stored via `put_aux` → **non-hashed aux CF**, NOT grovedb's
  hashed tree → **not part of `app_hash`**. A format change cannot fork the chain.
- `PlatformState::fingerprint()` (`hash_double(serialize_to_bytes())`) is **diagnostic-only**
  — every caller is a `tracing` log field (`init_chain`, `run_block_proposal`, `abci/handler/info`).
- **No ABCI state-sync snapshot handlers** exist in drive-abci → the aux-CF platform-state blob
  is never shipped between nodes → no "V1-writer → V0-reader" hazard during rolling upgrade.
- Gating the write to v12 (in-development, not on mainnet/testnet) means **released versions
  v1–v11 are byte-identical** — zero behavior change where it matters.

## Files

- `rs-platform-version/.../drive_abci_structure_versions/mod.rs` (add `pub mod v2;`).
- `rs-platform-version/.../drive_abci_structure_versions/v2.rs` (new const
  `DRIVE_ABCI_STRUCTURE_VERSIONS_V2`, identical to V1 except `masternode: 1`).
- `rs-platform-version/.../version/v12.rs` (point `structs` at `..._V2`; currently `..._V1`).
- `rs-drive-abci/.../masternode/v0/mod.rs` (unchanged V0; keep as the byte-identical baseline).
- `rs-drive-abci/.../masternode/v1/mod.rs` (new `MasternodeV1` + `MasternodeStateV1`, manual
  `Debug` for `MasternodeV1`, and all four `From` impls listed above).
- `rs-drive-abci/.../masternode/mod.rs` (add `pub mod v1;`; enum `V1` variant;
  `try_from_platform_versioned` gate arm `1 => Ok(Self::V1(value.into()))`;
  `From<Masternode> for MasternodeListItem` `V1(v1) => v1.into()` arm).
- `rs-drive-abci/.../masternode/accessors.rs` (6 accessors gain a `V1` arm).
- (also the guard-completeness fix in `update_state_masternode_list/v0` — separate, see below.)

Established-pattern confirmation (from review): every other `drive_abci_versions/*` family
(`method_versions` v1–v8, `validation_versions` v1–v8, `query_versions` v0–v1,
`withdrawal_constants` v1–v2) evolves by adding a new numbered const file and re-pointing only
the target `vN.rs` — `structure_versions` is simply the one family that never needed a 2nd
const yet. This change follows that convention exactly.

## Guard-completeness fix (independent, same PR)

`update_state_masternode_list_v0`'s refresh-trigger guard must mirror the predicate
`validator_refresh_from_state`'s full gate. The zero/range filter in `diff_platform_*_port`
means a diff carrying only `legacy_platform_*_port: Some(0)` (or out-of-range), or only a
`platform_node_id` change, won't trip the guard, so a stale validator is retained. Add
`state_diff.legacy_platform_p2p_port.is_some() || state_diff.legacy_platform_http_port.is_some()
|| state_diff.platform_node_id.is_some()` to the guard.

The guard is **intentionally a superset** of the validity predicate: a false-positive trigger
just recomputes `validator_refresh_from_state` and rewrites identical fields (idempotent), while
a false-negative leaves a stale validator advertised — so the guard must fire on every field
that can change validity, even ones `diff_platform_*_port` deliberately drops (`Some(0)`). Add a
one-line comment to that effect so a future reader doesn't "fix" the apparent inconsistency with
the zero-dropping helpers. This is validator-set in-memory state feeding Tenderdash P2P
advertisement — connectivity only, never `app_hash`.

## Failure modes

- **Protocol rollback (same v12 binary → protocol v11)**: safe. The shared `Masternode` enum
  knows tag `V1`, and the read path is tag-driven, so a previously-written V1 blob still decodes.
- **Genuine binary downgrade (to a pre-V1 build)**: fails **loud at startup** — bincode hits an
  unknown enum discriminant, `fetch_platform_state` propagates the error → `CorruptedCachedState`.
  Never a silent misdecode. Not a rolling-upgrade concern because platform state never crosses
  nodes (no state-sync handlers) and a V1 blob can only be produced under protocol v12, which is
  not activated on any released network.
- **`platform_host` is a hostname, not an IP**: stored verbatim as `String`; the validator path
  already treats `node_ip` as a `String`, so no parsing assumption is added.
- **Core 22 / non-HPMN on v12**: `platform_host = None`; reverse conversion falls back to
  `service.ip()`, identical to V0 behavior.

## Test plan

- **Restart round-trip through the real path** (`MasternodeListItem`/`DMNState`, mirroring the
  v0 test at `masternode/v0/mod.rs:312`, not the bare struct): a `MasternodeListItem` whose
  `DMNState` carries a Core-23 `addresses` host ≠ service IP → `MasternodeV1` (captures host) →
  back to `MasternodeListItem` → `state.platform_p2p_address().0 == addresses_host`. RED against
  V0 (which collapses to `service.ip()`).
- v12 saving path writes `Masternode::V1` (gate = 1); v11 writes `Masternode::V0`.
- Backward-compat: existing testnet-v0 / devnet-v8 deserialization tests still pass (V0 untouched).
- Reverse fallback: `MasternodeStateV1 { platform_host: None }` → `service.ip()` host.
- Guard-completeness: a diff with only `legacy_*: Some(0)` / only `platform_node_id` trips the
  refresh (RED before the guard extension).
</content>
</invoke>
