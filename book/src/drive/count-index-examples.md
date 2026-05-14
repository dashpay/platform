# Count Index Examples

This chapter walks through a representative contract and shows what a count-query proof actually proves — both the path query the prover signs and the verified element the verifier extracts. Every example uses the same `widget` contract (the same one the count-query bench at [`packages/rs-drive/benches/document_count_worst_case.rs`](https://github.com/dashpay/platform/blob/v3.1-dev/packages/rs-drive/benches/document_count_worst_case.rs) populates) so the proof bytes, verified elements, and diagrams can all be cross-referenced against the same data.

The chapter assumes you've read [Document Count Trees](./document-count-trees.md) — that chapter explains the three tree variants (`NormalTree` / `CountTree` / `ProvableCountTree`), what `Element::NonCounted` does, and how the schema's `documentsCountable` / `rangeCountable` flags select between them. Here we take that machinery as given and trace what each query *sees*.

## The Widget Contract

The widget document type carries three properties (`brand`, `color`, `serial`), opts into total counts at the doctype level via `documentsCountable: true`, and declares three indexes covering the count-query surface:

```jsonc
{
  "type": "object",
  "documentsCountable": true,
  "properties": {
    "brand":  { "type": "string", "position": 0, "maxLength": 32 },
    "color":  { "type": "string", "position": 1, "maxLength": 32 },
    "serial": { "type": "integer", "position": 2 }
  },
  "required": ["brand", "color", "serial"],
  "indices": [
    {
      "name": "byBrand",
      "properties": [{ "brand": "asc" }],
      "countable": "countable"
    },
    {
      "name": "byColor",
      "properties": [{ "color": "asc" }],
      "countable": "countable",
      "rangeCountable": true
    },
    {
      "name": "byBrandColor",
      "properties": [{ "brand": "asc" }, { "color": "asc" }],
      "countable": "countable",
      "rangeCountable": true
    }
  ],
  "additionalProperties": false
}
```

Three things to notice:

1. **`documentsCountable: true`** at the document-type level upgrades the doctype's primary-key subtree (at `widget/[0]`) from `NormalTree` to `CountTree`. The unfiltered total count is one read against this element's `count_value`.
2. **`byBrand` is `countable: "countable"` only.** It doesn't opt into `rangeCountable`, so `brand > X` range counts aren't supported — but from protocol v12 onward, **every countable terminator's value tree is stored as a `CountTree`**, so point-lookup count proofs (e.g. `brand == "X"` or `brand IN [...]`) get the same compact value-tree-direct shape that rangeCountable provides. `rangeCountable` is now strictly an opt-in for `AggregateCountOnRange` support, not the gate for proof-size optimization.
3. **`byColor` and `byBrandColor` are `rangeCountable: true`.** Their property-name subtrees (e.g. `widget/color`) are stored as `ProvableCountTree` rather than `NormalTree`, which is what `AggregateCountOnRange` walks for `color > floor` style queries.

The bench populates 100 000 documents under a deterministic schedule — `row → (brand_(row % 100), color_(row / 100), serial=row)`. That gives exactly 1 000 docs per brand, exactly 100 docs per color, and exactly 1 doc per `(brand, color)` pair. Those numbers show up in every verified count below.

## GroveDB Layout

The contract above produces this storage shape. Tree elements (the wrapping `Element` GroveDB stores under each key) are drawn as subgraphs; children inside each tree are merk-tree nodes. The doctype root and the per-property name subtrees are separate `Element` trees nested under the contract-documents prefix, just like every other index in Drive.

*Diagram conventions: green nodes carry a `count_value` committed to the merk root; gray are regular subtrees; dashed boxes highlight `Element::NonCounted` wrappers (children that store data but contribute `0` to their parent CountTree's count).*

```mermaid
flowchart TB
  WD["@/contract_id/0x01/widget"]:::tree

  WD --> PK["[0]: CountTree count=100000<br/>(documentsCountable primary key)"]:::countnode
  WD --> BR["brand: NormalTree<br/>(byBrand property-name)"]:::node
  WD --> CO["color: ProvableCountTree<br/>(byColor property-name)"]:::pctnode

  BR --> B000["brand_000: CountTree count=1000"]:::countnode
  BR --> B050["brand_050: CountTree count=1000"]:::countnode
  BR --> BMore["... brand_001 ... brand_099"]:::node

  B050 --> B050_0["[0]: CountTree count=1000<br/>(byBrand refs)"]:::countnode
  B050 --> B050_C["color: NonCounted(ProvableCountTree)<br/>(byBrandColor continuation, contributes 0)"]:::noncounted

  B050_C --> B050_C_500["color_00000500: CountTree count=1<br/>(byBrandColor terminator)"]:::countnode
  B050_C_500 --> B050_C_500_0["[0]: CountTree count=1<br/>(byBrandColor ref)"]:::countnode

  CO --> C500["color_00000500: CountTree count=100<br/>(byColor terminator)"]:::countnode
  CO --> CMore["... color_00000000 ... color_00000999"]:::countnode
  C500 --> C500_0["[0]: CountTree count=100<br/>(byColor refs)"]:::countnode

  classDef tree fill:#21262d,color:#c9d1d9,stroke:#1f6feb,stroke-width:2px;
  classDef node fill:#6e7681,color:#fff,stroke:#6e7681;
  classDef countnode fill:#3fb950,color:#0d1117,stroke:#3fb950,stroke-width:2px;
  classDef pctnode fill:#d29922,color:#0d1117,stroke:#d29922,stroke-width:2px;
  classDef noncounted fill:#21262d,color:#c9d1d9,stroke:#fb8500,stroke-width:2px,stroke-dasharray: 6 4;
```

Three layout facts to internalize before reading the queries:

- **`brand_050` is a `CountTree` with `count_value = 1000`.** That's true *because* `byBrand` is countable; the rule generalizes to every countability tier as of v12 (see [`add_indices_for_index_level_for_contract_operations/v0/mod.rs`](https://github.com/dashpay/platform/blob/v3.1-dev/packages/rs-drive/src/drive/document/insert/add_indices_for_index_level_for_contract_operations/v0/mod.rs)). The `color` continuation that branches off this value tree is `NonCounted`-wrapped so the parent's count equals exactly the 1 000 refs in `[0]`.
- **`widget/color` is a `ProvableCountTree`**, not a regular `NormalTree`. The yellow class above marks that — each internal merk node carries its subtree's count, which is what makes `AggregateCountOnRange` a single-pass primitive.
- **`color_00000500` is a `CountTree` with `count_value = 100`** under either parent. The same element layout would result from a query against `byColor` or against `byBrandColor`'s second level; the path that gets there differs, but the destination is structurally the same.

## How To Read The Proofs

Every example below has the same three sections:

1. **Path query** — the spec the prover hands GroveDB. `path` is the list of subtree segments to descend through (the proof carries merk-path bytes for each of these); `query items` is what to select once at the bottom; `subquery items` (when present) descends one more layer.
2. **Verified element** — what `GroveDB::verify_query` (or `verify_aggregate_count_query` for the range primitive) returns after walking the proof bytes. The `count_value_or_default` field on a `CountTree` element is what the count surface ultimately surfaces to the caller.
3. **Diagram** — the path the proof walks through the layout. Blue arrows trace the descent; the cyan node is the verified element; faded gray nodes show context.

All proof-size numbers come from running the bench against a 100 000-row fixture; see [`document_count_worst_case.rs`](https://github.com/dashpay/platform/blob/v3.1-dev/packages/rs-drive/benches/document_count_worst_case.rs)'s `report_proof_sizes` / `display_proofs` / `report_group_by_matrix` helpers.

## Query 1 — Unfiltered Total Count

```text
select  = COUNT
where   = (empty)
prove   = true
```

**Path query** (primary-key CountTree fast path; no index walk needed):

```text
path:         ["@", contract_id, 0x01, "widget"]
query items:  [Key(0x00)]
```

**Verified element:**

```text
path:        ["@", contract_id, 0x01, "widget"]
key:         0x00
element:     CountTree { count_value_or_default: 100000 }
```

**Proof size:** 585 B.

The descent stops at the doctype's primary-key tree — the green node at the top of the layout. Because `documentsCountable: true` upgraded that tree to a `CountTree`, the count is one O(1) read.

```mermaid
flowchart TB
  WD["@/contract_id/0x01/widget"]:::tree

  WD ==> PK["[0]: CountTree count=100000"]:::target
  WD -.-> BR["brand"]:::faded
  WD -.-> CO["color"]:::faded

  classDef tree fill:#21262d,color:#c9d1d9,stroke:#1f6feb,stroke-width:2px;
  classDef faded fill:#21262d,color:#6e7681,stroke:#484f58;
  classDef target fill:#39c5cf,color:#0d1117,stroke:#39c5cf,stroke-width:3px;

  linkStyle 0 stroke:#1f6feb,stroke-width:3px;
```

## Query 2 — Equal on a Single Property (`byBrand`)

```text
select  = COUNT
where   = brand == "brand_050"
prove   = true
```

**Path query:**

```text
path:         ["@", contract_id, 0x01, "widget", "brand"]
query items:  [Key("brand_050")]
```

**Verified element:**

```text
path:        ["@", contract_id, 0x01, "widget", "brand"]
key:         "brand_050"
element:     CountTree { count_value_or_default: 1000 }
```

**Proof size:** 1 041 B.

Pre-v12 this would have descended one more layer to `Key(0x00)` under `brand_050` (the legacy `[0]`-child CountTree). From v12 onward `brand_050` is itself a `CountTree` — the proof shape is the same as the rangeCountable case below, even though `byBrand` doesn't opt into `rangeCountable: true`.

```mermaid
flowchart TB
  WD["@/contract_id/0x01/widget"]:::tree
  WD ==> BR["brand: NormalTree"]:::path
  BR ==> B050["brand_050: CountTree count=1000"]:::target
  BR -.-> B000["brand_000"]:::faded
  BR -.-> BMore["..."]:::faded
  WD -.-> PK["[0]"]:::faded
  WD -.-> CO["color"]:::faded
  B050 -.-> B050_0["[0]: 1000 refs"]:::faded
  B050 -.-> B050_C["color (NonCounted)"]:::faded

  classDef tree fill:#21262d,color:#c9d1d9,stroke:#1f6feb,stroke-width:2px;
  classDef path fill:#6e7681,color:#fff,stroke:#1f6feb,stroke-width:2px;
  classDef faded fill:#21262d,color:#6e7681,stroke:#484f58;
  classDef target fill:#39c5cf,color:#0d1117,stroke:#39c5cf,stroke-width:3px;

  linkStyle 0 stroke:#1f6feb,stroke-width:3px;
  linkStyle 1 stroke:#1f6feb,stroke-width:3px;
```

## Query 3 — Equal on a RangeCountable Property (`byColor`)

```text
select  = COUNT
where   = color == "color_00000500"
prove   = true
```

**Path query:**

```text
path:         ["@", contract_id, 0x01, "widget", "color"]
query items:  [Key("color_00000500")]
```

**Verified element:**

```text
path:        ["@", contract_id, 0x01, "widget", "color"]
key:         "color_00000500"
element:     CountTree { count_value_or_default: 100 }
```

**Proof size:** 1 327 B.

Structurally identical to Query 2 — only the property name and the count-tree depth differ. The intermediate `widget/color` tree is a `ProvableCountTree` here (vs `NormalTree` for `byBrand`), but the *proof* doesn't care about that: it descends through the property-name tree and surfaces the value-tree CountTree at the bottom. The `ProvableCountTree` upgrade matters for Query 7 (range aggregate), not for point lookup.

```mermaid
flowchart TB
  WD["@/contract_id/0x01/widget"]:::tree
  WD ==> CO["color: ProvableCountTree"]:::path
  CO ==> C500["color_00000500: CountTree count=100"]:::target
  CO -.-> C000["color_00000000"]:::faded
  CO -.-> CMore["..."]:::faded
  WD -.-> PK["[0]"]:::faded
  WD -.-> BR["brand"]:::faded
  C500 -.-> C500_0["[0]: 100 refs"]:::faded

  classDef tree fill:#21262d,color:#c9d1d9,stroke:#1f6feb,stroke-width:2px;
  classDef path fill:#d29922,color:#0d1117,stroke:#1f6feb,stroke-width:2px;
  classDef faded fill:#21262d,color:#6e7681,stroke:#484f58;
  classDef target fill:#39c5cf,color:#0d1117,stroke:#39c5cf,stroke-width:3px;

  linkStyle 0 stroke:#1f6feb,stroke-width:3px;
  linkStyle 1 stroke:#1f6feb,stroke-width:3px;
```

## Query 4 — Compound Equal-only (`byBrandColor`)

```text
select  = COUNT
where   = brand == "brand_050" AND color == "color_00000500"
prove   = true
```

**Path query:**

```text
path:         ["@", contract_id, 0x01, "widget", "brand", "brand_050", "color"]
query items:  [Key("color_00000500")]
```

**Verified element:**

```text
path:        ["@", contract_id, 0x01, "widget", "brand", "brand_050", "color"]
key:         "color_00000500"
element:     CountTree { count_value_or_default: 1 }
```

**Proof size:** 1 911 B.

The proof descends through `byBrandColor`'s prefix value tree (`brand_050`) into its continuation (`color`, the `NonCounted`-wrapped subtree shown earlier) and resolves at the terminator value tree `color_00000500`. The count is `1` because the bench's fixture has exactly one document per `(brand, color)` pair.

```mermaid
flowchart TB
  WD["@/contract_id/0x01/widget"]:::tree
  WD ==> BR["brand: NormalTree"]:::path
  BR ==> B050["brand_050: CountTree count=1000"]:::path
  B050 ==> B050_C["color: NonCounted(ProvableCountTree)"]:::path
  B050_C ==> B050_C_500["color_00000500: CountTree count=1"]:::target
  B050_C -.-> Other["other colors"]:::faded
  B050 -.-> B050_0["[0]: 1000 byBrand refs"]:::faded
  BR -.-> Brands["other brands"]:::faded
  WD -.-> CO["color"]:::faded
  WD -.-> PK["[0]"]:::faded

  classDef tree fill:#21262d,color:#c9d1d9,stroke:#1f6feb,stroke-width:2px;
  classDef path fill:#6e7681,color:#fff,stroke:#1f6feb,stroke-width:2px;
  classDef faded fill:#21262d,color:#6e7681,stroke:#484f58;
  classDef target fill:#39c5cf,color:#0d1117,stroke:#39c5cf,stroke-width:3px;

  linkStyle 0 stroke:#1f6feb,stroke-width:3px;
  linkStyle 1 stroke:#1f6feb,stroke-width:3px;
  linkStyle 2 stroke:#1f6feb,stroke-width:3px;
  linkStyle 3 stroke:#1f6feb,stroke-width:3px;
```

## Query 5 — `In` on `byBrand`

```text
select  = COUNT
where   = brand IN ["brand_000", "brand_001"]
prove   = true
```

**Path query:**

```text
path:         ["@", contract_id, 0x01, "widget", "brand"]
query items:  [Key("brand_000"), Key("brand_001")]
```

**Verified elements** (one per In value, returned in lex-asc order):

```text
path:        ["@", contract_id, 0x01, "widget", "brand"]
key:         "brand_000"
element:     CountTree { count_value_or_default: 1000 }

path:        ["@", contract_id, 0x01, "widget", "brand"]
key:         "brand_001"
element:     CountTree { count_value_or_default: 1000 }
```

**Proof size:** 1 102 B.

The outer query enumerates `Key(in_value)` items at the property-name subtree; each resolved element is itself a value-tree `CountTree`. No subquery is set — the In values' value trees *are* the count-bearing elements. The verifier reads the per-In value from `grove_key` (rather than from `path[base_path_len]`, which is how it would for a trailing-Equal compound). The caller sums the two `count_value_or_default` reads (or surfaces them as per-group entries if `group_by = ["brand"]`).

```mermaid
flowchart TB
  WD["@/contract_id/0x01/widget"]:::tree
  WD ==> BR["brand: NormalTree"]:::path
  BR ==> B000["brand_000: CountTree count=1000"]:::target
  BR ==> B001["brand_001: CountTree count=1000"]:::target
  BR -.-> BMore["brand_002 ... brand_099"]:::faded
  B000 -.-> B000_0["[0]: 1000 refs"]:::faded
  B001 -.-> B001_0["[0]: 1000 refs"]:::faded
  WD -.-> PK["[0]"]:::faded
  WD -.-> CO["color"]:::faded

  classDef tree fill:#21262d,color:#c9d1d9,stroke:#1f6feb,stroke-width:2px;
  classDef path fill:#6e7681,color:#fff,stroke:#1f6feb,stroke-width:2px;
  classDef faded fill:#21262d,color:#6e7681,stroke:#484f58;
  classDef target fill:#39c5cf,color:#0d1117,stroke:#39c5cf,stroke-width:3px;

  linkStyle 0 stroke:#1f6feb,stroke-width:3px;
  linkStyle 1 stroke:#1f6feb,stroke-width:3px;
  linkStyle 2 stroke:#1f6feb,stroke-width:3px;
```

## Query 6 — `In` on `byColor` (RangeCountable)

```text
select  = COUNT
where   = color IN ["color_00000000", "color_00000001"]
prove   = true
```

**Path query:**

```text
path:         ["@", contract_id, 0x01, "widget", "color"]
query items:  [Key("color_00000000"), Key("color_00000001")]
```

**Verified elements:**

```text
path:        ["@", contract_id, 0x01, "widget", "color"]
key:         "color_00000000"
element:     CountTree { count_value_or_default: 100 }

path:        ["@", contract_id, 0x01, "widget", "color"]
key:         "color_00000001"
element:     CountTree { count_value_or_default: 100 }
```

**Proof size:** 1 381 B.

Same query shape as Query 5 — outer `Key`-per-In-value, no subquery, per-In `CountTree`s resolved at the bottom. The difference vs Query 5 is the *property-name* tree above is a `ProvableCountTree` instead of `NormalTree`. That doesn't change the proof's structural shape, but it does mean a future `color > X` range query against this property has a fast path Query 5's `brand` doesn't.

```mermaid
flowchart TB
  WD["@/contract_id/0x01/widget"]:::tree
  WD ==> CO["color: ProvableCountTree"]:::path
  CO ==> C000["color_00000000: CountTree count=100"]:::target
  CO ==> C001["color_00000001: CountTree count=100"]:::target
  CO -.-> CMore["color_00000002 ... color_00000999"]:::faded
  C000 -.-> C000_0["[0]: 100 refs"]:::faded
  C001 -.-> C001_0["[0]: 100 refs"]:::faded
  WD -.-> PK["[0]"]:::faded
  WD -.-> BR["brand"]:::faded

  classDef tree fill:#21262d,color:#c9d1d9,stroke:#1f6feb,stroke-width:2px;
  classDef path fill:#d29922,color:#0d1117,stroke:#1f6feb,stroke-width:2px;
  classDef faded fill:#21262d,color:#6e7681,stroke:#484f58;
  classDef target fill:#39c5cf,color:#0d1117,stroke:#39c5cf,stroke-width:3px;

  linkStyle 0 stroke:#1f6feb,stroke-width:3px;
  linkStyle 1 stroke:#1f6feb,stroke-width:3px;
  linkStyle 2 stroke:#1f6feb,stroke-width:3px;
```

## Query 7 — Range Query (`AggregateCountOnRange`)

```text
select  = COUNT
where   = color > "color_00000500"
prove   = true
```

**Path query** (different primitive — note the `AggregateCountOnRange` query item):

```text
path:         ["@", contract_id, 0x01, "widget", "color"]
query items:  [AggregateCountOnRange([RangeAfter("color_00000500"..)])]
```

**Verified payload** (different verifier — `GroveDb::verify_aggregate_count_query` returns a single `u64`, not an element list):

```text
root_hash:   0x62ee7348f4d28dd9d7cf86a6c725fa8276cfd446f6007a6000fb0e1dfefa6468
count:       49900
```

**Proof size:** 2 072 B.

This is the only query of the seven that uses a different GroveDB primitive. Instead of resolving N specific keys, `AggregateCountOnRange` walks the boundary of the requested range over `widget/color`'s `ProvableCountTree` and sums the per-node counts already committed inside that tree. The proof carries the boundary merk path and the running total; the verifier returns just the count.

The reason this works *only* with `rangeCountable: true` (Query 5's `byBrand` couldn't do the equivalent) is that `widget/color` is a `ProvableCountTree` — its internal merk nodes carry running counts. `widget/brand` is a plain `NormalTree`; it would have to enumerate every brand and sum their counts (which is what `brand IN [...]` does, but for an unbounded range that's not a feasible proof shape).

```mermaid
flowchart TB
  WD["@/contract_id/0x01/widget"]:::tree
  WD ==> CO["color: ProvableCountTree<br/>(internal merk nodes carry running counts)"]:::target
  CO -.-> C500["color_00000500 (boundary)"]:::faded
  CO -.-> CMore["color_00000501 ... color_00000999<br/>(in range, summed via merk-node counts)"]:::faded
  CO -.-> CBelow["color_00000000 ... color_00000499<br/>(below range, skipped)"]:::faded
  WD -.-> PK["[0]"]:::faded
  WD -.-> BR["brand"]:::faded

  classDef tree fill:#21262d,color:#c9d1d9,stroke:#1f6feb,stroke-width:2px;
  classDef faded fill:#21262d,color:#6e7681,stroke:#484f58;
  classDef target fill:#39c5cf,color:#0d1117,stroke:#39c5cf,stroke-width:3px;

  linkStyle 0 stroke:#1f6feb,stroke-width:3px;
```

The `ProvableCountTree`'s value isn't to expose individual elements — it's to make the summation itself O(log n) instead of O(distinct values in range). The proof bytes are larger than Query 6's two-element point lookup (~2 KB vs ~1.4 KB) because the AggregateCountOnRange primitive has more structural overhead per result, but it scales to any size range in fixed proof bytes, where the point-lookup shape grows linearly with the number of resolved keys.

## At-a-Glance Comparison

| # | Query                                | Primitive                 | Verified shape           | Proof size |
|---|--------------------------------------|---------------------------|--------------------------|------------|
| 1 | `(empty)`                            | primary-key CountTree     | 1 CountTree, count=100000| 585 B      |
| 2 | `brand == X`                         | PointLookupProof / byBrand| 1 CountTree, count=1000  | 1 041 B    |
| 3 | `color == X`                         | PointLookupProof / byColor| 1 CountTree, count=100   | 1 327 B    |
| 4 | `brand == X AND color == Y`          | PointLookupProof / byBrandColor | 1 CountTree, count=1 | 1 911 B    |
| 5 | `brand IN [b0, b1]`                  | PointLookupProof / byBrand| 2 CountTrees, sum=2000   | 1 102 B    |
| 6 | `color IN [c0, c1]`                  | PointLookupProof / byColor| 2 CountTrees, sum=200    | 1 381 B    |
| 7 | `color > floor`                      | AggregateCountOnRange / byColor | u64=49900           | 2 072 B    |

Three takeaways:

- **Query 1 is the cheapest.** A doctype-level total count is one merk read; everything else descends through an index tree.
- **Query 2 and Query 6 are structurally identical** despite covering different indexes (`byBrand` countable-only, `byColor` rangeCountable). The v12 generalization made the value-tree-direct shape uniform — `rangeCountable: true` only matters for Query 7.
- **Query 7 is the only one that uses a fundamentally different verifier** (`verify_aggregate_count_query` vs `verify_query`). Everything else returns an element list and reads `count_value_or_default` per branch; Query 7 returns a pre-summed `u64`.

The path-query builder these examples decode lives at [`packages/rs-drive/src/query/drive_document_count_query/path_query.rs`](https://github.com/dashpay/platform/blob/v3.1-dev/packages/rs-drive/src/query/drive_document_count_query/path_query.rs); the verifier mirror sits in [`packages/rs-drive/src/verify/document_count/`](https://github.com/dashpay/platform/blob/v3.1-dev/packages/rs-drive/src/verify/document_count/). Both the prover and the verifier reconstruct the exact same `PathQuery` via the shared builder — touching one without the other is a Merkle-root mismatch waiting to happen, and the byte-identical contract is what makes the proof bytes here reproducible against the bench fixture.
