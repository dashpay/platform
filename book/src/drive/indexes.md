# Indexes

Drive stores documents in GroveDB. Every document type has a **primary-key tree** (documents keyed by document ID), plus zero or more **secondary indexes** the contract author declares in the document schema. This chapter is a reference for the `Index` struct's fields, what they mean for the on-disk layout, and how Drive walks indexes during inserts and queries.

## What an Index Is

A document type's `indices` array tells Drive: "for queries that filter or sort by these properties, build a sorted lookup so they don't have to enumerate every document." Each entry in `indices` becomes one secondary index; Drive maintains it on every insert/update/delete so that queries which match the index prefix are O(prefix walk) rather than O(documents).

Concrete example. Given:

```json
{
  "person": {
    "type": "object",
    "indices": [
      {
        "name": "byLastName",
        "properties": [{ "lastName": "asc" }]
      }
    ],
    "properties": {
      "firstName": { "type": "string", "position": 0 },
      "lastName":  { "type": "string", "position": 1 }
    },
    "required": ["firstName", "lastName"],
    "additionalProperties": false
  }
}
```

— a query like `where lastName = "Smith"` reaches the matching documents through the `byLastName` index in O(log n) plus the per-result IO. Without that index it would be a full document-type scan.

## The `Index` Struct

The compiled-Rust shape — the JSON schema fields are deserialized into this — lives in [`packages/rs-dpp/src/data_contract/document_type/index/mod.rs`](https://github.com/dashpay/platform/blob/v3.1-dev/packages/rs-dpp/src/data_contract/document_type/index/mod.rs):

```rust
pub struct Index {
    pub name: String,
    pub properties: Vec<IndexProperty>,
    pub unique: bool,
    pub null_searchable: bool,
    pub contested_index: Option<ContestedIndexInformation>,
    pub countable: IndexCountability,
}

pub struct IndexProperty {
    pub name: String,
    pub ascending: bool,
}
```

### `name`

A short, human-readable identifier for the index (e.g. `"byOwnerAndType"`). It shows up in error messages and is the key used in `document_type.indexes()` (`BTreeMap<String, Index>`). If omitted in the schema, a random alphanumeric name is generated. Two indexes within the same document type cannot share a name.

### `properties: Vec<IndexProperty>`

The ordered list of columns this index covers. Each `IndexProperty` is a `(name, ascending)` pair. Order matters: a query has to match a *prefix* of these properties for the index to be useful. An index `[lastName, firstName]` answers `where lastName = X` and `where lastName = X AND firstName = Y` but **not** `where firstName = Y` alone.

The schema form is:

```json
"properties": [
  { "lastName":  "asc" },
  { "firstName": "asc" }
]
```

`asc` / `desc` controls sort order on result enumeration. Drive currently only uses ascending storage, but the field is preserved through the contract.

### `unique: bool`

If `true`, no two documents may share the same combination of values for the indexed properties. The platform enforces this on insert: a duplicate trips a `DuplicateUniqueIndexError` consensus error.

A unique index changes the on-disk layout at the terminal level: instead of a sub-tree of document references keyed by document ID, the terminal stores a single bare `Reference` element pointing at the one document that matched. See [Tree Type at the Terminal Level](#tree-type-at-the-terminal-level) below.

Uniqueness can't be enforced when an indexed property is null, so a document with any null in the index path falls back to the non-unique storage shape for that document. See [Null Handling](#null-handling).

### `null_searchable: bool`

Defaults to `true`. Controls what happens when **all** indexed properties of a document are null:

- `null_searchable: true` — the document is still indexed at the all-null path, so a query against the all-null prefix can find it.
- `null_searchable: false` — Drive skips the index insertion entirely. Documents with all-null index values exist (in the primary-key tree) but are not reachable via this index.

The flag only affects the all-null case. A document with *some* null values gets indexed regardless.

### `contested_index: Option<ContestedIndexInformation>`

When set, this index identifies a **scarce, contested resource** (the canonical example is a DPNS name like `dash`). Documents trying to register the same value under a contested index don't auto-fail with a uniqueness error — they enter a masternode-vote resolution where each contender's claim is held until voting concludes. Contested indexes must also be `unique: true`; the parser rejects the combination otherwise.

Out of scope for this chapter; see DPNS / contested-resource docs for the full lifecycle.

### `countable: IndexCountability`

Controls whether the terminal tree under each indexed value carries a count, and which count-tree variant. Three variants:

| Value | Tree variant | Capabilities |
|---|---|---|
| `NotCountable` (default) | `NormalTree` | No count fast path |
| `Countable` | `CountTree` | O(1) totals at the root |
| `CountableAllowingOffset` | `ProvableCountTree` | O(1) totals **plus** per-node counts that will enable future O(log n) range / offset queries |

The schema accepts both the legacy boolean form (`true` → `Countable`, `false` → `NotCountable`) and the camelCase string form (`"notCountable"` / `"countable"` / `"countableAllowingOffset"`). For the full design rationale see [Document Count Trees](document-count-trees.md).

## How Drive Builds the IndexLevel Trie

The flat list of `Index`es declared on a document type is compiled, at contract-load time, into an `IndexLevel` trie ([`packages/rs-dpp/src/data_contract/document_type/index_level/mod.rs`](https://github.com/dashpay/platform/blob/v3.1-dev/packages/rs-dpp/src/data_contract/document_type/index_level/mod.rs)):

```rust
pub struct IndexLevel {
    sub_index_levels: BTreeMap<String, IndexLevel>,
    has_index_with_type: Option<IndexLevelTypeInfo>,
    level_identifier: u64,
}
```

Each property name in any index becomes an edge in this trie; indexes that share a prefix share their initial path. An index "terminates" at a level by setting `has_index_with_type = Some(...)` — that's how the recursive insert / lookup code knows it's at the last property of a defined index, vs. just walking through a shared prefix.

Given two indexes:
- `byOwnerAndType = [ownerId, docType]`
- `byOwnerAndStatus = [ownerId, status]`

the trie is:

```
(root)
└── ownerId
    ├── docType    ← terminates byOwnerAndType
    └── status     ← terminates byOwnerAndStatus
```

The `ownerId` level is shared between both indexes. The `docType` and `status` levels each set `has_index_with_type` on themselves with their own `unique` / `countable` / `null_searchable` flags.

This trie shape directly mirrors the GroveDB path shape used at insert / query time.

## GroveDB Layout

A document under contract `C` of type `T` with index property `propA = vA, propB = vB` lives at the grove path:

```
[ DataContractDocuments,  contract_id,  1,  doc_type_name,
  propA_name, vA,  propB_name, vB,  0  →  <terminal element> ]
```

Let's break that down:

- **`DataContractDocuments`** — root tree byte (`u8` constant) for "this is a document index, not a contract definition or identity record".
- **`contract_id`** — 32-byte contract identifier.
- **`1`** — separator distinguishing the document storage area from the contract definition area within `contract_id`.
- **`doc_type_name`** — UTF-8 bytes of the document type (`"person"`, `"contactRequest"`, etc.).
- **`propA_name, vA, propB_name, vB`** — alternating property key and serialized value, one pair per index property, in declaration order.
- **`0`** — the conventional "terminal slot" byte under each value level; it's where the actual reference (or sub-tree-of-references) lives.

The intermediate levels (`propA_name`, `vA`, `propB_name`, `vB`) are all `NormalTree`s. The terminal element at `[0]` varies — see the next section.

## Tree Type at the Terminal Level

The decision happens in [`add_reference_for_index_level_for_contract_operations/v0/mod.rs`](https://github.com/dashpay/platform/blob/v3.1-dev/packages/rs-drive/src/drive/document/insert/add_reference_for_index_level_for_contract_operations/v0/mod.rs):

```rust
if !index_type.index_type.is_unique() || any_fields_null {
    // Non-unique branch: insert an empty tree at [0], then put
    // each document's reference inside that tree. The tree's variant
    // is governed by `countable`:
    //   NotCountable             → NormalTree
    //   Countable                → CountTree
    //   CountableAllowingOffset  → ProvableCountTree
} else {
    // Unique branch: store a single Reference element at [0] directly.
}
```

So the matrix:

| `unique` | `any_fields_null` | `countable` | What lives at `[0]` |
|---|---|---|---|
| false | (any) | NotCountable | empty `NormalTree` containing per-doc references |
| false | (any) | Countable | empty `CountTree` containing per-doc references |
| false | (any) | CountableAllowingOffset | empty `ProvableCountTree` containing per-doc references |
| true | false | (any) | bare `Reference` to the one matching document |
| true | true | NotCountable | empty `NormalTree` containing per-doc references |
| true | true | Countable | empty `CountTree` containing per-doc references |
| true | true | CountableAllowingOffset | empty `ProvableCountTree` containing per-doc references |

Note the last three rows: a unique index *does* go through the count-tree branch when any indexed field is null. That's why `countable` on a unique index is meaningful exactly when at least one of the indexed properties is optional in the schema.

## Null Handling

The `any_fields_null` and `all_fields_null` flags are accumulated as Drive descends the index property list during insertion ([`add_indices_for_index_level_for_contract_operations/v0/mod.rs:170-171`](https://github.com/dashpay/platform/blob/v3.1-dev/packages/rs-drive/src/drive/document/insert/add_indices_for_index_level_for_contract_operations/v0/mod.rs#L170-L171)):

```rust
any_fields_null |= document_index_field.is_empty();
all_fields_null &= document_index_field.is_empty();
```

`any_fields_null` becomes `true` the moment the walker hits any null/empty value at any level (first, middle, or last) and stays true for the rest of the descent. `all_fields_null` only stays true if every value seen so far is null.

By the time the recursion reaches the terminal:

- `any_fields_null = false` and the index is unique → unique branch (bare Reference).
- `any_fields_null = true` (regardless of unique) → non-unique-style branch (sub-tree containing references).
- `all_fields_null = true` AND `null_searchable = false` → the terminal call returns early without inserting anything; this document is not findable through this index.

This means *different documents under the same unique index can land in different storage shapes* depending on which of their indexed fields are null. A document with all required fields populated takes the bare-Reference shape; a document with a null in an optional indexed property takes the sub-tree shape, side by side under the same index.

## Insert Flow Summary

Putting it together, when Drive inserts a document into a contract `C` of type `T`:

1. **`add_indices_for_top_index_level_for_contract_operations`** — for each top-level entry in the document type's index trie (each first-property of any declared index), pushes the property name and the document's value for that property onto the path, computes the initial `any_fields_null` / `all_fields_null` for that single value, and recurses.
2. **`add_indices_for_index_level_for_contract_operations`** (recursive) — for each sub-level of the trie, pushes the property name and value onto the path, OR-accumulates `any_fields_null`, AND-accumulates `all_fields_null`, and recurses. If the current level has `has_index_with_type = Some(...)`, it also calls into step 3 *before* recursing further (because an index can terminate at a non-leaf trie level when another index continues past it).
3. **`add_reference_for_index_level_for_contract_operations`** — the terminal call. Decides between unique and non-unique-style storage using the matrix above; for the non-unique-style path it picks a `NormalTree` / `CountTree` / `ProvableCountTree` based on `countable`; finally inserts the document reference (or sub-tree containing it).

Deletion mirrors the same walk in reverse — see [`packages/rs-drive/src/drive/document/delete/`](https://github.com/dashpay/platform/blob/v3.1-dev/packages/rs-drive/src/drive/document/delete/).

## Query Traversal

When a query arrives at drive-abci, the document-query construction path picks one of the document type's indexes that "covers" the query — i.e., whose property prefix matches the query's equality clauses, in order. The picker is in [`packages/rs-drive/src/query/mod.rs`](https://github.com/dashpay/platform/blob/v3.1-dev/packages/rs-drive/src/query/mod.rs) (look for `fn construct_path_query` and the index-selection helpers it calls). For count queries specifically there's a separate, count-tree-aware picker ([`drive_document_count_query/mod.rs`](https://github.com/dashpay/platform/blob/v3.1-dev/packages/rs-drive/src/query/drive_document_count_query/mod.rs)) — see [Document Count Trees](document-count-trees.md) for that path.

Once an index is picked, the query-engine builds a `PathQuery` whose path is exactly the prefix shape the insert code produced: `[DataContractDocuments, contract_id, 1, doc_type, prop, value, prop, value, …]`. GroveDB then walks the path in O(log n per level), reading the terminal sub-tree (or single reference) and returning matching documents.

A query whose where-clauses don't form a *prefix* of any index can't take this fast path and falls back to a full-scan plan — which dapi-grpc surfaces as an error in most cases, since unbounded scans are deliberately discouraged.

## Choosing Index Settings

Quick checklist for contract authors:

- **Don't index what you won't query.** Each index costs storage on every insert/delete and counts against the per-document-type index limit (10 indexes per type currently).
- **Order index properties from most-selective to least-selective.** A `[country, city]` index is more useful than `[city, country]` for queries like `where country = "FR"`.
- **`unique: true`** when the platform should reject duplicates at the consensus layer. This is the right place for "this should be unique" invariants — don't enforce them application-side.
- **`countable: "countable"`** when you'll regularly call `GetDocumentsCount` filtered by this index's leading columns. Adds a constant-factor overhead on insert/delete; reads become O(1).
- **`countable: "countableAllowingOffset"`** when you'll *also* want offset / range queries on this index in a future release. Strictly more expensive than plain `"countable"`; only worth it if you need the capability.
- **`null_searchable: true`** (the default) is right for almost all cases. Set to `false` only when documents with all-null indexed values shouldn't be findable through this index — typically a niche optimization to avoid a hot all-null prefix.

For specifically count-related concerns — primary-key-tree flags (`documentsCountable` / `rangeCountable`), the no-prove-vs-prove paths, and the operator restrictions — see the dedicated [Document Count Trees](document-count-trees.md) chapter.
