# QA Contract — on-chain QA framework storage layer

A dedicated Dash Platform data contract that stores **test definitions** and
**test-run results** on-chain, so QA status is queryable and proof-verifiable
and a website can render it. Complements GitHub issue
[#3897](https://github.com/dashpay/platform/issues/3897) and the iOS test plan at
[`packages/swift-sdk/SwiftExampleApp/TEST_PLAN.md`](../packages/swift-sdk/SwiftExampleApp/TEST_PLAN.md).

This package is the **storage layer only**: the schema, a register script, a
seed script (kept in sync with `TEST_PLAN.md`), an admin-list tool, a submit-run
helper, and a read/verify tool. The website that consumes the contract is a separate task and
only needs the contract ID below.

## Contract ID

The live contract ID for each network is committed in
`contract-id.<network>.json` (e.g. [`contract-id.testnet.json`](contract-id.testnet.json)),
written by the register script. Read it from there — testnet resets periodically,
so the ID changes when the contract is re-registered (see
[Testnet reset / re-seed](#testnet-reset--re-seed)).

> The keywords that say who may write a document type cannot change once the
> type exists, so a change to them is a fresh registration with a new id.
> Consumers pinned to an older id must re-pin. History:
> `2qEVUbg4znNgNRs3FJQ4kof4NKpB8q4fGtYa7qBouLzw` (v1) →
> `2gevmsNEaWnWQURQpuWeN5QnLfC2ufrZG4SXkVMqeUgZ` (v2: integer `network` +
> `$ownerId` testRun indices) → `4PtPYwYJcjuPXgKigkficzcrpKLG9yucqkNKKK9UVmiv`
> (v3: normalized `app`/`tier`/`category` lookup types with integer foreign keys,
> `(testId, app)` unique) → `67ctgcKJgCs7U4hhAxGj1QQUVq15xkkvMk88CT2AbjCF`
> (v4: hardening) → `9tshSfq5TU3gAEJCVa6KWxmb7cS7kkzRSmSSTmteKGcU` (v5: tags,
> immutable lookups) → the committed schema (v6: an `admin` list the owner
> keeps, and runs recorded by the identities on it; needs protocol version 14).

```jsonc
// contract-id.testnet.json (shape)
{
  "network": "testnet",
  "contractId": "<base58 contract id>",
  "ownerId": "<base58 QA identity id>",
  "documentTypes": ["admin", "app", "tier", "category", "testCase", "testRun"],
  "schemaSha": "<sha256 prefix of the schema>",
  "planCommit": "<TEST_PLAN.md git short-sha at register time>",
  "registeredAt": "<ISO timestamp>"
}
```

## Schema

Six document types (full schema in
[`schema/qa-contract.documents.json`](schema/qa-contract.documents.json)). The
catalog is **normalized**: `app`, `tier`, and `category` are lookup tables, and
`testCase`/`testRun` reference them by an integer **foreign key** (`code`). Dash
Platform has no joins, so consumers fetch the (tiny) lookup tables once and
resolve `code → name` client-side; the canonical codes live in
[`src/codes.mjs`](src/codes.mjs).

### `admin` — who may record runs

`{ identity: identifier (unique), label?: string }`. Only the contract owner
creates and deletes these (`creationRestrictionMode: 1`); creating one adds an
admin, deleting it removes one. `identity` must be an existing identity
(`refersTo`). `testRun` reads this list through `ownerRefersTo`, so the platform
itself refuses a run from anyone not on it, the contract owner included:
`register.mjs` lists the owner right after registering. Manage the list with
[`src/admin.mjs`](src/admin.mjs).

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

- Recorded by an admin: `creationRestrictionMode: 0` with
  `ownerRefersTo: { type: deletableDocument, documentType: admin, findBy: { identity: "." } }`,
  so the writer must have an `admin` document when the run is created. `$ownerId`
  is the admin that recorded the run. A run stays after its admin is removed.
- Indices (all `asc`; none starts with `$ownerId`, so a test's runs are read
  whoever recorded them; `app` pairs with `testId`):
  - `appTestNetworkCreated` — `app`, `testId`, `network`, `$createdAt` (also serves the equality-only `…, network` prefix)
  - `appTestResultCreated` — `app`, `testId`, `result`, `$createdAt`
  - `appTestCreated` — `app`, `testId`, `$createdAt`
  - `buildRefCreated` — `buildRef`, `$createdAt`
  - `ownerCreated` — `$ownerId`, `$createdAt` (one admin's runs)
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
> before broadcast. That runs the parser, which checks `ownerRefersTo` against the
> `admin` type's unique index, but not the document meta-schema: the asc-only index
> rule and an unknown or misspelled keyword are caught by drive-abci, at register time.

## Identities: the owner and the admins

Two roles sign documents:

- The **contract owner** registers the contract, seeds the lookups and the
  `testCase` catalog (all `creationRestrictionMode: 1`, **OwnerOnly**), and keeps
  the `admin` list.
- An **admin** is any identity the owner listed. Admins record `testRun`s, each
  with its own key, so nobody needs the owner's key to record a run and every
  run says who recorded it. The owner is an admin too.

```sh
node src/admin.mjs list                                   # no key needed
node src/admin.mjs add <identityId> --label "CI runner"   # signed by the owner
node src/admin.mjs remove <identityId>                    # signed by the owner
```

The catalog stays with the owner on purpose: a document can be replaced only by
the identity that created it, so a test case one admin created could not be
corrected by another, nor by the owner.

Whichever role you sign as, you need:

1. A registered testnet identity with a **credit balance** (registration + each
   document create costs credits). Create/fund one with the SwiftExampleApp
   (`ID-01`) or any Platform wallet.
2. The **private key** of an `AUTHENTICATION` key on that identity with **HIGH or
   CRITICAL** security level (WIF or 64-char hex). The scripts sign with it.

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

## Install & run

The scripts use the recommended **`@dashevo/evo-sdk`** (js-evo-sdk) in trusted
mode (required so state-transition responses are proof-verified). Node ≥ 22.

**Option A — standalone (published SDK):**

```sh
cd qa-contract
npm install
```

> Use `npm`, not `yarn`, here: `qa-contract` is intentionally *not* a member of
> the repo's Yarn workspaces, and Yarn 4 aborts when run from a non-member dir.
> (Option B builds the workspace SDK instead and needs no install in this dir.)
>
> The published SDK knows the protocol versions of its release. On a network
> that runs a later one (a devnet or a local network built from a development
> branch), use Option B: the scripts that sign refuse to run when the network
> is ahead of the SDK, because a write could land while the SDK fails to read
> the proof that confirms it. `register.mjs` also refuses while the network is
> voting for a version the SDK does not know, and on a network older than
> protocol version 14, which does not accept the schema.

**Option B — in-repo (workspace build):** build the workspace SDK once and point
the scripts at the bundle (no `yarn install` in this dir needed):

```sh
yarn workspace @dashevo/wasm-sdk build && yarn workspace @dashevo/evo-sdk build
export EVO_SDK_BUNDLE="$PWD/../packages/js-evo-sdk/dist/evo-sdk.module.js"
```

Then:

```sh
# 1. Register the contract on testnet (writes contract-id.testnet.json and
#    lists the owner as the first admin)
node src/register.mjs                 # --force to re-register a fresh contract

# 2. Seed testCases from TEST_PLAN.md (idempotent; skips existing)
node src/seed.mjs                     # all rows
node src/seed.mjs --ids CORE-01,ID-04 # a subset
node src/seed.mjs --tier Essential    # filter by tier / --category / --limit
node src/seed.mjs --update            # push changed rows (replace)

# 3. Let other identities record runs (owner only), then submit a result as
#    the owner or as any listed admin
node src/admin.mjs add <identityId> --label "Sam's laptop"
node src/submit-run.mjs --testId CORE-05 --result pass --buildRef 45fdf33901 \
  --device "iPhone 16 (iOS 18.2)" --evidence "txid:30010050…17f840fc" --notes "2-output send credited both recipients"

# 4. Read back / verify indices (read-only; no key needed)
node src/query.mjs                    # self-check: exercises every index
node src/query.mjs --type testCase --tier Essential
node src/query.mjs --type testRun --testId CORE-05 --proof
node src/query.mjs --type testRun --recordedBy <identityId>
```

`--result` must be one of `pass | fail | blocked | skipped`. Add `--proof` to any
`query.mjs` call to fetch with a verified Platform proof, `--json` for raw output.

### Files

```text
qa-contract/
├── schema/qa-contract.documents.json   # the six document types (the contract schema)
├── contract-id.testnet.json            # committed: live contract ID per network
├── src/
│   ├── sdk.mjs                          # SDK load, connect, signer, identity-key, networkId, config
│   ├── codes.mjs                        # canonical app/tier/category integer codes
│   ├── parse-test-plan.mjs              # TEST_PLAN.md §4 catalog parser
│   ├── register.mjs                     # register the contract
│   ├── seed.mjs                         # seed lookups + testCases from the plan (idempotent)
│   ├── admin.mjs                        # list / add / remove the admins who record runs
│   ├── submit-run.mjs                   # create one testRun
│   ├── migrate-runs.mjs                 # copy the previous contract's runs onto a new one
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

# 3. List the admins again: the list is gone with the old contract
node src/admin.mjs add <identityId> --label "..."

# 4. (testRun history does not survive a reset — it is re-accumulated as runs happen.)
```

Re-running `register.mjs` while the committed contract still resolves is a no-op
(it prints the existing ID). Re-running `seed.mjs` skips testCases that already
exist, so both scripts are safe to run repeatedly. Commit the updated
`contract-id.testnet.json` after a re-register so consumers (the website) pick up
the new ID.

## Moving to a new contract

A schema change that a contract update cannot carry (see [Contract ID](#contract-id))
needs a new contract. Until its id is committed, `contract-id.<network>.json`
still names the previous contract, and `submit-run.mjs`, `admin.mjs`, the
`query.mjs` self-check and its `admin` and `testRun` reads refuse to run against
it: they compare the schema hash saved at registration with the committed
schema. The contract owner moves over in this order:

```sh
# 1. Publish the committed schema as a new contract. Writes the new id to
#    contract-id.<network>.json and lists the owner as the first admin.
node src/register.mjs --force

# 2. Seed the lookups and the testCases, once per app
node src/seed.mjs
node src/seed.mjs --app KotlinExampleApp

# 3. List everyone else who records runs
node src/admin.mjs add <identityId> --label "..."

# 4. Stop recording runs on the previous contract and let the last ones land.
#    A run recorded there after its test was copied stays behind.

# 5. Copy the previous contract's runs, once per app (they get a new $createdAt).
#    The dry run needs no key.
node src/migrate-runs.mjs --dry-run
node src/migrate-runs.mjs
node src/migrate-runs.mjs --app KotlinExampleApp

# 6. Run the copy again. It skips what is already there, so it should report
#    "0 migrated"; if it copies more, a writer was still recording.
node src/migrate-runs.mjs
node src/migrate-runs.mjs --app KotlinExampleApp

# 7. Check every index, then commit contract-id.<network>.json
node src/query.mjs
```

Consumers (the website) re-pin to the new id from the committed file. The
previous contract stays readable; nothing is deleted from it.

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
