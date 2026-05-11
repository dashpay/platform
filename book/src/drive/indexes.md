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

the trie that gets built is:

```mermaid
flowchart TD
    Root["(root)"]
    Owner["<b>ownerId</b><br/><i>shared prefix</i>"]
    DocType["<b>docType</b><br/>terminates <code>byOwnerAndType</code><br/><i>has_index_with_type = Some(...)</i>"]
    Status["<b>status</b><br/>terminates <code>byOwnerAndStatus</code><br/><i>has_index_with_type = Some(...)</i>"]

    Root --> Owner
    Owner --> DocType
    Owner --> Status

    style DocType fill:#e0f7fa,stroke:#006064,color:#000
    style Status fill:#e0f7fa,stroke:#006064,color:#000
```

The `ownerId` level is shared between both indexes. The `docType` and `status` levels each set `has_index_with_type` on themselves with their own `unique` / `countable` / `null_searchable` flags. A third index that *also* started with `ownerId` would attach further sub-levels under `ownerId` instead of duplicating it.

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

Concretely, suppose a `widget-store` contract has document type `widget` with a non-unique countable index `byColor = [color]`, and three documents stored: `A` and `B` with `color = "red"`, `C` with `color = "blue"`. Then drive lays out:

```mermaid
flowchart TD
    R["<b>DataContractDocuments</b><br/>(root tree byte)"]
    CID["<b>widget-store-id</b><br/>(32 bytes)"]
    Sep["<b>1</b><br/>(documents area)"]
    DT["<b>'widget'</b><br/>(document type)"]
    PK["primary-key tree<br/>(docs keyed by ID)<br/><i>not shown — own subtree</i>"]
    Color["<b>'color'</b><br/>(index property name)"]
    Red["<b>'red'</b><br/>(serialized value)"]
    Blue["<b>'blue'</b>"]
    RedT["<b>[0]: CountTree</b><br/>count = 2"]
    BlueT["<b>[0]: CountTree</b><br/>count = 1"]
    RA(["<b>doc_id_A</b><br/>Reference"])
    RB(["<b>doc_id_B</b><br/>Reference"])
    RC(["<b>doc_id_C</b><br/>Reference"])

    R --> CID --> Sep --> DT
    DT --> PK
    DT --> Color
    Color --> Red --> RedT
    Color --> Blue --> BlueT
    RedT --> RA
    RedT --> RB
    BlueT --> RC

    classDef countTree fill:#fff4e5,stroke:#bf6900,color:#000
    classDef reference fill:#e8f5e9,stroke:#1b5e20,color:#000
    classDef placeholder fill:#f5f5f5,stroke:#888,color:#000
    class RedT,BlueT countTree
    class RA,RB,RC reference
    class PK placeholder
```

**Legend**: rectangles are tree-type elements (intermediate `NormalTree`s holding sub-keys, terminal `CountTree`s holding per-doc refs); rounded green nodes are `Reference` elements (leaf pointers to documents). Amber highlight marks count-bearing trees specifically.

A query like `where color = "red"` walks the path down to the `red` value, opens the `[0]` `CountTree`, and either reads `count_value` (for `GetDocumentsCount`) or enumerates the inner references (for `GetDocuments`). Because the count is stored on the wrapping element, the count read is O(1) regardless of how many docs are inside.

### Shared-Prefix Indexes

Now extend the same `widget` document type with a `shape` property and a second, compound, countable index `byColorShape = [color, shape]`. The `IndexLevel` trie is:

```mermaid
flowchart TD
    Root["(root)"]
    ColorLevel["<b>color</b><br/>terminates <code>byColor</code><br/><i>has_index_with_type = Some(...)</i><br/>+ sub-level <code>shape</code>"]
    ShapeLevel["<b>shape</b><br/>terminates <code>byColorShape</code><br/><i>has_index_with_type = Some(...)</i>"]

    Root --> ColorLevel --> ShapeLevel

    style ColorLevel fill:#e0f7fa,stroke:#006064,color:#000
    style ShapeLevel fill:#e0f7fa,stroke:#006064,color:#000
```

Importantly, `color` is a level that *both* terminates an index (`byColor`) **and** has a sub-level (`shape`) continuing past it. That dual role shows up directly in the on-disk path: at every `[..., color, <value>]` subtree, key `[0]` (the `byColor` terminal) and key `'shape'` (the continuation into `byColorShape`) live as siblings.

With three documents — A: `(red, circle)`, B: `(red, square)`, C: `(blue, square)` — the layout is:

```mermaid
flowchart TD
    DT["<b>'widget'</b><br/>(document type)"]
    ColorKey["<b>'color'</b><br/>(index property)"]
    Red["<b>'red'</b>"]
    Blue["<b>'blue'</b>"]

    %% byColor terminals (at the color-value level)
    RedColorT["<b>[0]: CountTree</b><br/>count = 2<br/><i>byColor</i>"]
    BlueColorT["<b>[0]: CountTree</b><br/>count = 1<br/><i>byColor</i>"]

    %% byColorShape continuation (sibling to [0] under each color value)
    RedShape["<b>'shape'</b>"]
    BlueShape["<b>'shape'</b>"]
    RedCircle["<b>'circle'</b>"]
    RedSquare["<b>'square'</b>"]
    BlueSquare["<b>'square'</b>"]

    %% byColorShape terminals
    RCT["<b>[0]: CountTree</b><br/>count = 1<br/><i>byColorShape</i>"]
    RST["<b>[0]: CountTree</b><br/>count = 1<br/><i>byColorShape</i>"]
    BST["<b>[0]: CountTree</b><br/>count = 1<br/><i>byColorShape</i>"]

    %% References — each indexed path stores its own reference, so docs appear
    %% multiple times across the diagram (same key, same doc id, but a separate
    %% Reference element under each terminal that matches the document).
    RA1(["<b>doc_id_A</b><br/>Reference"])
    RB1(["<b>doc_id_B</b><br/>Reference"])
    RC1(["<b>doc_id_C</b><br/>Reference"])
    RA2(["<b>doc_id_A</b><br/>Reference"])
    RB2(["<b>doc_id_B</b><br/>Reference"])
    RC2(["<b>doc_id_C</b><br/>Reference"])

    DT --> ColorKey
    ColorKey --> Red
    ColorKey --> Blue

    Red --> RedColorT
    Red --> RedShape
    Blue --> BlueColorT
    Blue --> BlueShape

    RedColorT --> RA1
    RedColorT --> RB1
    BlueColorT --> RC1

    RedShape --> RedCircle
    RedShape --> RedSquare
    BlueShape --> BlueSquare

    RedCircle --> RCT --> RA2
    RedSquare --> RST --> RB2
    BlueSquare --> BST --> RC2

    classDef countTree fill:#fff4e5,stroke:#bf6900,color:#000
    classDef reference fill:#e8f5e9,stroke:#1b5e20,color:#000
    class RedColorT,BlueColorT,RCT,RST,BST countTree
    class RA1,RB1,RC1,RA2,RB2,RC2 reference
```

Two things to notice:

- **`[0]` and the sub-property name (`'shape'`) are siblings under each color value.** The `[0]` count tree is the `byColor` terminal at that color value; the `'shape'` subtree is the continuation that `byColorShape` walks past for the next index property. Drive descends one or the other depending on which index covers the query.
- **The same document is stored as a separate `Reference` under every index path that matches it.** Doc A appears under `byColor[red]` and under `byColorShape[red, circle]`; doc B under `byColor[red]` and `byColorShape[red, square]`; doc C under `byColor[blue]` and `byColorShape[blue, square]`. That's why each of A, B, C shows up twice in the diagram — once per index that covers the document. Insert/delete touches all of them; queries walk only the one path their picker selected.

A query like `where color = "red"` resolves through the `byColor` terminal (`[0]` under `red`) — count = 2, O(1). A query like `where color = "red" AND shape = "circle"` resolves through `byColorShape` instead, taking the `'shape'` sub-tree past `red` and reading the terminal under `circle` — count = 1, also O(1). Both queries are served by the same shared-prefix layout, just descending different branches at the `red` node.

### Range-Countable Indexes

> **Status: design.** Not yet implemented at the time of writing. Depends on a parallel grovedb change that adds `NonCounted<ElementType>` element variants — element types that behave exactly like their counterparts except that **their count value is not propagated to the parent count tree**, and which are only insertable inside a `CountTree` / `ProvableCountTree` / `CountSumTree` / `ProvableCountSumTree`.

`range_countable` is a *separate* per-index property from `countable`. Where `countable` makes the count of docs at *one specific value* O(1), `range_countable` makes the count of docs *between two values* O(log n) — answering queries like "how many widgets have a color between `red` and `tomato` alphabetically" without enumerating every distinct color value.

#### Constraints

- `range_countable: true` requires `countable` to be `Countable` or `CountableAllowingOffset`. It is additive to countability, not a replacement: range queries are useful only on indexes you'd already want to count by.
- The combination is meaningful only on **non-unique** indexes (or unique indexes whose entries can be null-bearing), for the same reason `countable` is mostly inert on unique-with-required-fields: a unique non-null terminal is a bare `Reference`, with no tree to hang per-node counts off of.
- Sibling sub-trees that share a prefix with a range-countable index — e.g., the `'shape'` continuation when `byColor` is range-countable but `byColorShape` shares its `color` prefix — must use `NonCounted<*>` variants so their counts do not pollute the range-countable value tree's count.

#### Mechanism

Where today's `countable` upgrades only the terminal `[0]` element under each indexed value to a count tree, `range_countable` additionally upgrades two more levels:

| Level | Without `range_countable` | With `range_countable` |
|---|---|---|
| Property-name tree (e.g. `'color'`) | `NormalTree` | `ProvableCountTree` |
| Value tree (e.g. `'red'`, `'blue'`) | `NormalTree` | `CountTree` |
| Terminal at `[0]` under each value | `NormalTree` / `CountTree` / `ProvableCountTree` (per `countable`) | unchanged — still driven by `countable` |
| Sibling continuations inside the value tree (e.g. `'shape'` for a compound index sharing the prefix) | `NormalTree` | `NonCounted<NormalTree>` |

The property-name tree is a `ProvableCountTree` rather than a plain `CountTree` because the merk-tree internal-node counts are exactly what makes range queries O(log n): walk the boundary path between the lower and upper bound, sum sub-counts at each internal node along the way. (See [Document Count Trees](document-count-trees.md) for the underlying mechanic.)

The value trees become `CountTree`s because the property-name `ProvableCountTree`'s aggregate is computed by summing each value tree's `count_value`. For that aggregate to mean "total docs at this property" rather than "number of distinct values", each value tree's `count_value` must equal "docs at this exact value" — which is only true if (a) the terminal `[0]` `CountTree` contributes its doc count, and (b) every sibling under the value tree (continuation sub-property names like `'shape'`, etc.) contributes **zero** rather than the default 1-per-Tree. That's what `NonCounted<NormalTree>` is for.

#### Layout

Same `byColor` + `byColorShape` example as before, with the same three documents (A: `(red, circle)`, B: `(red, square)`, C: `(blue, square)`), but now `byColor.range_countable: true`:

```mermaid
flowchart TD
    DT["<b>'widget'</b><br/>(document type)<br/>NormalTree"]
    ColorKey["<b>'color'</b><br/><b><i>ProvableCountTree</i></b><br/>count = 3"]
    Red["<b>'red'</b><br/><b><i>CountTree</i></b><br/>count = 2"]
    Blue["<b>'blue'</b><br/><b><i>CountTree</i></b><br/>count = 1"]

    %% byColor terminals (unchanged shape — same as before)
    RedColorT["<b>[0]: CountTree</b><br/>count = 2<br/><i>byColor terminal</i>"]
    BlueColorT["<b>[0]: CountTree</b><br/>count = 1<br/><i>byColor terminal</i>"]

    %% byColorShape continuation — now NonCounted to avoid double-counting
    RedShape["<b>'shape'</b><br/><b><i>NonCounted&lt;NormalTree&gt;</i></b>"]
    BlueShape["<b>'shape'</b><br/><b><i>NonCounted&lt;NormalTree&gt;</i></b>"]
    RedCircle["<b>'circle'</b><br/>NormalTree"]
    RedSquare["<b>'square'</b><br/>NormalTree"]
    BlueSquare["<b>'square'</b><br/>NormalTree"]

    %% byColorShape terminals
    RCT["<b>[0]: CountTree</b><br/>count = 1<br/><i>byColorShape</i>"]
    RST["<b>[0]: CountTree</b><br/>count = 1<br/><i>byColorShape</i>"]
    BST["<b>[0]: CountTree</b><br/>count = 1<br/><i>byColorShape</i>"]

    %% References (one per matching index path, per the earlier section)
    RA1(["<b>doc_id_A</b><br/>Reference"])
    RB1(["<b>doc_id_B</b><br/>Reference"])
    RC1(["<b>doc_id_C</b><br/>Reference"])
    RA2(["<b>doc_id_A</b><br/>Reference"])
    RB2(["<b>doc_id_B</b><br/>Reference"])
    RC2(["<b>doc_id_C</b><br/>Reference"])

    DT --> ColorKey
    ColorKey --> Red
    ColorKey --> Blue

    Red --> RedColorT
    Red --> RedShape
    Blue --> BlueColorT
    Blue --> BlueShape

    RedColorT --> RA1
    RedColorT --> RB1
    BlueColorT --> RC1

    RedShape --> RedCircle
    RedShape --> RedSquare
    BlueShape --> BlueSquare

    RedCircle --> RCT --> RA2
    RedSquare --> RST --> RB2
    BlueSquare --> BST --> RC2

    classDef provableCount fill:#ede7f6,stroke:#311b92,color:#000
    classDef countTree fill:#fff4e5,stroke:#bf6900,color:#000
    classDef nonCounted fill:#f3e5f5,stroke:#6a1b9a,color:#000,stroke-dasharray:5 5
    classDef reference fill:#e8f5e9,stroke:#1b5e20,color:#000
    class ColorKey provableCount
    class Red,Blue,RedColorT,BlueColorT,RCT,RST,BST countTree
    class RedShape,BlueShape nonCounted
    class RA1,RB1,RC1,RA2,RB2,RC2 reference
```

**Legend additions for this diagram**: purple = `ProvableCountTree`; amber = `CountTree`; dashed lavender = `NonCounted<*>` (the new grovedb variants); rounded green = `Reference`.

Walking through how the counts add up:

- **`'red'` (CountTree, count=2)** — its children are `[0]` (`CountTree`, contributes its `count_value` = 2) and `'shape'` (`NonCounted<NormalTree>`, contributes **0** — that's the whole point of the new variant). Aggregate = 2. ✓
- **`'blue'` (CountTree, count=1)** — same shape, 1 doc + 0. ✓
- **`'color'` (ProvableCountTree, count=3)** — its children are `'red'` (CountTree, contributes 2) and `'blue'` (CountTree, contributes 1). Aggregate = 3. The provable variant additionally stores per-internal-node counts inside its merk structure, which is what enables the range walk.

If `'shape'` were a plain `NormalTree` instead of `NonCounted<NormalTree>`, it would contribute 1 to `'red'` (every non-count-tree element contributes 1 by default — see [Document Count Trees § How Counts Aggregate](document-count-trees.md)). Then `'red'` would read as 3, `'blue'` as 2, `'color'` as 5 — a count of "docs + sub-property-trees", not "docs". The `NonCounted<*>` variant exists exactly to fix this.

#### Query — "count between two values"

With the layout above, a query like `WHERE color BETWEEN 'red' AND 'tomato'` resolves at the `'color'` `ProvableCountTree` level:

1. Walk the merk tree from `'color'`'s root, finding the boundary node between `'red'` (lower bound) and `'tomato'` (upper bound) — O(log distinct color values).
2. At each step, decide what to do with the *off-boundary* subtree using its pre-computed count: include its full `count_value` (subtree fully inside the range), exclude (fully outside), or recurse (straddles the boundary).
3. Sum the contributions; the result is the count of all docs whose color falls in `[red, tomato]`.

No leaf-level enumeration of distinct color values, no enumeration of individual documents — the count is computed entirely from the tree's pre-aggregated structure.

#### Compound indexes

`range_countable: true` on a compound index applies at the index's *terminating* level (its last property). For `byColorShape = [color, shape]` with `range_countable: true`:

- `'shape'` (the property-name tree under each color value) becomes a `ProvableCountTree`.
- Each `'circle'` / `'square'` value tree becomes a `CountTree`.
- Documents are referenced as `Element::Reference` leaves under those `CountTree`s, contributing 1 each to the count aggregate.

When the compound's leading prefix is also indexed by another `range_countable` index (e.g. `byColor` is also `range_countable`), sibling continuations under each color `CountTree` are wrapped with `Element::NonCounted` so a doc routed via `byColorShape` doesn't double-count under `byColor`'s color aggregate. The walker (`add_indices_for_index_level_for_contract_operations`) threads a `parent_value_tree_is_range_countable` flag down the recursion to decide when to wrap, regardless of whether the inner tree is itself a `ProvableCountTree`, `CountTree`, or plain `NormalTree`.

End-to-end coverage in `range_countable_index_e2e_tests` (in `packages/rs-drive/src/drive/contract/insert/insert_contract/v0/mod.rs`) pins the storage layout against a real grovedb — including the `count_tree_value_count_excludes_compound_continuation_via_non_counted` test that proves NonCounted-wrapping is load-bearing for compound-index correctness.

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

Visualizing the three terminal shapes side by side:

```mermaid
flowchart TD
    subgraph SA["Non-unique, countable"]
        direction TB
        A1["[..., color, 'red']"]
        A2["<b>[0]: CountTree</b><br/>count = 2"]
        A3(["<b>doc_id_A</b><br/>Reference"])
        A4(["<b>doc_id_B</b><br/>Reference"])
        A1 --> A2 --> A3
        A2 --> A4
    end

    subgraph SB["Unique, all fields non-null"]
        direction TB
        B1["[..., email, 'alice@x']"]
        B2(["<b>[0]: Reference</b><br/>→ doc_id_X"])
        B1 --> B2
    end

    subgraph SC["Unique with null in path"]
        direction TB
        C1["[..., a, 'X', b, &lt;empty&gt;]"]
        C2["<b>[0]: CountTree</b><br/>count = 1"]
        C3(["<b>doc_id_W</b><br/>Reference"])
        C1 --> C2 --> C3
    end

    classDef countTree fill:#fff4e5,stroke:#bf6900,color:#000
    classDef reference fill:#e8f5e9,stroke:#1b5e20,color:#000
    class A2,C2 countTree
    class A3,A4,B2,C3 reference
```

Same convention as the layout diagram above: rectangles are tree-type elements, rounded green nodes are `Reference` elements. Same key (`[0]`) at the terminal in all three panels — **what** lives there is what differs. The middle case is the one that's "special" — a bare `Reference` directly at `[0]` instead of a sub-tree containing references — and it's specifically scoped to the unique-and-no-nulls scenario.

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
- **`countable: "countable"`** when you'll regularly call `GetDocumentsCount` with `==` (or `in`) clauses on **exactly** this index's properties. Adds a constant-factor overhead on insert/delete; reads become O(1). A `countable: true` index counts only queries whose where clauses match its properties exactly — partial-prefix queries are rejected with `WhereClauseOnNonIndexedProperty`, not falling through to a slow scan. Define a separate index per distinct count-query shape you want to support, or set `documentsCountable: true` on the document type for unfiltered totals.
- **`countable: "countableAllowingOffset"`** when you'll *also* want offset / range queries on this index in a future release. Strictly more expensive than plain `"countable"`; only worth it if you need the capability.
- **`null_searchable: true`** (the default) is right for almost all cases. Set to `false` only when documents with all-null indexed values shouldn't be findable through this index — typically a niche optimization to avoid a hot all-null prefix.

For specifically count-related concerns — primary-key-tree flags (`documentsCountable` / `rangeCountable`), the no-prove-vs-prove paths, and the operator restrictions — see the dedicated [Document Count Trees](document-count-trees.md) chapter.
