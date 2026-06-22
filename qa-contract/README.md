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
| Contract ID | `67ctgcKJgCs7U4hhAxGj1QQUVq15xkkvMk88CT2AbjCF` |
| Owner (QA identity) | `85KjYZLZXA7YZBPyFEjiMaH36xcQpBBZisKGBHF3uKuH` |
| Network | testnet |

> A data contract's schema is immutable, so each schema change is a fresh
> registration with a new id. Consumers pinned to an older id must re-pin.
> History: `2qEVUbg4znNgNRs3FJQ4kof4NKpB8q4fGtYa7qBouLzw` (v1) →
> `2gevmsNEaWnWQURQpuWeN5QnLfC2ufrZG4SXkVMqeUgZ` (v2: integer `network` +
> `$ownerId` testRun indices) → `4PtPYwYJcjuPXgKigkficzcrpKLG9yucqkNKKK9UVmiv`
> (v3: normalized `app`/`tier`/`category` lookup types with integer foreign keys,
> `(testId, app)` unique) → **deployed** (v4: hardening — non-deletable lookup rows,
> `result` enum, `network` `0..3`, redundant `ownerAppTestNetwork` index dropped).
>
> The committed schema additionally makes the `app`/`tier`/`category` lookups
> fully **immutable** (`documentsMutable: false`, so a `code`'s name can't be
> relabeled under historical runs). That post-dates the deployed v4 contract and
> applies at the next re-registration; `register.mjs` flags the drift (use
> `--force` to publish it as the next contract).

```jsonc
// contract-id.testnet.json (shape)
{
  "network": "testnet",
  "contractId": "<base58 contract id>",
  "ownerId": "<base58 QA identity id>",
  "documentTypes": ["app", "tier", "category", "testCase", "testRun"],
  "schemaSha": "<sha256 prefix of the schema>",
  "planCommit": "<TEST_PLAN.md git short-sha at register time>",
  "registeredAt": "<ISO timestamp>"
}
```

## Schema

Five document types (full schema in
[`schema/qa-contract.documents.json`](schema/qa-contract.documents.json)). The
catalog is **normalized**: `app`, `tier`, and `category` are lookup tables, and
`testCase`/`testRun` reference them by an integer **foreign key** (`code`). Dash
Platform has no joins, so consumers fetch the (tiny) lookup tables once and
resolve `code → name` client-side; the canonical codes live in
[`src/codes.mjs`](src/codes.mjs).

### Lookup tables: `app`, `tier`, `category`

Each is `{ code: integer (unique), name: string (unique) }` (`app` also has
optional `platform` + `description`). Indices: `byCode` (unique), `byName`
(unique). **Immutable** (`documentsMutable: false`, `canBeDeleted: false`) — a
stable code table, so a `code` referenced by an immutable testRun can't be
orphaned *or relabeled*; owner-only creation — add a new tier/category/app by
creating a doc with the next `code`, **no contract update needed**. Canonical codes:

- **app**: `0`=SwiftExampleApp
- **tier**: `0`=Essential, `1`=Common, `2`=Thorough, `3`=Uncommon, `4`=Manual, `5`=Unspecified
- **category**: `0`=Core, `1`=Identity, `2`=Address, `3`=DPNS, `4`=Voting, `5`=Contract, `6`=Document, `7`=Token, `8`=Shielded, `9`=DashPay, `10`=Group, `11`=System, `12`=MultiWallet

### `testCase` — a test definition (mirrors one test-plan row)

| Field | Type | Notes |
|---|---|---|
| `testId` | string (≤32) | e.g. `CORE-05`. Unique **per app**. |
| `app` | integer | FK → `app.code`. **Indexed.** |
| `tier` | integer | FK → `tier.code`. **Indexed.** |
| `category` | integer | FK → `category.code`. **Indexed.** |
| `title` | string (≤255) | the plan's *Action* column |
| `layer` | string (≤16) | Core / Platform / Cross / Shielded |
| `implStatus` | string (≤32) | status glyph (✅ 🧪 ⚠️ 🔌 🚫) |
| `description` | string (≤2048) | entry point & test notes (last plan column) |
| `entryPoint` | string (≤512) | primary view / FFI entry point |
| `prerequisites` | string (≤1024) | fixtures/preconditions |
| `planCommit` | string (≤64) | source-plan commit this row was seeded from |

- Indices: **`(testId, app)` unique** · `(app, tier)` · `(app, category)`.
- **Mutable** + deletable (impl-status / entry-point updates; removing dropped rows).
- `additionalProperties: false`.

### `testRun` — an append-only run record

| Field | Type | Notes |
|---|---|---|
| `testId` | string (≤32) | the run's test (pairs with `app`). **Indexed.** |
| `app` | integer | FK → `app.code`. **Indexed.** |
| `result` | string (≤16) | `pass` / `fail` / `blocked` / `skipped`. **Indexed.** |
| `network` | integer | `0`=mainnet, `1`=testnet, `2`=devnet, `3`=regtest. **Indexed.** |
| `buildRef` | string (≤63) | build under test. **Indexed.** |
| `device` | string (≤128) | device / simulator |
| `evidence` | string (≤512) | txid / on-chain id / screenshot path / URL |
| `notes` | string (≤2048) | free-form notes |
| `blockerReason` | string (≤512) | why blocked/skipped |
| `$createdAt` | system | **run time**, stamped by the platform; required + indexed |

- Indices (all `asc`; `$ownerId`-prefixed so runs are queried per submitter —
  sets up multi-submitter; `app` pairs with `testId`):
  - `ownerAppTestNetworkCreated` — `$ownerId`, `app`, `testId`, `network`, `$createdAt` (also serves the equality-only `…, network` prefix)
  - `ownerAppTestResultCreated` — `$ownerId`, `app`, `testId`, `result`, `$createdAt`
  - `ownerAppTestCreated` — `$ownerId`, `app`, `testId`, `$createdAt`
  - `buildRefOwner` — `buildRef`, `$ownerId`
- "Most recent run first" is done at query time with `orderBy [['$createdAt','desc']]`.
- **Immutable + non-deletable** (`documentsMutable: false`, `canBeDeleted: false`):
  it is an audit log. `additionalProperties: false`.
- `result` is constrained on-chain to `enum:[pass,fail,blocked,skipped]` and
  `network` to `0..3`, so out-of-vocabulary values can't enter the immutable log.
  `submit-run.mjs` additionally refuses an unknown `(testId, app)` (the run would
  be a permanent orphan) unless `--force`.

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
All five document types use `creationRestrictionMode: 1` (**OwnerOnly**), so only
that identity can create documents.

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

To let any identity submit runs (while keeping `testCase` owner-controlled), set
`creationRestrictionMode: 0` (NoRestrictions) on **`testRun`** only.

⚠️ This must be done in a **fresh contract registration**, *not* a data-contract
update: DPP rejects any change to a document type's `creationRestrictionMode` on
update (`DocumentTypeUpdateError`, see `validate_update`). So apply it before the
first registration, or fold it into the next re-register (e.g. a testnet reset) —
which mints a new contract id consumers must re-pin.

`testRun` is already immutable + owner-stamped (`$ownerId`/`$createdAt` are
system fields), so opening creation keeps every run attributable and tamper-proof.
Leave `testCase` as OwnerOnly so the canonical catalog stays curated.

## Install & run

The scripts use the recommended **`@dashevo/evo-sdk`** (js-evo-sdk) in trusted
mode (required so state-transition responses are proof-verified). Node ≥ 18.18.

**Option A — standalone (published SDK):**

```sh
cd qa-contract
npm install
```

> Use `npm`, not `yarn`, here: `qa-contract` is intentionally *not* a member of
> the repo's Yarn workspaces, and Yarn 4 aborts when run from a non-member dir.
> (Option B builds the workspace SDK instead and needs no install in this dir.)

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
├── schema/qa-contract.documents.json   # the five document types (the contract schema)
├── contract-id.testnet.json            # committed: live contract ID per network
├── src/
│   ├── sdk.mjs                          # SDK load, connect, signer, identity-key, networkId, config
│   ├── codes.mjs                        # canonical app/tier/category integer codes
│   ├── parse-test-plan.mjs              # TEST_PLAN.md §4 catalog parser
│   ├── register.mjs                     # register the contract
│   ├── seed.mjs                         # seed lookups + testCases from the plan (idempotent)
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

`seed.mjs` first ensures the `app`/`tier`/`category` lookup docs exist (codes from
[`src/codes.mjs`](src/codes.mjs)), then parses the §4 catalog tables of
`TEST_PLAN.md` — `ID`, `Action`, `Layer`, `Tier`, `Status` columns plus the
section's `Domain=` (→ `category`) — and creates one `testCase` per row (126 rows
at the current plan commit) under app `SwiftExampleApp`, mapping tier/category
names to their integer codes. Seed another app's plan with `--app <name>` (add the
app to `src/codes.mjs` first). The `simulator-control` QA runs then post results
with `submit-run.mjs` (`--app` defaults to SwiftExampleApp), so the on-chain
`testRun` log mirrors what the automated QA agent actually executed.
