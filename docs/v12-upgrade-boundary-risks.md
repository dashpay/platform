# v11 → v12 Upgrade Boundary — Risk Spec & Test Plan

Status: in progress (2026-06-05). Branch: `v3.1-dev`. v12 is **not yet activated**, so both
fixes below are free pre-release changes.

The "boundary block" is the first block of protocol version 12 (an epoch-change block). On that
block, `perform_events_on_first_block_of_protocol_change` runs `transition_to_version_12`, and
three v12-gated block-end methods begin firing every block. Two risks are under investigation.

---

## Risk #2 — Genesis vs upgrade structural divergence (CONFIRMED bug, fork class)

**Claim:** the shielded pool subtree is built two different ways that must be byte-identical but
are not.

- Upgrade path — `transition_to_version_12`
  (`packages/rs-drive-abci/src/execution/platform_events/protocol_upgrade/perform_events_on_first_block_of_protocol_change/v0/mod.rs:623`):
  8 **sequential** `grove_insert_if_not_exists` into `[ShieldedBalances,"M"]`.
- Genesis path — `create_initial_state_structure_v3` + `initial_state_structure_shielded_pool_operations`
  (`packages/rs-drive/src/drive/initialization/v3/mod.rs:76`):
  one **`GroveDbOpBatch`** applied via `grove_apply_batch`.

**Reproduced** (root-hash equivalence test, currently RED):

| Path | pool `[ShieldedBalances,"M"]` AVL root_key | subtree root hash |
|------|--------------------------------------------|-------------------|
| genesis (batch)      | 160 | `05f10962…f062eb029` |
| upgrade (sequential) | 128 | `3e510b80…a3a6e0cff` |

Per-element bytes are identical; only the AVL **shape** differs. Cause: GroveDB's batch build
roots a sorted batch at `mid_index = len/2 = 4` → key 160 (`merk/src/tree/ops.rs:262`); sequential
AVL inserts settle at 128 regardless of insertion order (verified with a sorted-order control).

**Which path is wrong:** the design diagram in `packages/rs-drive/src/drive/shielded/paths.rs:10`
intends `SHIELDED_NOTES_KEY` (128) at the root (hottest subtree, fewest hops). The **sequential
/ upgrade path matches the design (128); the genesis/batch path violates it (160).**

**Blast radius:** an in-place v11→v12 upgrade converges on 128 across all sync modes (live upgrade,
block replay, state-sync byte copy) → mainnet does **not** fork. The divergence bites
fresh-genesis-v12 networks (new testnets/devnets, every `set_genesis_state()` test) which build the
wrong 160 shape and diverge from upgraded networks. The stated "genesis ≡ upgrade" invariant is
broken; comments are partly false.

**Fix (preferred):** unify both paths on the **sequential** builder. Extract the 8 ordered inserts
into one shared helper called by both `create_initial_state_structure_v3` and
`transition_to_version_12`. Yields 128 everywhere, byte-identical by construction, matches design.
Keep the equivalence test as a permanent regression guard (RED → GREEN on fix).

**Follow-up:** the same batch-vs-sequential pattern exists at the **v11 boundary**
(`transition_to_version_11` vs `create_initial_state_structure_v2`, for `SavedBlockTransactions`
children + address pool). v11 is live and hasn't halted, so those small trees (1 and 3 children)
coincidentally match — confirm with the same root-hash test and apply the shared-builder fix there
too.

---

## Risk #1 — `strip_unknown_document_schema_properties` (test against real node data)

Runs **inside** `transition_to_version_12`
(`packages/rs-drive/src/drive/contract/migration/strip_unknown_document_schema_properties.rs`). On
the boundary block it iterates **every** contract in state (and **every revision** of
history-keeping contracts), deserializes, strips disallowed top-level schema properties
(incl. the v12 flags `documentsCountable`/`rangeCountable`), and re-serializes any that changed.

Three failure modes to quantify against real data:

1. **Fail-closed → network halt.** Any contract that fails to bincode-deserialize, or hits an
   unexpected element type, returns `Err` → `transition_to_version_12` returns `Err` → the boundary
   block fails for every node deterministically. (Contrast `transition_to_version_8`, which
   swallows its errors.) **Question:** does any real testnet/mainnet contract trip the
   `CorruptedSerialization` / `CorruptedDriveState` branches?
2. **Performance.** Unbounded: `u16::MAX` contracts × all history revisions × one `grove_insert`
   per modified contract, synchronous, in one block. **Question:** realistic contract/revision
   count and wall-time on real state — does it threaten block/proposal timeouts?
3. **Determinism.** Must be byte-identical across nodes (`fetch_contract_ids` order →
   `document_schemas_mut` order → strip fn → bincode). Flag any `HashMap`-style nondeterminism.

### Test plan

Run the migration logic against **real drive state** from two sources, measuring error-exposure,
counts, and timing — without mutating the source data.

1. Dump the **drive** GroveDB volume from each source node, download to `~/Downloads`.
2. Build a temporary dry-run bin in `rs-drive-abci` that opens the dumped Drive and runs the
   strip logic in **read-only / detect-only** mode: per contract + revision, deserialize and
   compute `strip_unknown_properties_from_document_schema` *without* writing back. Report: total
   contracts, historical contracts, revisions scanned, count that would be modified, any
   deserialize/element errors (with contract IDs), and total wall-time.
3. (Optional) A `--apply-on-copy` mode that runs the real `strip_unknown_document_schema_properties`
   against a working copy to measure true end-to-end write cost.

### Results (real data, 2026-06-05)

Ran the `strip_dryrun` detect-only harness against live drive dumps from testnet
(`hp-masternode-8`) and a disposable mainnet-synced node.

| | Testnet (proto v11) | Mainnet node (proto v11) |
|---|---|---|
| Contracts scanned | 2988 (2846 non-hist + 142 hist) | 47 (43 + 4) |
| Revisions scanned | 3000 | 48 |
| Would be modified | 68 | 1 |
| **Fail-closed triggers** | **0** | **0** |
| Total wall-clock | 1.53 s | 0.43 s |

- **Fail-closed risk did not materialize:** zero contracts trip the `Err` branches → the migration
  would not halt the network on this data.
- **Performance is a non-issue** at current scale (2988 contracts scan in ~0.8 s).
- **Caveat:** the mainnet node showed only 47 contracts — likely not fully synced (or mainnet
  genuinely has few contracts). Re-run against a tip-synced mainnet node to be conclusive. Testnet
  (2988 contracts) is the stronger stress signal.

Open follow-ups for Risk #1 (lower priority given the above): the fail-closed behavior is still
brittle by design (one future malformed contract halts the upgrade) and determinism wasn't formally
audited — consider making the migration log-and-skip per contract rather than fail-closed.

---

## Runbook (local only — contains infra details, do not commit)

Sources:
- **Testnet:** `hp-masternode-8` = `68.67.122.8` (ansible_user `ubuntu`). **Live HPMN validator —
  read-only dump, do NOT stop the drive container.**
- **Mainnet data:** `54.185.219.212` — disposable node synced against mainnet, to be destroyed.
  May stop its drive container for a clean RocksDB snapshot.
- SSH: `ssh -i ~/.ssh/evo-app-deploy.rsa ubuntu@<host>`

Per-node steps:
1. Discover: `docker ps`, `docker volume ls | grep -i drive`, `du -sh` the volume mount.
2. Dump drive volume only (not Core/Tenderdash): tar the volume → `/tmp/drive-<net>.tgz`
   (testnet: read-only mount while running; mainnet test node: stop drive container first).
3. `scp -i ~/.ssh/evo-app-deploy.rsa ubuntu@<host>:/tmp/drive-<net>.tgz ~/Downloads/`.
4. Extract; point the dry-run bin at the GroveDB path; record results.

Local disk: ~231 GB free in `~/Downloads` (sufficient).
