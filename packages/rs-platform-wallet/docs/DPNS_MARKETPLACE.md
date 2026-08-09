# DPNS Username Marketplace — wallet-level design

Status: implementation in progress (2026-08-09). This document is the design
record for the wallet-level DPNS marketplace layer in `rs-platform-wallet`,
its FFI surface, and the swift-sdk wrappers. It also records the
browse-for-sale investigation result (§7), which is a protocol limitation the
wallet cannot work around.

## 1. Scope

The dashwallet-ios marketplace UI v1 (branch `feat/username-marketplace`)
composes the generic document-trade primitives directly
(`setDocumentPrice` / `purchaseDocument` / `transferDocument` +
`SDK.documentList`). This layer replaces that composition with durable
wallet-level operations the app can swap to without UI changes:

| Conceptual op (app v1)              | Wallet-level API                                   |
|-------------------------------------|----------------------------------------------------|
| search names + sale state           | `search_dpns_names_with_state(prefix, limit, start_after)` |
| my names (with sale state)          | local `DpnsNameStateEntry` rows, refreshed by sync |
| authoritative single-name re-read   | `dpns_name_state(label)`                           |
| set price                           | `set_dpns_name_price(identity, label, price, signer)` |
| delist                              | `delist_dpns_name(identity, label, signer)` (transfer-to-self) |
| purchase                            | `purchase_dpns_name(identity, label, expected_price, signer)` |
| gift transfer                       | `transfer_dpns_name(identity, label, recipient, signer)` |
| per-name history                    | `dpns_name_history(label)` (new capability — app had none) |

Prices are **credits** everywhere in this layer (1 duff = 1000 credits). The
duffs↔credits conversion is a UI concern.

## 2. On-chain semantics this design relies on (verified in source)

- `domain` v2: `documentsMutable=false`, `canBeDeleted=true`, `transferable=1`,
  `tradeMode=1`, all three `keeps*History` flags true. Indices:
  `parentNameAndLabel` (unique) and `identityId` (`records.identity`). No
  `$price` index (see §7).
- Purchase requires `$price` present (`DocumentNotForSaleError`, code 40108)
  and the transition's price must equal the listed price
  (`DocumentIncorrectPurchasePriceError`, code 40109 — carries both prices).
- **Both purchase and transfer remove `$price`**
  (`document_purchase_transition_action/v0/transformer.rs`,
  `document_transfer_transition_action/v0/transformer.rs`). Transfer-to-self
  is therefore the delist primitive: ownership is unchanged, `$price` is
  cleared by consensus. There is no dedicated "remove price" transition and
  `documentsMutable=false` rules out a replace.
- **`records.identity` is rewritten to the new owner by the protocol** on
  purchase and transfer (`rewrite_dpns_domain_identity_record_to_new_owner`,
  `action_convert_to_operations` v1). So the `identityId` index stays
  authoritative for "names associated with identity X" across sales, and a
  seller's sync pass observes sold names dropping out of its query result.
- History events are written to the **Document History system contract**
  (`6voHRaoiPcfmMhbqCA9dixH98xcgPQ9UEcuaXjpVu3LD`), doc types `transfer`,
  `purchase` (`sellerId`, `price`), `priceUpdate` (`price`), all with
  `$createdAt`/`$createdAtBlockHeight` required and a `byDocument`
  (dataContractId, documentId, $createdAt) index. `creationRestrictionMode=2`
  (protocol-only writes). This is NOT the GroveDB `documentsKeepHistory`
  mechanism — `getDocumentHistory` returns empty for DPNS and must not be
  used.
- A name in a live contest is **not in the documents tree** — trade
  transitions on it fail with a bare `DocumentNotFoundError` (40101), not a
  typed "contested" error. The wallet's contested guard exists to produce a
  typed error *before* broadcast.
- Purchase balance semantics: the purchase amount is deducted as principal
  first; processing fees must fit in the remainder, else
  `IdentityInsufficientBalanceError`.

## 3. Persistence: `DpnsNameStateEntry` (new store, not an `IdentityEntry` change)

`IdentityEntry` is persisted as an **unversioned positional bincode blob**
(`rs-platform-wallet-storage` `schema/blob.rs`), so extending `DpnsNameInfo`
in place would break decoding of existing rows. Instead, marketplace state is
a new sub-changeset, following the `InvitationEntry` template:

```rust
pub enum DpnsNameSaleStatus { Owned, Sold { to: Identifier }, Transferred { to: Identifier } }

pub struct DpnsNameStateEntry {
    pub document_id: Identifier,          // key; stable across ownership changes
    pub wallet_identity_id: Identifier,   // which of our identities this row belongs to
    pub label: String,
    pub normalized_label: String,
    pub normalized_parent_domain_name: String,
    pub price: Option<Credits>,           // None = not listed
    pub status: DpnsNameSaleStatus,
    pub created_at_ms: Option<u64>,
    pub updated_at_ms: Option<u64>,
    pub transferred_at_ms: Option<u64>,
    pub last_synced_at_ms: u64,
}

pub struct DpnsNameStateChangeSet {
    pub names: BTreeMap<Identifier, DpnsNameStateEntry>,  // LWW per document_id
    pub removed: BTreeSet<Identifier>,                    // tombstones
}
```

- Capability bit `DPNS_NAME_STATES` (next free bit), SQLite migration + writer
  in `rs-platform-wallet-storage`, FFI persister vtable slot
  (`on_persist_dpns_name_states_fn`) mirroring into SwiftData.
- Merge: LWW per `document_id` (every emitter writes fresh rows read from
  Platform or from a confirmed transition), tombstone wins over stale upsert
  within one changeset generation (same insert-XOR-tombstone discipline as
  invitations).
- Sold/Transferred rows are kept (status flips), not deleted — the app shows
  "sold" affordances; `removed` exists for hard-delete correctness.
- The legacy `ManagedIdentity.dpns_names: Vec<DpnsNameInfo>` label list stays
  (Swift `PersistentIdentity.dpnsName` selection feeds off it), but its merge
  policy changes from append-only-by-label to **last-write-wins wholesale**
  (same policy as `contested_dpns_names`, same rationale: sold names must be
  able to leave). Every emitter snapshots the full list from managed state, so
  LWW converges. This is a merge-policy change only — the bincode layout of
  `IdentityEntry` is untouched.

## 4. Sync

`DpnsSyncManager` (sibling of `DashPaySyncManager`, same
snapshot/quiesce/log-and-continue skeleton, default 60s cadence; not
auto-started; on-demand FFI entry as well). Per wallet, per identity:

1. Query domain documents where `records.identity == identity` (existing
   indexed query), full documents so `$id`/`$price`/timestamps are read.
2. Upsert `DpnsNameStateEntry` rows; update the legacy label list (add new
   labels with `acquired_at` from `$createdAt`/`$transferredAt`, remove
   departed labels).
3. For each departed label (present locally, absent from the query), fetch the
   domain doc by exact label to learn the new owner → flip the row to
   `Sold`/`Transferred` (distinguished by history-contract `purchase` doc when
   cheaply determinable, else `Transferred`), emit it in the pass summary.
4. Refresh identity credit balance for identities that sold a name (seller
   receives the sale price as credits).
5. Also refreshes `contested_dpns_names` (piggybacks the existing sync).

Pass summary fires `PlatformEventHandler::on_dpns_sync_completed(&summary)`
so the host can refresh profile/main-username UI when a name (possibly the
main username) left an identity. Rust has no "main username" concept — the
fallback choice stays host-side (Swift `PersistentIdentity.dpnsName`), driven
by the mirrored row updates + the event.

## 5. Orchestration ops (all on `IdentityWallet`, in `network/dpns_marketplace.rs`)

Common plumbing: DPNS + history contracts fetched via the existing
`fetch_contract_arc_for_document_op` path (context-provider registration
included) and cached in `OnceLock`s à la `dashpay_contract()`. Signing keys
are auto-selected (`AUTHENTICATION`, ECDSA, security level from the document
type's requirement — the same `allowed_signing_security_levels` rule as
document create); no hardcoded key ids. All broadcasts wrap errors with
`preserve_signer_key_unavailable_or` and the new consensus-error downcasts.

- `set_dpns_name_price(owner_identity, label, price, signer)`:
  authoritative exact-label fetch → ownership check → **contested guard**
  (`get_current_dpns_contests` — refuse with `ContestedNameNotTradable`) →
  `document_set_price` → upsert row (price from confirmed doc) → return state.
- `delist_dpns_name(owner_identity, label, signer)`:
  same guards → `document_transfer` with `recipient == owner` →
  **verify the confirmed document carries no `$price`** (honest delist —
  error if consensus semantics ever change) → upsert row price=None.
- `purchase_dpns_name(purchaser_identity, label, expected_price, signer)`:
  authoritative exact-label fetch → typed pre-checks: `DpnsNameNotFound`,
  self-purchase (`InvalidParameter`), `NotForSale`, `PriceChanged{expected,
  actual}` → **credit pre-check**: local purchaser balance ≥ expected_price +
  `DOCUMENT_TRANSITION_FEE_RESERVE_CREDITS` (0.001 DASH = 100_000_000 credits,
  ~2× the observed document-batch fee) else
  `InsufficientIdentityCredits{required, available}` →
  `document_purchase` **with `expected_price`, never the re-read price** (the
  consensus equality check is the backstop; a lost race surfaces as typed
  `PriceChanged` via the 40109 downcast) → buyer reconcile (label list +
  row + `refresh_identity` for the new balance) → seller reconcile *if the
  seller identity is also in this wallet* (label removal, row → Sold, balance
  refresh).
- `transfer_dpns_name(owner_identity, label, recipient, signer)`: gift path,
  same guards, recipient reconcile if recipient is ours.
- `dpns_name_history(label)`: resolve document id (live doc, or local row for
  names that already left) → three `byDocument` queries on the history
  contract → merged, `$createdAt`-ordered
  `Vec<DpnsNameHistoryEvent>`:

```rust
pub enum DpnsNameHistoryEventKind {
    Registered,                                       // domain doc $createdAt
    PriceSet { price: Credits },                      // priceUpdate doc
    Purchased { price: Credits, seller: Identifier, buyer: Identifier },
    Transferred { from: Identifier, to: Identifier }, // incl. self = delist
}
pub struct DpnsNameHistoryEvent {
    pub kind: DpnsNameHistoryEventKind,
    pub at_ms: u64,
    pub block_height: Option<u64>,
}
```

- Queries: `search_dpns_names_with_state(prefix, limit, start_after)` and
  `dpns_name_state(label)` return `DpnsDomainState` (document id, labels,
  owner, records identity, price, timestamps) read straight off the domain
  documents — the sale state the SDK's `DpnsUsername` drops. Cursor pagination
  uses `DocumentQuery::start` (StartAfter document id) natively, bypassing the
  rs-sdk-ffi `start_at` gap.

## 6. Typed errors

New `PlatformWalletError` variants (with FFI codes from the free registry
slots, mirrored in `PlatformWalletResultCode` + `PlatformWalletError` (Swift)):

| Variant | Trigger | FFI detail payload (JSON in `message`) |
|---|---|---|
| `DpnsNameNotFound { name }` | exact-label query empty | — |
| `DocumentNotForSale { document_id }` | pre-check, or 40108 downcast | — |
| `DocumentPriceChanged { document_id, expected, actual }` | pre-check, or 40109 downcast | `{"expected":u64,"actual":u64}` |
| `InsufficientIdentityCredits { identity_id, required, available }` | pre-check, or `IdentityInsufficientBalanceError` downcast | `{"required":u64,"available":u64}` |
| `ContestedNameNotTradable { label, ends_at_ms }` | contested guard | `{"endsAtMs":u64}` |

Downcast helpers (`as_document_not_for_sale`, `as_incorrect_purchase_price`,
`as_identity_insufficient_balance`) follow the existing
`as_address_invalid_nonce` pattern so consensus rejections arrive typed, not
stringly. The structured-JSON `message` convention for value-carrying codes is
documented at the FFI enum and parsed by swift-sdk into typed Swift cases
(fallback: raw string).

## 7. Browse-for-sale: protocol limitation (investigated, not buildable here)

A global "names currently for sale, ordered by price" needs an index on
`$price`. **This cannot ship as a DPNS contract v3.** Verified findings:

- `$price` is not in rs-dpp's closed `SYSTEM_PROPERTIES` indexable set
  (`system_properties/mod.rs`); an index on it fails contract parsing with
  `UndefinedIndexPropertyError` — for a system contract that's a node-fatal
  load failure, not a soft rejection.
- `serialize_value_for_key` / `get_raw_for_document_type` /
  `conditions.rs::meta_field_property_type` all lack `$price` arms — it can be
  neither an index key nor a typed where/orderBy field. Unindexed where
  clauses are rejected by drive twice over.
- Index definitions are immutable on `DataContractUpdate` for *all* contracts
  (`DataContractInvalidIndexDefinitionUpdateError`), and the DPNS owner id is
  the unsignable `[0;32]` — only the protocol-upgrade path
  (`transition_to_version_N` + `apply_contract`) can change the contract, and
  even that creates the new index tree **empty** (no backfill machinery
  exists; pre-existing listings would be invisible until re-listed).
- Required upgrade path if this is ever wanted (PV15+): add `$price` to the
  indexable set behind a `FeatureVersion` gated on `trade_mode` (the
  `$creatorId` precedent), add the three encode/decode arms + query meta-field
  typing, new `try_from_schema` generation, DPNS `schema/v3` +
  `system_data_contract_versions/v3.rs`, and a `transition_to_version_15`
  re-`apply_contract`. Plus a backfill decision.

**Until then the marketplace is search-driven** (prefix search + per-name sale
state), which is what this layer exposes. Partial aggregate discovery IS
available from the history contract (its user-defined `price` property has
real indices: `byPrice`, averageable) — e.g. recent sales and price history —
and `dpns_name_history` builds on that. A "recently listed" feed could later
be derived from `priceUpdate` documents by `$createdAt` if the app wants it.

## 8. Verification plan

- `cargo test -p platform-wallet -p platform-wallet-storage`, clippy, cbindgen
  build, `build_ios.sh --target mac` + `swift build` for the Swift layer.
- Testnet end-to-end (documented in §9 once run): register/own name on
  identity A → `set_dpns_name_price` → price change → `purchase_dpns_name`
  from identity B → `dpns_name_history` shows priceUpdate ×2 + purchase →
  `delist_dpns_name` on another listed name confirms transfer-to-self clears
  `$price` on the confirmed document and on a fresh query.

## 9. Testnet verification results

Run 2026-08-09 via `examples/dpns_marketplace_testnet.rs` (phase `run`)
against testnet DAPI, seller = wallet HD identity index 1, buyer = index 3
(distinct identities, same wallet — exercising both buyer- and seller-side
reconciliation). Every check passed:

| Step | Result |
|---|---|
| register `mktp1786261653test.dash` (uncontested) on seller | PASS |
| `set_dpns_name_price` 1,000,000 credits — confirmed doc + fresh query | PASS |
| re-price to 2,000,000 credits | PASS |
| purchase at stale price → typed `DocumentPriceChanged{expected:1M, actual:2M}` (pre-broadcast) | PASS |
| `purchase_dpns_name` at 2M → owner flips to buyer | PASS |
| purchase clears `$price` on the confirmed document | PASS |
| protocol rewrote `records.identity` to the buyer | PASS |
| local marketplace row → buyer / `Owned` | PASS |
| `dpns_name_history` → `Registered`, `PriceSet(1M)` @503688, `PriceSet(2M)` @503689, `Purchased{2M, seller, buyer}` @503690 — ordered, with block heights | PASS |
| purchase of unlisted name → typed `DocumentNotForSale` | PASS |
| re-list 3M then `delist_dpns_name` (transfer-to-self) → confirmed doc `$price=None`, owner unchanged | PASS |
| fresh query after delist → `$price=None` (with bounded lagging-replica retry) | PASS |
| `search_dpns_names_with_state` prefix search finds the name | PASS |
| `sync_dpns_marketplace` pass → 6 names tracked, no spurious deltas | PASS |

Findings folded back into the implementation during verification:

- `Sdk::register_dpns_name` never registers the DPNS contract with the
  context provider, so on hosts that don't pre-seed known contracts the
  post-broadcast proof fails with "unknown contract … in document
  verification" **after the registration landed**. Fixed:
  `register_name_with_external_signer` now fetches+registers the contract
  first, and the marketplace contract caches hold the **on-chain fetched**
  contract (registered with the provider) rather than the bundled one.
- Fresh reads right after a broadcast can race a lagging replica (a
  banned/slow node serving the previous block). The confirmed document
  returned by each transition is the authoritative proof-verified state;
  UI-level re-reads should tolerate one block of replica lag.
