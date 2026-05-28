# Shielded test-data seeder — performance notes

The `SDK_TEST_DATA` shielded-pool seeder runs at `InitChain` and inserts
`ShieldedSeedConfig::sdk_test_data().total_notes` notes (currently
`500_000`) into the chain's commitment tree before the first block is
proposed. This is CPU-heavy work that scales linearly with `N`. This
document tracks the options for making it fast enough to use on a real
devnet.

## Cost breakdown for N = 500 000

| Step | Cost shape | Dominant in |
|---|---|---|
| Pallas-base rejection sampling | ~1 ChaCha12 draw + 1 field check per filler `cmx` | release: negligible; debug: ~5–10% |
| Owned-note Orchard encryption | ~8 ops total (`Note::from_parts` + `OrchardNoteEncryption`) | always negligible |
| Random ciphertext bytes for filler | `N × 216` ChaCha12 bytes (~108 MB) | always negligible |
| `BulkAppendTree` append | `N` buffer writes + `⌈N/2048⌉` chunk compactions (dense Merkle + MMR + blake3) | release: ~5–10%; debug: ~15% |
| **Sinsemilla frontier append** | **`N × O(log N)` Pallas hashes (~9.5M for N=500k)** | **release: ~85%; debug: ~75%** |
| GroveDB Merk propagation | Once per `apply_drive_operations` call (batched) | always negligible |

The Sinsemilla appends dominate. Each Pallas hash is the inner loop:
~20k cycles in release, ~200k–2M cycles in debug.

## Wall-clock measurements

Apple M-series, single SDK_TEST_DATA dashmate node, fresh `yarn reset`:

| N | Profile | Wall clock | Source |
|---|---|---|---|
| 500 | dev | ~1 s | observed |
| 16 | dev | <100 ms | unit test |
| 500 000 | dev | **30+ min and not done** | observed 2026-05-23 |
| 500 000 | release | ~1–3 min | extrapolated from release Pallas timing (~10 μs/hash × 9.5M ÷ 1 core ≈ 95 s) |
| 1 000 000 | release | ~3–6 min | linear extrapolation |
| 1 000 000 | dev | ~60–120 min | unusable |

## Options researched, ranked by impact / cost

### 1. Release build (`CARGO_BUILD_PROFILE=release`) — **recommended first step**

- **Speedup:** ~20–50× for the Sinsemilla phase; ~10× overall.
- **Effort:** one config option in dashmate (see below) + one Docker build arg.
- **Tradeoff:** slower image builds (release optimizations take longer to compile), larger debug-info-stripped binary remains ~50–100 MB vs ~700 MB for debug.
- **Status:** plumbed through dashmate as `platform.drive.abci.docker.build.cargoBuildProfile` — set to `release` when you want fast seeding.

### 2. Parallelize note generation

- **Speedup:** ~4–8× on top of #1.
- **Effort:** moderate. Two-stage pipeline:
  - Worker pool: per note, sample `cmx` + build owned ciphertext or random filler.
  - Single consumer thread: feed `(cmx, rho, encrypted_note)` tuples sequentially into `commitment_tree_insert_op` (Sinsemilla append MUST stay sequential — frontier state depends on previous step).
- **Tradeoff:** new concurrency surface, harder determinism guarantees (need deterministic per-worker RNG seeding).
- **Status:** not implemented. Consider only if #1 isn't enough.

### 3. Option B — precomputed GroveDB snapshot baked into the image

- **Speedup:** seeding cost → **0** at every `yarn reset` (one-time precomputation cost when the snapshot is generated).
- **Effort:** significant.
  - Standalone tool that opens a fresh GroveDB, writes BulkAppendTree chunk blobs (`e{u64}` keys), tail buffer (`b{u32}`), chunk-MMR nodes (`m{u64}`), metadata (`M`, `mmr_size`), Sinsemilla frontier (`__ct_data__`), and the parent Merk's `Element::CommitmentTree(...)` element — all consistently.
  - Dockerfile change: bundle the precomputed `db/` directory into the drive-abci image.
  - Genesis code: detect the bundled snapshot and skip the seeder.
- **Tradeoff:** big code surface, but the runtime cost completely vanishes.
- **Status:** not implemented. The right answer if scaling to 5M+ notes or doing repeated benchmarks where seed time matters.

### 4. Skip Pallas rejection sampling for filler

- **Speedup:** ~5–10% (saves one `from_repr` field check per filler).
- **Effort:** small refactor — replace the rejection loop with `Nullifier::dummy(&mut rng)` which uses `extract_p(&pallas::Point::random(rng))` (always valid by construction).
- **Tradeoff:** depends on `Nullifier::dummy` being `pub` (it's `pub(crate)` in upstream `orchard` unless we enable `unstable-voting-circuits`).
- **Status:** not implemented. Marginal win; skip unless every second matters.

### 5. Deterministic-bytes filler ciphertext

- **Speedup:** ~5–10% (saves `rng.fill_bytes(216)` per filler).
- **Effort:** trivial — derive 216 bytes from `blake3(rng_seed || position)` instead.
- **Tradeoff:** changes byte layout slightly (still valid 216-byte payload; wallet treats it as opaque garbage).
- **Status:** not implemented. Marginal.

### 6. GPU / SIMD-accelerated Sinsemilla

- **Speedup:** 100×+ in theory for the Pallas-hash inner loop.
- **Effort:** way out of scope. Out-of-tree dependency on a GPU Sinsemilla crate; integration is non-trivial.
- **Tradeoff:** breaks portability (requires NVIDIA hardware) and determinism guarantees become tricky.
- **Status:** not pursued.

## How build args reach the binary

Dashmate's `dockerBuild` config exposes a generic `buildArgs` map that flows
through `generateEnvsFactory` → `docker compose build` → Dockerfile `ARG`s.
This is the **only** supported way to set build args — shell env vars are
not wired through.

`scripts/setup_local_network.sh` (run automatically by `yarn setup` after
`dashmate setup local` creates the per-node configs) sets two args on each
`local_1/2/3` config:

| arg | value | why |
|---|---|---|
| `SDK_TEST_DATA` | `"true"` | activates the `create_sdk_test_data` cfg flag → genesis seeder runs |
| `CARGO_BUILD_PROFILE` | `"release"` | optimised binary — without it, 500k-note seeding takes 30+ min in debug |

Both are mandatory together. Debug-profile builds with N=500_000 push
InitChain past tenderdash's timeout window; release-profile finishes in 1–3
minutes.

## Switching back to debug for faster compile iteration

If you're iterating on drive-abci itself and don't care about seeding speed,
swap CARGO_BUILD_PROFILE back to `dev` per-config:

```bash
for cfg in local_1 local_2 local_3; do
  yarn dashmate config set platform.drive.abci.docker.build.buildArgs.CARGO_BUILD_PROFILE dev --config $cfg
done
```

Note: any `yarn reset` will rerun `scripts/setup_local_network.sh` and put
`CARGO_BUILD_PROFILE=release` back. Edit that script if you want a different
default permanently.

The same `buildArgs` field works for any other build arg the Dockerfile
declares. The schema is `Record<string, string>`.

## When to revisit Option B

Re-evaluate the precomputed-snapshot path (#3) when any of these are true:

- Seeding takes more than ~5 minutes even in release mode (i.e. N ≥ 2M).
- The benchmark workflow does many resets per day and seed time becomes the
  per-iteration bottleneck.
- The chain config changes shape such that even release-mode seeding becomes
  uneconomical to repeat.
