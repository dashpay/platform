# QA Contract — on-chain QA framework storage layer

A dedicated Dash Platform data contract that stores **test definitions** and
**test-run results** on-chain, so QA status is queryable and proof-verifiable
and a website can render it. Complements GitHub issue
[#3897](https://github.com/dashpay/platform/issues/3897) and the iOS test plan at
[`packages/swift-sdk/SwiftExampleApp/TEST_PLAN.md`](../packages/swift-sdk/SwiftExampleApp/TEST_PLAN.md).

This package is the **storage layer only**: the schema, a register script, a
seed script (kept in sync with `TEST_PLAN.md`), a submit-run helper, and a
read/verify tool. The website that consumes the contract is a separate task and
only needs the contract ID below.

## Contract ID

The live contract ID for each network is committed in
`contract-id.<network>.json` (e.g. [`contract-id.testnet.json`](contract-id.testnet.json)),
written by the register script. Read it from there — testnet resets periodically,
so the ID changes when the contract is re-registered (see
[Testnet reset / re-seed](#testnet-reset--re-seed)).

**Live testnet deployment** (as of registration):

| | |
|---|---|
| Contract ID | `2gevmsNEaWnWQURQpuWeN5QnLfC2ufrZG4SXkVMqeUgZ` |
| Owner (QA identity) | `85KjYZLZXA7YZBPyFEjiMaH36xcQpBBZisKGBHF3uKuH` |
| Network | testnet |

> Supersedes the initial contract `2qEVUbg4znNgNRs3FJQ4kof4NKpB8q4fGtYa7qBouLzw`
> (re-registered with an integer `network` field and `$ownerId`-prefixed testRun
> indices). Consumers pinned to the old id must re-pin to the one above.

```jsonc
// contract-id.testnet.json (shape)
{
  "network": "testnet",
  "contractId": "<base58 contract id>",
  "ownerId": "<base58 QA identity id>",
  "documentTypes": ["testCase", "testRun"],
  "schemaSha": "<sha256 prefix of the schema>",
  "planCommit": "<TEST_PLAN.md git short-sha at register time>",
  "registeredAt": "<ISO timestamp>"
}
```

## Schema

Two document types (full schema in
[`schema/qa-contract.documents.json`](schema/qa-contract.documents.json)):

### `testCase` — a test definition (mirrors one `TEST_PLAN` §4 row)

| Field | Type | Notes |
|---|---|---|
| `testId` | string (≤32) | e.g. `CORE-05`. **Unique index.** |
| `title` | string (≤255) | the plan's *Action* column |
| `tier` | string (≤16) | Essential / Common / Thorough / Uncommon / Manual. **Indexed.** |
| `category` | string (≤32) | Domain (Core, Identity, DPNS, Token, …). **Indexed.** |
| `layer` | string (≤16) | Core / Platform / Cross / Shielded |
| `implStatus` | string (≤32) | status glyph (✅ 🧪 ⚠️ 🔌 🚫) |
| `description` | string (≤2048) | entry point & test notes (last plan column) |
| `entryPoint` | string (≤512) | primary view / FFI entry point |
| `prerequisites` | string (≤1024) | fixtures/preconditions |
| `planCommit` | string (≤64) | `TEST_PLAN.md` commit this row was seeded from |

- Indices: `testId` (unique, asc) · `tier` (asc) · `category` (asc).
- **Mutable** (`documentsMutable: true`) so impl-status / entry-point updates can
  be pushed; deletable so removed plan rows can be cleaned up.
- `additionalProperties: false`.

### `testRun` — an append-only run record

| Field | Type | Notes |
|---|---|---|
| `testId` | string (≤32) | matches `testCase.testId`. **Indexed (compound).** |
| `result` | string (≤16) | `pass` / `fail` / `blocked` / `skipped`. **Indexed (compound).** |
| `network` | integer | network id: `0`=mainnet, `1`=testnet, `2`=devnet, `3`=regtest. **Indexed (compound).** |
| `buildRef` | string (≤63) | build under test (commit/branch/build no.). **Indexed.** |
| `device` | string (≤128) | device / simulator |
| `evidence` | string (≤512) | txid / on-chain id / screenshot path / URL |
| `notes` | string (≤2048) | free-form notes |
| `blockerReason` | string (≤512) | why blocked/skipped |
| `$createdAt` | system | **run time**, stamped by the platform; required + indexed |

- Indices (all `asc`; `$ownerId`-prefixed so runs are queried per submitter — sets
  up v2 multi-submitter):
  - `ownerTestNetwork` — `$ownerId`, `testId`, `network`
  - `ownerTestNetworkCreated` — `$ownerId`, `testId`, `network`, `$createdAt`
  - `ownerTestResultCreated` — `$ownerId`, `testId`, `result`, `$createdAt`
  - `ownerTestCreated` — `$ownerId`, `testId`, `$createdAt`
  - `buildRefOwner` — `buildRef`, `$ownerId`
- "Most recent run first" is done at query time with `orderBy [['$createdAt','desc']]`.
- **Immutable + non-deletable** (`documentsMutable: false`, `canBeDeleted: false`):
  it is an audit log. `additionalProperties: false`.

> **Platform schema constraints baked into this schema:**
> - Indexed string properties are capped at `maxLength ≤ 63`, which is why the
>   indexed fields are short.
> - Index property sort direction must be **`asc`** in the contract definition
>   (drive-abci rejects `desc` with `JsonSchemaError: "desc" is not one of ["asc"]`).
>   Descending order is requested at *query* time instead — the index is traversed
>   in reverse — so `testRun` queries still return newest-first via
>   `orderBy [['$createdAt','desc']]`.
>
> The schema is validated against `rs-dpp` (`new DataContract({ …, fullValidation: true })`)
> before broadcast; note that local DPP validation does **not** catch the asc-only
> index rule — drive-abci does, at register time.

## QA identity (v1 ownership)

In v1 a **single QA identity** owns the contract and creates every document.
Both document types use `creationRestrictionMode: 1` (**OwnerOnly**), so only
that identity can create `testCase`/`testRun` documents.

You need:

1. A registered testnet identity with a **credit balance** (registration + each
   document create costs credits). Create/fund one with the SwiftExampleApp
   (`ID-01`) or any Platform wallet.
2. The **private key** of an `AUTHENTICATION` key on that identity with **HIGH or
   CRITICAL** security level (WIF or 64-char hex). The register/seed/submit
   scripts sign with it.

Provide them via env vars or a gitignored `.env` (copy `.env.example`):

```sh
export NETWORK=testnet
export QA_IDENTITY_ID=<base58 identity id>
export QA_PRIVATE_KEY=<WIF or hex>     # testnet key only
# optional: export QA_IDENTITY_KEY_ID=2   # pin the signing key id
```

The scripts auto-detect which identity key matches `QA_PRIVATE_KEY`; if detection
fails they print the identity's keys so you can set `QA_IDENTITY_KEY_ID`.

**Recovering the key from a wallet mnemonic.** If the identity was registered by a
wallet whose mnemonic you control (e.g. created/restored in SwiftExampleApp, which
mints identities via the Core asset-lock flow that the JS SDK can't do on its own),
set `QA_MNEMONIC` + `QA_IDENTITY_ID` and run:

```sh
node src/derive-identity-key.mjs --write
```

It fetches the identity, derives candidate keys from the mnemonic at the
platform-wallet DIP13 path `m/9'/<coin>'/5'/0'/<keyType>'/<identityIndex>'/<keyIndex>'`,
matches them to the on-chain public keys, and writes `QA_PRIVATE_KEY` +
`QA_IDENTITY_KEY_ID` into `.env`.

### Extending to per-team-member `testRun` submission (v2)

To let any identity submit runs (while keeping `testCase` owner-controlled),
change **`testRun`** only:

- set `creationRestrictionMode: 0` (NoRestrictions) on `testRun`, and
- register the change via a data-contract **update** (or re-register on the next
  testnet reset).

`testRun` is already immutable + owner-stamped (`$ownerId`/`$createdAt` are
system fields), so opening creation keeps every run attributable and tamper-proof.
Leave `testCase` as OwnerOnly so the canonical catalog stays curated.

## Install & run

The scripts use the recommended **`@dashevo/evo-sdk`** (js-evo-sdk) in trusted
mode (required so state-transition responses are proof-verified). Node ≥ 18.18.

**Option A — standalone (published SDK):**

```sh
cd qa-contract
yarn install        # or npm install
```

**Option B — in-repo (workspace build):** build the workspace SDK once and point
the scripts at the bundle (no `yarn install` in this dir needed):

```sh
yarn workspace @dashevo/wasm-sdk build && yarn workspace @dashevo/evo-sdk build
export EVO_SDK_BUNDLE="$PWD/../packages/js-evo-sdk/dist/evo-sdk.module.js"
```

Then:

```sh
# 1. Register the contract on testnet (writes contract-id.testnet.json)
node src/register.mjs                 # --force to re-register a fresh contract

# 2. Seed testCases from TEST_PLAN.md (idempotent; skips existing)
node src/seed.mjs                     # all rows
node src/seed.mjs --ids CORE-01,ID-04 # a subset
node src/seed.mjs --tier Essential    # filter by tier / --category / --limit
node src/seed.mjs --update            # push changed rows (replace)

# 3. Submit a test-run result
node src/submit-run.mjs --testId CORE-05 --result pass --buildRef 45fdf33901 \
  --device "iPhone 16 (iOS 18.2)" --evidence "txid:30010050…17f840fc" --notes "2-output send credited both recipients"

# 4. Read back / verify indices (read-only; no key needed)
node src/query.mjs                    # self-check: exercises every index
node src/query.mjs --type testCase --tier Essential
node src/query.mjs --type testRun --testId CORE-05 --proof
```

`--result` must be one of `pass | fail | blocked | skipped`. Add `--proof` to any
`query.mjs` call to fetch with a verified Platform proof, `--json` for raw output.

### Files

```text
qa-contract/
├── schema/qa-contract.documents.json   # the two document types (the contract schema)
├── contract-id.testnet.json            # committed: live contract ID per network
├── src/
│   ├── sdk.mjs                          # SDK load, connect, signer, identity-key, config
│   ├── parse-test-plan.mjs              # TEST_PLAN.md §4 catalog parser
│   ├── register.mjs                     # register the contract
│   ├── seed.mjs                         # seed testCases from the plan (idempotent)
│   ├── submit-run.mjs                   # create one testRun
│   ├── query.mjs                        # read back + verify indices
│   └── derive-identity-key.mjs          # recover signing key from a wallet mnemonic
├── .env.example
└── README.md
```

## Testnet reset / re-seed

Public testnet resets periodically; the old contract ID stops resolving and all
documents are gone. To rebuild:

```sh
# 1. Re-register (auto-detected: register.mjs re-registers when the committed
#    contractId no longer resolves; --force to force it). Overwrites contract-id.testnet.json.
node src/register.mjs

# 2. Re-seed every testCase from the current plan
node src/seed.mjs

# 3. (testRun history does not survive a reset — it is re-accumulated as runs happen.)
```

Re-running `register.mjs` while the committed contract still resolves is a no-op
(it prints the existing ID). Re-running `seed.mjs` skips testCases that already
exist, so both scripts are safe to run repeatedly. Commit the updated
`contract-id.testnet.json` after a re-register so consumers (the website) pick up
the new ID.

## How it maps to the test plan

`seed.mjs` parses the §4 catalog tables of `TEST_PLAN.md` — `ID`, `Action`,
`Layer`, `Tier`, `Status` columns plus the section's `Domain=` (→ `category`) —
and creates one `testCase` per row (126 rows at the current plan commit). The
`simulator-control` QA runs then post results with `submit-run.mjs`, so the
on-chain `testRun` log mirrors what the automated QA agent actually executed.
