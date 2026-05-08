# Document Count Trees

Counting the documents that match a query used to mean fetching them and calling `.len()`. From protocol v12 onward, document types can opt into a different primary-key tree variant that maintains a running count inside the tree itself, turning `count(*)`-style queries into an O(1) lookup. This chapter explains the three tree variants, how a document type selects one, the migration that protects pre-v12 contracts from accidentally landing on the new layout, and the two query endpoints that expose the feature.

## Why Count Trees Exist

The default primary-key tree for a document type is a `NormalTree`. To count the documents in it, Drive walks the subtree, deserializes every record, and returns the length of the resulting collection. That is fine for small types but becomes painful as soon as a UI needs "how many widgets are there?" on a contract with millions of widgets.

GroveDB has two count-aware tree variants that store the size of the subtree inside the tree node:

- `CountTree` — stores a `u64` count alongside the root key. Total count is an O(1) read of the tree element itself.
- `ProvableCountTree` — same, plus the count value is committed to the Merkle root so a client can verify a count against a tenderdash-signed proof. This unlocks per-subtree provable counts (range-countable queries).

A document type that opts in via the schema flag `documentsCountable: true` gets a `CountTree` for its primary-key tree; opting in with `rangeCountable: true` (which implies `documentsCountable`) gets a `ProvableCountTree`.

## How a Document Type Picks Its Tree Variant

Selection lives in [`packages/rs-drive/src/drive/document/primary_key_tree_type.rs`](https://github.com/dashpay/platform/blob/v3.1-dev/packages/rs-drive/src/drive/document/primary_key_tree_type.rs):

```rust
impl DocumentTypePrimaryKeyTreeType for DocumentTypeRef<'_> {
    fn primary_key_tree_type(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<TreeType, Error> {
        match platform_version
            .drive
            .methods
            .document
            .primary_key_tree_type
        {
            0 => {
                if self.range_countable() {
                    Ok(TreeType::ProvableCountTree)
                } else if self.documents_countable() {
                    Ok(TreeType::CountTree)
                } else {
                    Ok(TreeType::NormalTree)
                }
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DocumentTypeRef::primary_key_tree_type".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
```

`primary_key_tree_type()` is the single source of truth — every Drive code path that needs to know which tree variant to read from or write to routes through this helper, including:

- Contract insert and update (to `CREATE` the right tree element when the document type is added).
- Document insert / delete (to know how to update the count alongside the document).
- Cost estimation (so fees match the variant that will actually be used).

The contract insert/update paths use three thin `Drive` helpers parallel to the existing `batch_insert_empty_tree` / `batch_insert_empty_sum_tree`:

- `batch_insert_empty_tree` — NormalTree.
- `batch_insert_empty_count_tree` — CountTree, used when `documents_countable() && !range_countable()`.
- `batch_insert_empty_provable_count_tree` — ProvableCountTree, used when `range_countable()`.

Each helper goes through `LowLevelDriveOperation::for_known_path_key_empty_*_tree` (or its `_estimated_path_key_*` cousin in cost-estimation paths), so the contract setup, document operations, and proof generation all see the same on-disk shape.

## Storage-Layout Invariants

Because the tree variant is fixed at contract-creation time and baked into how the tree element is laid out on disk, two flags are *immutable* across a contract update:

- Changing `documents_countable` from any state to any other state on a `validate_config` update returns `DocumentTypeUpdateError`.
- Same for `range_countable`.

Tests pinning these guards live in `packages/rs-dpp/src/data_contract/document_type/methods/validate_update/v0/mod.rs`. Don't relax them: if a `NormalTree`-backed document type were silently switched to `CountTree` mid-contract, every subsequent insert or delete would update a count value attached to a tree element that physically isn't a count tree, leading to grovedb element-shape errors at best and consensus drift at worst.

## v11 → v12 Migration: Strip Smuggled Flags

The `documentsCountable` / `rangeCountable` keys are accepted by the v0 document meta-schema (the one pre-v12 contracts validate against) because v0 has no top-level `additionalProperties: false`. That means a contract created on v11 *could* embed `documentsCountable: true` in its raw schema bytes and pass validation, even though the v1 parser used pre-v12 ignored those keys and the contract was always created with a `NormalTree`.

When v12 activates, the v2 parser reads the new flags. If a pre-v12 contract had smuggled them, deserializing it under v12 would suddenly report `primary_key_tree_type() == CountTree` for a tree that physically lives on disk as a `NormalTree`. Every subsequent operation would compute fees against the wrong tree shape and could corrupt grove state.

The fix is a one-shot migration in [`packages/rs-drive/src/drive/contract/migration/strip_unknown_document_schema_properties.rs`](https://github.com/dashpay/platform/blob/v3.1-dev/packages/rs-drive/src/drive/contract/migration/strip_unknown_document_schema_properties.rs), gated to run on the first block of v12 in `transition_to_version_12`. It iterates every stored contract (and every historical revision of `documentsKeepHistory` contracts), deserializes the schemas, and strips any top-level key not in `ALLOWED_TRANSITION_TO_DOCUMENT_SCHEMA_V1_PROPERTIES` (which is exactly the v0 top-level set plus a curated list of JSON-Schema-standard keywords that are pure documentation or describe instance shape — neither of which affect storage layout). The v12-introduced flags are deliberately *not* in that list, so the migration removes them.

End-to-end coverage of this lives in `test_v12_migration_strips_unknown_document_schema_properties` in `packages/rs-drive-abci/src/execution/platform_events/protocol_upgrade/perform_events_on_first_block_of_protocol_change/v0/mod.rs`. The test creates a v11 contract with smuggled `documentsCountable` and `rangeCountable` in its raw bytes, runs the migration, and asserts both keys are absent from the on-disk contract bytes *and* from the contract that comes back through the Drive cache + fetch API.

After the migration runs the contract cache is cleared, so the next read goes through the fetch path and rebuilds a `DocumentTypeV2` from the now-stripped bytes — guaranteeing no in-memory `DocumentType` retains the smuggled flags.

## Counting Documents at Query Time

Two gRPC endpoints expose the feature:

- `GetDocumentsCount` — total count of documents matching a query, optionally with proof.
- `GetDocumentsSplitCount` — counts split by an index property, again optionally with proof.

Both endpoints have two underlying paths:

### No-Prove (Server-Side O(1))

When `prove=false`, drive-abci calls into `DriveDocumentCountQuery` (in [`packages/rs-drive/src/query/drive_document_count_query.rs`](https://github.com/dashpay/platform/blob/v3.1-dev/packages/rs-drive/src/query/drive_document_count_query.rs)). For total counts the path is roughly:

1. Pick a `CountTree`-typed primary-key index whose properties cover all *equality* `WhereClause` predicates (a covering index — see the limitation note below).
2. Walk the tree from the root down to the deepest level fully covered by equality predicates, pushing `prop_name` and `serialize_value_for_key(prop_name, value)` onto the path at each level.
3. If every index property was covered: read the `CountTree` element at the resulting path and return its built-in `u64` count. O(1).
4. If only a prefix was covered: sum the counts of all `CountTree` children at the deepest covered level.

For split counts the path is similar, but stops at the level *before* the split property, then for each value subtree under the split-property level reads its sub-count and emits a `(key_bytes, count)` entry. The result is wire-formatted as `repeated SplitCountEntry { bytes key; uint64 count }`.

### Prove (Client-Side Verify-Then-Aggregate)

When `prove=true`, drive-abci returns a standard `DriveDocumentQuery` proof of the matching documents themselves — there is no signed-count primitive on the wire today. The client then verifies the proof, deserializes the documents, and aggregates locally:

- For total counts the aggregation is `documents.len() as u64` ([`packages/rs-drive-proof-verifier/src/proof/document_count.rs`](https://github.com/dashpay/platform/blob/v3.1-dev/packages/rs-drive-proof-verifier/src/proof/document_count.rs)).
- For split counts the aggregation walks each verified document, reads `properties.get(split_property)`, encodes the value via `document_type.serialize_value_for_key(split_property, value, platform_version)` so the byte keys line up with what the no-prove path produces, and increments the per-key counter ([`packages/rs-drive-proof-verifier/src/proof/document_split_count.rs`](https://github.com/dashpay/platform/blob/v3.1-dev/packages/rs-drive-proof-verifier/src/proof/document_split_count.rs)).

Aggregation needs the split-property name, but `DriveDocumentQuery` does not carry it. The proof verifier exposes a dedicated entry point that takes it explicitly:

```rust
DocumentSplitCounts::maybe_from_proof_with_split_property(
    drive_query,
    split_property,
    response,
    network,
    platform_version,
    provider,
)
```

The generic `FromProof<Q>` impl on `DocumentSplitCounts` is intentionally *not* the way to reach split counts under proof — calling it returns an explicit error. This is a load-bearing design choice: an earlier version of this code silently returned `Some(BTreeMap::new())` from the generic path, so any caller using `prove=true` got a valid-looking but empty result. Erroring loudly forces every caller to thread the split property through.

### Known Limitation: Non-Equality Where Clauses

The no-prove fast path only looks at `WhereOperator::Equal` predicates when picking a covering index. Any clause using `>`, `<`, `between`, `in`, or `startsWith` is silently ignored, and the count returned is the count for whatever equality prefix happens to match — *not* the count of documents satisfying the full query. This affects both `find_countable_index_for_where_clauses` (total count) and `find_countable_index_for_split` (split count). The prove path doesn't have this issue because it returns the actual matching documents and aggregates client-side.

This is tracked as a known issue on the count endpoint PR; until it's resolved, callers that need exact counts under range predicates should use `prove=true` even if they don't otherwise need a proof.

## SDK Access at Three Layers

### `rs-sdk` (native Rust)

Both endpoints land on the standard `Fetch` trait:

```rust
use dash_sdk::platform::documents::document_count_query::DocumentCountQuery;
use dash_sdk::platform::documents::document_split_count_query::DocumentSplitCountQuery;
use dash_sdk::platform::Fetch;
use drive_proof_verifier::{DocumentCount, DocumentSplitCounts};

let count = DocumentCount::fetch(&sdk, DocumentCountQuery::new(contract.clone(), "widget")?)
    .await?
    .unwrap_or(DocumentCount(0));

let splits = DocumentSplitCounts::fetch(
    &sdk,
    DocumentSplitCountQuery::new(contract, "widget", "color")?,
)
.await?
.unwrap_or_default();
```

`DocumentCountQuery` and `DocumentSplitCountQuery` wrap an internal `DocumentQuery` (so they reuse where-clause / order-by / contract-id machinery) and expose a `with_where(WhereClause)` builder for filters. Their `TransportRequest` impls target `GetDocumentsCountRequest` / `GetDocumentsSplitCountRequest`; their `FromProof` impls go through the dedicated proof-verifier entry points described above.

### `wasm-sdk` (browser)

Four methods on the `WasmSdk` JS class:

```typescript
sdk.getDocumentsCount(query: DocumentsQuery): Promise<bigint>;
sdk.getDocumentsCountWithProofInfo(
  query: DocumentsQuery,
): Promise<ProofMetadataResponseTyped<bigint>>;

sdk.getDocumentsSplitCount(
  query: DocumentsQuery,
  splitProperty: string,
): Promise<Map<string, bigint>>;
sdk.getDocumentsSplitCountWithProofInfo(
  query: DocumentsQuery,
  splitProperty: string,
): Promise<ProofMetadataResponseTyped<Map<string, bigint>>>;
```

The split-count map's keys are *hex-encoded bytes*. They correspond to the canonical `serialize_value_for_key` encoding of each property value, so callers that need a typed key (`"red"`, `42`, etc.) need to hex-decode and interpret per the contract's index-property type. This shape matches the no-prove server response too, so a caller that wants to merge or compare count maps from both paths doesn't need a transformation step.

### `rs-sdk-ffi` (iOS / native bindings)

```rust
dash_sdk_document_count(sdk, data_contract, document_type, where_json)
    -> JSON {"count": <u64>}

dash_sdk_document_split_count(sdk, data_contract, document_type, split_property, where_json)
    -> JSON {"counts": {"<hex-key>": <u64>, ...}}
```

`where_json` is the same JSON shape `dash_sdk_document_search` already accepts (`[{field, operator, value}]`), so iOS callers can reuse their where-clause encoding. Both endpoints return their results as a JSON-encoded C string allocated on the heap — caller frees it via the standard SDK string-free routine.

## What's Tested vs. What's Not

Coverage at each layer:

| Layer | Coverage |
|---|---|
| `rs-drive` `primary_key_tree_type` dispatch | 4 unit tests (`NormalTree`/`CountTree`/`ProvableCountTree` + range-takes-priority) |
| `rs-drive` `batch_insert_empty_*_count_tree` helpers | 6 unit tests (KeyRef/KeySize/Key for each variant) |
| `rs-drive` end-to-end count tree creation + maintenance | 9 e2e tests under `countable_e2e_tests` (real grovedb, real `apply_contract`, real document insert/delete, fee differentiation) |
| `rs-drive-abci` v11→v12 migration | 1 e2e test that injects smuggled flags and asserts they're stripped on disk and via the Drive fetch API |
| `rs-drive-abci` query handlers | 7 tests across both endpoints (no-prove, with-prove, error cases) |
| `rs-sdk` Fetch | 7 mock-based integration tests including a regression that pins the proof-verifier's loud-error behavior |
| `wasm-sdk` | Mocha tests would require live-devnet vectors — covered indirectly through Karma smoke tests against built wasm |
| `rs-sdk-ffi` | Same gap; SwiftExampleApp / iOS SDK coverage exercises the FFI surface |

The biggest gap that remains is end-to-end proof verification through the platform-test-suite — there's nothing today that takes a real on-chain v12 contract with `documentsCountable: true`, queries it through the SDK with `prove=true`, and asserts the verified count matches the on-chain truth. That's a follow-up after `SDK_TEST_DATA=true` test-vector generation.
