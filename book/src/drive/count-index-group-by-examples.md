# Count Index Group By Examples

This chapter is the `GROUP BY` companion to [Count Index Examples](./count-index-examples.md). It uses the same `widget` contract, the same 100 000-row fixture, and the same bench at [`packages/rs-drive/benches/document_count_worst_case.rs`](https://github.com/dashpay/platform/blob/v3.1-dev/packages/rs-drive/benches/document_count_worst_case.rs). Read chapter 29 first — most of the mechanics (CountTree variants, the merk-proof reconstruction algorithm, `node_hash_with_count` and friends) carry over unchanged.

What's different here:

- Every query in chapter 29 returns either a single `u64` aggregate or a small list of `CountTree`s the caller sums. The verifier-side payload shape is **one count, total**.
- Every query in this chapter returns **one count per group**. The caller gets back a `Vec<(group_key, count)>` and can index it directly — no summation.

The most important thing to understand up front: **`group_by` is two things at once — a *result-shaping directive for the SDK* and (for some queries) a *proof-shaping directive for the prover*.** When you pass `group_by = [...]` in a count request, you're always telling the SDK "don't collapse the result into a single number — give me one count per group key." That result-shaping role is universal: it's what turns `Aggregate(sum)` into `Entries([(key, count), …])`.

Whether `group_by` *also* changes the proof bytes depends on the query shape. For queries where the underlying proof already commits one `CountTree` per matched key (single-property `IN`s, for instance), the per-group breakdown is reconstructible from the existing bytes — the prover ships the same proof, the SDK just zips it with the group keys instead of summing. For range queries and certain compound shapes, the per-group breakdown *can't* be reconstructed from the aggregate-style proof (which commits opaque subtree counts rather than per-key counts), so passing `group_by` forces the prover to emit a structurally different, larger proof.

The interesting question this chapter answers is: **which queries fall into which bucket, and why?**

## When `group_by` Changes the Proof (and When It Doesn't)

| Filter | `group_by` | Aggregate proof (no `group_by`) | Group-By proof | Proof bytes change? |
|---|---|---|---|---|
| `brand IN [b0, b1]` | `[brand]` | [Q5](./count-index-examples.md#query-5--in-on-bybrand) — 1 102 B | 1 102 B (2 entries) | **No** — byte-identical |
| `color IN [c0, c1]` | `[color]` | [Q6](./count-index-examples.md#query-6--in-on-bycolor-rangecountable) — 1 381 B | 1 381 B (2 entries) | **No** — byte-identical |
| `color > floor` | `[color]` | [Q7](./count-index-examples.md#query-7--range-query-aggregatecountonrange) — 2 072 B (1 `u64`) | 10 992 B (100 entries) | **Yes** — different primitive |
| `brand == X AND color > floor` | `[brand, color]` | [Q8](./count-index-examples.md#query-8--compound-equal-plus-range-bybrandcolor) — 2 656 B (1 `u64`) | *not allowed in this form* | — |

The key observation: `IN` clauses produce proofs that already commit one `CountTree` per resolved key, so adding `group_by` on the same property is purely a verifier-side relabel — the prover ships the same bytes, the verifier just returns them as `Entries(...)` instead of `Aggregate(sum)`. This is why **G1 and G2 below are not new proofs** — they're [Q5](./count-index-examples.md#query-5--in-on-bybrand) and [Q6](./count-index-examples.md#query-6--in-on-bycolor-rangecountable) reinterpreted.

So **why pass `group_by` at all if the proof bytes don't change?** Because without it, the SDK has no way to know you want the per-key breakdown. The same `brand IN ["brand_000", "brand_001"]` proof can answer two different questions:

- *"How many widgets total are made by brand_000 or brand_001?"* → caller passes no `group_by`, SDK returns `Aggregate(2 000)`.
- *"How many widgets per brand?"* → caller passes `group_by = [brand]`, SDK returns `Entries([("brand_000", 1 000), ("brand_001", 1 000)])`.

The bytes on the wire and the cryptographic guarantees are identical; the only thing that changes is which result shape the SDK delivers. Think of `group_by` as the count-query equivalent of `SELECT brand, COUNT(*) ... GROUP BY brand` versus `SELECT COUNT(*) ...` in SQL — same scan plan, different projection.

Range queries are different. `AggregateCountOnRange` (chapter 29's Q7) walks the boundary of the range over a `ProvableCountTree` and sums per-subtree counts directly — it never resolves individual keys. `GroupByRange` (this chapter) has to *enumerate* the distinct in-range keys to label each group, so it produces a different proof shape with one `CountTree` (or `CountTree`-feature-typed element) per distinct key in the range. That's where `group_by` genuinely earns its bytes — the prover has to do additional work because the per-group breakdown can't be reconstructed from `AggregateCountOnRange`'s opaque-subtree-count commitments.

## Queries in this Chapter

All proof-size and behaviour numbers below come from the same bench helper (`report_group_by_matrix`) as chapter 29's. The dispatcher's group_by surface validation lives in [`validate_count_query_groupby_against_index`](https://github.com/dashpay/platform/blob/v3.1-dev/packages/rs-drive/src/query/drive_document_count_query/validate.rs); the per-mode path-query builders sit in [`packages/rs-drive/src/query/drive_document_count_query/path_query.rs`](https://github.com/dashpay/platform/blob/v3.1-dev/packages/rs-drive/src/query/drive_document_count_query/path_query.rs)'s `group_by_*` family.

| # | Query | Filter + group_by | Complexity | Avg time | Proof size | Verified shape | Notes |
|---|-------|-------------------|------------|----------|------------|----------------|-------|
| G1 | [`In` on `byBrand`](#g1--in-on-bybrand-grouped-by-brand) | `brand IN ["brand_000", "brand_001"]` <br/> `group_by = [brand]` | O(k · log B) | 38.6 µs | 1 102 B | `Entries(2 groups, sum = 2 000)` | Byte-identical to [Q5](./count-index-examples.md#query-5--in-on-bybrand) |
| G2 | [`In` on `byColor`](#g2--in-on-bycolor-grouped-by-color) | `color IN ["color_00000000", "color_00000001"]` <br/> `group_by = [color]` | O(k · log C) | 62.1 µs | 1 381 B | `Entries(2 groups, sum = 200)` | Byte-identical to [Q6](./count-index-examples.md#query-6--in-on-bycolor-rangecountable) |
| G3 | [Compound `In` + Equal](#g3--compound-in--equal-grouped-by-brand) | `brand IN [...] AND color == Y` <br/> `group_by = [brand]` | O(k · (log B + log C')) | 106.2 µs | 2 842 B | `Entries(2 groups, sum = 2)` | Per-In compound resolution; two parallel Q4 descents sharing L1–L6 |
| G4 | [Range on `byColor`](#g4--range-on-bycolor-grouped-by-color) | `color > "color_00000500"` <br/> `group_by = [color]` | O(R · log C) | 762.9 µs | 10 992 B | `Entries(100 groups, sum = 10 000)` | `GroupByRange`: enumerates distinct in-range keys instead of Q7's boundary aggregate |
| G5 | [Compound `In` + Range](#g5--compound-in--range-grouped-by-brand-color) | `brand IN [...] AND color > floor` <br/> `group_by = [brand, color]` | O(k · R' · log C') | 737.5 µs | 11 554 B | `Entries(100 groups, sum = 100)` | Compound In-fan-out × in-range distinct keys (G3 outer × G4 inner) |
| G6 | [High-fanout `In` on `byBrand`](#g6--high-fanout-in-on-bybrand) | `brand IN [100 values]` <br/> `group_by = [brand]` | O(k · log B) | 1 532 µs | 10 038 B | `Entries(100 groups, sum = 100 000)` | Scales linearly with `\|IN\|`; reveals every byBrand entry when `\|IN\| = B` |
| G7 | [Carrier `In` + Range (`byBrandColor`)](#g7--carrier-in--range-grouped-by-brand) | `brand IN [...] AND color > "color_00000500"` <br/> `group_by = [brand]` | O(k · (log B + log C')) | 255.9 µs | 4 332 B | `Entries(2 groups, sum = 998)` | Per-In aggregate via `AggregateCountOnRange` as a carrier subquery; one `u64` per branch |
| G8 | [Carrier outer Range + Range (`byBrandColor`)](#g8--carrier-outer-range--range-grouped-by-brand) | `brand > "brand_050" AND color > "color_00000500"` <br/> `group_by = [brand]` | O(L · (log B + log C')) | 1 260 µs | 43 638 B | `Entries(25 groups, sum = 12 475)` | Outer-Range carrier with a platform-wide `SizedQuery::limit = 25`; one `u64` per in-range outer key (capped at L = 25) |

**Complexity variables.** `B` = distinct brands in the byBrand merk-tree (≈ 100); `C` = distinct colors in byColor (≈ 1 000); `C'` = distinct colors per brand in byBrandColor (≈ 1 000); `R` = distinct in-range values returned by `GroupByRange` (capped at 100 in this fixture by an implicit response-size limit); `R'` = distinct in-range values per fan-out branch (similarly capped); `k` = `|IN|` for the In-outer carrier shapes; `L` = the platform-wide outer-walk cap for the Range-outer carrier shape (G8), hardcoded at `CARRIER_AGGREGATE_OUTER_RANGE_LIMIT = 25` — see [G8](#g8--carrier-outer-range--range-grouped-by-brand) for the rationale. As in [chapter 29](./count-index-examples.md#queries-in-this-chapter), the total document count `N` doesn't appear — count proofs read pre-committed `count_value`s rather than enumerating docs.

**Avg time** is the criterion-reported median of `cargo bench --bench document_count_worst_case -- 'document_count_worst_case/query_g'` on the same 100 000-row warmed fixture used by chapter 29's `query_N_*` cases. Each row reflects **10 samples × ~3 k–130 k iterations per sample** with 2 s warm-up and 5 s measurement; the median sits within ±2 % of the mean across reruns. G1 and G2 match their [Q5](./count-index-examples.md#query-5--in-on-bybrand) / [Q6](./count-index-examples.md#query-6--in-on-bycolor-rangecountable) counterparts to within ~3 µs — the residual is the SDK-side zip-vs-sum cost. G4 is ~11 × Q7 because `GroupByRange` enumerates 100 distinct in-range CountTrees rather than walking `O(log C)` boundary nodes; the time difference is exactly the complexity difference predicted (`O(R · log C)` vs `O(log C)`).

## Group-By Shapes That Are *Not* Allowed

Several plausible-looking `(where, group_by)` combinations are rejected by the dispatcher before any proof generation. The rejections fall into four buckets — operator/group_by mismatch, missing range window, no covering index, and one currently-deferred aggregate variant. All are surfaced as typed `QuerySyntaxError`s; the precise error strings appear in the bench's `[matrix]` output.

### 1. `group_by` field constrained by `==` instead of `In` or range

```text
where    = brand == "brand_050"
group_by = [brand]
```

> `count query supports only ...` (rejected because `==` produces exactly one entry whose key equals the where-clause's value — grouping by a field that already has a single value contributes no extra information).

**Why.** `GROUP BY [field]` is meaningful only when `field` can take multiple values in the result set. An `==` clause pins the field to exactly one value, so the group_by is structurally redundant — the dispatcher rejects it rather than silently returning a single-entry response that would look like a bug. Use [Q2](./count-index-examples.md#query-2--equal-on-a-single-property-bybrand) / [Q3](./count-index-examples.md#query-3--equal-on-a-rangecountable-property-bycolor) (no `group_by`) for single-value `==` queries.

Applies symmetrically: `where = color == X, group_by = [color]` is rejected for the same reason.

### 2. `group_by` contains a range field but the `where` clause doesn't range over it

```text
where    = brand IN[...] AND color == "color_00000500"
group_by = [brand, color]
```

> `GROUP BY on a range field requires a range where-clause; the range field must appear in `where` for the distinct walk to have a window to iterate over`

**Why.** `group_by = [in_field, range_field]` (`GroupByCompound`) routes through `distinct_count_path_query`, which needs a range window on the second field to know what values to enumerate. With `color == Y` the second dimension collapses to a single value, so the compound walk degenerates to a point lookup — and that's what [Q4](./count-index-examples.md#query-4--compound-equal-only-bybrandcolor) / [G3](#g3--compound-in--equal-grouped-by-brand) are for. For compound *plus* range, the `where` must carry a range on the second field (which is what [G5](#g5--compound-in--range-grouped-by-brand-color) does).

### 3. `group_by` orders fields in a way no covering index can serve

```text
where    = color IN[...] AND brand > "brand_050"
group_by = [color, brand]
```

> `where clause on non indexed property error: range count requires a `range_countable: true` index whose last property matches the range field`

**Why.** The covering index for `(group_by[0] = color, group_by[1] = brand)` would need to be `byColorBrand` with `rangeCountable: true` on the `brand` terminator. The widget contract doesn't have that index — only `byBrand`, `byColor`, and `byBrandColor`. The dispatcher's index picker walks every declared index, finds none whose `(properties, last_property_is_range_countable)` shape matches the request, and rejects with the "non-indexed property" error.

The fix is contract-level: declare a `byColorBrand` index with `rangeCountable: true` if the application needs this group_by order. The dispatcher itself can't infer alternate index orders from the request alone — `rangeCountable: true` is an explicit opt-in on each index because it changes the on-disk tree shape (NormalTree → ProvableCountTree on the property-name subtree).

---

To put these three buckets in one place: every rejected `(where, group_by)` shape on this contract reduces to one of:

- the `group_by` field's `where` operator doesn't admit multiple values (bucket 1),
- the `group_by` has a range slot that the `where` doesn't fill with a range (bucket 2),
- there's no covering `rangeCountable` index in property order (bucket 3).

All three checks happen at request validation, before any GroveDB work. The bench's `report_group_by_matrix` exercises one example of each and prints the exact error string, so adding a new contract or index shape is a quick way to see which checks each new query shape hits.

> **Historical note.** A fourth bucket — `group_by = [in_field]` with `where = in_field IN[...] AND range_field > floor` — was rejected before [grovedb PR #663](https://github.com/dashpay/grovedb/pull/663). That PR added support for `AggregateCountOnRange` as a *carrier* subquery under outer `Keys`, which unblocked the natural single-field-group_by shape (one aggregate count per In branch) at the merk layer. The dispatcher now routes that shape to [`DocumentCountMode::RangeAggregateCarrierProof`]; the worked-out example is [G7](#g7--carrier-in--range-grouped-by-brand) below.

## G1 — `In` on `byBrand`, Grouped By `brand`

```text
select   = COUNT
where    = brand IN ["brand_000", "brand_001"]
group_by = [brand]
prove    = true
```

**Path query** (identical to [Q5](./count-index-examples.md#query-5--in-on-bybrand)):

```text
path:         ["@", contract_id, 0x01, "widget", "brand"]
query items:  [Key("brand_000"), Key("brand_001")]
```

**Verified payload** (the only thing that differs from Q5):

```text
Entries([
  ("brand_000", CountTree { count_value_or_default: 1000 }),
  ("brand_001", CountTree { count_value_or_default: 1000 }),
])
```

The SDK zips the In values with the two resolved `CountTree` elements (in lex-asc order) rather than summing them as Q5's `CountMode::Aggregate` does.

**Proof size:** 1 102 B. **Proof bytes are byte-identical to [Q5](./count-index-examples.md#query-5--in-on-bybrand)** — same path query, same merk ops, same hash composition. The dispatcher recognises that `CountMode::GroupByIn` on a single-property `In` clause resolves through the same `point_lookup_count_path_query` as `CountMode::Aggregate` does; only the response-shaping at the very end differs.

For the **verbatim proof display**, see [Q5 in chapter 29](./count-index-examples.md#query-5--in-on-bybrand) — every byte of the 1 102-byte proof is the same. Or [▶ open the proof interactively in the visualizer ↗](https://dashpay.github.io/grovedb-proof-visualizer-widget/#f=text&d=H4sIAAdCBmoC_6VXXW9ctxF996_YRxvQA2f4MRwDLfJRtAGSFgEauA-BEHDIYWxUsIK1nA8U_u89d7WSLe-udTcmpNW95NXd4Zkzcw7_sb3-1f_21ffb6-v5gjb_e7LZfNf-8O1uYne72fyyXD_f_NO3_326m9hswvPN92_fvHz6TVs-vvz3Nz_aYKXJWjVOK7OMwoFCFUmlNyzERlLHCNUpYspKJxZXah4LR4uXz57t3037d3_74kW7euu7r_jiYvPD1v1p8sFcOElmFW5TZZpQs5lCEePoBWGUVqgFnjEbB62VdA6POrjqs4vNLtrUcuPajSwFvLC15dkZRg4sRcakyKXamKlILSzD8AY8lrO0PqZ8EC0j2rb11zf7-3iADGlXTgDGk8SmARikyiIjGGG6916HxdGrhG6dYnCROOMQ9Zaxu_fflZ5vvn756mrc3l9d_-bbn66WbL15vk_VZvPF5i9_vb85kszbcSSlDxP7Ifjh98-FfZ-9QHf4Z9Pg5JrZxH1qiL3rMB6K__QYU4syxUPtbXjmXoy9RXyf5WIlZWDy7IO4j0Ox39FnR_8Q0U_i-gi6p4onqQspBUQoHDX7EO4zRFeJWsWHF0-aMgDJkiYIggfyTN3Fwcd6-QCNu0FHcxlonw2RogUEL3jlfV6GtjDbqA0M5FzCrF41h05eU5PpzVozJhO1Ljy9J3A5Ja4pOfs8Hsh9jRwufipzd_kLdCwDK_KwKhunef_bq_Gz3-zRQhfgQsXLPVal55BnjpPdUK4eYuidW2-pxJEssTa3yKEpALXaHB0FuIXYmM1K_YjD56FyO24jPI3OaozOQOoUhxWNsmoyKi2Ion3WOfosKL5u3H2p415RbakktdlLx2_CY0UtuA69PInGp_hs2_Z6HKQozxhiifE-VdWKymiqo3iT0Gp0SySaRm9iLWqsaMdN67S5ZK0PUF4tWh5hTn48to9l4NQ4lIfSYxFW12jUIkEaJ8rOuYJWBZQKjphTHhYqOmPJRCAaQz-1oxhreTy2h7Jxaqzl3O3Y4f4Y8c6k39kkPFm6f_d283brP_zxi__n1c3L3e7f0-WnEMLF5uvrt69vbnkT4Vd6mcIXG9qtzav2M0D4EZfLz-UdjxRkWLyNtgilItKslYkrRLuIcIJCDRaTLDA1oczYawpBS3F4iyh2ebH5qr151ZfN_et6-P61YFqutcaelB2GqRYaRl7aWBRB4VWU4BJiDG1mSBk6bi3DFLU0SpX8OAVOFtCjSNGfQyrVBNtH05L3OhPkwmlKbTup9wCxh4yB3pkAlk4LBMXjLNiQacl8AqlgkzhwhCwBcVjIYRDBTrlGOAfod40VNrMsPqJLIS1LJlQlB0AsuhapteV8qqjhW4KOBlMXuIlPiEOOiDS1oUqwmlD7GdEjBelbzI5ltMcAVsERjmZr47wr7VUP5_v0vw_UF09k6IZeCf045s46BRhLGi1P6Rq4ZCf2AVGbuU1a-B-CwxTopLWBlrMAlQNAc5gRrngEGt4qHLXCQauUNOas3OFFQjCUD_Yxc8ezTItvzxFnC3TUuTbOeg6gegTQDpYvwKCZA8VkMK4pSkUzgI1lgrelqAxrmSBQFXaObdBsybpTH11W13I4C1GiQ9PZ6uhUS1MIc08IN_O0jNbTU0A3k4mm5JYNLEbQXbEFsKPiILNsoayOlM_BlOIRUAf45jCqEbDhFDVSCQHJViP4rhEjSBCK-lB0yRoT7AZccYGlz3OgA69mKaXzQM0HoMqkQYIWFnGctJFSGK3PDPdMgTO8Mk54oxL0HOWfoeQE-FvG7qh77XV1pOUsUOUIqNFHqvhq1DwUC1ISWVMVuDGcjWqrNKRn8x5z7VESzijwJkNz8WgGuFeHWs8DVQ9A9YYj4uw5hhTQ6FHRkSkFTZQApTE0Bc0fqgmhSU0ZesnMQJelTdjM1V0_nAMq0xFQJ05VMJY4sQmMKGmSAPdWY-uUmqHTDiuthVQIqgVvnKFkeaB5CaEftNV9is9TKD6UKIuzd2uiiANXGWdNmJu2WN9MzOgNkjOOxk5JzGfRxR-XSlyAuOn6SNfZz2W8e_I5659aPb12auX4_LHZw7mPZx7ef3j3_vru6vbv8vnuybsn_wczPWgtnxMAAA) (same encoded payload). The diagrams below show the result-shaping difference.

```mermaid
flowchart TB
  WD["@/contract_id/0x01/widget"]:::tree
  WD ==> BR["brand: NormalTree"]:::path
  BR ==> B000["brand_000: CountTree count=1000"]:::target
  BR ==> B001["brand_001: CountTree count=1000"]:::target
  BR -.-> BMore["brand_002 ... brand_099"]:::faded

  SDK["Verifier returns Entries([<br/>(&quot;brand_000&quot;, 1000),<br/>(&quot;brand_001&quot;, 1000)<br/>])"]:::sdk

  B000 -.-> SDK
  B001 -.-> SDK

  classDef tree fill:#21262d,color:#c9d1d9,stroke:#1f6feb,stroke-width:2px;
  classDef path fill:#6e7681,color:#fff,stroke:#1f6feb,stroke-width:2px;
  classDef faded fill:#21262d,color:#6e7681,stroke:#484f58;
  classDef target fill:#39c5cf,color:#0d1117,stroke:#39c5cf,stroke-width:3px;
  classDef sdk fill:#21262d,color:#39c5cf,stroke:#39c5cf,stroke-width:2px,stroke-dasharray: 4 2;

  linkStyle 0 stroke:#1f6feb,stroke-width:3px;
  linkStyle 1 stroke:#1f6feb,stroke-width:3px;
  linkStyle 2 stroke:#1f6feb,stroke-width:3px;
```

### Diagram: per-layer merk-tree structure (Layer 5+)

Identical to [Q5's Layer-5+ diagram](./count-index-examples.md#query-5--in-on-bybrand) — same merk ops, same byBrand binary tree, same two `KVValueHashFeatureTypeWithChildHash` targets. The only difference is what the verifier returns at the end (`Entries(...)` instead of `Aggregate(2000)`); the per-layer structure is unchanged. See chapter 29 for the diagram.

## G2 — `In` on `byColor`, Grouped By `color`

```text
select   = COUNT
where    = color IN ["color_00000000", "color_00000001"]
group_by = [color]
prove    = true
```

**Path query** (identical to [Q6](./count-index-examples.md#query-6--in-on-bycolor-rangecountable)):

```text
path:         ["@", contract_id, 0x01, "widget", "color"]
query items:  [Key("color_00000000"), Key("color_00000001")]
```

**Verified payload:**

```text
Entries([
  ("color_00000000", CountTree { count_value_or_default: 100 }),
  ("color_00000001", CountTree { count_value_or_default: 100 }),
])
```

**Proof size:** 1 381 B. **Byte-identical to [Q6](./count-index-examples.md#query-6--in-on-bycolor-rangecountable)** — same path query, same `ProvableCountTree`-style boundary commitments (`KVHashCount` ops carry running counts even though the SDK doesn't read them for this point lookup). The single difference from G1 is the underlying property-name tree type (`ProvableCountTree` for byColor vs `NormalTree` for byBrand); that affects the merk-boundary commitments but not the dispatcher's GroupByIn-vs-Aggregate routing.

For the **verbatim proof display**, see [Q6 in chapter 29](./count-index-examples.md#query-6--in-on-bycolor-rangecountable) — or [▶ open it interactively in the visualizer ↗](https://dashpay.github.io/grovedb-proof-visualizer-widget/#f=text&d=H4sIAAdCBmoC_6VY244dxw1811ecRwnQQ5PsGwUkcOIgMZALDNhQHoyF0d0kLSELr7GW7BiB_z01e5W0e6Q50Eh7dOaimZ5isaq4f7u8-MX_8uevLy8u4iUd_vfkcPjH-M0vrw5c7R4OP23fXxz-6Zf_eXp14HBILw5fv_351dOvxvbxp2---m4aKwVrV4lZo1rlRKm3lusaOCGDWjdL3UlwaNZF3FxpuFSWKWfPnt3cm27u_feXL8f5W796xBfPD99euj_NbsyVcyusjUdoi9lozMiptsniFcuoo9JIHFImJ-2dNMxFjbs-e364Wm0eZXBfk2ZOuOEY27WRrCRutVmQcO3TItfWKzebuAMuK6WNZdHeWS1jtePSf3xzsy8PkCFdyhnAeG4yNAGD3Lk1S5NweK3VbYqt3tKaiyR5axJiTX0UvN39s_KLw5evXp_b9f75xa9--f35Vq2fX9yU6nD44vCHP97tPFLM6-2Rkr5f2HfBT__9XNhvqpfoFv8yNTm5Fp7NPTTJWmqTTfE_XSQPadE89TXMC6862YfgebPUWXMBJs_eWffjUNy80Wev_n1EP4rrJ9A91jxZvZFSwgobixa3xiuSuDbR3ty8etZcAEhpOUAQXFAiL28OPvaz99C43ejRWia6qUZrVSsIXnHLu7qYjhTD-gADudQU3buWtMh7Hi18zDEm02w6V-PwlcHlnLnn7Ozx-ELueuThyY9V7rZ-iR6rwI467KrGcd7_-tp-8Dc3aEEFuFL1eodVXSWVKBLsE-3qSdJaPNbIVSzPzDp8CqehAHT24VAU4JZkMM9Z-wccPg2V6-16hcfR2Y3RCUgd47BCKLvmSXWkppDPHraiovnW5OVbH6-Obss164xVF34yLqs6k6vp2VE0HvL5_qmD1b0Hpzq4CKxkVhs51ljRaxac86JclmnTWJbRaUVTjRQafbp8-qkfCvyxTR5h0Lo4v7h8fgD2v4x57l9evP3xzTWbBBa5amxdLOnuTxEStCelbbvlWdNS1YoWm63MCbqpraEw1LI0bfq5AjKmWmFataMlKTmnBlWDhtL69Cu-7yvHtr2kvN6uXv1TzDyRnyez9Ghv_9XHm7eX_u1vP_m_X795dfX29xX7Pt1szw_3Jdv2aPuI8_EDAPgOX7e_Z7d1Wl46QzqTjdmiMzeY-zSQbjRnyyWLqEexUdII9IcLjTWhFogbc9WzD4jitr3ivy7Mn9I9G8bmSIT4IjABLg76pJJrjDGgPR6U15QsSBZp9OpGRtVKWFVf6hKfZsNR89gHGp0GGvJXBlRRSQoiAXfqCpLbVLyHwXxkeCStG7XTEGM0eajSIpvITx8BTe5Bg3NyLrh3NbQQN5iYGkGTUTAutJypm2a1lFAJR9hgSBYhBgjxyrYXtL1acSwqbqVDeB5pmtQZtS2VVnvItDlstQXPR4WFqTBzQMEgqTPAKWRXXLt3nbcNv-vi8p7sXqF8vVr2BGAhqcMqQgkj6hN1BslEyqbKsxbqsmhLEyaeI6hBsxwRe6Y8UbuGGu1bcj0J2vYA2m4NKxGMHQgrXtcaZSH5wJt9EoNtK3tL8C_bsJYuSdFflkoSHhlE27nOfgq0egxaZIZm1TEO6JKcyoDqI5CtbSSJ7CtBVLTC2eYsJl1DBxDFINUVrjILoKWyG1tKJ4FL9ABdDHWEfAQFhDMRkpHxQs0jUp2OmWq2CcBR9lwG9ZKnbJMhvGuLSTO33arEp8BLcgzfpR01Z7gkRLg1HRFzCcIsXBYc1Yz3qLmBFoR0nBYmsxQVsr5FcUsGfIX245tPw7c8FIYehMeW3jGubI2VQNMuiFiGUarnVTVDPS1b1oaF46KGtqvS4T81r9341pPwbcfwxUQ7McCYTKhroP07hH51KrMrOr-WFFbIBfQtw3OKlgljdSbOfSzpwLfKfnz7afjqA3wZ-MI_Z0IYVU5eoMABNBPGRqjDAA8MwyB0ranlXqRiSh-QC6qtjLxbeDmdgi_TMXxhDi0VKANoGeglSFRU5ViU5vBkpo5YMbaZZU40IuazQOhmECM4j9j0gfeLL59mbPzQ2bpPbwJmOrLJln7hsAU8BXU3pwOXrQFQp6QGq5AoA4mlwuc7hlD13QCfZG181NsmB_K1ZKguAhrw3cwgSsXIi-kiq_RpqW0z8MBS6ep3C7n1Eb11bbwJBJf9Csyn2Rs_9DdDkCSsWpsQEToMC49tnJC1EBCKpMkZ77St2bc5w3LbVizIUwZO7Ab4JIPjow4ntWfJbSDjuBtSbcT2C4bldfg2C_WKvLwIaadGwQ7acbFUYpxaqWzhodB-CZbTLE4esbi-tDTIcAHIWKMKB0XfyGqYsyphSpuFMYhFWDPYX4ZtbGqM1crcDbCcZHFy3OKQWzwhTQPCDugUU_nGaOA8F6cSA-Iro61mVAJ9BxkrmSAKuWwYn93NoTvXfZrJyUOTo0ELyb4ipE9y8qmpw3xB44mU1tkaoiZWPFoVG-BH4MUQfygjvyNU7ka47pp3t-33J59z_mNnj587dubx448dfXjswyPv77-7d__99tv1v9vn709-f_J_sY4oiTEYAAA).

```mermaid
flowchart TB
  WD["@/contract_id/0x01/widget"]:::tree
  WD ==> CO["color: ProvableCountTree"]:::path
  CO ==> C000["color_00000000: CountTree count=100"]:::target
  CO ==> C001["color_00000001: CountTree count=100"]:::target
  CO -.-> CMore["color_00000002 ... color_00000999"]:::faded

  SDK["Verifier returns Entries([<br/>(&quot;color_00000000&quot;, 100),<br/>(&quot;color_00000001&quot;, 100)<br/>])"]:::sdk
  C000 -.-> SDK
  C001 -.-> SDK

  classDef tree fill:#21262d,color:#c9d1d9,stroke:#1f6feb,stroke-width:2px;
  classDef path fill:#d29922,color:#0d1117,stroke:#1f6feb,stroke-width:2px;
  classDef faded fill:#21262d,color:#6e7681,stroke:#484f58;
  classDef target fill:#39c5cf,color:#0d1117,stroke:#39c5cf,stroke-width:3px;
  classDef sdk fill:#21262d,color:#39c5cf,stroke:#39c5cf,stroke-width:2px,stroke-dasharray: 4 2;

  linkStyle 0 stroke:#1f6feb,stroke-width:3px;
  linkStyle 1 stroke:#1f6feb,stroke-width:3px;
  linkStyle 2 stroke:#1f6feb,stroke-width:3px;
```

### Diagram: per-layer merk-tree structure (Layer 5+)

Identical to [Q6's Layer-5+ diagram](./count-index-examples.md#query-6--in-on-bycolor-rangecountable). The byColor `ProvableCountTree` at L6 carries the same `KVHashCount` running counts; the SDK ignores them for point-lookup group_by and reads only the two resolved targets' `count_value_or_default`.

## G3 — Compound `In` + Equal, Grouped By `brand`

```text
select   = COUNT
where    = brand IN ["brand_000", "brand_001"] AND color == "color_00000500"
group_by = [brand]
prove    = true
```

**Path query** (per-In compound resolution — outer Query on byBrand, inner subquery on byBrandColor's `color` terminator):

```text
path:               ["@", contract_id, 0x01, "widget", "brand"]
query items:        [Key("brand_000"), Key("brand_001")]
subquery_path:      ["color"]
subquery items:     [Key("color_00000500")]
```

**Verified payload:**

```text
Entries([
  ("brand_000", CountTree { count_value_or_default: 1 }),
  ("brand_001", CountTree { count_value_or_default: 1 }),
])
```

Each `(brand, "color_00000500")` pair has exactly 1 document in the bench's deterministic schedule.

**Proof size:** 2 842 B. **Mode:** `CountMode::GroupByIn` over the `byBrandColor` compound index.

**Proof display:**

<details>
<summary>Expand to see the structured proof (8 layers — two parallel brand-X → color → color_00000500 descents sharing L1–L6) — or <a href="https://dashpay.github.io/grovedb-proof-visualizer-widget/#f=text&d=H4sIAAdCBmoC_-1bW49cx41-96_oRxnQA8m6kQY2yGWRBNjdIEAC70MgBKwqVmys4AkmcrJB4P-er1qj60xrutsKIAduzUh9Od3NwyK_C-voV7c3f43__Plvb29u1pd8-Mdnh8N_-9_j9vjE8eHh8Od9_4vD_8Tt_z05PnE40BeH3377l6-e_Nr3Xz_73a__0KcYLzG1tHpddVYhJm0t1-F4ITk3nZM0OOGpXgdLC2OPVCX19Ozzz-8-m-8--7--_NKffxvHr_jp08PvbyOe5JgiVXIrYk18WVu9sfeVqbYuKSrCqF7ZSVYqXchU2daMZFPUPn96OEabvbjo6Nwz4QPd97GLZiFptc3FSar2uXJtWqXNjk_AYaU0H3O1t6IVROu38c2Lu8fpXmbYhklGYiK35EbIQVZpbVJnPD3G0NnTHNpo9MGJorW00mwWXnB2b74rf3H4xVdfP58vHz-_-Vvc_vH5Xq2_fHG3VIfDTw__8ZPXDx5YzJe3B5b03YV9O_n0_9837XerR_wq_6UbBYcV6S1iGaUxbHaZhndGStlTWy1Ih88oMmqX8ITv66X2mgty8vlbcT-cirsz-t7Rv5vRD-b1keyeap5s0diYEGGTZCVmk7EohbVk2mJGjWy5ICGl5YUCwQFl5REtUI_67J1svLrxg2tJfLcarVWrKPCKj3y9LtOclk91VKCUSktDrdDg0OxthXf3Ltyb9dFkxcio5ZxFcw6J9XAgr3vk_osfWrlX60f80AqcsQ5nrcbpuv_b1_NP8eIuW0ABqVyjvs5VHYXKKmlJdLRrUKIxxIfnmmbuWcyjJyE3JLSrBxAFeaPkIr1Xfa-GL8vKy9vLCE9n5-wcXZCpUzVsAEq13Lk6NQN86ppjVTTf6DJi9_FQdFuu2foadeA347BqncKmPTuZjQ_Vc7_1b-a9JSorUaopvV4q7dXadLNZwxu5puiZm-U5vHVPlhRw7Karr71qY6LkradeJq0lj8f2Pg2cut2nhzpSbWJhqbMnBjUutF2IoqwqSooCMecyOymQsRZmFJqAP22gGbU-Htu7tHHqdm7Nvbwd8_5Y4V1YfhcX4enWPYb3RyJ6evjFzbffvHhZHwm6ZNTV5OmBj6-t5_4nnOwfcHf_PHtVL4ZF3xrGPIGRmK2YCouCnGtrksFEU1pvpUG8UF1paCayWgMaIrX--Jo8UtEIna8LPWuG3uLVcwxdGTgdvJr6kWODwLLgD9RVYURvqxODaqQ0CLZutci5oZ9b8KfKHsxONh2yh8RbLMBnSSC07NOMIcbAhysBRZq2suVALwAQwnpAM00_O8Wviv-sg8vr9XgTaGzV0IEXoQzESmWIrcZCLU8vqw0jqSVYYgL2V_HFu3KIArRpi88NtF6U0HYvoYVWgm6cxDNcoTkNGtNazXMtlQG2JurZBOexysCxwlvZlgT1DcxZ58aplyTUHkjoQDnuxADukMXcIe1yaoo2gtAThvrjZALxlQHhCsEjffLy3EfwmKOd3Vx0UUaZ78sy1zlYqxuoa2SEW2T1wrOPTMCBtgowuZeOKkbQw3AKqA6F1N-nUM-OVC7JKacHkjpRbwEpl5A2-IyZKxEW2zpDmcyUUARULabBbWjKIGToxgrRW9YEdp1dpZwvS2q5l9S2eHID1iQYrj5zpuljFehLJilQk_BAUxmMh_Yv4DpG-r3g7HiEDj070npRUtsDSU0xs-Kr0fPA-lxnEsvaoFfgHtSVZxulx0hFR2oZKh7sPa3USL0j3WeHqpcl1e4lNRwmao2SKJNCzfSRhDNZ5oxUdgH4k0EsMBghu0nBrwiyK80XhNjZqE-XJFX4gaQu-A5IL3iaBqnGlhtB32jywdk7kHZ2-HHKlQPPxCywIGUCvBoDD_xsnJLLGEruU1RPa4zuzRAH7hW4McgC3-KwsAiwoZUC8xicW49VbSvIqiwVGe92fqTnCbRrZNpbYm2rofME21Wy7UrxdspNzEql5940KgzvbC01Dq8wUgnYlaJooMmszpZGngkkB_woUGBcUy56Nkmc1mHj5vnN7dPDb26-OcqwmE-QjL96fx4PybKj43j1p4AM-KRQe6XUUFIDCiHDfy9IICXNYEbre3qmbYAYXQzg41RlJTY47pQW5zmlw4nLZaf4AQ_-MUvt5e2Yu0uK7XuU3PcqvFPlt3JA2W0H5gE6B_ND0k2uDA-51arBtEGqDvKydUAGqSqBYb3CKKAoL1ub94twx3EssjupTAUeGWDN-NpAZSyFt-34Vl5wKI16kT1vWFqkDBjgNfZbtjRZkM_07OkBrvGKiC5D0A_r_Z4IvOkDAqoX0jHxT54xlphvyayzNmYQgo5wGPVaO2UP7hVN1mhek9F8KqMtMqxESM6gSYE1UoFsBn3rUqIJvMFiA1kUMnmVDjVKDWJ5lJJtLU_IqJRyRUTl6ozWexmVvhDmHg3W0bbiBAwmb0MLKN-EoZtoDBDWqL0OByhNLiSh3SYwi67JaDuZ0YnKd9aSx4ztPgIJGlP7HrVo0go_F9towJeusZBqIRzcrUB-WCvIKEu7IiK9OqP3VRQVKGW0GuSnQPDDzC_pCWXQos8GVTp2wdYsXHqpe55fFtoO_iAB9XVd1fV0su3roC3R6vRW4dwc8hPQAlZgJbj6KjVNgFNHs0sx8dzWoFnQ8R4WjpTWdE1EfHVKt5F5L6dDJloNTayoOtCZjkbLRXJKG8_cIQVHc6j7ZqtkgsLL22aj_yq5jqtymk7lNDkTEBrWuAT8sq1u2ldeAMvKfUALIzDYphUd8i7EE6QcVnk4A_S1IqfpGiS90DY9YqFQmyAeoPyekDgzgQtsqLQk2hbgNHiNlms72oBBde9mCLQTLEsr56v99-3UidbvTi6pWbUMUcwqMvNEHAHLAdgXAs5iOcFVQp4VnVT2RC32juEURU6vafyjY7sypXpf7_0y_MW3t_H7v_85_vfrF18d9fgbHbjFM4GC35kn7kd8ehJX4bPRlbDfWaZXkrRqwqfoJB6w3SsWXEMHue1Vayg2CVhgDpbOMXetvaM4Y25185ubGU9e76PBusOEUMrVC2-QpwEzXzxKjOGBRiLbU4lFE-GAF0ABg2GuIaUriOKqSrBTlYDvwfeVmiMcRMn7DGGaJ3xbFuGqQLCy5zg2QF7dx0QBAPxzdjTXGru7rpEpdHUlyP2hT4-kMjgtOlJ_tK1MzOuWAyjagAMxMvRclL2DAN_HcH44yqX1THFNSuWiAdD7tvXKd55URzQguxLkZcIJzaSUiYCHZaJg9tYW9wnpoeCihnqCDIaUUm0gwiRztbW5_BpxJNerI7kvj1YvUDtZCqwhvBNWCkJya4-EcvNVo3fYq5Yhi0DfrtNKAfqzl9550-c1Z9CuXg29-p127TsTXf1OvvqdV1d6OknqIBXgqi-HtfCaouD-jJKaum9CCkhTeKUG0plzLKg21UVSM1ZZ2LZQ2lb9ivVO19N6uk_rEJgdQM1QnDMH8KUwJBLOrABcIEQ6oYIRb4YJlMhQgWarwpTsPUzjfE3FpnrB7OnN7bvP_hXHnnvkecd9d9FsjH84szHI5FatA8JswvgbdF1JcCipDRvRU9bS09g7xXDVAecFrKba6oq8oY5_ALMx-HBN6Nixdy7I4XnKEZltogEKJO5oe1MIAsPAPehqCIhJJnBsXoLtx9nYv2421sDxfcCz5Vl9lhHRYDNShgBSGqOkIm0xJKgb1di-r2PBUl-lQdaaf9TZGKy6AS37HsKD7YGHUwi2xyF1V_E-s4-EIACqe1Q2wP0ZBlPw626zfRKzMYVugg0uiNkRtUASd42RrPWYxfYMZW-C5V5m4uymDtFSOhgsFNpzfNTZGBjTes3ex-yjIW2SSsNP0729mBJV14oEg56Ual9SIflbY0rbyZf8SczGYvnorFPSGKOO3GlmBJ60zUFFWKikXjtcKpwI_BnDNqWirBGxLx36qLOxTt0BUcUHQTdXZCol6FCp0rHOgDVSMD1KYHAg27AadSM3wRdZarN_ErOxyOID9ketlzFnRcomqg_s4s7AZUGkSQYEDCxpXnvzRkcFIiClDXLm487Gyr5kN9UKNgDJRDKFO_NplSWvpAnAlJotlTk1gxA7gS8GcgmsarnopzEba0WiotH3bA-Q5CpOY1dq7pzRdtRpie85ylreewXmjhELzmtfj7L0487GIJMJ6jOrTvUWc3WuQNayr3hGZs32pBlIm0u1GbkwTFWk1ja6Oil_GrOxuvK-YgtGbhi6Cg1FW_8TNecFGBDJ26vCw6LBVCtgzet2ragjuNz8cWdjSWUPj2FGFATVERtb8zU8oZda4aKlGJJaaW902IJ-qzVKDlmtZJN_19mYKk-WlHQJrMxAMtCcBUqRkhgaGs59UgUvJqVQBZhUQyahaIs1Ps4MH52NeRYhZ5ijOWHpF9BqJTNwAJgqz32R6hooDZQxQEy75op1UFq1pmVSPu5sTInV4FM1ZYLtFKiWBDEEei-t7ms8s-wNsASa4Cl1aLPore3ruNoCW30aszHf24_QAD0P6A4TS9Nj76FBdQP3uWLZdP8_CVaol71Rb151OfAiLeEf_mwM8nK7Eo-1BAoJJYyFixizaGtlT7k1-fShph1gvSTWaIoyxmpvvfFpzMb2xcYwTgTiMVirnlYfC3JZgZqQ6IEGTGtOQzmKOIi_aLgLQ0Hnmuv4cTb2Q5mNdagiOO5ZVlkTfG1wQamhOjmDZtTxM-GF8oCCByZ7W8CofTXmjDKhRz-V2RjMD4_huQBpKoO9FRYU0GgJUAndNKYNh0LZO7WLvOXWBR4KJjUgB2z-OBu7-KjHj3nsiA-__qFXT7926pWHn3_o2fvPvf_Mu4_ffvTm_qt7L__df3_32Xef_RM2G-h0TzkAAA">open interactively in the visualizer ↗</a></summary>

```text
GroveDBProofV1 {
  LayerProof {
    proof: Merk(
      0: Push(Hash(HASH[bd291f29893fb6f6d6201087746ca1f23a178dd08e1346cb6c127e91ae3623b3]))
      1: Push(KVValueHash(@, Tree(4ed22624752972af97fb71abf4067b23e6d296a61a02f35b2098819fde39d289), HASH[4a5a28cb1b40226aa35b2f0d502767df13268bdf4678627dbfde26a557acdf73]))
      2: Parent
      3: Push(Hash(HASH[19c924989e473a90d0848277d0b1498ccc8db3dc870cbc130e773f3d79ea5b71]))
      4: Child)
    lower_layers: {
      @ => {
        LayerProof {
          proof: Merk(
            0: Push(KVValueHash(0x4ed22624752972af97fb71abf4067b23e6d296a61a02f35b2098819fde39d289, Tree(01), HASH[5b90e1e952b7eef903cc9db2d9098e334a37f7e08cade52c6b2ea3bf4b56b645])))
          lower_layers: {
            0x4ed22624752972af97fb71abf4067b23e6d296a61a02f35b2098819fde39d289 => {
              LayerProof {
                proof: Merk(
                  0: Push(Hash(HASH[49e7191075272395ed72cf03e973987ede6e4945e08574fe77d725f4ce7ecdf8]))
                  1: Push(KVValueHash(0x01, Tree(776964676574), HASH[5d9a0fad8a3f32560f8e8950c1e84a7feabaab21b79bc72fec4482442844e2ef]))
                  2: Parent)
                lower_layers: {
                  0x01 => {
                    LayerProof {
                      proof: Merk(
                        0: Push(KVValueHash(widget, Tree(6272616e64), HASH[6c505f53f2ebf3de030cc2aca463d4b429aeb320a9fadb8ae68bb7903a22bb68])))
                      lower_layers: {
                        widget => {
                          LayerProof {
                            proof: Merk(
                              0: Push(Hash(HASH[9862894b16a0792688fdcf64edcb2ceade5c8b234649bfc6cfc6426869b0e9d9]))
                              1: Push(KVValueHash(brand, Tree(6272616e645f303633), HASH[68b697da99d6ea70a83eb41794dca7ba3938d0ba98fbfaeb3cd0c19b3b5d0ff2]))
                              2: Parent
                              3: Push(Hash(HASH[6c36729e93b1a316cbf60fe282eb630c0ed6e45db088e365110302b6c9caba86]))
                              4: Child)
                            lower_layers: {
                              brand => {
                                LayerProof {
                                  proof: Merk(
                                    0: Push(KVValueHash(brand_000, CountTree(636f6c6f72, 1000, flags: [0, 0, 0]), HASH[90ff6f6d9a3d901195982128130677243bfd27b75736206f3c8400966ef0d37b]))
                                    1: Push(KVValueHash(brand_001, CountTree(636f6c6f72, 1000, flags: [0, 0, 0]), HASH[484ca11fb4ec8f479be1f78af903ce0c9d4fe630517579fb0172c2576d6b9652]))
                                    2: Parent
                                    3: Push(Hash(HASH[8ca09dadc802a7efe03534ce4ad991b2f191f368878754a37b5e5c03d9498dab]))
                                    4: Child
                                    5: Push(KVHash(HASH[e5297b3ebe81c6435c29f712074da5f7c90265e12ed3d4f5af1f6d900e50c9f1]))
                                    6: Parent
                                    7: Push(Hash(HASH[50f373fd01dea89c992779764dff82cc7200b492be8f5cf3721627d5323bcbff]))
                                    8: Child
                                    9: Push(KVHash(HASH[cf78c9f1b1a1204bb2e437806f52c21e331392de3436388572bd1fa4bce1cdc7]))
                                    10: Parent
                                    11: Push(Hash(HASH[4a8dc186a95c8c4a1252fb51dbc407727f588eb5bdc8313c96f5c29889e13926]))
                                    12: Child
                                    13: Push(KVHash(HASH[d00ee7653e34e47d46004929b13ded33dff069ed9cc88342cecdf66a65fd8401]))
                                    14: Parent
                                    15: Push(Hash(HASH[7f1d17b9632f0bd440dacf5e841025482bc1d8145df3650301a95a5ee71ce8c8]))
                                    16: Child
                                    17: Push(KVHash(HASH[3ed48a5e35cb7546d329487b0e1ab8a81d7c5bec358c37449e6cbd956e3bb069]))
                                    18: Parent
                                    19: Push(Hash(HASH[eaef9fc530408393bc321409414814b290309a861f474a925a922250327affc6]))
                                    20: Child
                                    21: Push(KVHash(HASH[f776417ede76e6194706e483ac14ab7b3db6aa0461ec14ed5f8e5d20071363af]))
                                    22: Parent
                                    23: Push(Hash(HASH[b3fccba79c14fcc5e97ff6a3cd051228dc755e6de147bef690ba9681264b2b9f]))
                                    24: Child)
                                  lower_layers: {
                                    brand_000 => {
                                      LayerProof {
                                        proof: Merk(
                                          0: Push(Hash(HASH[d605b4b78e674fd77371ea6adb32ce3e58ee3b96d73c4d34df84159661634587]))
                                          1: Push(KVValueHash(color, NonCounted(ProvableCountTree(636f6c6f725f3030303030353131, 1000, flags: [0, 0, 0])), HASH[fccc0c94657f2a78084f789bb6f687c4bba295e3a062f3199bc33f14dd2b7fe2]))
                                          2: Parent)
                                        lower_layers: {
                                          color => {
                                            LayerProof {
                                              proof: Merk(
                                                ... 37 ops — same boundary shape as Q4 / Q8's L8,
                                                terminating at op 18 with
                                                Push(KVValueHashFeatureTypeWithChildHash(
                                                  color_00000500, CountTree(00, 1, ...),
                                                  HASH[6834...], ProvableCountedMerkNode(1),
                                                  HASH[840c...]))
                                                — TARGET 1
                                              )
                                            }
                                          }
                                        }
                                      }
                                    }
                                    brand_001 => {
                                      LayerProof {
                                        proof: Merk(
                                          0: Push(Hash(HASH[f54769bf6e9d24b9dba53ebd37c9ceb3485b3c6511f8de6f17860676fe4d9331]))
                                          1: Push(KVValueHash(color, NonCounted(ProvableCountTree(636f6c6f725f3030303030353131, 1000, flags: [0, 0, 0])), HASH[8f883171c33df0aba2541a5b9d6195faac7bd1ffef93e8ddcaf9d092f0fa5e19]))
                                          2: Parent)
                                        lower_layers: {
                                          color => {
                                            LayerProof {
                                              proof: Merk(
                                                ... 37 ops — same boundary shape as brand_000's
                                                color subtree, terminating at op 18 with
                                                Push(KVValueHashFeatureTypeWithChildHash(
                                                  color_00000500, CountTree(00, 1, ...),
                                                  HASH[881d...], ProvableCountedMerkNode(1),
                                                  HASH[a422...]))
                                                — TARGET 2
                                              )
                                            }
                                          }
                                        }
                                      }
                                    }
                                  }
                                }
                              }
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }
    }
  }
}
```

The two parallel descents below `brand` are the structurally novel part — every other layer above `brand` is byte-identical to Q4. The byBrand layer (L6) inlines `brand_000` and `brand_001` as `KVValueHash` siblings (ops 0–2), then descends via the `lower_layers` map into each one's value-tree continuation. Each continuation (L7) carries a single `color` key whose value is `NonCounted(ProvableCountTree(…))` — the byBrandColor terminator. The terminator (L8) walks the boundary path through its in-color binary merk tree to land at `color_00000500` with `CountTree count=1` and a feature-typed child hash.

The bulk of the proof bytes (≈ 2 × 1 100 B = 2 200 B) is the doubled L7+L8 descent. The L1–L6 prefix amortises across both branches (≈ 600 B shared), giving 2 842 B total — significantly less than 2× Q4's 1 911 B because the upper layers aren't repeated.

</details>

```mermaid
flowchart TB
  WD["@/contract_id/0x01/widget"]:::tree
  WD ==> BR["brand: NormalTree"]:::path
  BR ==> B000["brand_000: CountTree count=1000"]:::path
  BR ==> B001["brand_001: CountTree count=1000"]:::path
  B000 ==> B000_C["color: NonCounted(ProvableCountTree)"]:::path
  B001 ==> B001_C["color: NonCounted(ProvableCountTree)"]:::path
  B000_C ==> T1["color_00000500: CountTree count=1"]:::target
  B001_C ==> T2["color_00000500: CountTree count=1"]:::target

  SDK["Verifier returns Entries([<br/>(&quot;brand_000&quot;, 1),<br/>(&quot;brand_001&quot;, 1)<br/>])"]:::sdk
  T1 -.-> SDK
  T2 -.-> SDK

  classDef tree fill:#21262d,color:#c9d1d9,stroke:#1f6feb,stroke-width:2px;
  classDef path fill:#6e7681,color:#fff,stroke:#1f6feb,stroke-width:2px;
  classDef target fill:#39c5cf,color:#0d1117,stroke:#39c5cf,stroke-width:3px;
  classDef sdk fill:#21262d,color:#39c5cf,stroke:#39c5cf,stroke-width:2px,stroke-dasharray: 4 2;

  linkStyle 0 stroke:#1f6feb,stroke-width:3px;
  linkStyle 1 stroke:#1f6feb,stroke-width:3px;
  linkStyle 2 stroke:#1f6feb,stroke-width:3px;
  linkStyle 3 stroke:#1f6feb,stroke-width:3px;
  linkStyle 4 stroke:#1f6feb,stroke-width:3px;
  linkStyle 5 stroke:#1f6feb,stroke-width:3px;
  linkStyle 6 stroke:#1f6feb,stroke-width:3px;
```

### Diagram: per-layer merk-tree structure (Layer 5+)

Layers 5–6 are like [Q4](./count-index-examples.md#query-4--compound-equal-only-bybrandcolor)'s L5 + Q5's L6 combined (one `KVValueHash` per In brand at byBrand's binary tree); Layers 7–8 fork — one `brand_000`-rooted continuation chain and one `brand_001`-rooted chain — each shaped exactly like Q4's L7 + L8 descent.

```mermaid
flowchart TB
  subgraph L5["Layer 5 — widget doctype merk-tree"]
    direction TB
    L5_q["<b>brand</b><br/>kv_hash=HASH[68b6...]<br/>value: Tree (descent into byBrand)"]:::queried
    L5_left["HASH[9862...]"]:::sibling
    L5_right["HASH[6c36...]"]:::sibling
    L5_q --> L5_left
    L5_q --> L5_right
  end

  subgraph L6["Layer 6 — byBrand merk-tree (TWO INTERMEDIATE TARGETS)"]
    direction TB
    L6_t1["<b>brand_001</b><br/>kv_hash=HASH[484c...]<br/>value: CountTree count=1000"]:::queried
    L6_t0["<b>brand_000</b><br/>kv_hash=HASH[90ff...]<br/>value: CountTree count=1000"]:::queried
    L6_boundary["Boundary commitments (22 merk ops):<br/>7 KVHash sibling brands + 7 Hash subtrees"]:::sibling
    L6_t1 --> L6_t0
    L6_t1 --> L6_boundary
  end

  subgraph L7a["Layer 7a — brand_000's continuation merk-tree"]
    direction TB
    L7a_q["<b>color</b><br/>kv_hash=HASH[fccc...]<br/>value: NonCounted(ProvableCountTree)"]:::queried
    L7a_left["HASH[d605...]"]:::sibling
    L7a_q --> L7a_left
  end

  subgraph L7b["Layer 7b — brand_001's continuation merk-tree"]
    direction TB
    L7b_q["<b>color</b><br/>kv_hash=HASH[8f88...]<br/>value: NonCounted(ProvableCountTree)"]:::queried
    L7b_left["HASH[f547...]"]:::sibling
    L7b_q --> L7b_left
  end

  subgraph L8a["Layer 8a — brand_000's byBrandColor color subtree (TARGET 1)"]
    direction TB
    L8a_target["<b>color_00000500</b><br/>kv_hash=HASH[6834...]<br/>value: <b>CountTree count=1</b><br/>feature: ProvableCountedMerkNode(1)"]:::target
    L8a_boundary["37 merk ops:<br/>9 KVHashCount boundary commitments<br/>(running counts 3, 7, 15, 31, 63, 127, 255, 511, 1000)<br/>+ subtree hashes"]:::sibling
    L8a_target --> L8a_boundary
  end

  subgraph L8b["Layer 8b — brand_001's byBrandColor color subtree (TARGET 2)"]
    direction TB
    L8b_target["<b>color_00000500</b><br/>kv_hash=HASH[881d...]<br/>value: <b>CountTree count=1</b><br/>feature: ProvableCountedMerkNode(1)"]:::target
    L8b_boundary["37 merk ops:<br/>same boundary shape as L8a<br/>(different hashes — different brand's subtree)"]:::sibling
    L8b_target --> L8b_boundary
  end

  L5_q -. "Tree(merk_root[byBrand])" .-> L6_t1
  L6_t0 -. "CountTree continuation" .-> L7a_q
  L6_t1 -. "CountTree continuation" .-> L7b_q
  L7a_q -. "NonCounted(ProvableCountTree)" .-> L8a_target
  L7b_q -. "NonCounted(ProvableCountTree)" .-> L8b_target

  classDef queried fill:#1f6feb,color:#fff,stroke:#1f6feb,stroke-width:2px;
  classDef sibling fill:#6e7681,color:#fff,stroke:#6e7681;
  classDef target fill:#39c5cf,color:#0d1117,stroke:#39c5cf,stroke-width:3px;
```

The two parallel byBrandColor descents share their L1–L6 commitments (the doctype prefix + byBrand merk root) but each gets its own L7 + L8 sub-proof. Proof bytes ≈ shared upper layers + 2 × per-brand byBrandColor descent ≈ 2 842 B.

## G4 — Range on `byColor`, Grouped By `color`

`GroupByRange` is the proof primitive that enumerates distinct in-range keys with a count per key, as opposed to chapter 29's `AggregateCountOnRange` which collapses the same range to a single `u64`.

```text
select   = COUNT
where    = color > "color_00000500"
group_by = [color]
prove    = true
```

**Path query** (uses `distinct_count_path_query` with `limit=100, left_to_right=true`):

```text
path:         ["@", contract_id, 0x01, "widget", "color"]
query items:  [RangeAfter("color_00000500"..)]
limit:        100
```

**Verified payload:**

```text
Entries(100 groups, sum = 10 000)
```

The 100 groups are color_00000501 through color_00000600 (the first 100 in-range colors in lex-asc order, capped by the limit). Each carries `count_value_or_default = 100` since the fixture's deterministic schedule gives each color exactly 100 documents.

Wait — but [Q7](./count-index-examples.md#query-7--range-query-aggregatecountonrange) said there are 499 distinct in-range colors and `sum = 49 900` over the same `color > "color_00000500"` predicate. So why does G4 see only 100 groups summing to 10 000? Because `GroupByRange`'s `distinct_count_path_query` applies the 100-entry response cap (`Some(limit)` in [`execute_distinct_count_with_proof`](https://github.com/dashpay/platform/blob/v3.1-dev/packages/rs-drive/src/query/drive_document_count_query/execute_range_count.rs)). Without that cap the proof would scale linearly with the full in-range distinct count (~5.5 KB for the full 499 colors at ~110 B per resolved CountTree branch). The cap is a response-size safety control — the verifier ceases the walk once it has 100 entries.

**Proof size:** 10 992 B — ~5.3 × [Q7](./count-index-examples.md#query-7--range-query-aggregatecountonrange). The structural reason:

- **Q7 (`AggregateCountOnRange`)** walks the *boundary* of the range and emits one `HashWithCount` or `KVDigestCount` per merk-binary-tree boundary node. Total boundary nodes ≈ `O(log C)` (≈ 36 ops on the 1 000-color tree). The verifier sums subtree counts directly without descending into individual keys.
- **G4 (`GroupByRange`)** walks the *distinct in-range colors themselves* — emitting one `KVValueHashFeatureTypeWithChildHash(color_X, CountTree count=100, ProvableCountedMerkNode(…), …)` per distinct color in the range, not just per merk-tree boundary node. Total ops ≈ `O(R)` where `R` is the distinct in-range colors (capped at 100 here).

The trade-off is exactly what you'd expect: `AggregateCountOnRange` is `O(log C)` in proof bytes but loses per-key resolution (returns one `u64`); `GroupByRange` is `O(R)` in proof bytes but preserves per-key counts.

**Proof display:**

<details>
<summary>Expand to see the structured proof (5 layers; bottom layer enumerates 100 distinct in-range colors as `KVValueHashFeatureTypeWithChildHash` targets, each carrying `CountTree count=100`) — or <a href="https://dashpay.github.io/grovedb-proof-visualizer-widget/#f=text&d=H4sIAAhCBmoC_6WdWc_dyXGf7_UpeCkBuui1qktAAic2EgNZYCCGcmEIRq-2EMFjjCU7RuDvnqcOZ-FohtS_j6kRyfd9D89SXf1buqur_-vXX_3z_ov__Fdff_XV-XX88P9-9uHDf-__ur9-feP15YcP_-h__9WH_7G__j8_f33jw4fwqw9_9Yd_-vuf_2X33_7T__rLvxkrWTzJmuUz5MiSFGJoqkVm5we5R21rhbZj5ltDZky6LfadJeWRf_OLX3zz3PGb5_5vv_51_90f9usl_uyXH_76671_XvZKSVLRmkxTP6ZnaOzjlCA6Ut7C25AusYd0ch0pWGvRztrZVmr2i19-eL3b0mtPbY44SuAJe_fHnrBqSCq6TsxJ2liniDZJugbPwMNq1T7X0U_ebeLd9q_3P_z-m6_zjyITbVoqBGYXzd0CMSgtqa4wIt-ec7Y18ppNwxwz5rBV88lLbffKp_v-tcqvPvz53__2d-vj17_76l_213_7Ox-tf_rVN0P14cOfffgP__G7L35iMD_--okh_eHAfhr88H__vWH_ZvRC_Db-dVjYcVtNQ_c-FvKctkZaxr_cOZee9egObfa1a5oy0u6Z1xtVhpRKTH7xyfv-6VB884n-3e_-hxH9Ylz_RHQ_N3mKbY0WA-9QU7a6l6Z5Qt6m2ZrutWUXK5WAVC2HBOEB9ZS5dZOP7Tc_iMa3v-JPjmWI34yGqpiQ4MJTfjcuy3o4fbVOBqYq4bTdrIYZdytdz-6j95HiUBtT09mzkMulpFbKTvv89Bv5bo78-IdfGrlvxy_EnxqBB-PwaDQ-n_f_8tv1d_v330QLFEgSZct3sZJZQz01n7QH03WHHOZMffYieZVRkvU9cgrdCOhofYMoxC3kntIY0v4oh--i8vHXx3f4-eg8jtFFpD6XwwZQNisjSg9qwGc7ax5h8s2R5vZ5PBuzrUixcaZM_l94mNgI25b95rPR-HE-f_-qPdne7aQgPdUMlQxZvZzZ52lSMj_b1VKdy9TOXIWZVi3ICcdOGzv_6Vf9Y4D_3K_8Exk0v_rdV1__8gOx_-c-frf__Ks__MPvP2ZThiKnHJ_FOXz3v5pjZnrG4L--zTO1Kraq1TW0jkG62ZrdINQ6LTh-zgOMmQmkJY0pGcNOQUE1MDTOP_0Rf8grn_v1NCk__np99D-VmZf5eZ2ln8tVUmO2XjPYOaMCc5BLAFdL6Ltp1SBmXSzx_Q0zDwGZPZQ9wcojpD8d0p_K2Nfwf3wDWVrJRbvo3Hv1lc5xeJ1beEEyoUktecYTi5zKF4u5lLLExI9mqOM3v_xQI2ny8H08zeHPSZjKTE0AXD7znIUMioXInNaCMd2MBC3IPT351J0LoWuFBM6OgeqE_jRe5XPx6h1dFkqICmyUqnOuimobexQQpMPXI4ygecUZJwhbDiqtRYY5x9l2J16p1sfxqlfxkh_FK8COC1GpyMzNEFqo1nWGJOMYSRdnWnEtJu6EzNPp_A7hBjRJGgjCp_HSz8WrJlKmQMw2Vt2omVXrsTJjHDuNOA1EjDGRT0yFM5GnG0Gu5Dvv8VSPF_L8cbzaVbzsR_E6Z9c2IE4UYDwWRyjbJgPY6iiKwmin9RVRKlVb0WV4icDsrLUvmUkez8fw2QQbA_G9kYT1RNuhk9lodVAVbzJVpupkDIcTOYhAALPYbsQMBxGnEjDJj-MV41XAYvqxYtTU1JZAo22HiCJEzhKnhY5Fk_D-Iz9S5ubSmNYeIcx2OlwYRitWH0csfxbC-swNARmcxDc4CjVN66k3-LZ7iPaB30o7U2fPvW_4TlJtytQARYhYfo5gsdxFrP7Yhs0CsApYdcCFXdByWgD5ucM5p_mbjBiEUPoqekqZEksiB1NaGXwejyMmn4sYCt5yJX1iYMJVwKsG_oT1sRlkPJg26uyN-QeEdTzQxHiipzJyKudKxJ5Pyah3AWvfveu_-O3f7X_6_cf3_WLxv33JEdDzW_OsI7icQ4KkzUTFvCzNAvgzeWHN1lIAi7FZIVWcFxNqBSO-ieml5wUuzz-H_VhX_Zfdf_-Hr_df_-s_7v_929___Uu7fK-3vn27CKnvBZe_-ei_nd_1v0O-_A1_9f9-863KMqi25LzRjyXjRE8MdiB9jYWf4B7HEscnnUranjwT1lRjrXvwrRJ_80cyby8XKP_zq7V_nr_XcjO1us9ozIteWlvbmKa75QDkAcCEbErTgT0eLfdQMBYvRObNtBW1P03CFK5GP8V3g5yugowNONCO-BIIBL60A7mBidfwt5lPi3xYgt2FpcioOBxgNyqerMGRly8EOX4fZEtlpYwaWYF_ta203DrhrOTlnPlsnKqm0aPy5O0IL7y0LslREo8qj4OcvtHNzx6drx5d3h2RfDUiIrvH1MLErfLXXMMOU0DJPlooOTCn86iIz4Vlqp1RK6VbbMvOwHv1L41I_X5IeBJgf6h0Jfdh1lP8aYdJjBkMSSdozJ0JB0pozkPAwQxz9R77kPN4SO6UW5J3o1yuotwE3MfDjYh-CCvFWOABK3y2Bg4ULF6yWpAds_czIE6SccYGueOgy3yW90HRnGml1cyCTtilukTJOUF8hSc8shjKsNc6e7YshTk1QSHFNCZ5rNKTvhu1ehU1J5XeZDZB7m2mMBkpo0KjanmRNphgPtwo5cQaO4we40JNB3ktvcgzSA4gOBIZdb5PzjEFjF5s8TQJ8zBGJQ6sV-jHIIbiHmMliQulgwTDij-O2p1ITm_znlwFGc5u0NgWWbth3hYyvCwLpOdoWAV-Dw1rGhtuAZOFp4gzQ_gQVo5mz1ITk7hKZcBGiQEcPmX1grCqLbgcagsnGXFIM9jMIgev2RnpgRLShD1-GuQcbkA2v017epfIAUlprfS5Jh-WONoMqMveAcSNelb3XNhqotFcy_viQEbML9O4U_hCjPX7GGOW8HYEDjViu7SxkeIDmiu7RmxUdPI13kEfjAN2Ct2O0G2CzVqIm8cxvlxNyO8GuV0FOZmbMcCPgJ1dhlpvCbQz9LyvAkz4qFVrWkdIsWJdSPGe2xyokDbSs0RuYE6Do_h9LZ1lxazpZZ-HpR0kbGRKSENkMDsYj67ktiSMI9JRHqNFfpv_7W76F-ZdL7DFTiKwxMomp8ETHVI6fWrFG9WpUEaodQ6rxqxHj-p2ZfwMYytUc-qYmCtTyUkx1gmC2yXlkZybdkKwb0sgLZYbuG0IkQXiBuGFHkftjv7zu_Qfw1WQtWOTPTHMnTnsYr4qjfyZfPiMKWYu-kKNr_ti9NJe4ADE3DqJpGU_S82OkQ0NWueJCCaCSyLouQ0zQ-4Np1BsJR5sGq72kJ19CVZ4b58pjzVW1iuMbVePtptHlyu0L_Hq0VfivlyJ-1KuHl3fzdI7Bzz1QAPB9hGn-8JMHXEw74dhV-tZZw4Mcs2o01ai72D7EvCpTGybX3TAP9zQqLkklxgSUQS6XhxFomMCMdIdpx1qcamayj6uNnyFch8bkrHIvT9e7StyhQblXVkb70ywbykQ4yh5Y4Dm6YjchP8017sp8MGPHkEq4PwxZDtBz31Y6b6clkJ4hga116Rno2MztIhYzlUzQFCyVJMQo3U3GUW6iALI7STQaIPzSGENj9VAae9G7c6oIvorGknGSmXiUSvkIaBjSOSMSfGtE3wrhiceyH5U2NdXcVD2QYrUZ0RFFEhxLwcBfIFfccVa_c22Hs6cr11eVBMIGnoPBUIbvlq9kKsW4_PUtKvUrOHdIN_51GrqqnzCVAN73gszvvXeDXbKK-0S8sQwbHQqT44uGHlsGBwm78tt-qPUREOs7PuTu8wiL9m7sb97lxJ8p7OISCL6G9hJpGNHIK8IHyoyLZ7HuyP1CvBrejfGd642xPMKo3VBhmoZONhIEMm3oi1bQ48aIogYIbm0Fh2WFd9U9xL-2TMz4DVHOIgZAuYWL9tL3mlqT4j_NeqcZiLZzBBfoLEZYFENgG15MCaPDVfNd4n8rqyNd64W4UNI5cy05ZyuMcR8cGEDwXWaZV9qOUhO3E8oEn2hIeG29pYSsZv5aSIvEjmsUMpcuIlaWh97jwNbFc2hbD3jxGKFt2GMIIOKc5g4kjViDI-D_LYCuPOpTDDot5KPNXbBNEJHfUyCaIHJCA3ZAVd1kKYVJD7YytNC8nWCuOZ4hrG8iDAOntLxoPixxfgwL78LmolkDmluxAjKv2AWZtdU9-ynL7cE-TEz1Tv6r2_T_6VP7e6huu939qaRGIS9mfLpbEXv5Dpi2m0EaAtzvibfWHr63qukhkd6lpoQFjqfdDweNIZtA-Vn1eG7WaZRQ965k_GafKOQiKcwy4HxKsKrtsdBvpL39Urey9usd-eBpZaNnS1DiA35XqAzTJihgrMhkqr4snr1KoeNO8bLx5mTLwL0k_sXEfnTNXCgoNpMObrxq_4_HdH3RAegj8s27HFKGx9uy9c1wbwSl2pr24ByfTokcre5LO_yXrozwROGHxEN21Zxg6tY3-zrNWvnFiQQiZgAUIKggEtzQ7AmCO61EiWMh3s_5HlKE5HY0Q8lhY2-tuYlx9BgnBU56GUXExkMREvbzC4IYG--Xac9DvK7q1rpzpTlFKpoYxb3RUYWwuUb7j1MNdJUQel98P1g6Ebfk7vMezJJWo--fPp0DXwVXb1J7r61iXXAoG0wZKOHY0KKJYAh9Q2NzoFXAVmAK6tVNfT0WJHJ3S6-vMt76c6RIeLhnLZ0NyNBgWPEPJ8MmtbVdIwWc1KmJZ--NsO92RZFMmGDgeiHqRkJVNrrHHLa2lwoX809ayMXkQNKgnb-TgL3KPFVL8Tcr0wDzBok-zjIcgWy79JeuvNv_un5xEx8rz6SpqX2ouRQB2RJYGIRmIm41WwR-POtb0ART2XDF8Gfyd4kPEXHvSB4cQwINEiTsepEepfqShreReYS4xHQaTF149nBmBW1PZ_-d5s58u5mTrrzbxkPJRXeinxaw6-htYIRUuAwgBnBHbH2fNAYZGBHyEE7nuIRiRDqw80cVKxiEVeraRVGdTlVNuyJepXCXEiiPELLtQJZi3eUCkBbAf9o4Nbj6rZ3-T_dOTLeurXY-RhFdmJ6znmgh257TRxDG4S1Y_m9Zi9lXXlkcbMqUEnWvJ5hLCTegxeDzwKyrjqBEd8Vyr5tMEh-0Ls1WR3bXQGhWIt1khX6rzPOx8uzekf_-jb93zmyEGJLJu00uINUabWnF7VnPNdgEp4yvKAsnh3Jpl5zObXhBTQefMnDLfCMwPJ9BpSFapJJEp6WN5Aw14C90G12sDVIEd81DnbWyL6WsfZiMjwu8tKr9V69Wu_VevXod_cw0p03JH4Z7yAgiCyvyaiadupl9zhH0-wrFxZ7GI4yyOaU8Ddis-GyW29fWhrKn25iYAgRhNk3f3LHFsIFPe18OuY6OM6EElrxgsKqlZne-4SZmbABh73y4wG8K4_Td1c50505FNDnCDJ4IuGsCS4YktwlT9PZJM0RjMQduG7ROWssqYSESUCngeX6bJYUfMv2c1U7DxcfMgGdOnzFOGG8R-W_Nk4qrvckM6Y9ebU5MLT6bI-LR_Vt2rszcGP17sf54Pgxpvmm7xZhahO4BpwXRUWIr3AdW2gPRC6wM7QUpMWy9nBtGLi3vmLVFbb0vDAV7lxQ47P4IlseTSYq-XiRekq6E2oHexEKwX2-ot7uavfau0UMOVxuW5CHacyTzK1Z93Ki6iuzJeoaIoYjlo56FU0R-RY6QUoI3B63l_c9S80dmNHb155QZbE3dX_TVPJZyMFSiasMfdUxdETMSXFgUnDn08v66uOd4na1vdfedXv5zu3VaijUXII4f5UOmRWvj0HN7ZxaaRVLMHuMfgwknRXOYUzQy-T_jmM8E8k1BKfZFUceXqGN0-ElLMYNKuRurQsCWfeYyct0cCpgOvhScJcTmH8c4zu31951e_nO7aWN9shAZ99-EmzY8ULQqgWdJbWgm4uiVzsyAm_nJSW11dOhthPGGg8rnmKt5RCyELSiK0dZVUHoHPB1XtwNJK2FJwrZOpCuCTO9GPfWIv_lx5a6vcv_OV965O5sE8E3s0TQpOZR0FOoNfETqzP7IjAcYmjmGCBr5EHLmDD83nookr0qkn_ied2DV5qgticCETmx_VD2hhoNWejf2yn7aalFlH213RoG83HU7ui_vUv_-c6_YdfMdw9geJAVderb3zvuiXcryJyaqxHZrIUQjBaDL2ZVVwg9l_hFIvvUvw0yrWOqc-kkph-tDgJLDt8nwZHElcESVEDEFaX2Kkub0U-bHD9X9Hjbol2t9tpVMYe9zXp33hA2AxmgL1lIi9rX8P2JChZsPzw1j-qYbh0HA5GYEzNARS7FEAgnPa2PTgmFh9oqlSdOq4nXW4Los7qJRj1kXGhy0cZUOAIQj-61mcK8i8jup0Nid7V79jbv3ZnDYlEwEV1rWeimKR0UJrHDAlsOYMycr4PApBa8WH_7GT8mRrL9OjL_sD46K1qhtFgI6lx4wN0ZydiXFozKQewqY2CHgOLMYb78qunjZbDm5zHv2bubnPlyuw5lbidsPkPg7Zn17D7okDlgQ9WImWo57rr8kImSleB2SYFcZTL38wySh4bqp6AMqPHFEa0rRT_WOXdAcx_Ys4IdsQ1fDfbys9Tj8S1sgBzV_Thqd7V79jbv3TmyVaotC2AjHz8hPI-fNTWXoiIoVAGKo1uD0TEcBcJH9BUC1roXUz9ct4iK5ls-nPjc7VO--DYdkZ_diGjRg7DOC80A41lMEF8do7cYwZrn2xZ2Vbtnb9PenX-bmawhXxKJhd73M4sAX8RPNGb6rNLJLUNelNJ6qiesnn3dFv2qR09-uDZsDdWC2tWx_PS8hLpxt7pQaQtwtZIZaz8JWMDiklNIZ-D0kBejQpKPY3xX20Nk3oxyuTNwJTB380zSJtE8OSmqrVkPqCey_KAymLFgYNySfOeHUOSC9GC2k-wPayKKn6okom1FT-bm_JQTCg0hs325CJRSokuGJ4116oy5psNLl1DCeH7MMryrAEq8LN6vC5JJqSrcrFgA5uDqvSEDQAZCyjcmdO_VZ0UmkEe-VIyvK_6tz1AWaMnDl7Z8No_ip9_GhhbrEvLUkz5Ppj7I4OMVyFkkIZJk-OHVmMrzsKXL7HxXApQ7V7akiSA5_bwXZAvhi4Xky5TZFQ-GuPiupHUoaBff2hwJESs7jYY3eygBfBtTApP9JCm1afIi03AYLN_ig-GiCXy6SMnl-_6WNz4YK47zzrnm52G-WvKNod49XO4erncPfxf2y52lTGgwLO_0nYHTRtkIWgwO3u_VYcjLKvspewxGRHc-dqoviC6E72JCfKngWD6ZWhld2SbTd0OW3rzIvOeTCqopbRln6EKwnIooOjwv6SBAWN0AFVgY1_MxvwT--Dbw37nKHaX2GaNu8crtGgrwG-IBt2raFbpF3pD7oJe9hExgSBB42qMlWOPZ1CKeOeNeu2pkwupo5XRGzASaHnt58Z1A7AwjWInDtSBj7lnqsgM5PA5zfBv4L48f-ibxNvXF3QTomLc7ORNLfHTzQaBTPyIeG56cNF0tH0gpHYw64HQeAj_P0hkcbMiEdGPf_txJkUPeTu94HcxGZpfQfXucLJaUE1nJo6DO0p6H7RL449vAL7fHDvDY3rKlLGw0Cacj8vH9VJoXsEU5mwwJY--5JqkajtfMwcN-lugp8DNuXoQZQPLVom-zCNm9JXpHwoq9U6bIxEfpXKd475hWZPCioUhZZT4P8x3wx3cXPcudVyxMvRBXicCsb2qTyJaHrwRv1ePVb15GJF4LGPvZvigZESLepNH66uuZxPauesmbXLTdV8Y0BvElwxinAOXGC2OtT3Zv45La2tLohVgzrl6kh-dRlstkfrfKpdy5RXgkk9AmOXt-kq42dWDsFdk5Fo5QgFcLXgoxibcsr-2utabunWziw11uwuetEeFGpGeI_bX4CYTn5isBW7xcMfMmVjgTc54B5ZPRU7isVPp5Hua3lcCdAWQ2DoleMo22FeC1xsS7Rd7lcQxz6N2K-LjgquU4aiOq0yvnmx_I-OIB2U8Xl5UZPnZ7nbL1fpx9B172hBq67eZdENpI3qKgSeppRQCgHkXj7-Clb8_DdikE0rtCoIbL3eXR5WXDXqY2zIHu8raXQw755Iuc0zfsdiqpVi9Sh_ayr-YcL5p_mJ2p8yx-yHu07DRfYUeUxYiNqdFgMVC9O7xP4KLM6XGPaZ--O-mcn-utdHW-I96164jpXf6rl8fu3F06KuNnJontO9YCUALNI8fmK3buz200iy3WVhAiO0VklDIpdnu6wozmKjZ3XXhHr93iCaThMiconWrBAw0_OIX7Qs_ktiSlZmmlPTe8cTEql52a0rsMWO8MJhk5tIkfgB06K3M8dLTnCMS2QIa1wHZe_VleDUp8VRiWVN_FiurH459B8_ReJr35ge-o1UQlgllVsgRwueNmV9cCZrtg5EW314N7oRz4d-bzzoJvty6plw1ifDUnKJTlteveMERVAZLoW3ITOeXHsySA0BYNivNFdtvIC8l9YeOfQXPpfujubNz_GXXmCfwkr4IPda7SGbAdvPDYazU7xiW-ToRWhuVVUf68nVm6bIuV3mXAeuvRahxjHj9_B3GXkrq3ms6pjooNhh9r4y-g5HBkLn6Iq5C-fpbAt2IfHhFHMRsuDMO1hpD2JwclN82bLJWUCwgD7DQsIup44bYTru7UxjD4Ot3zMF9t_cX8NgHeWbrO7ITiS8VwldQQoPxaZr2uqYlE7_B9ysv7rBpSQULZ0EzX6UcyVnyoghmRLX6-rnU_fbZ6RAcf5kz0nZfS4mjYKx3e3KC-evyc0sxPtq4Qsz3vzXfZRjC_W-tZL7fzspdag6YjoLaG6VjeduvVA8btQTuVRyATSN_cRFrrzPSdZSNPssjTKrYQQIvSvKthkun9G_1QbjWDuWY9x1cfdk-OWKmgbMJC5aEo8ZkSnjvntzuY1DuPhtDVIF6YBaqO4GfiYakO0Z06ytkp-5Kv7hnRayd5k9Owqq8LoAiwbw8XHLybXQ65WwpQ3iDiB5tDWjIwHYEMYfWSTgRsIWdvA1R90VTwxTDoc_OQL4VAflsI3Hk00zz7jq0d11ZnVfDNT3bv5Koo2q6hbtm-ugwbTUQZrinKyTi3hFp7lp2-92lJg3WMmp20MHspqHc9md11nq9q911t-TnnrOWIn3_0VytjX3i0fLcWfNe9I96174j53YrPemcYCyKu-XUSimM4vlLkRZ852w5-_sFyGM0rCCsZ7brFm9wvZtAMbVRr8rAaGdja5uUyuRc_9bR3QTjO7T0azrZevAghnIxTXExLX8KIEfEewvZuBc9Fc7mr-Yzl3TVQuXOM5kfG0Vd8QD-ss7tWsAH95x3umRvRT4B0771W0bw1wpNabI-jOZb8xaMRP6jadwoUNYv1RCaKNUYNj4ha6Zt5lxfvwntVeVNHneHo2Fnj2d6z0Z4vzpV3CVDuPN3xPlC7-TJ7aHHvFl7EJ0PLLvtVcxgGWBC909xq2N4-clHpx-ysaQ-RvOfh29MZQZPFV6aB8jBD9TPZK-YFnpXhp06ben9ZP2fM078Oo3sHl-dhy5fZ-W4BjFy2Ex3pZFvR-yTp9GK1mRFuhvOS6SZX4_a-Z8iEgpsbCAJFIAC7pPCsT49SewMMz_cxlo0JTzYYugbftPD9E5HN2KHzukmHN6ORx2p1xLLivAnz3SZgedcByp0DHHiRIKgCfAdCzqJ3At3ZO3HXOrwpx3h1tMKOofvRdHwLCnWjYX5I5GF1htd8QAMd6vXGd32j4HbNDdxJg1fNFUvE8y2_mASc965WrU609TYI9HmULx3g2w1u5M4BTnLTjk3B_i4_47Ek-_bYKsGrkCva0NVhQDD4JohG9AkSw2uNFqF4KJrj9tytBXne_WzNOd5kvZiv2Ks3JQgyrE4CXPxUcT0LzIgaV9rLJ9TzML-rBOTO0vFeK75h7tWhg7F69i25TWaeWbEIzFP1Hmt8E0FNpPogdAWiSjGkL54r_RRq1XyJ2s-i-rJHaCWMws9TsmGtAL4oBD8O1XAxwVuwNpT0iTCmt6h_XJ8Z66UQqG8Lgcvje_5B9zyQGnZ5G8zvtVTSvcdhRvBAaDLRXX490JhuLWyGgJsrU3fpD0_vt4CsStL8GqapWG9vCwAnMpxAEAONfNtAxGxnITg2gNwQIzEqM0Cf7x_Vu7Xgmu8e_jb_6eVuU56jaq5-zOiorAE28idTmKBV30qtZ5zgG51wfg7N7xaz5vseK439cOk4w6VINLDndA3HEN279VP9WjO8d_Az1NMKtgZ7RAbs-GoM5_dHTaTJ8_WMWi-T_20GvKwBhYiWLyak6lv-3pxillyXYiF3l9CQHr7JGV-9wPy6FkZk-q4K2iE-1RkAyfBKR0ygd2PjqxNa7ytrPoZrQcE176PfcBayjx8dHp0Ql412RPI9D_O7m6FyZ9IWhtq8iLMMMi6FjEOA2pnKYHVbsno_uKdOdmkafpmZ3-yR8xbfFsr5GTQvTX3iAskeRHcdYJMvoO6cVgqMARJ8twM-TMUUnrlib60j3NCOu8bnwqG2y-x8lwE1XFYo43ABzVPMWzQfBFPNQRZud8TuBQ9j-qVk6osbIU7Jo3Zv-RVdvObwtHIOy52WrwV64YVP-FEwx68b-cJp05td9Zi9JS4q3DekSylAxukJxni-sSFX50CivEuAenk_BO52Mzn9nLh5D1W_0gw1VRs05IsEvqWPOhVvO-HloEkt-7ZeL33r06N5Y7TUoqgOAk1mxzWlwbdpeBcQ-VgXDcee7QWpyZcjNBN49OFZOT33GnJZDfR2vxu9bMWSCiZtOaShhsNqhh7G0aYdj--Tene6nV8LkyDtKtCMlTUKg-OHIx-qYL_qYOQDynjpXyfMy3jR6MUEki35eoauzkzZifEMr0vBml8PWruu8Rwz5F0loHcerXqXI79gJXmLX3Ej7I3GCVBd8FM_HQW1_fgYsj-0xETlszZVv9BxzYdHQTLpBkq3HREX1TcsZUP-Qfvwaw2bNw7rzYhROugxpn7ZURUUAJdSfL7iLpdCQN4VAnp5Pk9aX96zmcl3XlQvaP2QxvCL-9rYC692QunCZGQk1K_Q8Au1Shlt6cP-KsErqNR6r0XRFIxcRs_lRjqusLG8i0mPVuTXKq_bZKXBnN6xm5lynt-GJXdrwXK3Fix3e4B6h_t6V_zxdpsTvbSjI8_Em2PUkBmrT2YLLtQbRE6XbPH4_Z-GgG45wCUYxdL87LUfxAbUv5Qh6ROiQPgZoByZ8LxJZvuQQE4ucsa7phySbyU3SEPxBVl8YSZL9Cske9SLJVO9XPvTtxHvzpAy-0A43GXSNgXj4ZdN9ZJco4_kp1mCDoKA9KuTMPm9hBgX_gm8LefhhjmTC6szw9IyhnaNfmVkh5YqIeapO54ngHJ-AsTrPvo6zlAjjJhtnecbAPru5pfeWUaEsF99Wby9_hH1i5d8Ke3Vdj8G22Zzxn5O8XKg7itN-JiYQkLDh9Af3qkC5wyT0tXvX9q8SV-gxUMBXSNu0dR6HJpROX5HuC-v8yp-Idbhb1iq52G7rAPVd62P3jlGX7JbaOADFtQCEsSiY1pedrYL4pBx5b70v8LZipb2wo89kolmFyVPl0sk9mjDK5fyOLt3v36Yl0J_-vaDn9cuupIgYGbLWvKJMtClJ_h25vNyDr1D_rf7vuidwYy-utSIcIIbrZgfTkUSe6O55TvW3Q_3oDyaXyCZ_Tpg2HH4qn0vwW_GfVbO0XYwON90ekMHFQAAN3liIdjTb_vIfh2qbm-25mUfBh41hPyZrdxU2142folvd35pdwbzDPMSlrgK_FKsW6vTwFiPK65vnh3K4lPj3cdCqsQ0cfsHNyoh5_y0KwFR0-V3pCa_lh1Fhf7G3vhG_U6-VY6XSOI53TIil6dfvsmI8GKureerTO1dKdDiZXZ2MJWEge41dd62ioMqOYE-SMUPcqiNM6QdZv7ya93zOd6Be3aRh8sfEVsoqXvn4OknPMPA-FcMqncdQnMiPjKOBY8zpfoJEm02_J5L82_151amXQqB9q4QaJebgCjfsF-3es_XkS31UnIcSeeT5tAXljxvcjPlvALxwNsxBNu7rQMFD-s6R9boF78yft6mszEjvBOWRJy_79CAvcpgLizl3L48IsULatB2bS6EyfMw320Ctrvqj_Yu_7U7P7pcbvk5eaByMQlcdjCdMfG75yPeyiUHiCgE37waa_la0hDAJIdlX9yV-XRlerfhB1S8-wbCFmntx6VyJA1MSx19Z7_5MS2ZvouYdoiROVKKFV-xXhfQfLn2195lwHZnSPHhLQaQ5HTfpt3q_ZSDN_ZHqBbcuXcCnUtDmpYGIOqFGb5Pi49FlzxcLmn1-Il49Tu0oDW_ZGMiKaKig8Gv5EcqZEB64lc3oN9yQ814g43ja9bPMcberQNtdy6tbV-q8-Yi6h3cvJBOQ9wQI2jpVyZCMV4mVLwFTvQeZxLgGVmu6iGihw3j4hjYtKXOlpFhEaAXv5d8wZpsNdV9UGXJe8f41SZnHCmhDe8hkZ_fLBjtsg7U3mbAyzpQLMbaErpXwPg-M-m3O1PU8XJp06H4NPEWnd59MyGyRu37ZbTQrA_7KjM6MOk-zSQGwHadUbAaJ0AJ3qK6n3JK217GHrT7LiLWRIAdHKLqhUezu13At9vAtDtLh_hK6XRZvqO9Vqy5klLKvN-t6oqAbbGda2cIvM1TJruxfqgFvxQ-l4cXilTmNajJjM6-2LyqyTk8gTeQLEyW6kd817GBjcQ2-21FkYE8uqqfqn8e5cu1v7cbwbTL1pxivmRMlL36lgnc_ZSot6zeSoK9Tnl0r6Y4m0cOoIK_JxLdIJ1W4uO1v7mWg_ir52Fos9QyiiqofZbXB1sCHHyPEQjbS3hHuNo0GfIozxtBRXtbCVxuAjK3o18qWlf1PWHzU6Z-31XWJUucTY6KL4d1FP_AwgU_9ouPaN5a8mmTohpmkOqX1xTU4Eh9II1Wm2ON152ounwLsglTfhR8CxjkFXFtejFXeL4cZpdCwN4VAnbn0bw5E-TiJ6TnGC5VF2YgDeQVEjjNGryJ0FjetKjDRepnTzHCljF2IT9UwTzcn_NjGcDrCCFGBs0NLGhcs1cDUIhu7rVWPxk0y14Dbecbk5Dr8-vLrxaDU4h3D7-7uPvtFih2aRhHR6Y1P4Vnpn5Is2_za-EmU9yCn06vDJqtHKTnUrxf2vYuiCPuOexpUXP0a_Wy4c1L4i3WGMqr5IBBtZfZ9gbOe-JPRVMxXSDbGeYH5qeXXT8fxLsDACm8uwZqlxfwCSpkto0Nnsk7vyw4LKSB-PMifL_gpffYJn8CxorJmL5aBrDkDPQ_PAAQfB2o8DqTWdb8QqPB4EGGvCDwl3wTPQS_CtULSeESQF56nd6cyk6y52F-lwDtco_RWwhI3L4TNSF-8y3T5idvqrfgPNiBOc-qUT2HBPVRYl6ZD1hnK-3hegbSArxO42RvI86_K6sLqkIZokrKL2RMkwbe-ZnsAtSToDNNNA6esl-ETS-z8906ULuzdH4TSN3BGxDBhMkSAFpnFa2-7ZiHX2Ps3RIyUraMV6My9Q0Qb-pVTn-42nY2NGh-7wF8UQHxnZbhi5tW3yOCL9fEKQPnsfU8Vhve16WhPc6GTfV5mK92AdPbPXHs0gEq-He6X-BEVo8eO8KqnRFeh3WHt7PdTJAk2-srilQs3MTqzrXLyOthUfPI6icbSNNyAsZk9iSe323vgTxJcy_MJbM-gSygUdiCQop-plVAhMclXSneOcAU33WAducA_Wo4x89de9Ju7bWZtP20CKTmW0GhveoJwImMO4xeyQgOdj_MAAw-bD1EYHPTvCQKwcsFViSaqBPgyqkTbtzoIG9IEEroNVbvghA02nztnj4P89tK4HKXzstb0xiGiCurKZEZPv3O9jDuIUOaeFMVX06YH4-yRX-oep_U8VA09579VgcrgLOfPl3SYp9-dnIsZgNWZ3eHnJM02dQcz_BLVV_yAA39HGrjpRB4uymO3Vm64ldZ9VNjr30Ub4dlCy5GIi_8x8oLj6xIBMmt-tUkjsJpSgUEm5RSnh66NsHV7L2rlhyr9QYARJz6EK-vH2WMAL1MjF7y0-xnZ8HNTJ8nVsbzMMsd1Ordw9_mvzvD6LfhSPV1fLSnMTTQnXdC9eagKa3QYyrekeL4-u6rITBgmlFnuMgav6zOPlk6Djv6zDp4oFjG9stQu5_THn4f8vRbzTAtCO62cjnTW30ZfFund3XV9LzjQLpsBpPebQYj4c4x5trbqsuz3Q98pFxzGSt6W1siqaPxhd-76xurq0BGw28j6-mEvGNaD4uaU9ZzsIKj7bJebfdz9pPLNnLoxjcSHKnmt30yCcTa8FtlksvwHkiAx2FO32-GemReb-nn35wSe8n20qsvmqzVQkRbIhhgAlPyGvJoXhRDekhnZoubLyhq59Xzq8GYg-bT95Euh_tbSnmN50fAyHWBub7y4_Q1zE5Jzobm_e_NW9TiSnKeBSexiBxC0VvN82ZnjfMiZle9ytInnVr-KMRWG2_wtNRVSkuvhrqymTwm1dsOWvMbTGM564gPd5zLPxrZh7ofw62BXoRYLkOsPwpxCseviLQO8Ppp-uwdRfC4NSA_LPl91Eiw7ncYTMP4BUwGFsf6wD1DmM9D3O5CfKeW890ySf7sHOmMSi9kHNaDZO8T_J2nB4hvrHOa75VEh8MZkkcln-jnnvdqLWmePoByMUny5STJP54kGR5AWKc9fSdLgnc483vES_V7CxSW7WUHv8aN8eronIqxUohDC1Buj9tNpnw3SXK9e7h8bkiCuZ7LG8GMsgWMvb57NjJ0TIw2qN2zL67hyLHD3m_OC8UXUrsll78MSar1Ykwu_XBuP55V4ntajEWb2Bs_jaHi8Ope1m9VkQhRr_zNHXvqst0_ADozjt73xZjcTZMSPhdk8WbD0fx2SGjKm7JgCJIX785SAC5k9R7n1X629OJV0xOJiD0sNcrHe8JRws-DXC592ncH1j9J_Izp92UiKKI19KRXzMSYCKC3lvJKOk21V-J-LPmFVXNHPyBzLHQmwuMgl3wX5G_nyZ9--n_72b_n51_66ed_9rmf_PT3f-q7P_7eH3_nh19_-tX3f__2bx__9N__7Wf_9rP_D4GnvriHpQAA">open interactively in the visualizer ↗</a></summary>

```text
GroveDBProofV1 {
  LayerProof {
    proof: Merk(
      0: Push(Hash(HASH[bd291f29893fb6f6d6201087746ca1f23a178dd08e1346cb6c127e91ae3623b3]))
      1: Push(KVValueHash(@, Tree(4ed22624752972af97fb71abf4067b23e6d296a61a02f35b2098819fde39d289), HASH[4a5a28cb1b40226aa35b2f0d502767df13268bdf4678627dbfde26a557acdf73]))
      2: Parent
      3: Push(Hash(HASH[19c924989e473a90d0848277d0b1498ccc8db3dc870cbc130e773f3d79ea5b71]))
      4: Child)
    lower_layers: {
      @ => {
        LayerProof {
          proof: Merk(
            0: Push(KVValueHash(0x4ed22624752972af97fb71abf4067b23e6d296a61a02f35b2098819fde39d289, Tree(01), HASH[5b90e1e952b7eef903cc9db2d9098e334a37f7e08cade52c6b2ea3bf4b56b645])))
          lower_layers: {
            0x4ed22624752972af97fb71abf4067b23e6d296a61a02f35b2098819fde39d289 => {
              LayerProof {
                proof: Merk(
                  0: Push(Hash(HASH[49e7191075272395ed72cf03e973987ede6e4945e08574fe77d725f4ce7ecdf8]))
                  1: Push(KVValueHash(0x01, Tree(776964676574), HASH[5d9a0fad8a3f32560f8e8950c1e84a7feabaab21b79bc72fec4482442844e2ef]))
                  2: Parent)
                lower_layers: {
                  0x01 => {
                    LayerProof {
                      proof: Merk(
                        0: Push(KVValueHash(widget, Tree(6272616e64), HASH[6c505f53f2ebf3de030cc2aca463d4b429aeb320a9fadb8ae68bb7903a22bb68])))
                      lower_layers: {
                        widget => {
                          LayerProof {
                            proof: Merk(
                              0: Push(Hash(HASH[9862894b16a0792688fdcf64edcb2ceade5c8b234649bfc6cfc6426869b0e9d9]))
                              1: Push(KVHash(HASH[a29ee8f206a253362b6da4fcacf8643ee8e5925cd979fcd449e5906f0f9f8be3]))
                              2: Parent
                              3: Push(KVValueHash(color, ProvableCountTree(636f6c6f725f3030303030353131, 100000), HASH[79569d595db75bbf2e9dca93a15c90b7eecf7b299632668ec410e2076d27f71c]))
                              4: Child)
                            lower_layers: {
                              color => {
                                LayerProof {
                                  proof: Merk(
                                    ... 18 boundary-descent ops walking the binary tree from
                                    root (color_00000511) leftward to the cut point ...
                                    18: Push(KVDigestCount(color_00000500, HASH[47b0ade5...], 100))
                                       // op 18: BOUNDARY (excluded by strict `>`)
                                    19: Push(KVValueHashFeatureTypeWithChildHash(color_00000501,
                                       CountTree(00, 100, flags: [0, 0, 0]),
                                       HASH[9146433eb6d43db2f109f5f7714146624bd646b27c7310f3c2cad7155eb7c741],
                                       ProvableCountedMerkNode(300),
                                       HASH[c285efb8724a488de916ce8301b06c197fc687b5b9b83a04bf3a026f1098d17a]))
                                       // op 19: TARGET 1
                                    20: Parent
                                    21: Push(KVValueHashFeatureTypeWithChildHash(color_00000502, CountTree(00, 100, ...)))
                                       // op 21: TARGET 2
                                    ... 98 more KVValueHashFeatureTypeWithChildHash targets
                                    (color_00000503 ... color_00000600), each emitting
                                    `CountTree count=100` plus its merk feature/child-hash glue,
                                    interleaved with Parent/Child ops walking the binary tree
                                    in lex-asc order. Every target shares the same shape:
                                    Push(KVValueHashFeatureTypeWithChildHash(
                                      color_XXXXXXXX,
                                      CountTree(00, 100, flags: [0, 0, 0]),
                                      HASH[...],
                                      ProvableCountedMerkNode(running_count_at_this_node),
                                      HASH[...]
                                    )) ...
                                    220: Push(KVValueHashFeatureTypeWithChildHash(color_00000600,
                                       CountTree(00, 100, ...))) // op 220: TARGET 100 (LAST)
                                    221..244: closing boundary ops — KVHashCount running
                                    counts (300, 700, 6300, 25500, 48800) and Hash subtrees
                                    proving the still-out-of-range portion to the right of
                                    color_00000600 covers the remainder of the merk root.)
                                }
                              }
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }
    }
  }
}
```

That schematic gives the shape; the bench's `[gproof]` output (run `cargo bench --bench document_count_worst_case` and grep `[gproof] G4`) has all 245 ops verbatim. The compression in the chapter just elides the 100 `KVValueHashFeatureTypeWithChildHash` targets since they share the same structural template — only the key name, the leaf kv-hash, the running count, and the child-hash differ.

**Why so many targets?** Because `GroupByRange` *must* enumerate every in-range key with its `CountTree` value — the SDK needs each individual key→count pair, which the aggregate-style `HashWithCount` commitment hides. So the prover walks the merk binary tree's in-order traversal across the in-range portion (here, left-to-right starting just past `color_00000500`) and emits one `KVValueHashFeatureTypeWithChildHash` per distinct color it visits, until the response-size limit is reached.

</details>

```mermaid
flowchart TB
  WD["@/contract_id/0x01/widget"]:::tree
  WD ==> CO["color: ProvableCountTree count=100000"]:::path
  CO -.-> C500["color_00000500 (boundary, excluded)"]:::faded
  CO ==> C501["color_00000501: CountTree count=100"]:::target
  CO ==> CMore["color_00000502 ... color_00000600<br/>(98 more in-range targets,<br/>each CountTree count=100)"]:::target
  CO ==> C600["color_00000600: CountTree count=100"]:::target
  CO -.-> CRest["color_00000601 ... color_00000999<br/>(beyond limit — opaque)"]:::faded

  SDK["Verifier returns Entries(100 groups):<br/>(&quot;color_00000501&quot;, 100),<br/>(&quot;color_00000502&quot;, 100),<br/>... (&quot;color_00000600&quot;, 100)"]:::sdk
  C501 -.-> SDK
  CMore -.-> SDK
  C600 -.-> SDK

  classDef tree fill:#21262d,color:#c9d1d9,stroke:#1f6feb,stroke-width:2px;
  classDef path fill:#d29922,color:#0d1117,stroke:#1f6feb,stroke-width:2px;
  classDef faded fill:#21262d,color:#6e7681,stroke:#484f58;
  classDef target fill:#39c5cf,color:#0d1117,stroke:#39c5cf,stroke-width:3px;
  classDef sdk fill:#21262d,color:#39c5cf,stroke:#39c5cf,stroke-width:2px,stroke-dasharray: 4 2;

  linkStyle 0 stroke:#1f6feb,stroke-width:3px;
  linkStyle 2 stroke:#1f6feb,stroke-width:3px;
  linkStyle 3 stroke:#1f6feb,stroke-width:3px;
  linkStyle 4 stroke:#1f6feb,stroke-width:3px;
```

### Diagram: per-layer merk-tree structure (Layer 5+)

L5 is identical to [Q3](./count-index-examples.md#query-3--equal-on-a-rangecountable-property-bycolor)'s / [Q6](./count-index-examples.md#query-6--in-on-bycolor-rangecountable)'s L5 (color queried under an opaque kv root in the widget doctype tree). L6 is the structural novelty: 245 merk ops, of which 100 are full `KVValueHashFeatureTypeWithChildHash` targets and the remaining 145 are boundary-walk glue (KVDigestCount / KVHashCount / HashWithCount / Hash + Parent/Child).

```mermaid
flowchart TB
  subgraph L5["Layer 5 — widget doctype merk-tree (proof view for `color`)"]
    direction TB
    L5_root["KVHash[a29e...]<br/>(opaque kv root)"]:::sibling
    L5_left["HASH[9862...]"]:::sibling
    L5_q["<b>color</b><br/>kv_hash=HASH[7956...]<br/>value: ProvableCountTree count=100000"]:::queried
    L5_root --> L5_left
    L5_root --> L5_q
  end

  subgraph L6["Layer 6 — byColor ProvableCountTree merk-tree (100 in-range targets)"]
    direction TB
    L6_boundary_l["Left boundary descent (18 ops):<br/>walks from merk root color_00000511<br/>through KVHashCount running counts<br/>(51100, 25500, 12700, 6300, 3100, 700)<br/>down to color_00000500"]:::sibling
    L6_cut["op 18: KVDigestCount(color_00000500, ..., 100)<br/>(boundary — excluded by strict `>`)"]:::boundary
    L6_targets["ops 19..220: 100 in-range targets<br/>color_00000501 (count=100), color_00000502 (100),<br/>color_00000503 (100), ... color_00000600 (100)<br/>each as KVValueHashFeatureTypeWithChildHash<br/>with ProvableCountedMerkNode(subtree_count)<br/>interleaved with Parent/Child glue"]:::target
    L6_boundary_r["Right closing boundary (24 ops):<br/>KVHashCount running counts<br/>(300, 700, 6300, 25500, 48800)<br/>+ Hash subtree commitments<br/>covering color_00000601 ... color_00000999"]:::sibling

    L6_boundary_l --> L6_cut
    L6_cut --> L6_targets
    L6_targets --> L6_boundary_r
  end

  L5_q -. "ProvableCountTree(merk_root[byColor])" .-> L6_boundary_l

  classDef queried fill:#1f6feb,color:#fff,stroke:#1f6feb,stroke-width:2px;
  classDef sibling fill:#6e7681,color:#fff,stroke:#6e7681;
  classDef target fill:#39c5cf,color:#0d1117,stroke:#39c5cf,stroke-width:3px;
  classDef boundary fill:#d29922,color:#0d1117,stroke:#d29922,stroke-width:2px,stroke-dasharray: 6 3;
```

Three things this diagram makes explicit:

1. **The cut is named.** `op 18: KVDigestCount(color_00000500, ..., 100)` exposes the key at the boundary so the verifier knows the cut sits exactly between `color_00000500` (excluded) and `color_00000501` (first in-range). Without that named op, a malicious prover could shift the cut and the verifier wouldn't know.
2. **Targets carry their own count, not a running total.** Unlike Q7's boundary commitments (where `ProvableCountedMerkNode(N)` carried a *subtree* count), G4's targets are individual keys with `CountTree(00, 100, ...)` — the `count_value_or_default = 100` IS the per-key count, not a subtree aggregate. The `ProvableCountedMerkNode(N)` on the merk feature still carries the subtree count (e.g. `300` for `color_00000501`'s subtree), but G4's verifier reads `count_value_or_default` directly from the CountTree element.
3. **The right closing boundary doesn't enumerate the rest.** Once the limit is hit at `color_00000600`, the proof commits the remaining ~399 in-range colors as opaque subtree hashes (`KVHashCount` + `Hash` ops). The SDK returns only the 100 visible groups; the remainder are provably present but not enumerated. This is the limit's whole point — bound response size without sacrificing soundness on the visible groups.

## G5 — Compound `In` + Range, Grouped By `brand, color`

```text
select   = COUNT
where    = brand IN ["brand_000", "brand_001"] AND color > "color_00000500"
group_by = [brand, color]
prove    = true
```

**Path query** (outer In on byBrand fans out to per-brand `distinct_count_path_query` on byBrandColor's color terminator):

```text
outer path:         ["@", contract_id, 0x01, "widget", "brand"]
outer query items:  [Key("brand_000"), Key("brand_001")]
subquery_path:      ["color"]
subquery items:     [RangeAfter("color_00000500"..)]
subquery limit:     100 (shared across both brands)
```

**Verified payload:**

```text
Entries(100 groups, sum = 100)
```

Two brands × 50 in-range colors per brand = 100 distinct `(brand, color)` groups visible in the proof. Each `(brand_X, color_Y)` pair has exactly 1 document by the fixture's deterministic schedule.

**Proof size:** 11 554 B. **Mode:** `CountMode::GroupByCompound`.

This is the most general group-by shape supported on this contract: outer `In` fan-out × inner `GroupByRange` walk. Structurally it combines [G3](#g3--compound-in--equal-grouped-by-brand)'s two-branch descent with [G4](#g4--range-on-bycolor-grouped-by-color)'s in-range enumeration per branch. Proof bytes ≈ shared upper-layer descent + 2 × per-brand byBrandColor distinct-walk. The bench's `group_by_compound_in_range_proof_limit_100` benchmark uses the same shape with `|IN| = 100` brands instead of 2 — yielding 17 256 B at the much higher fan-out.

**Proof display:**

<details>
<summary>Expand to see the structured proof (8 layers — same descent skeleton as G3, but each brand's L8 enumerates 50 in-range colors instead of one point-lookup target) — or <a href="https://dashpay.github.io/grovedb-proof-visualizer-widget/#f=text&d=H4sIAAhCBmoC_7V9W6-lyW3du35FP0qAHqrIYpElIIETG4mBJIYBG8qDIRh1tYUMNMZYsmME_u9Za_dMz6Uvs3edZHSZ7nPOPvvbLHJxLRaL9V-_-fpf9l_857_-5uuvz2_zu__zi3fv_nv_t_3N4wuPv75790_882_e_Y_9zf_65eML796l37z76z_98z_-8i87_-8__c1f_t1Y0vKRFk3PqKeuKimncC91dnxDe_ZYK8XOii-NOrP4brlvraJDf_erX337u_O3v_u__fa3_as_7cdb_Nmv3_3tN3v_suwlUqW4SXPpp_kZnvs4JVUforviMWqvuSc5akNSi8jtrK1tSbRf_frd42lLty4xRx4l4Rf2zp89aVkSr75OVqkx1inVo4qvgd-AHzPzPtfxHzyt4Gn7N_sPf_z27_qRZXKbTQoMs4trbwk2KCHuK42ML885Yw1dMzzNMbOm7a5Hl7fdDZ_u-_cqv3n35__4-6_W-79_9fW_7m_-_iuu1j__5tulevfuz979h__44S-fWMz3_3xiSX-8sD80fvrfbzX7t6uX8nf2t9HSzruZDN_7tKRztjVkNbxyq5aufnynmH1tk1mH7K54v2F11GKwya9-8NyfNsW3n-jNT_9ji37Rrj9j3c8FT2nbc8sJT-iizfZymSfpbq4tfK9dd2nFYBDzcuAg-AE7ZW7f8Mf43Y-s8d0_-ZNrmfK3q-FeW4WDV_zKD-uyWk-nr-jwQLGaTuxolmbeUbqf3UfvQ_LwNqbL2bPAl0uRKGXLPp9-kA8x8vE3v7Ry361fyp9agSfW4anV-Lzf_-vv1z_sP35rLaCA1Fx3_WCrOi3ZMT2yB8J1J01zSp-9VF1lFGl9D5XUGww6om8gCuyWtIuMUeMnPvyaVd7_8_4JP2-dp230gqU-58MNQBmtjFx78gb4jLPmqQi-OWRuxvEMRFuppY0z68T_Cn6stpF2W-13n7XGl_x5fNP_sD5aIjuatKp-WKoYtfnqra26u6ceukfJ3sqa3UfXpgE47i3OOFy1ueDybeiwlc6Rn3-2n6aBz_3zcXqoU6tL201H7pqRGg_CbkvArSpcKm08c7E1UgAZq-UMRxPkzzYRjFF__tl-nDY-98-zPvf-n4fdf87xXnS_l53w86H7eLy_Tyn9-t2ff_2nP_zxvX8oeMmsx-XX7_Lje-er_g_4sH-HP_K_v_vOXxoWnRymdUVGyrlZC8kSSM7VXQoy0RIfbg7ykurRGSWlVusGh1AfP78mP-PRePR89-glCvhWPqPsGacAp3c-Hv2RY3dClkX-gF9ZxtO3M1JGqhFzELbRqsmzj_6sw3_O7ZHZU1sdtCdJ930An6ZIaKWv1jLIGPLhUaCIhxvpwDAASMJ6gDOt_rSJv3P-p37YPqzH9w-6yRoG8GJHBmKpTWnHsyQvq9vx2ZJU21n2Auwf6yfTc1LaSJvt5GcftL5kUP_IoJaOgjeulNfuAc7ZwDGb17LOCZnI1imN0gSf49jEz0omszUF-wbmnGefM14xaPuEQSfckYYB3MGKZYDaFfVAGIHoSQb7y9oE5KsAwgOER8bKp5cxd55r-tPBlV6yaM4f07Iea-aovSF1zYLHNTnD8hqzJOCAHwMmDxvwYjz0bPgI8I4A1edHqE8_qbxi06yfMOqCv21QOYXZoDNWqSlhsdvIYCZLFU6QaturQW2EFiRk8MYK0mtnAbue9tJcXjOqfWRUP3llB9YoBNdYpaTV5zHwy5zEwCahgVZkZDyEvyHXZZi_Gz5dnjtmPP2k9SWj-ieMqnuVwFsj5oH1pS6VVsLBV6AeokdePm3sqRZTvYDFI3uvZnXrGDD3048arxm1fWTU3SGizjRNJQXYzJgquaRWcoEphwD8UwNZyMgIpTcx_E8E1hXvB0TsadRPrxhV8ieMeqA7QL2gaRxULbfiCfwmtM9c-gDSrgE9nkrNG1_ZyyBBbAG8PAMP-tM4Ja9lKPk4RQ09c47uDc-BPxnUGGhBJzm0LAJscDOIx52Lj31qI4OskaXC4qM9_6TPEbQbmvYDskY29Bxhu6Jtl-Ttc2pi1WSjDI9dIXiXu3revUJIKbBLt8VGkLW6XGdZiiQH_DAwsFy1WDydJD7Pw-bXX339za_f_dXXf3jQsL1-CWP8Sx9f7U_Rsofi-O4_hmSQP0vUvmNqcKkJhlCgvw8oUKQoyIxtsHoWPpEYuzSAT09VjuYGxa16cllLBpS4vPYRv6DB_1-62vt_HrZ7xdne4HJvcrzPud8pG8yOCqxvpHNkflC6lWuGhiRbbRBtoKozdSMPKEiqkZBhe4VQgFO-tjY_dUI-x8PJvqXKyaCRAdYZb7vhGSegbQfeNR8oFE_DhPWGEyY2IYDP5EtITQ7oc_rdr99BNV480WsI-mW-PzQhb_YJAjUsxVz4V1l7HmmdlDlW9ZyREGLuDqFe60il7zwqgszTurFo-ZxFfRdIiS2lIE0KpFEIaDPSd5xIaQFvsNhAlgBNPjbARpODLE-z0s7pCouK2cUT2bVF60cWlXHwmCwN1ulknIBB7T7DkPKbZPCmNCcS1qyjzg5QWtmS7BhtAbPSjUX9sxZd8Pyew8pcm-pjw0BzxWCpJTQq9Nym0IAuPfPA1JLww6MZ6Edzg0Wz-MUTxbVFP2ZRycCUEWqgnwLCDzF_ZCjcwPdYDlY66bC1SLZhlfV8Owg76AMF6se5ivr02bCvM5Gi1dW9Qrl10E9AC7JCjgRVX6XqAjgNBLtYk178zLQMEd932x0mrXrzRPnapBQyP7HplIVQQxAHvA7pLKan00WKKvGsd1DB6R3s3tuxksDwCmU24q-mHvPKpvo5m2rPCQgNaWwbermd0WKccgCWNY8JLowHg2w6e4DebekKKodVnj0D9KPCpnqDpC_Kpp-RUPBNJB6gPCskPeeEXNBmiKuEH8Dpzmd6qf6QATNV7mYIuBMki9vzbP-ncuozoT966qLeaisgxTlEVll4jg3JAdiXBJzFciJXSeolEEnGitrmjuGSgE1vAv-h2C5NGh8-y1_8_h_2P__x_ad5sBbS5IRkmz6UjhVg2iGii6xek-ipip-IlfKEeD77gPsPpCja3uEysiFk884y8l70mCuHaR9T0v-y-x__9M3-23_7p_0_f__Hf3xIhu-p6ncP_qO6IT9G_kKxMHNnMuUBBwJzlhGe65gzGzCNOzvcEAOsgH9A2HbI8GLI4LlanjWniQ_3I1K8FwnYX3299i8_1N6njnIK2O2u3FxDcqhrpXG2Q_M5_g3RmqwCwCCnz4kORPWaweG1CkDhxlklXfuG5FvDywuGBzsD3ZE2QczKybE2HAvM0S0gecswndtlNmTu5BOaUw5gdJUCigJcyl8w_Ic91rIH0rzCxEj4thGhsSBStbL6pxM8DFQSrCxk4h0DDGcHcg3oQzqj5CvGJS9Vsn6qvy9fWW5XTF9YsYMk1sRADwBruQagDug2Ee61YhWrbuh_QG_foDZnLtAw2PKAoHVkGfEvrZh92KfCy0YrIJqs5NVGTAUhbRAWeySWHhKJVumAHj8D8XggiLGoYUhP64qAyD0nlXpr-fKC5Tu4-k5D904rIXtN7hK0ZFYXePuUtgJh08wRSWv42nuCZlYA2zZ8356KlQUGlRbzaDoZvxrruqbk5hkCS0ZxTe594budQQMGs8C6WfbPINZ3IOW31rMXrLdP6c3goIBv5CeD4nEonchz-ZrI3GdZUwA0PrueCSMfMN-8Z92uBt31BMQv4tY8W8_CL4OaA3bvDc7RweyQNSWB2dXuZkigeMOeR-ouJeN1yccVH5F74i_XubW-klsn1JAAwYEScJsK6_SeonOrsOFX2ShYlgOFBINAyo3umvECCA6pZ_Sn3DaNNkrqBHCTtUbBgkLr9YiD8ADwT4Gkdm0tgElYpCZVmteDL5S7MoWmW6DW69TqrwD1qTWNCgCFIMrgv71GmlwF8LOhYME2-5zCGrh2i4FVCrUjEbt0q1-wu3_AaQOlnhsqsMGQisUDMB2DCmwpabLO0ovvYmoBgtiX1-Kiaze1ZlKu7P6GaozeGj5eMLxYHhVCmZt7A_81W0m9W1bkqC0JvpiBrQVrAyyKEFh7qnnvrU_IrmccfoCdak-NeNUa2HcZrqtU0ZkRSH2BJ6XeHs03HZnSFLQzQ49UrK7vK5zWa37RXrBecENZqT8BDXlZHXuCEo4kIBqZBMNN4MOpUqXaySztzFKBJXNoeYqKl5Kzr9hmAtMXIPxpSbZBvuTYG4QQAAJeHkdTiT6QSutjB7k1gItf4bTe0wu9pRc5vWD4CScRE0ma2YCIP4NwQ2Dr7DIzZND0keFhSIfh-L6Z4U9rAXMnFuEpepGXegEcI-MmrCvYHBCjTpDGE9yDcyAJaDfiTcHmommDHosKwJBlY8mV4f0ap-P6le32leU6q5R8_cprsVKuxUop16-022h4pSIAWmgAF_EBkS5Q5hsKtI1hR4-N00PWHrETMhyiYyLD9rI2-75XIUv_UjTgYT7g-GQzawerXyA8wh0BpGrPIEkjJd9gRLvvNaBgoWwUP1JYy85zCJvkxtVuQL1GonJL1fMrRYEORQko8LSlI0VW1gIsOtRGDm6Rh9RC4EZ-wN_rIpZjNRbsBHnY4hkkakyYc62qRfCA3FE8E5T96BwWGhO_3tlwlSFfuy6bLU9m9lCoX8tXho9b670i0EUVBPoI7MEGHBBAGAkq7SSCbs4DJJyycPd8tkjNs7Mo6jOvXgTa-YkECo5tLJJ43RsqCS-LhWRRQTGhQ6E4j-yTa4YsT825vxSjOtdSK2TRVTG7tGu3tXRr-Ff0OXTOaaMe3Sa5AgQcX9A2BOwXirOV7FmssH8KdDnYwm6jr8Jmft1fNPwHt405tdWD1yTwuDVHVMljAJSSxJE5_ECVrz0T1qWtBu0OxgLmXUrHN6-Yi10nFpNbu7-i7OFqArM4YvPhyETgAzO1lQUYcSAtYwyeYmEZhDIwHD6a4MGaEA7PCB2s1xwg8xNo0bdskPvsY4D_ZARWrXOJDhDTSM4CSktdp1krxzVJa3Zld713-Fuqnl9R9rXuXLsVKMaVI8aOxBZaSwDkVnvSBhI3HH6auFnYRJ0Fq1LYG6ZfVJgfHD4t6Jq9iwpETM2Iq8kNE2hNqMq0a_Q-AXfICT4nO7wiF52xADMTOXVfGf6aYbyiz9vUwvb7PBPYA2QFNDdct2vArpA33r2Y9bSRk_pMcbyCfyj0XDPolKeEDhTnZpk7G7RT2fjVwY5nMO4WyGiVlZmx4aF7QxwO1sVSQcIF2Y6U9xW9sHt6Ydf04hV9zn1s2fkAKWMBAVKH9EaOnSyYZASwSN4lKXta4L0xY_S-eTxhgqhVf8Zt8TtG8eYJAO0d7puqJIirdfJ-bJgpgmKD3tnuYID4KxZ1s4GrsHk2rgx_LVfsWq7U68z6Sk3gZKSvTBazPC2QMLBfkabckEbk2wZvBvILaFw-Oa3R1ql7tok_YBn6U3sOCL3FnWME4HI5px0Hv1mIhQQ0gdYFciXTsooMUNNA9CgCFJmlZyl3sVLvWwzqbW6VV4oCYzq4d40D_WMm84HrViF1DDEyodtXKnWxRwwxJEsBN1gjQPQ5e_tTxVvFitWN4GNtEEiH9_FUBl9-QFURig6pBeSbgvjklgZCMreUO5ZXz1URsd5WAuUVEclmbUGSzF5GXg5eiATYyzQ8ObzZ1hFjA_BQbqtst4AgmfBfAVUEq3kG4itP5OYKpg8EUVE45eQBM3cH2TltWwH9DOglnvyC82YQfTxYcqwsourKevddHPU2t8orCjJBIcJd4DBlVTlQiL7q2r6h9opMn0fZux7BnWW4K8-UFUh9eNNQAMBTew4NiXPNHgvsmyfS8vBFCCH9h3qkrEwnzTPZEsWWXEDFCXz5NED_1R5lrddAfZta5RXtOTdsykQGh58L3FqoBVsvruxrb5u7x5EQ8aUotE8fsB5A_UCntPnF0vf3ew6N3V-ld2LTHAJzcgui9xwdASV7jIdSl-gIv7lzG1EI1I4lOnLn8PebbPV2k01e0Z6l84ybbaQ-EA_4OPIhHK5B-49jyaMLtwegWgLiCMgDdjPZSF3xyipPOfw4RaByYE7uLUsLPy2yHMjRiUzpBwijW72nnuYEHB1kggCpeRz_bVd9FH7LL-SlveFg51IDvwC1aGwSHWMAJC0DLGuHqtDY0O-rZZ4S2pNmzhvsEYiqNp7BaXjjiTVnr-VU9w2RXlOFEvJQaHMVIENXPHeUFUyJgzwf754H_liuhIzf0wu_phevKEhSiEpVWOQRzuztNIN58mZbaRegc9RzxtxYFah5KHzgeZkJK1TaU1tlAcnUAMkAmg406JMbn5IecyW84rd7PqVCrgOcqqiVWWcemftW0PfWrwx_XRf367q42_Urb_eX5BW9yw2jAkIN1iEQmVYazJuBGjLUZs-95KoC1O4hcoBNIENceWW0aPlStUC_T8p98vxZNlaCAxk91wxCySJuAI_Wysj8hqyNyGJlCJBGFbYAlNZzuUoOft-A6bcVYXlF8LIcJc3BnAcLwUn3fHTyTLZsGkvngqCyjDVYkXtDBi2g2Lb18HTqU1F2kkOELagmhdFJskY_ZLnWgHkxOwCPa86eUHaClTM6lPFsa3fg4Z3hr1PrS-KzK3TPaqBuTBApg22v5NkNAJFYQaACymrdegx4rDr4DAzRuZ_8Zb_9UC4oPAYAaQC7JeZjGCxgnwlV0HPuoDFis8wDWh8AL-F5O6ycsn6AJ7mxXtz3hsZtA4u-oj2Bzym3ZUBrFQdOp1kGaz21NYRzW4-iX2gKw2-ezvZZmN8Z26fs_YzbeitYRC9rbHhtUjm9KtNt4h5QaxINC6-QoOPorHtXbvml2pBE9s53hr_ebo1b1aqvqNaeEat11t4SOHOGLu2qkg-cvYtMMGuLHXMlwZKU3YWg4UBYmD48nqqnL25haFEeF4aQz_BkiH17sEddUKuIt2a1ZDbongrlBunka3BaCiDkqn8l7lVr3KpWfUm1Hqh0wAqLXICSpqDwzQHBJ9uZ9QSkfMK6eK2A18SjTge6Hy8L95GfcngxrWvWmkDZ26MvKCY0APTC4-yxsJKlAO221zSWcJR7HDMbKGpD4rwy_C2_0Fe050gVNpsLbgXRP5HAKgQIj9jhk52lssZxfOZSIEJbcBcC4V7ZgGhtmD1VT-dpVAj9LCDlyKgg59Ctc-SYLPaKnYie8kRKU2Q_-HXNpXNIBpTZvju8F_f0Im7phb6078mWbBFQaE0Hwn-swC8ILAG7_HbhIDmsgYexiwFZbKXIBiDxBNGYntKee_CHvbJdH96bAUmxN4Klsw-9VmSJUSH1K0eWwX2NvRjxSKUVIvSqRhjXVfF23cTTrjOrvdTDbwvpDewBbptzNNbjey8tVAZS4PE6YsTjO5tnTzUtwx8rq2Wt21P1dMYflmCvXRRku2QrqzsWA6nauVfoFALK02aq4N5RBpAPYq-lJGNfbVW3-97Qdp1bXxG8ZL72sESrChMvlra1CdvfWnNABRjjWZY1kij0ymoiCaJldW7UPbX3BDTPCSat7HJH6HmLOlYJWLxwGljDNxGKyDLbpo4zgX9xMnfB3NcdxLfbDWd9SUK2fNIYZ0S1TWW43aDywJln8Q7KwhNCFYStkzemAa8TD_AGspXQp3r4DSYvus_x3UCxR2ulg_Rt5L8DYrQr0qLkrDN4CgWMM5CEkXkK-L83vWolb_e9oe06t76iIGtKOlqZls5s3MKH1-dOlpCA-sASLZ60p-0tuWFVeDS_J2TjWeoY-lxv6DgFPw-iB7metk6kD3DRzcMTGcudBWs8kG1LBz2cCmFfOGKVkyZiXeXWdt0b2q5T6yvac7AmdXYNKBpOlC07QwJxbCoimX2yYhwaoK1XiM-UzmDiQyj7PJBH5RkqnhU6hkPboNrBabBm-I21ad-Lv6lyswJejhUxCNB6kN9BHREfBcpgjjuHv-_lyum2JFxe2_iEA1bhNDb63eEG8IFDIzmGGUC8nNzmmGZpwt0TIMEr8BSvmiv2U0DN8m6qmYNSwLMLMMSnrayrYUnS2HgTqRusn9Vi_GjDm5uO9Zgn4Ofu-HK6ZRjlJQ2pS2aUx1i1vNmzz4m10WeC5nNgh_Nwj57FyVYFBLHIprhPVnuBnHwGqXf2bDwrz3Nwm72NYH6yRHnYCpQbwL_x-xKHV60BHcvzAkrCeHqeSe_MJ2_w3FuKUV5RkQL_7NFy9cnZYjo4yzelOpEnV--5J54d4cl__AicFuLneIC3c9zgOE_RcQ5t1s4JT4a3gKNy6vX23KGGkKSR1bHETTNsXTgQ95DfNeTk7kggK9-Z_ro0jkxx_9J6_1K_f-lteikvtQqXrazQHjB05cgedi8iRBII1T57NPhS4t5Mh4qyecASzziPE-p6kGS-4Cr1B4RqlOYQwyWfKMjmIWzgAw70AA89POozehfOdI3B4eI9rdTL2F3AVe8GX6Q3JJh8nWBeUc2sIINTQQpB-WwO5GU590AXjdhHYXTlSTFuh0QCeEbhKENWg9ves8tTTe5HHsMDsJKmuXTI5oxYHZtnb1bfwu4lMKyRmseZxylEFBwbSQgk4C5M83WCeUXC1r2lg4loPVlm5-HDjQS54kAvAQA5ySHGo3HskLfjZ9mgbqD1a7jvp1preGT30UIPaMzI7LYECHkaT7RWzdOEE6m6rPeD-SChReaUlaWdmved-d6QYPJ1gnlFw_Lj7ZLzQugWJHjLxzmQVvdaW3lcZQJPtic50hpnWiC1c8yt7Qny8pQYYHF5IXFAY6jDmlqS8bT26IcFBIP71nNqRCwoslI4TBGpP_W18pp2N3cs3yeYfFsgLq_o31GbpQW6zgJkhtYFXDSkcu5gsivh7NkyzA_-6VDEDhBXKDfjQVTIsnhGDiByeCYyWlcF6tctXuqxWoaqCsGqCGeTdCDHKJln-KSIuQeeKJ9Lp69vcPrbzqbyigJuATmEz9jYTRYg_yM4KMGQwMB5VpqVZ3uFXSRFlhJ1AUOpwCPttDafqs2HDm_g-2BjiCoe5rVVYPHmxRo3VTuPFQgW3VPvFhsxlXqSEWWcWe5Mf800XhGy0KxJAY5pdMA1Hp_H3TJQVVNtZRRIms5ejBSLe2_A7XR0c1KSKsc4Pzd9oSxuJW_v0X3CRzdSXxt5ZfYDThlYoNVBLaA4Ejv9kPcg4WrirKJ0ab43EA25JRr2ipKNhbQoSJcFaMCj_711mL3XUmzV2lPkOFVDdUxosg5NBo0JGUsI789to8pBYg2AcMkdWitWzAka04BP2cwl67ayQng2bLdaTkPiWGB4YHxplKtzvVmuzyXl-_E8WW5zrL2in3eONtOuebkW0OapMGsFX87lyGO63ToD2KN2CufyH8kGQrnYvcq5PE9V6A9XOW0DwcmsDHP8dGLZp5XJY3qsO8WeBr3IY3zgqnPpAH3frZ_ocbdqb5gAJ7dZ1l4R0NAobTpoRZ8OGgmrB7ifwxaFt9IED2i0VgHEPAjqVdl8AHWbvKZWx1MzqczOzkwoyCgd78VCtY-1C9Bw9VzBYcynIwcg27Qa7ISrEM8dWJ8BXnemv60V2yuiMrFkBbHIQdELWQvOFOYBE80NRlgTTSVRQcxBntlnx8uNIDqCRcv6VOlnskA5x2Z3gAC0oGuMQ7zHoyUhgqP-yE5PPayZDhZMH72v2eqKu-N4Wd4waE9us6y9oikrsiondKeBYHYgLSjbMK1A4hn8s1RLYGocs0M8AZCkAr4MjIHiq0-VfiDJO7tgM8eTp-oQPGAgnRSdJ55iLix4Wytgdjt1L-g6pCCQUN4wZuPO9NdbsVmvk6y9dKZMe61u7J1TNvjD6yYFtiuISM5Dee1Z4rys4gUh3jnjXY9WMh55qmemR1lY4AAhClDy9GhAgOwZ4DcHb783L8mprOJV5QjRAvHP-1hAotKwywmibxiCqrc9xPaKHFUe2OjJ4WgOSOZhFzh4Ko2DtgqoDQj7oplB8SCe2uxIqhmk0yanTz0lRwH-yK7AG06_LYWtoyxWH2RQvlnvi50PyADIvYUzpXxvkPM1FxTT6Zemv2Yar2jKaZytDmYBt8wAU2m8MYfjg5TlDrJvYGrDnzgSusDP8gnQ9ZRnOHToU8ycl64UTmfdG5oUuAOX5Jjrph2agE2PYgcR18DcmTkyeKhudv2BksS6M98biIZeE41XNOVaoM0tZdrVARubB9J5sjxzZ3mPWsuugzeIDWIJcOXk910tY7Ft_alCCi_Lmm1ITF7zN2PNFGy5PduwpGzv3SxEgl_sBoLOWRFYZCQ7ab7mXQlQ72vm99N68v24nqy3ncT2ighW8J_MNpd8bMYuwmnv-Bs3Yjb4iea-Z7Iup0C7Do5R60jRtQPahFc9PdMB70uWudfOCZCrLmRonTXvCG0GoQvkIyVTAXgxFBuiPvN8RJOpxe6kWLnvJc7ltl5cX9rP7UDwMharigu6CVyEBYfAx2_UQXul6XU_zgLWVoH1kMZgUJBZSND61KwCMM5uiKks3RBwmwWJdSisCzjpwNofZ5RBFyPohgj01Eocb167zXlH6sttkq35pd2KVVqeE08O0FrZeCVNDuHk8gHlr9w3kspLSvK202vz7Zx0fng7XHmquVJ4By0yprGhJwveIVIdnNWR9Wy8D4_vgfqAvyM42BzuChCbUwfSh1-aT9_gubdNT_UVOTqsZ6huSB3AxbQ8z6jzcfL64IOXPJA84dtHJovyZQrHVXMmfhkcNfNU_SYBA9jMNOCpDU6JsGjLWuRUVkU2giTuPFSIJIFlIKkCapzoUMHs3L7rRCj3m7LlVsnWV5Tssmqrnbl4nICHG7XF0AEH30jIvXAz4rDzt3doKm_g27zIGXwPX0XCfYbUhxj7kTtii-R0rwcNA1JxdiSWtHUsRlMQsD5H5-jOhiRhuksTNk3cWf4NSvZ6iFV9RcmuDQ4W3HMgq-46nf4J6SOlQEm2Lv645KFMeKzDRuVYzvvRxsRDwc84PbBq8LoTXgumZUteHQlYpgfJkhxEw6REAMkFoPOKQGvg_auzzU_XXb243DKN-ooc7XPDKFId4LnT4EFWh90Arh2-tQWiJYAlq4FpQ7co6_CnF01QjLzt5Rm4htBZWjsnWmVe9WQyUu6P25rSlP64s9tlzTa5XXU2VO8Y4BjQNYW3nl-Zz95ANOyaaLw0FElhc96JpqKcoQnxua1PzsfdnKidMieCcfJ8psAqq6-TsVIHoDqqtadqMMmRC_HbAEZ6asX6Ilf2sXem8m8bMK28VSgm2YZoy6IIF7jDLFAQd6a_r5mb3r_0Ose-IoIR0j20Is55KhiG8zLBOFZPiT1LyHEJ2TQl3mrVgP9A5fS4daY2jp0tT5XbtS7J89EmdTRLE9ZHe2DxwLsbOFNmbXf0wTYs31s1cuM1kOc0AzLdrZq9IWCus-xL45jAmXmZHUeorQJZi7hoLVsY-DNP6Bi72Qen_XdkyAoMKMjEmstKw54LGG6d8sLKDIrTYxgPqoHFzEGzex7akrBVnDcFQ_aCYoZVRCSSMhjnuWsnuJ5mVV8RlbkWW8vVOGCx9srtf_hygsyBe6VR2ZQhJ80OCG7grBAkPFfNfiue63qq0_LklCSlDlRHop3VYCFImiquusMBab4Ha5Jw8KOeOO1eLHU9vIH3Em_iDZ57m2X9FU0pswzQDgG2-zjcaDZNk_enH-i6OSGO2qoJToXcGD2BOcSYdcGd6wClecZzkUY2JBSy7BCOSrbImvF27A3QcQpSS3K21wjUFtuDoxde7Y6VVvznDjTq9fmlXG-TrL8iR49DtNfJOVHz7NMECbFDCvPYTF4DJCQKtKhBHClsXlYzSZBMqmuC1fkzzDxJcO4rgSHYU4vUCqID0RXmCIMRZ0PUs2-7QZ0at2MX3s9Kl96i32101Dd0f13PtPJX5OiE18P16hxAmz7SoUpCRnxcszV4gQDIZU1Ijhmm8zAe3eDeBxLls3I0V4DGrjKJWwJaj_dj82MS8J0BjClrTtE9wGXW4IU9R-o6sZAyRi93cF1vmYa_oik5EjR3DRHfSdvJIAAlIcsFN9WOFY5I4gRflj3KOjyVDlpSenKOqGlPMfMDmBmQrIkXYK8acNwqClXDu4MjvHEMGUvNnG638IOcjqHm-Ch73A0fzPUNRKPeEg1_RVPyIm3ej5Br7AFrerDZqBtvtAPVOsYZCzxLMxOCmqITLGy2ijzqEnk8VUjBa3RAl3akYwfNwO_SAgInPOAOfYm_aQKlA_GTGVBnYTOwNq0mduHfmf6-Zl7va-b1fk_W7_OL3zf9XI80cnsJH3lYi1NbE4h8MvAkSw2eF4UdkyIgJ8VOSqOAnEnd6VjnrbVyort8cVNHvj-0BcIHarAe12GNQkfKPTdtla3QyibCwiNJnCpVd4XYdIOACedsSxl3VX5_Q63Ur9H1FfE9ezcrwhnD3tg5CGPnhM9tO00Ed9QCaoa8XTgC7RhLSrEWT4s6p2Y_dbzWCrSHQ_Z1KO2YC1Ayh62E5HNGTwB2SZxhNFIbwA-boHylmHRFjkl3vSd-uyHp_hJCjtoPPpQ-Bu1PbnLzUtfOuW7pUJUedviB_kCsQWJ33lJsxTlDq0V-qsqfx-Icl2YDPDnL-zvjN9tWZnhl2WoKT5qC-CLJV8hwM6THygH25e6YZ_Y39Bf7rYzzV1Qwr2SyBonlyBVIGFCgrfDkDMecnShpcV6ngvL2BAWcE6gmGAKvRwE_zU8VPI9ZgmibxUGIgUC9tOKVwBM9czh9wrKDHhdQEQfH8sTbaVc6wC9G053p7zPM9Xwnf0VA8zK7tdvi8Fre22wl9Zyc5xcFqBqQUwDaGcjZjW3fSNmwz86P3oQ20zNa4lTOneed6WqcOcL5fj6hGgL8udfaClU0NHyDagQ7Ucho_AUMbWgp9a5_5A0DnvL1hKd4RUBnuGFn7TjpcDYpbBbeeDm3MzUWnZsXoGdOcnZZ4_0IeM4HFJjPnoJrA5ODTF6pSSu58D7JAJpM3piAX7uc-MPxb_jvmUjH1bBMvA9Q9EDi3Zn-lmrES7fWCqcLc7JbyWy3W5SovCV7QFPYyadywqEg5ydYq0TacyUQXp6E4s3vz8B1f1TdQjkifmygjmzjmIoeXBE4Z9659N0QIZNn8q0vEGtOws0WXe4wI95ANOKWaIS8djx8nl6XhCvT1cYn54FxhYZy6b5GOFDcecSvwZ8eJ0EC2pW6b6-nms42BYEz2fIaSicw2OkSR9fEG2AVeZtfns53SdI5cntwTpZ1Hnm-22CJ-03ZuO_6idscG69ob6i-Ja0v-Gg9OnjD5_YBBQzabMK5Bm2ye7LEAlhQI3vy4OR-KJb55Rz7fZXfD9BezuY9TlgS4alL2Sv7gJwvcJv8GKSBJHx4Pwv8oswFJKyDN7XcKcB4Q600brNslJfqHjUFu81s8YROpJUAJvjYG7jSc-VxShBCL-NQPm-JAf7ppxTIIx1PVfmnsaM8QMQHtBeySmwIH1ZOK_s0A8R2ze7B7RRl6RRhheVq0FVYkcsG2XbbXxwv9Rcrd0N4i31M-EsDB1QFcwmW84efuaLvng5HcqQjoz_-bqWy1AD0fqqpPnkqTK7HclHQlRpIvmq7PGZocx5_5_FOPVKysF2xeGZFlEO4-uVB7faG_uJ2nWVfur_WOTAPbI9dNv0sfVhApoMpGG-IHONxe4HxEgMQtrQd2milLKFl-lNN9bMU7i7yAInWxK0_djwUmz2lOVcdpOg8UQH1Ol04s5vDzs6ulTeYtDvT3-_KXo97ipdusI0pQxNv-I4zA9L0VGNzfd8C9hHgbLzVL22QECiilmblZl4DReFt1U811YPzZVaxFTJLK-_v4h0TGx5dtc9HU_nqbNA007Nb5g4CIwMxONj1dmf5N9RKrwc-xWubsqXxUkJIEQ4pB_QU9olLkiG8AA9BUIwHnoCqja3xacdOSLCc9R5ynoPrnXjxdqlgsJl7XLzNCb_CeVWulNaU7SI2IHVPNh9dOWl-bltYq7ib39KumcZLmjLAuEvt4Ie1nrrZrVX1QLGzm0t4d8wq-ODw15049MEbSx9tpMQb4Z6q8sP-Obp3XoyaC3e2c4yZuOvYJ-ACaXbwXubiWBIbxu0Ttu0hOdQjl9MJ2huIRrslGu2l467IjXWujUQ2YITFO2qtBTdMW01dIQAVRHo99vR4Z5BDegqRBp6V5KlrbMH3F09_CEE6Vmze7lc52NJ8AY0462iGcsKejArNf4BKZTUoo72WXXWqSroumkvK9y-V-5fe7ke2lyZNMU9CwHMkXDTNu3DqB9x-JlkHeNQ771ocPG_VEId1shH8IATBM-fUp5rqwaACdGtDMLSMcNamnAKQtxryUAP4FR7lROIfAPcoOxqStpclGYA4_W7B7w-wSLqtF7eXVDAAmee9nbvfCB8fHBucMw-WC693dlJHnv_m3ZgHCnmBQALOl1Q75akGCCblPntHTBeDKlhjj6UVrHOwLeuAmOFr4PnsS8kcYlcXZDiEsrB4tO9Mf5tk2yt61G1C6ECyrH6IVRyl0U8BrbRzkhZwGN7uVConLAoITeX1lqlxkFkHL3mq3C69zwRLIaVONePFJbxDcPmGLTm52HbVya3UXTuE-Gb_E8_JgmV5voQqf4Pn3vYXt5fkaE8a8JcBKQptz7mnoHMNhBu0D7kepH5bgpISzjKGTYbDqXgJY-5V6nMbRakDnE7OfekZrDOCOFY2m8GHmYWqzJ5Pn1AReE9xB8XhjkpDykrp0vTXu7JyPfeqvaJkXQtAMu3sKYHxTO5YntFS4zkHnj4cBSHhdfDwH0imTraNgCKNs2fqT82wYXsVTykkhU8LB41sENh9Oi-3xLe85V7icQVk4sEcyTyimyd-torcbXRIvleykm-VbHtFyTY_vB8R2t7yqchVAEqWDdue7eylg2MY-2YfZgLsKvhQzFQ4KAQRoumpk7IAYyQR5GB494KK3Vl45_MuIKkSiLPONOmp5C6rr7RbaGb_mnSzXe5Mf800XjopCx8Zrkj0xqm0bjySAdiGI80idaW2gBPKmyqR7cZO5XiiZH0_His91brD27Ez78XhZXGT3bAiFbExoIdA4jtvRbc2pblv56KxhxwyChAj4-4SSslvIBrXg6_aK3J0Q6TbgsVtg4ANUIk1grvFyPBFePIaZtDNFlXudSwD6xpE1iENMfKUHK0cXAHhBL3WoBfYnIPf2AxxEMaLzn1AqR44eFrg9tDEBxQNpGNu3k_f70xf7-Ha7196nWNfEcGtsetYtYB3gxgFL02Zi4Mk9TEldCOPHu4ecWM_2FjM-dpgJ3sEhNhzM2x4zmItLlepckDJTwM_igJu05WHLnSPjpAsNSCXN4824636brz8iPT_atXuhz7J7dCnml467trKSeAqrbvqcXaeuBpvJebUuNVAviGQhoOMczs7H-5Np-LO6xTkuemSlScO9-MOO7aU2wDWT3xJFAmd10uxmh-eJTiNV9j6zKvpWxjLOvPq0KDI9xvTtNDj4X75nuk2ELaeEFLqC7AMTtdDDAoMjK6lVFjNa0jxnK-j4jxhPU9KpGNNK91Nr55I3uAM3yWux2q_Z40reH9ZjsV2GAQICBFkzqgFISSOxeJhCZbcR020LotmC8ZPx5AS7hxaricfyg9mMv1kQQYPmLUDssupKAIYHdk5VbbJhFzuCe4wHh_XjvC2CBZuALFZOZWssd_Srz5MfcOC-EcLsiDz2etcGIRg8XmuvWIb8go4C1hU4W30Xs4cGmD7ZrkPkJtdsSpQkncLEvcLcq8A9L7EpJ-NywJeBx6YDm_S7OsxizXNHAMfUqAy6wmgPjh4bwbrLqwBz8dxIxZ6yAlF9Sow9Q2BqR8HZqdEtCrIYGtJOtASDR6MgISu9kYZB4_f3zauJF7MZrw0rgQPYua7GpDeB6ba_Uvr5xbzrKKaCi8lYO-McdhtKSc7Ataqgr5K61vr46wcQn0UkACIr-oD4vpxYaeYXVniDTUFjY9WU3h1X0K05qa5n2mnQiUuXjxZdFORQrszP4QVJkvQb8Bu29GSgprfRbXeh2ZJn4XZEQbC04ITtoUTckF97WRIKohstrhGdO6ILzZSQmwDIHu3iiCEFjTGV4m4-TjlDYr3w9iK75cEyqdB9EPhcKekdvC47nVtqngeq2cYAXY5e0h1w-sGiAYkY4vMGc9X-89S9H5JvovN197233_x_-Nnn_3J537umZ_6-Z_5uZ_48ve_9N3Pf-9z3_n01z_11Y-_9tOv_PjvP_zb93_-7k_v_83___df_Psv_i-_r7THZrwAAA">open interactively in the visualizer ↗</a></summary>

```text
GroveDBProofV1 {
  LayerProof {
    proof: Merk(
      0: Push(Hash(HASH[bd291f29893fb6f6d6201087746ca1f23a178dd08e1346cb6c127e91ae3623b3]))
      1: Push(KVValueHash(@, Tree(4ed22624752972af97fb71abf4067b23e6d296a61a02f35b2098819fde39d289), HASH[4a5a28cb1b40226aa35b2f0d502767df13268bdf4678627dbfde26a557acdf73]))
      2: Parent
      3: Push(Hash(HASH[19c924989e473a90d0848277d0b1498ccc8db3dc870cbc130e773f3d79ea5b71]))
      4: Child)
    lower_layers: {
      @ => { LayerProof { ... contract_id descent ... } }
      // L2..L4 identical to G3 / Q4's first three subgroves
    }
  }
  // L5 widget doctype merk tree: same as G3 — `brand` queried, opaque siblings 9862 / 6c36
  // L6 byBrand merk tree: two KVValueHash targets (brand_000 + brand_001), 25 boundary ops
  // L7a brand_000's value tree: single key `color` with NonCounted(ProvableCountTree(...))
  //   L8a byBrandColor's color subtree (under brand_000):
  //     proof: Merk(
  //       ... 18 boundary-descent ops walking from the merk root down to color_00000500 ...
  //       18: Push(KVDigestCount(color_00000500, HASH[...], 1))     // BOUNDARY, excluded
  //       19: Push(KVValueHashFeatureTypeWithChildHash(color_00000501,
  //              CountTree(00, 1, flags: [0, 0, 0]),
  //              HASH[4192...], ProvableCountedMerkNode(3), HASH[c3b4...])) // TARGET (brand_000, color_00000501)
  //       21: Push(KVValueHashFeatureTypeWithChildHash(color_00000502, CountTree(00, 1, ...))) // TARGET 2
  //       24: Push(KVValueHashFeatureTypeWithChildHash(color_00000503, CountTree(00, 1, ...))) // TARGET 3
  //       ... 47 more KVValueHashFeatureTypeWithChildHash targets, each CountTree(00, 1, ...)
  //           — color_00000504 ... color_00000550 (50 per-brand_000 targets total) ...
  //       ... closing boundary ops covering color_00000551 ... color_00000999 for brand_000
  //     )
  //   end L8a
  // end L7a
  // L7b brand_001's value tree: identical structure to L7a, single key `color`
  //   L8b byBrandColor's color subtree (under brand_001):
  //     proof: Merk(
  //       ... 18 boundary-descent ops (different hashes — different brand's subtree) ...
  //       18: Push(KVDigestCount(color_00000500, HASH[...], 1))
  //       19..220: 50 in-range KVValueHashFeatureTypeWithChildHash(color_X, CountTree(00, 1, ...)) targets
  //                + interleaved Parent/Child glue + closing boundary ops
  //     )
  //   end L8b
  // end L7b
  // end L6
}
```

The 344-line verbatim is available via the bench's `[gproof] G5` output. The schematic compresses the 50 per-brand `KVValueHashFeatureTypeWithChildHash` targets at L8a / L8b — they all share the same template (`CountTree(00, 1, ...)` since each `(brand, color)` pair has count=1), differing only in key, leaf kv-hash, running count, and child-hash. Once you've seen [G3's L8 structure](#g3--compound-in--equal-grouped-by-brand) (single target) and [G4's L6 structure](#g4--range-on-bycolor-grouped-by-color) (100 in-range targets at the doctype level), G5 is precisely the product: two parallel G3-shaped descents that each terminate in a G4-shaped distinct-walk.

</details>

```mermaid
flowchart TB
  WD["@/contract_id/0x01/widget"]:::tree
  WD ==> BR["brand: NormalTree"]:::path
  BR ==> B000["brand_000: CountTree count=1000"]:::path
  BR ==> B001["brand_001: CountTree count=1000"]:::path

  B000 ==> B000_C["brand_000/color: NonCounted(ProvableCountTree)"]:::path
  B001 ==> B001_C["brand_001/color: NonCounted(ProvableCountTree)"]:::path

  B000_C ==> T000_501["color_00000501: CountTree count=1"]:::target
  B000_C ==> T000_more["... 48 more color targets<br/>(brand_000, color_00000502..550)"]:::target
  B000_C ==> T000_550["color_00000550: CountTree count=1"]:::target

  B001_C ==> T001_501["color_00000501: CountTree count=1"]:::target
  B001_C ==> T001_more["... 48 more color targets<br/>(brand_001, color_00000502..550)"]:::target
  B001_C ==> T001_550["color_00000550: CountTree count=1"]:::target

  SDK["Entries(100 groups, sum=100):<br/>(&quot;brand_000&quot;, &quot;color_00000501&quot;, 1),<br/>...<br/>(&quot;brand_001&quot;, &quot;color_00000550&quot;, 1)"]:::sdk

  T000_501 -.-> SDK
  T000_more -.-> SDK
  T000_550 -.-> SDK
  T001_501 -.-> SDK
  T001_more -.-> SDK
  T001_550 -.-> SDK

  classDef tree fill:#21262d,color:#c9d1d9,stroke:#1f6feb,stroke-width:2px;
  classDef path fill:#6e7681,color:#fff,stroke:#1f6feb,stroke-width:2px;
  classDef target fill:#39c5cf,color:#0d1117,stroke:#39c5cf,stroke-width:3px;
  classDef sdk fill:#21262d,color:#39c5cf,stroke:#39c5cf,stroke-width:2px,stroke-dasharray: 4 2;

  linkStyle 0 stroke:#1f6feb,stroke-width:3px;
  linkStyle 1 stroke:#1f6feb,stroke-width:3px;
  linkStyle 2 stroke:#1f6feb,stroke-width:3px;
  linkStyle 3 stroke:#1f6feb,stroke-width:3px;
  linkStyle 4 stroke:#1f6feb,stroke-width:3px;
  linkStyle 5 stroke:#1f6feb,stroke-width:3px;
  linkStyle 6 stroke:#1f6feb,stroke-width:3px;
  linkStyle 7 stroke:#1f6feb,stroke-width:3px;
  linkStyle 8 stroke:#1f6feb,stroke-width:3px;
```

### Diagram: per-layer merk-tree structure (Layer 5+)

Layers 5–7 are exactly [G3's L5–L7](#diagram-per-layer-merk-tree-structure-layer-5-2). The difference shows up at L8 — instead of a single target per brand (G3's compound point lookup), each brand's L8 walks 50 in-range colors via the same `KVValueHashFeatureTypeWithChildHash` enumeration G4 uses, plus the boundary descent / closing boundary glue.

```mermaid
flowchart TB
  subgraph L5["Layer 5 — widget doctype merk-tree"]
    direction TB
    L5_q["<b>brand</b> (queried)<br/>kv_hash=HASH[68b6...]"]:::queried
  end

  subgraph L6["Layer 6 — byBrand merk-tree (two intermediate targets)"]
    direction TB
    L6_t0["<b>brand_000</b> (queried)<br/>CountTree count=1000"]:::queried
    L6_t1["<b>brand_001</b> (queried)<br/>CountTree count=1000"]:::queried
  end

  subgraph L7a["Layer 7a — brand_000's continuation"]
    direction TB
    L7a_q["<b>color</b> (queried)<br/>NonCounted(ProvableCountTree)"]:::queried
  end
  subgraph L7b["Layer 7b — brand_001's continuation"]
    direction TB
    L7b_q["<b>color</b> (queried)<br/>NonCounted(ProvableCountTree)"]:::queried
  end

  subgraph L8a["Layer 8a — brand_000's byBrandColor distinct-walk"]
    direction TB
    L8a_targets["50 KVValueHashFeatureTypeWithChildHash targets:<br/>color_00000501 ... color_00000550<br/>each CountTree(00, 1, ...)<br/>+ left/right boundary glue"]:::target
  end
  subgraph L8b["Layer 8b — brand_001's byBrandColor distinct-walk"]
    direction TB
    L8b_targets["50 KVValueHashFeatureTypeWithChildHash targets:<br/>color_00000501 ... color_00000550<br/>each CountTree(00, 1, ...)<br/>+ left/right boundary glue<br/>(different hashes — different brand subtree)"]:::target
  end

  L5_q -. "byBrand" .-> L6_t0
  L5_q -. "byBrand" .-> L6_t1
  L6_t0 -. "continuation" .-> L7a_q
  L6_t1 -. "continuation" .-> L7b_q
  L7a_q -. "byBrandColor distinct-range" .-> L8a_targets
  L7b_q -. "byBrandColor distinct-range" .-> L8b_targets

  classDef queried fill:#1f6feb,color:#fff,stroke:#1f6feb,stroke-width:2px;
  classDef target fill:#39c5cf,color:#0d1117,stroke:#39c5cf,stroke-width:3px;
```

The 50-targets-per-brand limit reflects the shared response-size cap. In the 2-brand case the cap kicks in at 50 colors per brand; if the In set had 1 brand it would be 100 colors; if it had 4 brands it would be 25 each. The dispatcher slices the cap evenly across the In fan-out so the *total* number of returned entries equals the limit, regardless of how many In branches share it. That's why the bench's `[matrix]` row for this case shows `Entries(len=100, sum=100)` rather than `len=200, sum=200`.

## G6 — High-Fanout `In` on `byBrand`

```text
select   = COUNT
where    = brand IN ["brand_000", "brand_001", ..., "brand_099"]
group_by = [brand]
prove    = true
```

**Path query** (same shape as G1, scaled to `|IN| = 100`):

```text
path:         ["@", contract_id, 0x01, "widget", "brand"]
query items:  [Key("brand_000"), Key("brand_001"), ..., Key("brand_099")]
```

**Verified payload:**

```text
Entries(100 groups, sum = 100 000)
```

Every document in the fixture, partitioned by brand. Each `Entries[i]` carries `(brand_NNN, CountTree count=1000)`.

**Proof size:** 10 038 B. **Mode:** `CountMode::GroupByIn`.

Same structural shape as [G1](#g1--in-on-bybrand-grouped-by-brand), scaled from `|IN| = 2` to `|IN| = 100`. The byBrand merk binary tree at L6 emits all 100 brands as `KVValueHashFeatureTypeWithChildHash` targets — each ~100 B (key + leaf kv-hash + `CountTree(00, 1000, ...)` + `BasicMerkNode` feature + child-hash) — plus minimal boundary glue at the binary-tree corners. The proof grows linearly with `|IN|`: G1 (`|IN|=2`) was 1 102 B; G6 (`|IN|=100`) is 10 038 B; the slope is ~99 B per additional In value.

Compare against the `byColor` equivalent (`group_by_color_in_proof_100_rangecountable_branches`, 10 512 B): the `ProvableCountTree` overhead from `byColor`'s `KVHashCount` running counts adds ~5 % to the byBrand baseline, even though those running counts aren't consumed by a point-lookup group_by. This is the same `ProvableCountTree` overhead [G2](#g2--in-on-bycolor-grouped-by-color) carried at the smaller scale (`|IN|=2`).

**Proof display:**

<details>
<summary>Expand to see the structured proof (5 layers; bottom layer enumerates 100 brands as `KVValueHashFeatureTypeWithChildHash` targets — 192 merk ops total at L6 including binary-tree glue) — or <a href="https://dashpay.github.io/grovedb-proof-visualizer-widget/#f=text&d=H4sIAAhCBmoC_6WdWa91uXGe7_UrvksL0AWLrOIgIIFjB4mBDDBgQ7kwBIOjJUSQjLYUxwj83_PU7tbcS73W0lF36wz77LN3sfgOZLH4X7_6xf_Z__mv_varX_zi_Ei-_L_vffny3_u_7a8-3_h8-eXLP_vnP_zyP_ZX__svPt_48iX88Mvf_upffvIXf9P9P__p7_7mH8aKTU5staUz8skrxyChlqJ5dn6QupS6VqhbEt8aeUosu0nfKcc00o-___1vnlu-ee7_9qMf9Z_9an_-xF_-4Mvff7X3X-heMeaoxWIrsZ9WzijSx9GQy4hpZ15G7ll6iCfZiKHVKu2sndqKtX3_B18-r1a79VjnkKGBJ-zdH3vCshBLLutIirmOdTSXmmNZg2fgYWalz3XK77zayKvtX-2f__Kbr9MfRUbabFEJzNaSegvEQGssZYUhfHvOWddIa9YS5piSwi4lnbRK2914d7_9W_rDL3_9k5_-bH399c9-8a_7q3_8mY_Wv_zwm6H68uUvv_yH__ibL75lML_--JYh_f2B_d3gh__754b9m9EL8uv422hhy24WR9n7tJDmbGvE1fjNnZL2VE7Zoc6-tsWZR9w98feG5ZHViMn3f-d1f3sovnlHf_ar__2I_sm4fkd0ryaPtl2kSeAVlpia7VXiPCHtVlKrZa-dtzY1AmJFDwnCA-zo3GWTj_XHvxeNX3_It45lkG9Go5TcMgmeecrfjMtqPZy-aicDo-Vw6q7NwpRdtZez--h9RBmljVni2VPJZdVYVXfc59tfyG_myB__8E-N3K_HL8i3jcCNcbg1Gtd5_68_Xf-0f_lNtECBmCXv_JtY5WnBjqUT92C67pDCnLHPrjktHRpb3yPF0BsBHbVvEIW4hdRjHCPXP8jhZ1H5-uPrV3gdndsxehCpqxxuAGVtOiT3UBrwWc-aJzP55ohz-zyeldmmWds4M0_-VR6W2wi7rfbjy2j8qXweX_Wfrz8aIjsppJzSb4aqjtzK6q2tvHsJvaY9VErTNXsZPbVUgePe6hnHR20uUr6NNGyFc-J3v7Y_pIGrjz-mhzxTLrHtlob0JFDjYdrtWEmrTEqFzWtWWyNUkDGbCIkW4c82mYw1f_dr-33auPq4m3Nff3zi_l2J9zD9Hifh5dT9L7v_8ldf7b__t3_e_-unv_zJ593_Nl3-MYTwgy9__Ytf_fyXX-dNQq_MfEr8wRf5_Oz8rP8TQfgHPvV_fvzrPGokg2ub1hNMJdKs1SixQtq5lKgw1IplFCuImpBPmlVDaDlvtEUq48c_-PJX_V9-Ov3N_c9frP3N05JpVmtNU1vcCKaaZQ3ZuS9nhIZWaYJKSCn0Y1AZiFvzGo25tHIt9t0pcDmBvjNS8i5SWhXZJ2fonvUodLHllNo_VL8DZA-Nkd4mBKudEQTGi1Z4Q6NlixeRCuNIDDFBS0QcCbkGJDjFakI5wN81VWRmdh0xS5aWfSRaKxYIcWl3I3V3Ov_-pH4S1_gyrsDSCVoCSWKKaiD3FjKA998J5F6A7lDNG_0Y9ukyqrU4C2TeVpd8EdddlyVQKOgoR_tsLsGAyLX6IIok_Co9IedT7IqYFwVnJcRQe0A757tx_TUU3XqwvQhrejmxO_RtcPiEG7bUlBtz3Sx1bfz6romJmJjie4lNmIXpyFTvpnUQ3nYR1ujDZVm0aGhxpTS6z1hkKFSUg2ZQIp4aQgmo_lonQmqeVSztlaDJu2HNj9K1vIirvosr8qhVWZqRrjsBi31PBBMWKs8d46qNXJoWa6iCnG1EPKiBg3MhTMe5iCuyAs622HvtceUciLC1M6Ns_m2n6kanFmUOaJEJakKu_Am0rMrJ9W5c64tI2btIBailyyyhkVApVsARiseEHJ_VcZCYvWkmgepKKwYsY5eF0uxjpK16EalcyLVqPZ9ZEDJr7B0P0p38wumsmIdgTEUwoGMXIADraZuY4i5s7tuRao8yUN5wdn4XWCzmHu0guceaKREwRFZRYA57L4hQJ-zYRuXbaEaYArfY2igqrc8arjhbbB8MEtoNskEOlO4DGCODGDRJCWW0Mn25IMiIHduKg3U_K6HrnLc5W55ApsRHj35DXOXdMEDKsRPqEw4pzoBoWWjuELFRKIVKzufgJrc0kAFZzqctkI3gYUWRXwyDJeD6AARb8R3I5hgwWUG6KL_WkvWdwhmtAe5OmSfGFDYqO-SxV4i3h0GfJfgb7qovMbZKnXO5-3QsaD1uDXPxHg3iR5NuPMOZoERJe4KTPZ5zho2KT0MZXES296myRJKudJLsoNBdn6BGWZaNAK565h4R2VsBqAy0NqQCwkNam7e5S_KLULV3oRKLZWER-WcuKGlh9PhvbbI7Zj6ToEZaIKsR8ocRkQK_Z7TTPhVldBEqdZHVdiIChZzbSeMy0tBynni2UwCcjW6YgUGYc8ZFBGP0BZQO2PTboSrPkvAFfclLZxRHs5Dh3JMJQPSA2Ahx74DwX6sZUvWMVc0EuMRyY2PURlsVGNAtVyhrALSYMSp5Hnw5BIiUH7ORfRgm0hqJWsvkW2D4qTW4CggAbdkPdKm0J7gZX9CXvDRSfa1yKjapaw_NAMsY9EhjJmcrpdXe01nbFwiD9npqzK3qwkD6KrPuK12w1-DvT6e_jdQFd3sxRg3nYMtAxuqv0aqvPgQcxR6MKCp51kLqnttGSh6lbIwvIvvSSvlSZrKmoarBGAbgodRPGbYWLnNkcC1kPVA4_p3wt113NAuNAZlyFdltGLCj9SCxdvGh6SMajrWEI3nUXC3XXYxXCG63jBKZOC6EQfWJcd-iviBveWmPCgJ-dCUctqZZQc_s4vSQFoQKk6SlfZxi_WxjBgo8Dy2TsbJbudRQG0OERMrAravyhnSX3pnyU_GaZHgih6Fv8u-I5rLKEECV4eodL3s_CZ-Rd3xB3vLSIKGAeupLiWiog5lVYsjxFGa2I9l2ysW02MikUD8lzL4r3DU0ms5Tr4xnOLFkDT0r1DXKqCJkN__GFmsBrQ_GPjJA6QT1lyjbmhPYMD15345sfoSb5dGj66NHtxeD9tKr-WzOAUFvAV5vtViZpHldZlCRRUUPnbPw-tDhAVk2CYwMyIza6nIlI0KW4yKh7R6nw3MCSWSsjRnJsZ6jLjHQsB2JjFBp0xcRYkuV567n9jJgCs9Wt14sG8pLs9ZOyDnjZ9cZtaDpE1N-x35cVja874HSWqjrnFn5GIDDtBEx-ocIzSstW2sZG7UakMImKDvGi98gxDqsCiQXtmHOZsVV97kPeCdMQOwykkZuR_YNfZW3oUrQMqZp1rinkG-KjU0b7t6kYnB8jmToTn2fFD-CX2o9WQq-Pl0tWQ0QKQ9pLuOQa3JwVRMYJtXPSciMSBLm1nvDdGnq-LleFrw4t2pbt7VsSs-SUF9E9qWhaiirWSr2PkSCoBWOnsFXZNZgwi_3QssQTG3CgwjUriORl8s3AaRdrfLj-tvIQPLRyLOCEkAucxsFZxWMrxolmc1EWk7gnqiD8JPxO7i7dFsYJHuCm-mF_ZKX9msDYbFU24IrWhNnSUBPm7PmlQYZpaGtWSWAbNb4vIxxYHpJZGxccqllfR8aRFDpu-eBV0CEYfEmHIicWMc0lX6ab0VKKmU1Irp9YdYAl3I7sM_sV3phv-JL-zXIUOTTLLiwKhrPhi6Gb77gQK3jwdKWkpJC8zW0pGmhO3T02m310C8XuXBn2VdvefqK-I-NWZUlTjTtWBGUaQFJ6Ow0Eu427OIARJof6w_A4AV5x5eGCpevCMrSEEEl1NpjzjVsqEU_-6voV3wBpA4KJM2CFVJf_4AREoG7ws3SIGaZCV86UyJaBUbZp0pLXmuEbNtu2Do53w5onBocn1OqYGfn79_eQXlG3vqCvONLQ1U7kw6jatHwTeOoJTl7Q7UlB4x_9QVXEmWXEFZYkEdfGZArrndatUvyFqnixl-GZvwEc310pjupLKWlOPhlV7olLt85LY0_UOWMyCvAw92O7KO1U02PHv2CvuJLsybYTPXFOqn4Tj2hIqJKCuBp91K3OGQxg3FvULyaTNQBuhSw7Lj8feVrW56lr1baIeadv8GgkMttBI_ymWnDXHVuEB2dq4D6KIzaOb5ZvvT2grfaswR_wV_xrVmbISnJjAXe2VdOnPM3cWxlj8g_4WjrqAbsa4wj4Nt0LKx91QbKXG1qYxGaodRyBl2zL0qmo9P_EEAU0K2D3CaUKfWJ2D28Ch5VLfs22Jbbul9fbPzFlxYphlpEE5QEcadG0nS0TlVUzTjVSoKcjlfrdKktdywtRrWJkjQg775C2Xl8CQp0BsChublR6qNLCkfGsbwshlWmABkRI7aa7_pn8S1qNP6672u1PkvCN_z10iKdFpH54tvHQCuKZpfuW6ru5mvjGwEtqf30qBDcKvipqaiAZdn10NVGYc1eLbF2aUutWupLBIzYa0Sd0Dn6FvxGw6a4Q0gxx-MLCbiwD4iEu5G18KgE4A19vTRUJrylE3lX6ewSx0kYoeaVN0l6bjY6jltnWRDPDLKSKtnrBX9SBId6hZtYp4CD72Yq2LGAwrc5QkWx5h4cdiNxbsuwVwHOSqs33BRey2tXbtOXPatZsRfLh_GlocIXmRjhOr211g8uignMDC1n-aLr0VVHh4GQX3DSiVu7b2yfsQbAcLXItcwJbkSpwU2w9Ih1RaWJF1OojM2v98TwVSAhuHWTHNueIDZzZd5P2Tfk_dIixdx2Xh4pV9XIzk-ll0pDiVoqQOlwIN0JE7_RmKMeEhIe6qmQPlf1PTl9YktMU4Q_omQcKMEdwwW9Ta-LXL7aXXsYxHIkvD2mFJcUQq-3V1rtGXnbC_JOb2v3sJi-gI_l2YShH9JhB3Qk2blLIczYcKyOWsOB9h0ryHfM6yW8tu_KfFYvXCsGHJ9R5vKaHh_AqqskZOha0FkcumyTeowoWtdgw7xkYw3S7W1Se7R2ao_WTu3R7ld-hOD5BYKnl9ZuojaTnQNpjdEjwjNMACAXoHwA4DnM5gN0atKA1gDbca_N6oyjoGGvRMf2mkuVE07lF_DVoSRJXqs4XVLDiCtDkDKZs0xGS16VmAs8KgdWuV3F9QzB8wsET2_LDtuwgI2VmS3MIYr_Z0JBlWBFUF8bFE_sAsK0NXC_uUxQBoM3W6vz0lOILyr40Y4eli3FRmCUu5vCnENqJqiQqAWg99CC3LJMFwKZF11uw1J-geDppf1a8SCI8AgIh5ahIHR7mKI5lI7TnauYH3ApOS6F_4ocsgrW92L4Pu1qr8wPf5TQNWOBccygdM-EJkxfzOWZyWyMNyDuVW41KCKCcUGoIGGW3S_HyM8QPL9B8Jf2Cy7KOpskwUzgfnYcKyAr9tm4oLIzv33QpaEV3JgMwwKcOBFtCFoCcxHZVGDSOMhF8m1Aoi0FM98rsINaCHtDAmlBC3lU374FOJoXOfBDIq-3I_sIwfOL1cP00qzpwtFrdKsKazXe1dHTa90ZGgTodMe8C-IYhZZQDwaACtNUMjCIwLpauGkjYdhwbGcOktWY07EyZENDmxnjlohit9JHtVGQJshrqDPJCKeH2-uy-VnxYXlRvZHemjUvN0pM3O1VV_hgieLV8KGm44XAu-OJMVTVF1eORv61QXond7bhcqewNSAmnDUlkvoHHlvY3lNPAkYP0poPc0nXsnlN_dpp-OZi7-hg3OHdyJY35P3Sfi0MUcq-37KkewHsORZRTzbqkByY7lLSMB212SkD5TsW6diD7xTOc6XPJhPWhdxZPrld6Aae03cTDxqgGeqt9THVqz-TKVZZTrdVYPneer-9hF2ekXd5Q94v7deco05iN7Vt84pIMqJ8lgID1DrRrTxPaQLIqRXfxG4qOkc5yOCcLkuLCWSbq2njSb0CEHtF1DampMUeGulZscURwM65xIgDxPyFhYbgW-s2eZdHhwbKo92v8oa-Xlo73G6qwWsyc10nbMsindTDwwa0gaCctJKUvutHsNYeeOBzkEhYu5SuVg_9qMJu9VMTl32ZazY8sQ5xn5cwxKQyXmavUBRzCSyI-Wqk1NJt59slcuXZ7ld5wV_60tqZS8i1-q694tR4w02EmCBOMbcg6ujQWg_hU3TZc_TdXaulHjQmqXtp7YLrNzteHrcCY6Npz7xxAAg8-DAcP8s5cZOSxogwZ40Z9gy2T5r3UfbF6qG-tEhbQ2zTprSSTjAZGSnvdW5RmMsKEfVVI6kR5vos-fHuMvrIpldgjMvVQ8DYzCVoEx4eckcr9WyzI4IDrwttWne3yaBsL57pPNIXczM5z5-7fSDj2e5XfcFfGt_WFu-wFCGqR_aA3LvB914VTHpoXcy-MhafwTUaU941-CYhDrJPCPtq28U2HO8-VWIYuZmvveImVjJDQoAjSLbq69syeg5-Alfi_BQf7YTRuB_ZR7tf9QV96UtDhZtZJad6vAA4IQHcTfmMh8eY4uuzenhQD-SY7tQrdsfIX-QTUkrKVaELwSFVxyqrEmMwYTbG5Ow8dpxpSokrA8DReF4BCPqZlhMGNB4__347sM-KD-uL4kPVt4c01zj5YJ98d7VvM965MjVzK4Uwh69Xtqrv6hddQzFIiP5RhOkOJ12lbGZIOlMdliH1Z1I_JTv049RQCchVpFfMzm3aZwKK8PvidV4r9XV7vaS-IG99W_JXO0gFsI-U3QHlkuoMZSs4Zv6pVHj4U4w2S8MtbT_RUubws7stXJauIPilHfPyrRmarE8FN_HFtEpR41dLDd4VBO2QC_gZvOSWHETJyv0l7PqMvOsb8n5pkXo5C1QzJxzzkz7kV3GXeI5v9HWvylvo9yFQTBqNjB34mhRAPJjmqnSlkdGt-FkV5jWoiy7ojJwVmfG4twpeeLxj8HWVExFfra6Gbdql5FPvR_bR2ml7tHbaHp39ai-KD_WlWQMqofgYFYpHTC1JO1uDp87GpjAuTAjMhWSvxfACy5z86PrgoxyvdrlYMVgRbRywJ8Xbn-j8MJm0FbqX0YQQewOLvRxp7yVeXG-V-TIFl5jL7RqD9qz4sL1YPtSXZg3snHVC5rwxjC1KXr1GuyRrR9rY9dP-BQ0VGmJ0pLibjmPFiwUN_r86MRsHgn81Px9TW9mhYYJrO6C4K1uwlwlRJpo4JBkN0eJFjyjkgtCFFW5H9g19vTRU6hpnDt-RdRYzVPqIIaMQYihh-SmCFdzoBy_mBbXjrrCMlw1YB1Kvit6C7mLqedqJGCPgHaxGJQ3RDF6HiBLIuzFGpW8Ev1eHbWRdS75jfltxtWenttuL6g17a6jSQsFGL6Gcy0_D5JO8sKdq6xjJGaYvXJUOUR1DFvDW-zihC46BB15Vb_jpTrWysaWnMoWDnxgpR8jbk9VLDDbCdseyh61IRiI9aow9-ZLYub_S2h7tfrUX9svenuYiU1Q7kFWbVZ24RixoIKLnYP39KCXetSIv3RT47nVETEjkoWpE66r4MLne8or1rNtgOzH-xvAC5QGj6uwYMEt5acLpffwfukJdn50p9-s0eXMPz3m_MGD20oDZUhylmYm3EJkVSEM88LW3IPD1UPVOC2fU6Adjh8bTDAQNfs6rM6evJBrJxyQwfDEpW0at9Yy4rTRvXEI808zTT-_AUxLTQvL6woP3Mmutt3A_ti_42972u0BolTTcoK4A-uMm68TuzJi8SGL5ftWcyc9y5Yk_YgaKr0l5PxfTJJf9LmIFNLKkqA09nNdnC1BTRikPUpnJHyI5XsZce-QPFpCXM0niFen9WKWHefiCwe2lq_I-DbP0EXwhLswcas0L5zjDwK7jULHw3rYtrC2Qu1dr-gmOeWyCddeCNnuHC8UDWCm9nUxCr4y_CKl3ZU7vzORPJfiRoF5rWmjoNTAi5Gx_EttHK6iA9LOHv6Gxl6atxrbTihJLBEF9Z_SM42t0CMyE9kcIfLZfrccwxOuQWmx-ANSL5rVcnvDUHFNHI_QhlpIGuGkm5Kh6HefxtcU294p-KKx_mhMFRPIxzKKvDt4_lBzqwyx_w2QvbVuJfetZzGc_XpD8wHzBlW7-546-FMRS9ZTeE80JJAYvURR0wyZieiW-FKDuy1vpMUtmTqanf5qjKXRpfiBs4IQlHtWln91e7IJo6ZKyaVz322q82AW0l24pYN_RPcrMnLI96bx7opiQNjHglc7yI548QH2XJDoe-J5WrlnXylcrWI3BQx5NP7s8cpAJonpvi4A7WBUF508L9FVDXrXOEB1UcW74KyJXHsTq2SFukTdM9tYvleYKvMWA2GHautuHvjCDa8BmdaWiDofm0hTDEwZzcUsYQ7BElzXZmCTz4wvp08vR25ZsP_LlX9Thy6054rcguTxO7vucXkfCLbktQ07cR1t5dI5A5A2RvfRXuQ9E5e69Ms0BNGKXREr5iAKfnwFSn96T6IRYAhmbhkI2q1aUrF6dg5t-kPP4SmtbK1Y_xjoGz9N3L1p4YSUAqbEPlNrxPhLTV3gXGHBaD_eLfkTsYdq-WE_MLx0Wfj4GyVgrxZBiwZsXPOF-vP9dx3cT-y2bWGbvlpd041i9MJMkziDI1dKrn0YGQSVamnjdUVuL57TMHJhyYgVZQCMkCIYBEcZ73n7QWLwZR9D7nQTlBY3nl6bJ23wJQZJyzvJiB-gWmukadzS01knIrbJW91ai6r0yfJtuRRPs0-mXJ7amE4xXr68KeyzvYDKbN4ix3avXnhVAA_EAsmJqdww95BFsljwy6Xk_Vg9pXF7QeH57Zmt5dxomKwgp1sTyZxFlrc0zEOnqbYM-DbLyap_yoBFIMIIWq2_sX22ueFGkHx72g9X1iCzc7ZnT1qcTo6Y2vd0dKjaixj7706Q8GFP66jzz7djG8Kw91sNuWs_aacVnWB712cNfLK3ll_7QXQvA06WeWvcJqGKvqfXWEAWPPY-7PH7SvRATuNbqrfxa0TS7kSmXS2veZw2DdNrIjLstb2th_azlJ9jT9uZoGaL2YwHDYul9pTlWHbjwfv-krsRna2sS38DZS3-IxGttZz9eDuZ7mzImRsreca-Acudz0v4gqysibWORT0m6_HRvcxFytQbE_N0p2RJtC2vjtUt8Kq3I2p8-1JWnXOnUs8_K-SCfETE5oMfRMF3ux_bF5lB-6eBGNBSziyudYC5-wIygoB02EdEDcBeAWZZ3pykosdL8iBnosjO650qCoNMGaG5-3M78GOScPfuxxi1pBfHdyFmzeaXpGF6H57ug0e8DSMkPnt6P1cO-iOmFK8kvHVzYScIZPbQ1xAuGiWVoMfkBMHzV0DaT8j3cFbqaLPQDuH6LAd_tJ1429fF-SfAIsznu0pjVo07x9NsoGWiD_PTm6JthzF7_WSS1k86CM6as-3mYnmH5m94c-aXhQz6g1rzxqzYi5WfuMSZMct6_uIq2kLQCd22bQ-cuelLXT23YaJfq7iATsZE2mMK-UBxKGwexgquWqsfMb9to-BRvkDK9MqI3EKUY7qXL_TN68rA5h7zpzpHr2w7IB8_r5-xziYOZS-7mJX7sokxNGL3o7STVySQCph7UlHL0EpJu9Sq2qxYp3jnOryrxtsbmZrnvOJeL5J5gqMIQFRnbgnq1LtntTdq7-fnq-7F9Q-NvT5OJgnFM3ekV3itM37JS67nr9IsmsmZUnGGt_FaN7B0RjmDAmpKL4_LM0yjTDzWAGsWLHrCE6G8dOwMLU7191xp-JmE1Etz7nKk368OTe18TzffNcXpI4-kFjZeXDk69EdFYZFRSZ1Hz0iM_gOt9MdFOMW71M44h92QH61BWiCRq936Gc9llw17roManzIMs9zON2iMmse0Na4uXyVcCjTFGeq2DNPAu5wq5h8KD7se2PoPPR3UOoi-IrLz0h4AANC5wN06jxjR8zUJ26TX77gIyALPojY8a7J1Sxez0yJQWENRb8lwRGTiDMU_ezmRHmKwaksCvoJnNm_yBt-Ktwrt5y3RvR4HmAMgRwTjteX-dQh8ur-kLJisv_eEqG7V-eve1tdl0Le_Ud7qSphl-W2PphNyXHzGHekh60vFYWp_d26t6UYGVZj3eYDkySH4LT6kOPXH0480WfWl0b7I68pyrVG-TaqUH76SMALkf2xeFiuVtoWJTBLaXc4V9JB3faV0h-D1GIn6J1KzJ345L7xaaN5myYRvOQbkiOa-WxImm5XlWHJ8VzRnNMGPdu6aAqBJm3dsYgzq3Vl5px5n5YXX1xnMPlnn1YY9jfcFk5aVpClGdaHoLXg6wk6_NAqdngpcL6tmWRoCww0rDRvTN7dB1Wy-b9N2XjeSB12YtkVrJm3SQeCv3Du-XPdZaEcwI05vZkKte4oxxiIC6Ev8z71eJiT7bJXvTqqO89FheLgCP-5VTCBto3_tLbi211ZbdcXWx4-cZBySACKpYruxVtr4DtPu6PJ0fMaJM4EjOduLmDanOp0yha_cDDebLejsjTv3ImdVjJZt5W_8xwv3ltYe9OuRNs47y9vxXmAs427sm0lHi0P7psjNVP83jc_aLDlCe3qez4DQ_p51M5ipZsuU_sQO5QRuvt88TocYzAyoOxOonZUvN3txKAZ_Nj4_gvkTWCq1HLwG7HVt7Q-NvGxpq7U389rssdfqtBY6Ly7VO8gscQMu8vDyoqx97jQBp9P6DvKXmDd4uO8Sf5ccNuy3fAdsDdPArYhJqIGxMAU6sJBwYvqzCb97Jc_g1NNEbcZb7HeLtIY3bGxp_2yM-NQE75Qi84ScEsoy4-vCWZ4eUPGkZ_Grisz2ZH5oFWyXjLKd3RbjaymGiWlhI3oMcCGn7cXdv97SU-a5opZ4cQ4KuYK2pePl8b4jgiTUdD_Lw2cqqPVtZtWcVD2-6VpS3LRMXkiH4wdjpnSZlwmfm7mqO4q2QydPQK8QVsQfVO0VWK2ENhuGgR_vlpOBNe5dmPxc2XBc3nm-2CGZMYxTbBJURvT6i3m1VmYYhBQlW4gn39Zc9bFlvLxYXa3h7Ymf6FQhgQoaBCiGTT9Mk74J2ZvJ2EW6fizfjzBaQtyi30ar6ck4Wuezz6c3pvLe3-h0M01dvVUqoMfh9iqDwxAdOJO3s3Zc3xx75nFWKF1KO--Uh9oLI6tvWGjXWriiuuMkcWCt6J0PkqAbvgIznjX7NRlCgU-dBLnmvWSQr7zy2fbUQO-Y83QtAkxCrU8iudCRio09VnkxQ1GBWBOzjDGd2b6oYa0rGnIj3z5BKfli4-KZtSX3psTChYIpCTd4JyRcIpu3dippXxe6CMCftDmKh4IxO99aJBYk_ySsjY6_aSUP4Ef2PUfC-XcubA9c4SWv10SzB_MwPGYo-0YpJBmhWNtxF5tv5wQUez3bJ3vQtqW_rHH0HEj3hx0Yw-2fN1VfxRgPeKS3XT9_S6FcltEYUOilbwlmddCu-OnB5dgwg3OLbmWMl9av5EoOzELqposwkn7iD-7ZO-Nde0TTvutW71c15v942P7Rk-YUlq2_v9iJMG_t5mK7Fe6bGjDJDWvghr129R0zI3qLAi-5y570Er8LzFX-v3L-ETx-dpJv5v8pB6hWmxTkCv5HBBcU3kLxEPXU_ibaa9_YaAWFddICu92P7gsbr23aI2kovA_3tRSx-hI4MOsGbtPjZguUa1pdZvD9q4gGaEV2nxXmgBPDvaorvjmPy21XHLlGjeD3ZdFknyUtGlud7INZ5-MzuBPMsBK8Gq6PZ_UXr_JDG33QvqW8v-Np-7g6y8C6TTeKyFo9XBHotQ2y-YgpH-V0ohJ3kaU4vLj8_wLAv28IgorSaSyks6hgWgvdMMAyKN_CbyC1LUbMkn_PTL0TCy25vD9Nj3A9i-2xltTyreHjTwqO-dHC-oz_9avVc54DUqsvS6Hwmy0vsEZDF91TJShJQ_UTpZtTSCUfTLJe3KAZQxFDCKNueyjDZwEMuJDq2o2j0i61wM7N6l3q_dIg5lvuMfvhvPDjd8LCHh7xp4lFfOjhZM_VahBcZjy91Myf90EjMw1tNDRBlqR9RRSEcJPFBNPWRkp-1UcJ3tQDmvTsbihYzXHx_4fi9TH7oxBraFzmMrytkdypa-JtpMMUAKG9Rqbym-7F9sUVY317zdcA2v6vU2xd4jcUOZcEVI63j9ynrKXt6eWEFhHvirXlbMcP6nHhOvSxqkpjAVj81s6OfTvXTZQFianNXLANyrXqrCUAG5Z9j3mHXbucQTKTBfdNUHhYuvuli0t5uewkmvsRPi8gD6aoXYJxIZhTvpZ6ae0UMDfRd-w7Rz-I3wdd9jpFfXjsB01n3ixbGMtUZevYe6tnW7nEsX5HpCYQV-L5r8TL7MDLwDcpknfG-WC3lGXy-ILImbzsgYahMU97Lm9S3kMqn2cC0lAbKdAQymK9OEu93wBfTyzvxSTtYuyy6WhL0MDbFJZP5dalAtIqXYAGsCIyyTkvNdxUZQi3JPlvfzPMUe3-wj1UeFrvUF4uL7aXHSmn3z_I-RHFI4bMw8XGjnKLffYxCwreKb-kNPwUbwVe_yjv4jjXmUS6b-26MlZ8pAayary1qd1c2GwgE9vilYtZwB4XEY5qYL6kf3OuCn3K_L1bfdDJp6e0NCWEsFBOsmkqfffmp-zb8lOKoWEiZiMcGmBIjfxd1N79vpjUm_ZqXi4UxbF-ELUG1-x2JGLFWtfmi9fxMecK0P_esrPh1q3kgZfNHUHGdvL8fq4c0_qaZSXtb_FdWsWVJQiZPeh1xIm7mhFf3IQwRyq2ji19VWlT68BMFpXjpUPS-RVcLsb0TfRiN596tYAL8nAnGyDscRWveebJhan3nMOE5ku-3Ig2kyRnhwXGF-mxltT5bWX3T0aO9PRxWwL6EefVrjHJp6yTsnfSzMap5z-Tn6klzYqYM10al7rS8XsbvO92XXdEaGreMOZo3CsnJe4QnxvAQ5er3CgdhpP2CZU0bmbzi13eLB2jU-v0Ll6U-vQz0DZO97unBpEUJNbJxqjfXGJ9LPGfwdeYjQMhx8F1kYMk1Hj8vUoYfJkc3XZZtBr-OZmRvCzgrHsC3hQ5moETxshsv3e-fS5rQvPvwjGQ7A5ard-ut_QGCvFhZbW-3vWJfSMgU55TYYvGtP8AP9rEW8hhnmzN0HIgswBeMtTF0-VEb3r5c5WFSnO72bgfqff6wV3q88gjThXhLq-cpfnuA7-uuhc6aO0Uimo__ablfgd8erqy2N0z29nBY9Y7683jLZ78JPSQ7w69CSJ-LjOzwhvcqa1e_JZ0p-Slg3xr89jRA92qJanqfYu_PYaiH2lCsbej0Oo-jQ3xvMvgN7n6vjzaxudbGkLgRGMHT-3ZsX2wRtpemqR-kCVE4sUvoTQMKf33YYfoNsNCVN4jkjYwKjs2AJPUuRISiKfLpsgwgaDvbwS2MsryRV1e_c8qXCn0hwAuGvAnJanYQEfgEr_1x3ZUgRbtfDtyebRG2Z0TWnhFZe1bu0Z6Zkt_0t_ju0Pz79_6cn_-pn17_7Oon3_79b_vuH3_vD7_z-1__7le__fzXn339__7ff__ev3_v_wPod7yV8pAAAA">open interactively in the visualizer ↗</a></summary>

```text
GroveDBProofV1 {
  LayerProof {
    proof: Merk(
      0: Push(Hash(HASH[bd291f29893fb6f6d6201087746ca1f23a178dd08e1346cb6c127e91ae3623b3]))
      1: Push(KVValueHash(@, Tree(4ed22624752972af97fb71abf4067b23e6d296a61a02f35b2098819fde39d289), HASH[4a5a28cb1b40226aa35b2f0d502767df13268bdf4678627dbfde26a557acdf73]))
      2: Parent
      3: Push(Hash(HASH[19c924989e473a90d0848277d0b1498ccc8db3dc870cbc130e773f3d79ea5b71]))
      4: Child)
    lower_layers: {
      // L2..L4 are byte-identical to every other query in this chapter
      // (the @ / contract_id / 0x01 descent into widget); see chapter 29's
      // Q1 verbatim for the full L1..L4 chain.
      ...
      widget => {
        LayerProof {
          proof: Merk(
            // L5 widget doctype — `brand` queried, opaque siblings 9862 / 6c36
            0: Push(Hash(HASH[9862894b16a0792688fdcf64edcb2ceade5c8b234649bfc6cfc6426869b0e9d9]))
            1: Push(KVValueHash(brand, Tree(6272616e645f303633), HASH[68b697da99d6ea70a83eb41794dca7ba3938d0ba98fbfaeb3cd0c19b3b5d0ff2]))
            2: Parent
            3: Push(Hash(HASH[6c36729e93b1a316cbf60fe282eb630c0ed6e45db088e365110302b6c9caba86]))
            4: Child)
          lower_layers: {
            brand => {
              LayerProof {
                proof: Merk(
                  // L6 byBrand merk-tree — 100 targets + binary-tree glue
                  // (192 merk ops total; structurally a fully-resolved in-order
                  // traversal of all 100 brand entries in the byBrand merk tree)
                  0: Push(KVValueHashFeatureTypeWithChildHash(brand_000, CountTree(636f6c6f72, 1000, flags: [0, 0, 0]), HASH[90ff6f6d9a3d901195982128130677243bfd27b75736206f3c8400966ef0d37b], BasicMerkNode, HASH[19b58883c492e746861db1e6ad07529a5a91cc8330af522682486db9346d6875]))
                  1: Push(KVValueHashFeatureTypeWithChildHash(brand_001, CountTree(636f6c6f72, 1000, flags: [0, 0, 0]), HASH[484ca11fb4ec8f479be1f78af903ce0c9d4fe630517579fb0172c2576d6b9652], BasicMerkNode, HASH[0bf12023f8e067c12db4cec1583909a0283878d6d909c76196736299750b5879]))
                  2: Parent
                  3: Push(KVValueHashFeatureTypeWithChildHash(brand_002, CountTree(636f6c6f72, 1000, flags: [0, 0, 0]), HASH[4c19f047068654e71813dce7839a579edfdcb446e3d70efa1b8592c73259da16], BasicMerkNode, HASH[e8d5372904b7f4ac9334aeb4ddab619d9ad7a308732a4f231416e10208a0a356]))
                  ...
                  // 97 more KVValueHashFeatureTypeWithChildHash targets following
                  // the same template — brand_003 ... brand_099 — interleaved with
                  // Parent/Child ops glueing them into the byBrand merk binary tree.
                  // Every target shares the structure:
                  //   Push(KVValueHashFeatureTypeWithChildHash(
                  //     brand_NNN,
                  //     CountTree(636f6c6f72, 1000, flags: [0, 0, 0]),   // count_value=1000
                  //     HASH[<per-brand leaf kv-hash>],
                  //     BasicMerkNode,                                  // NormalTree (no count on the merk node)
                  //     HASH[<per-brand subtree child hash>]
                  //   ))
                  ...
                  189: Push(KVValueHashFeatureTypeWithChildHash(brand_097, CountTree(636f6c6f72, 1000, flags: [0, 0, 0]), HASH[92adee932cc12927cd76ad9fd25906bbfe547df2bf21e826845bb4d3b47f5314], BasicMerkNode, HASH[34b69e1e424aa023c74f61554db2823da6c19dcbc51bdd5dece32e3f6f9fd219]))
                  190: Parent
                  191: Push(KVValueHashFeatureTypeWithChildHash(brand_098, CountTree(636f6c6f72, 1000, flags: [0, 0, 0]), HASH[68e02fcf66f86797035fbc8d53290185fe3fed7de897a8654743cae4007c47c3], BasicMerkNode, HASH[acfc3a88b852e8895449b4c7e01f4b1cc25028e6a80e4915cdde578ff6eb029b]))
                  192: Push(KVValueHashFeatureTypeWithChildHash(brand_099, CountTree(636f6c6f72, 1000, flags: [0, 0, 0]), HASH[af9667a8f2a10a9402b3d1fb0ac6e0b64d1e3dde5b8829c03b8d2c9cfc94e16d], BasicMerkNode, HASH[d049fe7e250b7dd763a4a5daa4227dcd2e41733dd95fd0758641ac06c63c3b51]))
                  // + closing Parent/Child ops binding the last few entries
                )
              }
            }
          }
        }
      }
    }
  }
}
```

The 254-line full verbatim sits in the bench's `[gproof] G6` output — same template (one `KVValueHashFeatureTypeWithChildHash` per brand, all with `CountTree count=1000` and `BasicMerkNode` feature) repeating 100 times. The schematic above shows the first 3 and last 3 targets so the structural pattern is clear without reproducing 100 near-identical lines.

**Key observation:** `BasicMerkNode` (not `ProvableCountedMerkNode`) is the feature type on each L6 op. byBrand is a `NormalTree`, so its merk binary tree's internal nodes don't carry running counts — only the per-brand `CountTree count=1000` values stored *inside* each brand's element matter. Contrast this with G6's `byColor` cousin (`group_by_color_in_proof_100_rangecountable_branches`, 10 512 B): there the L6 targets would carry `ProvableCountedMerkNode(...)` features because byColor IS a `ProvableCountTree`. The ~5 % size difference is exactly those count fields × 100 nodes.

</details>

```mermaid
flowchart TB
  WD["@/contract_id/0x01/widget"]:::tree
  WD ==> BR["brand: NormalTree (100 entries)"]:::path
  BR ==> B000["brand_000: CountTree count=1000"]:::target
  BR ==> B001["brand_001: CountTree count=1000"]:::target
  BR ==> BMore["... 96 more in-range targets<br/>(brand_002 ... brand_097)"]:::target
  BR ==> B098["brand_098: CountTree count=1000"]:::target
  BR ==> B099["brand_099: CountTree count=1000"]:::target

  SDK["Entries(100 groups, sum=100 000):<br/>(&quot;brand_000&quot;, 1000),<br/>(&quot;brand_001&quot;, 1000),<br/>...<br/>(&quot;brand_099&quot;, 1000)"]:::sdk
  B000 -.-> SDK
  B099 -.-> SDK

  classDef tree fill:#21262d,color:#c9d1d9,stroke:#1f6feb,stroke-width:2px;
  classDef path fill:#6e7681,color:#fff,stroke:#1f6feb,stroke-width:2px;
  classDef target fill:#39c5cf,color:#0d1117,stroke:#39c5cf,stroke-width:3px;
  classDef sdk fill:#21262d,color:#39c5cf,stroke:#39c5cf,stroke-width:2px,stroke-dasharray: 4 2;

  linkStyle 0 stroke:#1f6feb,stroke-width:3px;
  linkStyle 1 stroke:#1f6feb,stroke-width:3px;
  linkStyle 2 stroke:#1f6feb,stroke-width:3px;
  linkStyle 3 stroke:#1f6feb,stroke-width:3px;
  linkStyle 4 stroke:#1f6feb,stroke-width:3px;
  linkStyle 5 stroke:#1f6feb,stroke-width:3px;
```

### Diagram: per-layer merk-tree structure (Layer 5+)

Identical to [G1's L5–L6 shape](#g1--in-on-bybrand-grouped-by-brand), just with all 100 entries in the byBrand merk tree resolved as visible targets rather than just two. The byBrand binary tree has all 100 keys exposed — no opaque sibling subtrees (`Hash` ops) at all, only `KVValueHashFeatureTypeWithChildHash` (full reveal) plus `Parent` / `Child` glue.

```mermaid
flowchart TB
  subgraph L5["Layer 5 — widget doctype merk-tree"]
    direction TB
    L5_q["<b>brand</b> (queried)<br/>kv_hash=HASH[68b6...]"]:::queried
    L5_left["HASH[9862...]"]:::sibling
    L5_right["HASH[6c36...]"]:::sibling
    L5_q --> L5_left
    L5_q --> L5_right
  end

  subgraph L6["Layer 6 — byBrand merk-tree (ALL 100 targets fully resolved)"]
    direction TB
    L6_t0["<b>brand_000</b><br/>CountTree count=1000<br/>BasicMerkNode"]:::target
    L6_t1["<b>brand_001</b><br/>CountTree count=1000"]:::target
    L6_tmid["... 97 more KVValueHashFeatureTypeWithChildHash<br/>targets, each CountTree count=1000<br/>(192 merk ops total: 100 Push + 92 Parent/Child)"]:::target
    L6_t99["<b>brand_099</b><br/>CountTree count=1000"]:::target

    L6_t0 --> L6_t1
    L6_t1 --> L6_tmid
    L6_tmid --> L6_t99
  end

  L5_q -. "Tree(merk_root[byBrand])" .-> L6_t0

  classDef queried fill:#1f6feb,color:#fff,stroke:#1f6feb,stroke-width:2px;
  classDef sibling fill:#6e7681,color:#fff,stroke:#6e7681;
  classDef target fill:#39c5cf,color:#0d1117,stroke:#39c5cf,stroke-width:3px;
```

Because the In set covers *every* brand in the fixture, the proof has zero opaque-sibling subtree commitments at L6 — every binary-tree node is revealed as a `KVValueHashFeatureTypeWithChildHash` target. That's the most efficient byte-per-key shape `GroupByIn` can hit: at `|IN| = B` (where `B` is the total entries in the property tree), the proof bytes ≈ `B × (kv-hash + count + child-hash + glue)` ≈ `B × 100 B`. For `B = 100`, that's exactly the 10 038 B we observe.

By contrast, smaller In sets (G1's `|IN| = 2`) pay the boundary-proof tax: the byBrand merk tree has ~98 unresolved entries, each contributing one `KVHash` (opaque-key commitment, ~33 B) or `Hash` (opaque-subtree commitment, ~33 B). The asymptotic crossover at which "reveal everything" becomes cheaper than "reveal-some-and-commit-the-rest" depends on the ratio of `|IN|` to `B` — for byBrand with `B = 100`, the crossover is around `|IN| ≈ 50`.

## G7 — Carrier `In` + Range, Grouped By `brand`

```text
select   = COUNT
where    = brand IN ["brand_000", "brand_001"] AND color > "color_00000500"
group_by = [brand]
prove    = true
```

**Path query** (carrier `AggregateCountOnRange` — outer Keys per In value, ACOR subquery over each brand's color subtree):

```text
path:                  ["@", contract_id, 0x01, "widget", "brand"]
outer query items:     [Key("brand_000"), Key("brand_001")]
subquery_path:         ["color"]
subquery items:        [AggregateCountOnRange([RangeAfter("color_00000500"..)])]
```

**Verified payload** (verifier returns one `(in_key, u64)` per resolved In branch via `GroveDb::verify_aggregate_count_query_per_key`):

```text
[("brand_000", 499), ("brand_001", 499)]
```

Each brand has all 1 000 colors in its byBrandColor terminator; the strict `>` cut at `color_00000500` leaves `color_00000501..color_00000999` = 499 in-range colors per brand. Total `sum = 998` documents.

**Proof size:** 4 332 B. **Mode:** `CountMode::GroupByIn` routed to `DocumentCountMode::RangeAggregateCarrierProof` (the new dispatcher arm wired up against [grovedb PR #663](https://github.com/dashpay/grovedb/pull/663)).

This is the natural answer to "give me a per-brand aggregate count over a colour range" — same per-In-aggregate semantics as the no-proof per-In fan-out, just verifiable in a single proof. Strictly smaller and asymptotically better than the alternative two-field shape [G5](#g5--compound-in--range-grouped-by-brand-color):

- **G5** (compound distinct walk, `group_by = [brand, color]`): `O(k · R' · log C')` bytes; emits one `KVValueHashFeatureTypeWithChildHash` per resolved `(brand, color)` pair → 11 554 B for `k=2, R'≈50`. Carries per-pair granularity the caller may not want.
- **G7** (carrier aggregate, `group_by = [brand]`): `O(k · (log B + log C'))` bytes; emits one `HashWithCount`/`KVDigestCount` ACOR boundary walk per brand → 4 332 B for `k=2, log C'≈10`. **~2.7× smaller** than G5 for the same input data, at the cost of losing per-color resolution (which the `group_by = [brand]` caller didn't ask for anyway).

The win vs Q8 (`brand == X AND color > floor`, the same shape with `k=1` and `group_by = []`) is asymptotic: Q8 is 2 656 B, G7 is 4 332 B for `k=2`. The slope `(G7 − Q8) / 1 = +1 676 B per additional In branch` matches what you'd expect: each brand adds its own L6 commit + its own L7 + L8 ACOR boundary walk (≈ Q8's L7 + L8 ≈ ~1 700 B), with the L1–L5 prefix amortising once across all branches.

**Proof display:**

<details>
<summary>Expand to see the structured proof (8 layers — same skeleton as G5, but each brand's L8 is an ACOR boundary walk instead of a 50-target distinct-walk) — or <a href="https://dashpay.github.io/grovedb-proof-visualizer-widget/#f=text&d=H4sIAC5zBmoC_-2bW4-cx3GG7_Ur9lICdNHVXX0oAQqc2EAM5IAACZwLQzC6-iAJJsSAlu0Egf57nh4uKS3J1e6OHIYONBIl7sw3s93Vb72Hr3v-_sXzP61f_d2_vHj-fP9Gbv77o5ubf-z_tV5cnrj8eHPzH-fvn93803rx-48vT9zchM9u_uWPf_jq41_385-__ddf_9ZnNNnRmqXtZZdZYpDQatUyOi-kLrXNGdqSxFNehsS6TPpKJSZPX3zyye1ny-1n_8NvftOf_XFdfsUvPr35txdrfaxrxlii1hytxr6tbq_SfWso1WNahWGUXqSHuFP2GKw1sT1XshmbffLpzWW02nOPbbi4Bj6w93PtDjOHWEudW1IszefWUluJdTqfwGU51z7mrj8YbWS0_cX65tvbn9NblREbFpXCLK2pW6AG2mKtM7jw9BijTU9ztBqGD0lh1Zp2mtVWz8zu-9-ln9388quvn82XPz97_uf14nfPzmr94bPbpbq5-cXN53_z-od3LObLxzuW9O7C_rD44T9_atlvVy_Iq_pnt7BkWY5e19oW0hg2PU7jnSsl7anuukIbfa4cR_G4euL3eS5eNFOTT34w7neX4nZGP3n0dyv6o3V9oLr3NY_aqmISGGGNyfKaNY4d0rKarNU1V1lqmilIrroBCBfkrWPVBR7bF3eq8eoh71zLILerUWuxAsALH_l6Xab1sPtsHQTGXMJuq1kOQ1bTXvfq3rtH8Wo-atxrKFhWjU11xbXfPZDXPfL2iz-2cq_WL8i7VuAR6_Co1bgf93_-en65vr2tFiwQi5RVXteqjBzyzmnH5bTrCimMEfvoWtJU12h9eYqhGwX11heMQt1C6jG6l_YGhp9WlZePlyO8vzqPrtETKnUfhg2ibKYupYdq0Gfbc-xC8w2PY50-Ho1u06Lme5TBH-WyYh6WTfvi3mr8GJ79Rf9mvrVEeaeQSkqvl6p5sTq72Syr19BbWq5STefo1Xuy1KDjbm37Pqs2JpA3T55n2Ds-PLY3ZeC-x9vyUEYqNdqy5NKTII2btluxAasCpMJizJqnhwYzliwC0CL6aYNmbOXhsd2Vjfsej8Xcy8el7g8B74nwezII72_dy_B-F0L49OaXz__4zbcv8ZHwJaPsGj-9kctr-1n_ksn-lr-ef794hRdj0Y-HsZ5QJBHL1qLEhjiXWqOiRDNWr7liXkLZaTQNwUpZeIhU_eE1eQDRDF2uG7o2xW_Jdl2jbYWnl-za-kVjV0Bl0Q9wlYXR2_YgSE3MFcPmVnJ87NAfC_j7YI-yB5sd2xNir2tDnzkhaNqnmWDG0MOdYJHaaj52wDMEElgPPNPsjy7xK_A_6uL8ej2-H-g6rsHhi9UExkp5RNtVYqg6e951WIglL4lrQvs79y0HOSEsZNO2PHag5UkFrW8VNIed8I0zyFy94TkNj2m16Ny7xYFah-BqkXnsPLg2ynG2OeG-4Zz92HG2pxTU3lHQARxPYaA7qqiOtdNUG22E0YuC-5NkEfOlUHjD8ESfsrv6WDLmqI9urvCkioq8bct6m0Na6YZ0DWW4OW7PMn1ogAfqznCyZwfFDHoYUwAdDat_plAePdL4lJpKekdRJ3hbWLlE2cgZU0sILLa54ExmSoAgFFvTSBstKYKMbyyY3rwn3PVolIo-raj5raLWLVMqXJMIXD5Vw-xjZ_ylhJhxk2Sg2QTFo_0zWieUv2dmJ2O10R490vKkotZ3FDWtqY1fTc_D9Vpmiqat4ldID603mXVkXyPlNlJVXDzqPS2Xldwp96OH2p5WVHurqKsTovbIKWhouBkfKYoGU1FK6RHyD4ZZEBRBu8XMnxipbqx9Y8QezfrhKUWN8o6ibnIH1otMU7FqYloD_qalPkS7w7TTyeNBiyyeWTMTQfKEvKrAB_3RPBWfplDxbYnytMfwXo1x8LdMGsMW9GMOs8QIN9ScCY9LtPraxY6DLE1ioeJujx_p4wzaNTbtB2btuKHHGbarbNuV5u2-NDFLyK5e2yoE3llrqrJ6IUgluCut3BZNZmXWNHQmRA7-yDgwKUlze7RI3O_DxvNnz198evPPz7-52LA1P6YYf-r-bL3Lll0Sx6t_MmIg9xq1V04NSA0cgpK_NxaohaYoo_m5e9bqQBh7NMinhxJ3EiNxp7RF54xOEo9Pm-KPZPC_JNRePi61ewrYfgLkfhLw3obfv3_97VeXBf7493_63Vc88fnLm3hdQeNCtdpeGBU8ya74fpKY7HXulwgOcTkJOExAOOBbQQrAsFhS_-LTm2drf_v5LVu7DaS4Vo1pa04TYkE-CkqY10D6UoegdTl-90RQX6Xx-2KP0aYUPuzF119-dftpDUO1sCCRXJIKaJImOXfyCWKfiShEkoIrDK3snVNCSBmbRn5jdowLnzbOhD-POT8JUm_2zq--_nL94duXxbsg4FBOQNTzLeJrSDKSWiDIzikqmPwQmW5rBHcGVDLEL17xL4wXLkW4itIZ0nAFnZESf68Y49Ok4L7gcg80rLPE0P0mmXQqb7jwFXvvOGsb21o31maMTXKPQ7C6lXiQUd2jJVvvQAPVyxDaIJDtrI6S9Ll5K_ksIe05z6EtY_CpyA7H6Huuc9ZjO6R1vQuN7IZiVsyg1-CY6-O8D9yC1Yn8OwrWQsa1EHBIgrNHEixJotcSEenX0JBYryi7PgSN1NItNHAkIVG-kLxD_doJwH6mi6lKu0D1E5_axoiY3B7DwJHPcxe9SsXkREZ6HXzz1dAoD0ODMJiznJCrumD5VGCEXCLeVpNMyq4jd-lIFoGtnLtAM8dgIGkQLOIdaCBu27KdW1y5pDpCOJLRQ161x5ycLOqj74IFHL4AUBDZrRoeT1VLuwsNyxu7FUkzWmMDi6UorFA7bEF2HIdGGHguWgZEV-yQyBhTYp9Ld3gNjZKuqHp9CBmq9RVpAAMhwQySAcY7n9uJDLuGzagk9VXjjs2SHR6NJknHbllT1w3L9EOW16G3XY0MexgZAUy37kFGM2IZ4baBBBuLFvQZyfBhEzFcIPyzH8IUl4ZN3si9bA13kDHIpnsiPumiOTAoaYqMD_ljRCYhC980K-uGIGC9SYVIBmIydu_F_C4yMN0bSJLBbHob4PVs1Yw4Y10jYL1i2AcWq1ZoYi9TN68QirUVoLTXyEjXULWEB6FR7fVtXZQ45UgwxPaRaPZaDYHThGNczVtRfkZ0TtCgsPjDgHckgILo5AcaV6H3cqvgOmicqP8QNhj9buXMazNI2FpqxAVqwiI4JpcJ7iksNuHNajSTtmo7oeTYwmx3sBHR_wRB1EJPCzRR4AUjYHW8C4QSdwvTcZ5ipV4gkfDcrlDrxi-Mu9jwnjwNIjFj27ougTdbcx7AIzGAFgU1j_1YdCxObBvWNh5ZV_ieNeQqq5EexIa98hpMF6WD4WDJjn5Ox0QF0kI6PIvK0Cobl5HPLl_BXkSN6OfKmSvSOvO-Dr96PTbyI3gjyzgTC5ByxR_0yc-5oAZOLqh27kvu3ImuZIqOQ9BKcxhRujdsgr3hQ6249bbEEc9JFdIAUpqwY_ns5kO3BJQMdeBH8bFxKW84dxVxwLW-wRuJStZIhMGxmDgmF8bC78icoc4QYY_W4TGCDd3IauCSu294PFfa9HveuKbs5WFovKKNrvW49bOnv-ZmvUeFy7LvQsQ_23PTh08HG47tD0NwzUsG9r8WnPahy2v05HKL6UpktIdml0_UvCVFHPYR6akR-Qvx2CeugCBkZKhlbeffUkNsZ4-dULEwprKEKLPmRS-vmZ09PER5tUch50AEBs-xplKityrFx5BcQJ-2ePbhFRzbWYiee0bN4sb3yCgSLr15TQoIVy9AlEfkgHOnQm0T43fcp8wY2lF6FxgRQQ2leFdCASQVdEjL3ltrJQ4aBjzeac3wEx93W_Mv8Gm3tH1N2Z90d_vNe3JXvlMfRuOr6LED7hYDbb7IxaWhA0S6QV-UgmSWtIoFuLSvVeIemPk8e9sEkm7b42HBq9QsXh894iOyB64gMvpOj5GWBDYQR4IhMFz8jNiG45XNGslQshM1cXNjdZ3HVujd7JFJJoYvtDGdOS-IJCIFsjqhPflQxEYSfIMD2zqgUFJpFLFjvVp6w2H6DN0abm0na2kRYwruw1PASaZ27gHjV3DG0kRn7avXc2fbNjF6rbH3azheQ8OxXg2qdvU77dp3pnD1O-Xqd17dsOlBg5bllQjImDlvohnGBZEKYXkJ5sTYtNHi3uJJNCtgT-KQoRUtnuucMJwaVjiQOjdYr0BAut6ipUdYNCew9nYO_HWV2LxvrAYzbQOhrQg1pE8eC0fZ8oZDLgcQc_Ewtmi-23hJ-Syi2sa0koAHQYxYS8v2IaXg3RIpf5zYWIvujphgw0mSrdCafeS7jbdl2LmBUkPCoiG2q_O-c3QJjhMyZMTgZCzwDCdG6ZGwnRdz4UXe_LrxtLVrCl-esNHx_eO7j_43rn3slY-77rsnbcTIX89GzM5KdMQbL5sRfp7eccM-Ux02lifFxqRxjiXtNlchaTWM_bkvoNNSkr-CjZi2WyO6yjjb5KF7j1mlZ-aKLNGhfdRzAgG5IhiSokZH_APJOWySsNjPGzHvcSOmQX5Ym6qj4L-CZBvn-FVte880Ous1VtClmfwjIVseJdZzhoRLHCW5w65NPeQ2Wf_qtRzK2zgleLXxwax-SvjEUpAq2HTK2c4ZpQ3VZKG0nd7YiLG2zyF4bbZ2rhjGPjapOpdZiDzaUw_SiQC9kY3zDKT3tfBn53Q8VvL9bcTkskOB3JMqXWq1o1NKtw4cVus6AtowZt2XAU4CejyHxxJig4LFNT_MjZjZRknQ0YyD8CUYhXM_qmNcc6SxeWHOjcCxuhtvmYpGzOS5K7r2hMzuQKMTC7qec-NN6uXwBCCaDaVeIwQ88jj7EvvE2C7ArZ0jNYh8QlLR7DegEWvYK6xzO6oIOcIusXakcVKE1FAGDrcAq2O0SYgE4QLepqTL7yvvbyOm40KGjtTHUOukcFi8we20GxRYSlDGmTMF3tb7NAz62Y7Cx3uSHMuHuREzY4v9mJ9ehIDnGCQ7ndgCAQgCCT56rZUVaTQp3qwSb8bwrVPOjtjdbA6M6mlv-IXFE8JgDNgo80ZDxZMUC4yRYsOQnePDurlCOzpJvhK7C42FtyIwUelusqSZxBOlstdzs7KPc28X90iUqic0rT7PrcpcyGyMrZX3thGzzj7APN9JKVqr-jlMeE6bpsNtLUHEeTLPwniVnBZyyeccVMnNbIzL_tMHuBGTzjHY6qNQ9pE1-VzMBHnRHLDBe466Mdd52PLFldnjxAt5tNnbbHeRcbbno-xiI9IrtLhDp7WUer56kmpXxAP8FR05T_LsatAH5iIWS3mEfRcZZbgm2KWvqOiO4vpnKuQJVdtK0i7bPJAZgC3tyMq0tqn2qBbyCvb-NmIcDc0WRystZIpQz7e7oC1jUGF2PadQC4G_ZCPkj0hkId5l71itQl0-0I0Y1tCoMZMggzI9iRuldPzBIH3ts7FOLxSfuA3trUYZvSerC-T4bHd39mcLBDeuPcf4Ey3B0qJWewQNsc5zbmnnAcc6usOvQqNrPbrbE6ba4l1sVI27FoFqalp75FAh5nDOIwI_n-RExfoc5hhzd_6Pb4WVAuY8HSZ6fxsxkT7CTYljLWIqGd3UfDyc5FXzeWEdiYWfZ8BxCADJEnAlbeK-Zv1AN2JAcFnhnKSYa2A0Vqarz7YJPlLJSYSDwLQMilitI7C5o_Q4jeOi-kx3sLHzoR8_OyVC97bLnh0FmxWkIcJdCqJaiyc_R2BXa2dLdsIt6oLDfcNsIFAOeCqmdtCaaxiiFlGicYSuXM4FJ3F8bUm9EmZijcDJvGM7ury3jZiYZp6HedVPoup4qnXYFZwWl-FydqJg3QYPMnNPpe4aRBK57JiND3sjpjWZElNCE_HMAwEZdWYCakjRylg5RCJASxEXwoq6erHYKkEaEGHI38NGTMLsK4XEuZ5vrLXZqa8NZEoEJ4JhQUnGORt0vqQV6OGzWYgXPsfL12U38QPciOk9nHMI5ytSVNsKsGpa5Xi-sw0Tt-yUYFSjH8vZDDuJkXaulg9Z7Z83Yv6PNmIw6H5c8tncRg_P-Z7zBWiaJM_lOaMJa_RcNWzi-prZwgSnAbXENacPdCMmpniOdO3gKZAwzsGfWVcR2p-ZAkoLQ5nS-dZfr3LOeAhsp6ukGcK4uy8oco4E-rnV0baFDlPkRAQJzkdEgoeTRBGi8yVdLOXEhodznx0p8ZV3eNNh5rLGZftnHmvru2DbFqIS6oQNesbT3QYOzGWI6EVmLi74GOn-80bM_4ONGJV2vn6FhV8s88RO6uUL3ousaYvEb4uMssmxFTj1oXgJYl8ZPehMOXywGzGp0QqN3OQjGp57Yyk3ikaudh2HayRoKaW38yVEzA8h1i1pPN89SBfn-YPGOzcdK81Et_QNLbWxSs-ppLHOsbLjoqbluW34OayQaFKsl5RzeAs3_8bBXKs7bm_Ba6Klz5HLlBUBXsf-Lpezi3oOoMyAK7B9trwsL7O1YMmaf96I-SlXPXzNQ1f8-Os_9ur9r933yruff9ezbz_35jN3f_7hT9___dXfXv7__Pe7j7776H8AekjPWClGAAA">open interactively in the visualizer ↗</a></summary>

```text
GroveDBProofV1 {
  LayerProof {
    proof: Merk(... root-level descent, identical to every other chapter query ...)
    lower_layers: {
      @ => { ... contract_id descent ... }
      // L2..L4 byte-identical to G3 / G5 (the @/contract_id/0x01/widget chain)
    }
  }
  // L5 widget doctype: brand queried (same as G3 / G5 — opaque siblings 9862 / 6c36)
  // L6 byBrand merk-tree: two KVValueHash targets (brand_000 + brand_001), 25 ops
  //                       — same shape as G5's L6
  // L7a brand_000's value tree: single key `color` with NonCounted(ProvableCountTree)
  //   L8a byBrandColor color subtree under brand_000:
  //     proof: Merk(
  //       ... 36-37 ACOR boundary ops over color > color_00000500 ...
  //       18: Push(KVDigestCount(color_00000500, ..., 1))          // BOUNDARY (excluded)
  //       19..35: HashWithCount / KVDigestCount boundary walk
  //                 — same shape as Q8's L8, summing to count=499 for brand_000)
  //   end L8a
  // end L7a
  // L7b brand_001's value tree: same single-key shape, different hashes
  //   L8b byBrandColor color subtree under brand_001:
  //     proof: Merk(
  //       ... 36-37 ACOR boundary ops over color > color_00000500 ...
  //                 — same shape, different hashes, summing to count=499 for brand_001)
  //   end L8b
  // end L7b
}
```

The 186-line full verbatim is available via the bench's `[gproof] G7` output. The schematic compresses the L1–L4 doctype prefix (byte-identical to every other 8-layer chapter query) and the two parallel L7+L8 descents (structurally identical to Q8's, with different hashes for each brand). Each brand's L8 contributes ~1 700 B of ACOR boundary commitments — exactly the predicted `Q8 - L1..L5` overhead per branch.

**Cryptographic guarantee** (via [grovedb PR #663](https://github.com/dashpay/grovedb/pull/663)): every per-brand count is independently committed to the merk root via `node_hash_with_count`. A malicious prover can't lie about brand_000's count without breaking brand_001's verification (and vice versa) because each carrier ACOR subquery has its own hash chain back to the merk root.

</details>

```mermaid
flowchart TB
  WD["@/contract_id/0x01/widget"]:::tree
  WD ==> BR["brand: NormalTree"]:::path
  BR ==> B000["brand_000: CountTree count=1000"]:::path
  BR ==> B001["brand_001: CountTree count=1000"]:::path
  B000 ==> B000_C["brand_000/color: NonCounted(ProvableCountTree)<br/>ACOR boundary walk (color > color_00000500)"]:::target
  B001 ==> B001_C["brand_001/color: NonCounted(ProvableCountTree)<br/>ACOR boundary walk (color > color_00000500)"]:::target

  SDK["Entries(2 groups, sum=998):<br/>(&quot;brand_000&quot;, 499)<br/>(&quot;brand_001&quot;, 499)"]:::sdk
  B000_C -.-> SDK
  B001_C -.-> SDK

  classDef tree fill:#21262d,color:#c9d1d9,stroke:#1f6feb,stroke-width:2px;
  classDef path fill:#6e7681,color:#fff,stroke:#1f6feb,stroke-width:2px;
  classDef target fill:#39c5cf,color:#0d1117,stroke:#39c5cf,stroke-width:3px;
  classDef sdk fill:#21262d,color:#39c5cf,stroke:#39c5cf,stroke-width:2px,stroke-dasharray: 4 2;

  linkStyle 0 stroke:#1f6feb,stroke-width:3px;
  linkStyle 1 stroke:#1f6feb,stroke-width:3px;
  linkStyle 2 stroke:#1f6feb,stroke-width:3px;
  linkStyle 3 stroke:#1f6feb,stroke-width:3px;
  linkStyle 4 stroke:#1f6feb,stroke-width:3px;
```

### Diagram: per-layer merk-tree structure (Layer 5+)

L5–L7 are exactly [G5's](#diagram-per-layer-merk-tree-structure-layer-5-4) L5–L7 (widget → byBrand → brand_X's continuation). The difference is at L8: G5 enumerates 50 distinct `(brand_X, color_Y)` pairs as `KVValueHashFeatureTypeWithChildHash` targets per brand; G7 walks the same color subtree as an ACOR boundary cut (like [Q8](./count-index-examples.md#query-8--compound-equal-plus-range-bybrandcolor)'s L8), emitting `HashWithCount` / `KVDigestCount` ops that commit a single aggregate u64 per brand.

```mermaid
flowchart TB
  subgraph L5["Layer 5 — widget doctype merk-tree"]
    direction TB
    L5_q["<b>brand</b> (queried)<br/>kv_hash=HASH[68b6...]"]:::queried
  end

  subgraph L6["Layer 6 — byBrand merk-tree (two intermediate targets)"]
    direction TB
    L6_t0["<b>brand_000</b> (queried)<br/>CountTree count=1000"]:::queried
    L6_t1["<b>brand_001</b> (queried)<br/>CountTree count=1000"]:::queried
  end

  subgraph L7a["Layer 7a — brand_000's continuation"]
    direction TB
    L7a_q["<b>color</b> (queried)<br/>NonCounted(ProvableCountTree)"]:::queried
  end
  subgraph L7b["Layer 7b — brand_001's continuation"]
    direction TB
    L7b_q["<b>color</b> (queried)<br/>NonCounted(ProvableCountTree)"]:::queried
  end

  subgraph L8a["Layer 8a — brand_000's byBrandColor: ACOR cut"]
    direction TB
    L8a_target["<b>Aggregate count = 499</b><br/>(committed via node_hash_with_count)"]:::target
    L8a_ops["~37 merk ops:<br/>KVDigestCount(color_00000500, …) — boundary excluded<br/>+ HashWithCount/KVDigestCount boundary walk<br/>over the in-range portion"]:::sibling
    L8a_target --> L8a_ops
  end
  subgraph L8b["Layer 8b — brand_001's byBrandColor: ACOR cut"]
    direction TB
    L8b_target["<b>Aggregate count = 499</b><br/>(committed via node_hash_with_count)"]:::target
    L8b_ops["~37 merk ops:<br/>same boundary shape as L8a<br/>(different hashes — different brand subtree)"]:::sibling
    L8b_target --> L8b_ops
  end

  L5_q -. "byBrand" .-> L6_t0
  L5_q -. "byBrand" .-> L6_t1
  L6_t0 -. "continuation" .-> L7a_q
  L6_t1 -. "continuation" .-> L7b_q
  L7a_q -. "carrier ACOR subquery" .-> L8a_target
  L7b_q -. "carrier ACOR subquery" .-> L8b_target

  classDef queried fill:#1f6feb,color:#fff,stroke:#1f6feb,stroke-width:2px;
  classDef sibling fill:#6e7681,color:#fff,stroke:#6e7681;
  classDef target fill:#39c5cf,color:#0d1117,stroke:#39c5cf,stroke-width:3px;
```

The "carrier" name comes from grovedb's PR #663 terminology: a *carrier* query is the outer multi-key query that carries an ACOR subquery into each branch. The ACOR primitive itself is unchanged — it still walks one range over one subtree per invocation — but it can now appear as a subquery item under outer `Keys`, which is what enables the per-brand aggregate proof shape G7 needs.

## G8 — Carrier outer Range + Range, Grouped By `brand`

```text
select   = COUNT
where    = brand > "brand_050" AND color > "color_00000500"
group_by = [brand]
prove    = true
```

The caller does **not** pass a `limit` — the platform enforces a hard cap of `CARRIER_AGGREGATE_OUTER_RANGE_LIMIT = 25` on this shape (see [the rationale below](#why-the-limit-is-hardcoded)). The dispatcher rejects any caller-supplied `limit` outright; the cap is part of the per-shape structural contract.

**Path query** (the same carrier-ACOR shape as G7, but with a *range* outer dimension and the platform's hardcoded `SizedQuery::limit = 25` capping how many outer matches the carrier walks):

```text
path:                  ["@", contract_id, 0x01, "widget", "brand"]
outer query item:      RangeAfter("brand_050"..)
subquery_path:         ["color"]
subquery items:        [AggregateCountOnRange([RangeAfter("color_00000500"..)])]
SizedQuery::limit:     25
```

**Verified payload** (verifier returns one `(in_key, u64)` per in-range outer key, capped at `limit`, via `GroveDb::verify_aggregate_count_query_per_key`):

```text
[("brand_051", 499), ("brand_052", 499), …, ("brand_075", 499)]
```

The bench's 100-brand fixture has 49 brands `> "brand_050"`. The platform's hardcoded `SizedQuery::limit = 25` caps the carrier at the first 25 (`brand_051` … `brand_075`); each carries the per-brand ACOR count of 499 in-range colors (`color_00000501` … `color_00000999`). Total `sum = 25 × 499 = 12 475` documents.

**Proof size:** 43 638 B. **Mode:** `CountMode::GroupByRange` routed to `DocumentCountMode::RangeAggregateCarrierProof` (the dispatcher distinguishes G7's In-outer shape from G8's Range-outer shape by the carrier clause's operator).

G8 is G7's natural extension from "k specific outer keys" to "L outer keys from an in-range walk." Same carrier proof primitive, same `node_hash_with_count` commitments per branch, same one-`u64`-per-branch return shape. The structural differences are exactly two:

- **Outer dimension**: G7 emits `k` `Key(serialized_in_value)` items in the carrier query; G8 emits a single `RangeAfter(serialized_floor..)` (or any `Range*` variant) and lets grovedb walk it.
- **Limit**: G8 sets `SizedQuery::limit = Some(25)` to bound the outer walk. Per [grovedb PR #664](https://github.com/dashpay/grovedb/pull/664), this is the load-bearing relaxation — the predecessor PR #663 allowed Range outer items at the validator level but kept the leaf-ACOR rule rejecting `SizedQuery::limit`, which made unbounded range-outer carriers impractical at any reasonable dataset size (49 brands × ~1 700 B each ≈ 83 KB; with the hardcoded 25-cap we land at 43 KB).

### Why the limit is hardcoded

Two reasons the platform fixes the limit at 25 rather than exposing it on the request:

1. **Prover/verifier byte-for-byte agreement.** `SizedQuery::limit` is part of the serialized `PathQuery` and feeds the merk-root reconstruction. If the caller could specify a limit, the verifier would need to know which value was used, requiring either a separate "limit" field on the response shape (added complexity) or a side-channel handshake (consensus-fragile). Hardcoding the cap keeps the prove path byte-deterministic across callers.
2. **Proof-size bounding.** Proof bytes scale linearly with the limit (~1 700 B per outer match, exactly as for [G7](#g7--carrier-in--range-grouped-by-brand)). 25 keeps the worst-case proof under 50 KB (Tier-2 for the visualizer's shareable-link guidance) while still covering useful "top-N brands by an outer range" queries. Callers that want fewer results narrow their where-clause range; callers that want more results call repeatedly with disjoint outer-range windows.

Complexity: **O(L · (log B + log C'))** with `L = CARRIER_AGGREGATE_OUTER_RANGE_LIMIT = 25` — 25 outer-key descents in the byBrand layer + 25 leaf-ACOR boundary walks in each brand's color subtree. Independent of how many keys the outer range *could* have walked without the cap.

**Proof display:**

<details>
<summary>Expand to see the structured proof (8 layers — same skeleton as G7, but L8 contains 25 per-brand ACOR boundary walks instead of 2) — or <a href="https://dashpay.github.io/grovedb-proof-visualizer-widget/#f=text&d=H4sIAJqMBmoC_-y9XY_myXHld7-fYi4lgBf5nhkCZKy9C3gBv2ABG_LFQhAi3yRhCdGgKK0NY7-7f-fpGZL1cKanujQs1hDd4oymq6ue_v8zIyPOyYg48T__-lf_ev7j__Sff_2rX92_id_8f__um2_-V_9_z68fX3j89ptv_m_9919987-dX__Xv3h84Ztvwl9985__5Z__4S_-k-tf_-P_8Z_-y9zJ4k02LN_ZbtsthRhG76Ut5w-yxz72DuPEzJdmWzH1Y9FPbinP_Ld_-Zfffnb89rP_l7_5G__lv5zHX_Hvf_HN__nrc_6inJ1SS6XXZD35tX5njz5vCa3PlE_jMZq36CHdXGcKNka0u0-2nYb95S--eTxt8epprBlnCXygu773hl1D6q3vG3NqY-5bWh8t9T35BL6t1u5r3_57T5t4Wv_1-afffPv7_AcrE21ZKizMKT27BdagjNT7DjPy5bXW2DPvNXpYc8UcTu_55t3teOXtfvd3lb_65j_8wz_-cn_6_S9_9d_Or__ul9qtf_6rb7fqm2_-_Td__T_89jffs5mffn3Plr7c2N9f_PD__FuX_dvdC_G79a_TwonHapr9nGshr2V7pm385Mm5eO63nzCW71PTajMdz_x9s7bZSmVN_vL3nvv7l-LbN_o3P_3LFf3suv7I6v7Q4Sl2erQYeMKestWze1o35GM92-hnn3aKlcqC1F4uBsI31FvW6Qd7HH_7YjW--xW_dy9D_HY3em_WMPDGR_52X7Z5uL6HY4GptnDHGVbDimcU7_f4dJ8pzm5z9XTPKthyKWmUctK53_8gvz0jf_iHn9u57_YvxO_bgVfsw6t244ft_r_94_7785tvVwsvkFpsp_12rdqqod6abzqT43pCDmslX15a3mWWZH5mTsGNBZ3DDx6FdQvZU5qzjScb_rJV-fTr0xP-8Oq8eo2-YKV-yIYNRzmszNg8dMN9jrvXbRy-NdM6OsdrcNpKKzbvaot_Ct_WbIZj2_72B1fjc_Y8f-3_tP9gi-rNIbecf7tVYzbr2812O96Dj3xmid3KXt6nZ8sDd-w27rzatbUxeZt51h3uTT_-bM9h4Id-_WF4aCu3nuxYntFzJDRejt1JA7NqmFQ4PHOpe4aBZ2w1RgwtET9tcRhH-_Fnexk2fujXa23u06_Huv-Y4X2h-X2xEf6QKd5ZzxSoiBmHtnD-p8UbQrl3lsyqnRhKYsuHR8xhrhbOOAQZTz1We8WSPhvk7_7ufHYZXk-ua_Za2s7JyujYOBFn-Ii7rzrPynWs3Auenx3fVtvJc4Zmr_27X2twP4xKzqo9BwfrlGO-bIQ5j1taRMmRYlrjEGlqwJ91n7Xc0IFq7YTl6dbXPmf5njUaWnJCVzyl1Tn22gRlLNwMfFVq3W2ySr1ab3vGRFDOp1rS01x-xl_7d9cvWqP2B2vkAVxYLt662hyxWaoZ7-D8HhixVl7VVsF6ticQzBiDhZqt2-6lxzZe-5z9e9YI7zjjDgkcNO4dNmutbFMk3gbsFxsawNWkcG2y9zZkyqCBWPvO-9VrNL5ojey3T_of__Hvzz__5pP__btQw3cAIu8J9MeNrbU3QLpdXC0wohANcaqBSACi8R7HrRzGYpzHG3ivI5_8avOP4Vun9rrv_sHQwaODh_7Dr_7ln37zKYZkuMtqt6df8HcEXuv-0v8eh_hf-E_972-_iym4jlOKg-QBSKOsOVPDM_OK94IsU8ljWhxEfF8lEWM4V-1EkNIiPvVXH6D4ZSc95s-8anrbq9Zd1oX81NimNfxCij4Wvweccjz54ztCuHOkBVYH-XCaz8KbsQYe_b76Vctnnj2_7dmhE7MTCozYH_AeueJ4d9sL3B9b2ivCRvHGQZBkE2FnjCnWTaQ4teT46mf_MmcT22detbzRIt3KwgOFBZYIuKUx2q4trwCTDeVgnFkeIuyDG2ExGtj9rHUrIRJf9-pX7V909sYXfbd9Zl3q29ZlgK7yJqL1xOmD2h1Q3hqptU1EOwl02iM-tuLFga_xFkt2O1RzjVr6eXVIDl9kAulzTqm97VWht6fcDXOuPnnpyX9v3o__W9k6vHGeof0-a2AAAyQpb41PPrmEkl_9qukzz97f9uwh5lvgoi0k6Kb2RBwz1ugLFnXwqxsKPPmGwgEtScY7OhRjtLJ3669-9vxl2_Q5pzTe9qpABPhPthRApxhZXxbxRK2vPZeAKcjQfGB9RMVhN8y1gEsAtJiw0PTqV61fcvbS53ySvfFNfS4DVvrAKNfBD4HKY-8Pd7twSOGUuQrQ-4bUA_uZZ-lgwjEIoWW--k37l23q-OFXbeGNZy_XFGID85RT0g4D-NpCAYwRNb2Py1IAeXk1YWgDdfuKuKHJFjfM_dWv-hkX2d4IZhIbFHno2O_d1yLhfuEzvKSTKlHlZgJL39vF1suosWf40k41Amuuvxq25S9zkfkzLrK9EcyMfcHmmBbHC7IXa7MyU9j78Am8-Jj59D6h5A3uuAgRM_RUoeBpWMyvJoc5fcnZy_mLvrt80Xd_kRfI7Yu-u39mh94I2YRcOCIORRhw0EAs3nPOcuGMwNAFcFmNP_F8bw-ELY4UEbuXvLy2_Xpj_DLukz937t4I2QhkZqedWwPOooA_RStbq7d0jqO-DJsgmI_b-qn93J7LthSXhbPr61l4-MyzvxFWzVSJ09fHhAInbyDrykOGew5PCCWIoE084p5eRr_Nu7FDw4hkjXjyatde4hdtU_kMNGlvhFXh5Ai_8WB7Rly682rBUrZdaktQBVu58LVYOtGcTfIapxI-fNVvCq9-1S_yAuUzyKS9EYT5yASl2iAIxoOfOsBzMkmYXVTsriEXmPo22BHn7vRys-MbVyfKvT6IlS-jS-Uz0KS9EYQR1YD-uRGPe5oAZHa27Qg7z33BXIGYuCB4_NkxcRL1jjm31Busto7Xv-rnXKS91W-UGjh8AEbLvsOarpszbw4DxC5baQSreiwrXdbStH7j7ZxAtmqmV7Pa8mUusnzGRfY3wqpSjkFgWfAMNcDsTgYZ6yZ_4xRPSOmUkvoKzXO9juXvkNhHb7HWtV_tIusX3SjV-EXf_Rmf1N8I2bBY3Gg8MQ-IXMq6vPV4uo-Wcs04WPAbKxQt55vzAPB4asfjNNDnfDWOqV9Gl-pnnFJ_I2SDOrjd6y6utAy2F2F910vH4nFVe-6yUhi7wNhtVywiWrs174h3COXVr1o_8-xvBDPDiu8cDQdyLgy3JVs7BCW3Y1RlwRo58ch3l2DBWl29zgrXy4TM0l99Umv7sm36jFPqbwQzEDa7sbuF5HkejDBC6hpMr9dd5jkVKB32CjvPOlMfJQUvp3o_7O559TVh_aIbpfo5n_RG6BPLiL2ZsvQ1FBzqwNGc0ocNawJCHisfBQiI2VbsAyTUoMBV1zPH92vftH0ZXWrxD1IYqmOo_WbAGghz1Am7wUe2sJRMWenk2S4cdN89W-95F_M5Ugt6zvH6m4b2RWynfRHOad-XQ8LdE7x3hayxZ3WAz7auEcpsc1pRFqsNDK2PPs48rc88LNRNeDzCaq9-sS9MIv1hFmnhnZX7xh3teYtzuq-ybCsDHMES26qAVSuBQzMhmx3kTKxO4P-Oj3_1k37RtWwbr8oKvyU3_HsZYqVXXpclflOu-I0Z4x8sYbjKDPPINwCu2Jx4IC8Ox6l5dchOftwAAKwurg3cBQaDnu3eC7jq9SmdHy5nWL_65a9-_Ytv_vdf_dPDLZ39FyzGv_r85fk-N_Uoc_ju__C2Of6g4_odarhn5p6B-N0FeHmxoIydgx8yIRMIYSVGs_sogeMgRehmrURUUFX6slf8TOHPT2lqn3491u5LjO3fYHL_JsP7Q_P7v_7xN__w2OC_-K__-nf_wBf--hMZm5GQSjQhxHRvC-RWLm6ttTLcLW883VwWEwylBvxa0AX62Hz_TpDSv_3FN7889zfffpipeOTECFODtBcbFWfUxxyljqnqpB1bPcnvKQ6FyDb7BkrjqUK0E_mwX__j3__Dt58Wq4BN2z4XVnKNMINJ4V3teCGu1BlyGoJkEePJOLuzDmCsEh2tW-PTll74r1OtX2RSz2fnUx760-I9LODvgn4lJYu-RSV3Xuy5lh0LyxmIc9trSkDiu8adO5wJKaoqBlr8CeAseIYqpZZhWTxpjfENz_hledsfqtb4AdNYdtYANF7iGpv-KFYc-e6xSs9Wo83CCe-P0itwiXJAZ90yequ-9qwvTMNSKLYwH-LT3MvbvTHAHqevQsg8mFc-bXXdaQB9zsrlptVU7wWKi_elaaiQaneLkaBbiL-hHFBfzlC1MOu1OndOgkh3rNksB1dtp-9bl9-55m9NA3j8hmUvP2YarNN3F43C2SuE4XB6z5uFLIkvVUhybkC3zlNBLgPceXJkNifHrr6YBCH03m8z3_pm02g_bhoAutRmKewbmz63r8MhnaPNbQPoAWbaaWPb_NfdzVXhtm9oBxvqcYUXplEALoVNweXYGKdOPAZoNmeVSkfnuJfgYde2gnU8CuHC48YmNscdm3lpGjWVGc196eYoXFByCZAh1lS1EavyaaqZgACF3NLOStbegwVCFwoe6rem0fIbVr3_mGWU0r-1jEyQjaJl6Uwcq8p-DIjGu6cc49QfHw6djbAg1Epm1VHBDSzWsDUOD_o26x1vtgz7ccvoKlYhgsCWiQZj4tfmbI771oVfVMqnr7W6nAneskGiO6wVT9JUqP3SaQAjIgdZdevtQGoJIcVvv6oVw8zw_hHuh0vJE_-SgvypR1dRBVt_-0vL8NKAGE0XEwEScnsdOcdUM44YW-ZI4n8wPps2brK-LHtYzTCyrBrk31pGfourjuFHTaPbd4RedKpm480K4XicEmsaDnJ3TBg-H4kjRtjGUrza8FR4n7OrztZ6xJM3WW-MbzaNmH7cNnJvBb9QOk4utqYrbrACdBZCEuqpBG4s4bKTK0Hfi8PoUwIjtLaIm-uFbSwObgvehm8oTei2diQGL2K_UoZ-C5-J98GrRgJDTBGckdrZndB723yyDbjpWBEIATgBUqiKzA9rWfEYxQjts-wMmb-AWyJSOwpjoZeNkZe2fxdQ3gQ18o_ahn2HNfJcSuaAnTHzBfEj7sU15uMCBPcwe6zQbVXZd6sWooc-cXB5eK6h6IC9zX7L222jvgJszOy1dJAcsZMzPIuueSA8BPBu8AeiQ8Prq-R2JnYGp_7g4Jh_Lj29sI18MTIs44EzDzHj2G31NgcGDCJuURm07btuVHtQn5N4NeI8BGeb9ck2CuAkNIh1n5hcHAMwp9DNt3M-H4nrMCZbAaap6j3Ccu5pxPGunov6O7_xlmVvP24a37mNRISdSpdYj0EFmm6giXgJc2djEScSYTdY3HIdAz9aYXc2Su-suWpef_HNW-JJ7G-3jPFjb1fDd2WfCSoyVGM9IQRBdfg39NVwIGuXopqkuwonvnBK89XhaACUmjmpFk6RhbzJ7u3HHzF-VwjYp0LIMA9VWf7AohMQFXTUBoaFr2xXVbReeoFMY1jDw8SvQ6Ue2OMtNvKFRWHfWyD2maMZAGMTsg4a8XLA5tgYEJ4zdEHdhzMBEdjlRCB5BbSsIJLfrPad0nhy2-Hf-Ovl0fwJPu1bt_2WZf-i-8jn8rA3_mT5cWv8jnpwOJqH0ZJa_pKVtYE18GwHlt0e2EyYh0HUCv4AUz1nJD_DIV8Dsqd9e1M0S2-nHukV3OOEnrIT3IEHy0yJuNUuLhcz7euGY1Br_Hu7RI6sW9pOoIampBFsWHmJMMeyEXHtVsCqa8E3Xbik6tpqRoiNUs4BGDrw507gd4szAQvbIKjsl-Y4tI4zNh6k8n3FHORi1lRALxwfOwgkW5qsdTsuJkJ4u_NRyHzj7xDmW9xw6m82qvHmn7S3_mQOb_7J-OaffPOBzT8K0OA5310GAQROEWTvd4JHg_rRwokdM62cx8yO17OulWjWCVcDgz4ReH9rG-nB6_jAN1hAfjtEy6-AaKCo3WFd-Aj1iM6ZZuogiRo7f-ILvAH5A46Bx9fC_XdI2VEvckzZykuItlTzBkoq0PhLsPQUiRilVhALrEwXxDupuBhSEImVzaFpMISzF0RwjKc44KMS9nsFG0M_ecAEGAKctZvUfheA9G3mOyCbG2ikHpa-68586bZVfnvwIBVvWfj2BYmO3_367__uj_G9r_3O133ff_-iREz6-SRi4qwqEYa_tVF2NIBY6U70iBEGfYmMj_Syij5G8WRNDfZwj5DqhKWmn0EipvgKDrSEnKhXZk5dvu26dmpz85ZuTlzKxMmcZi_ExqYTewJrcu3kr4mYd0zEqIMo7GnseINixjJwYT4gDHi0rF3EDmEQDViBI3TAtfdkqcey2L7xRICzqWYFKgIs2jvErB4N5WPOMug17g93GbFnvoxBjMQnYgk7RULUEwG-vptaW3WJkERhcMV3x6hWrVXA_hyWZjb9KJ0jAqDuM_UVpK1s3_slYhL0Y1aoCidUiQTe64EPc9HVI5xx9d3UqlOLWpe21Uy0OklvyEK2j5mIuboCLlPXI5s3i1V1HnFnXqeUE7zgxgag80awRwXAh3L3Zgui3Z6zvzCNjfcgMt88rt0ViIc7lxghC92Jt2PXmlTaNHrebODBYVTsMTfVCu_9lKObuKI277Id7N5r_aR8N5_Tsrr1clUp_8m-s43ZjlxpnWt33A4Usp33S8Rk_vIOSDgw8c7jgErmdTsNtuA5sCblNg6DSJEahq_xZcMldtxoHfYxEzFSkOlEsofmy0w5Bq8X225lPy4YbHiMZW4_vNrdOo9W3GdNhLb4xM0jJ_xCS8Cv6ead1Z6oa7ldOCQdGCZxk73KwKfE3ZW3xWFNCGZqC8z20jTyMFf7-cU7nSGDCgHnpSRY3ZA43bRtLz5KT-UqvHlfo4GEYwELxndLxDSVzCnL0PCK9YYSeqyu3FE42xtgPZ8BOgCoa7mBtDVyxHhmA5fG-TETMQV8W2TrV-JHducC4XDUV9zx9FZvZQO34aZBB0WkNmbMJNRV19zzJU3G42wIzb0jdvzoUir-nNJVSZRq3AHfFNaMNtLGFUEGCDegq3t0-778KZ6UeXVTl1KWusRJu8UEqj9B5W2J0LEJfvGUULHqBaEPuRs-ZWbMZ_T3S8RUuIFHYlpiw8NYkJUTVRM6OmSl4CyaFBXyrHmN5eth0qN5DbxVvPGDJmIkilQnu9lTA1hg3BabUqBBrfKphNlVek-4uWeUNU4nYlrk3Rzk-ajT-J1tjJS788pZQj2Fo-yEipScv2GXmpzA0jpogS1PHZf7yNDBc4lhBp6wp0RMPSDQsmtcKti0Og23NYkekEUctyL13E1dpxEPs--MCaII7zSVksz3S8SMUrutCWe3zFvyRzhDDH_WGhYnD79hy1MHduRrLrcHz5Deg27mw0dNxNSoJtk1T6rgzrbPNDOQ327lnHEGdF0XYIQXi7D2TvhpWXFCKHP4ywRu4KBE0Ej0GCSuxSlu7SHDkWLE9lJZEPqdDhQlN0DlloaWeuFWMYtPthHHqrmcdghdLoYPz683aUXlo4EhWEzFrMbIj0KzJio3YE-jSBbo3RIxpmqqMS52e7zXsy-oMp9b1ZBqlpR38bAMBpZXA4s2XIkTfHf2Prx_7ESMgF7340VF__uohAEMSViAPoICsfzkxBTOPKQSnwJWVXFfBnAFTMneIRGD8wL5NaBMB7qCQoGnhlPrLRLmOZaGm4krV8C0fI2f1I5tfgYQHB4pu4-YiDkVILUmmAN3s4zFXb0U9VxMMHYZKpFRk1zA5NUorHAKMg8znH5b_ZqI-RMlYmZwvFmKUB6ZmLZH6eZHN8nNIG58VOvHZhFGj_qCB4O8QuLA3h80EYNTu9hgW2o_aenMVBtxOPWLJXLaQXU1h3SgeeCIo6aTBO1KpRpnstyX3AM0Is3KuyIhBjiJp-9d1aiHM8vvvQBTk3reK0DQQiI6XXBphpCA0Z-KwAgoM_J3zXT6mDmpkZKH5KwsBZDbeXL4Ebx1ZTDL9qiazNaEJG7aXxMxfwaJmDQeokGjz9HKFZtICY7MscrE5iatVf5RfRVALs84xgZTKD7vec7j2uSDJmIabFwB-ADDwo6EtqVrvAXQakCP1lVJn2vPVfWNnIEW1bMI0tBlV3-ZiIm6qlOiKg6h9TgdKh4LxO1szm5Uer-0sVYC6BJT1d3XVWVmnYP1VJir7qSTCtDszDmttiz0znpCB0ZfhCFwzgKfhX35SsDhnTSL56sGmgc2-JqI-QkSMfnnk4jhwMEDVcU5UtyhK-MyYxkXQ4vNvJ-ostdTrrrF--DMLmwnSkg36wrxZ9ARk-AkBPTQm6k_wEBmEGTwv3urISm8WZv3kY2MvrzabYG1cJYiRPuaiHnHRAzb2yo4bYym5lJdZcPWWk_X14AUh3MDe4NlPnr296MisGCYq23DIb-8OAPrpFTC3iXvji2DOwz2r66Q_ChEhh4qMQGLBZTcoMtQKBNfmD2UpzT3AcnnWowwpZqyvLNhRLVfqDUeFcR44wIhKcSl7X2rvjYOCXNtjlJ_v0TMPgRX1bNnFfQTR2o8ILqwT7DB--dat7eZ8066WyqPuh0OV7LrY6zxMRMx4Mn8yMN8Ess0iajcJKH3rBScnbKS7kNOPrr_jIBiI4py9hugP740jby2SchQ0vQl2JRQ0KoHa5mnn71rK677uSKBjPxQcYndzJfM085zAbMKglcbPM8lYofZRkwns_sz77JwqBICk2iDKql5ppXX8Ah_B5CH3ys9-qMnYjy5rtGlEeB79KHFAXV0deao8bG3nM_hZCjREHXHfq0FkyJNKjf3j5mISepMWw1wo2uEOtroDcxzl0smAR60HxEkHqhIauPEy--wodGk43pfkiHDkk5Vt_4oHHFCTLkKEKUviP2OmM6Y5uMAnCIOaRaVzOyyawBrzaeOmCJRMI1DmAkKNOrpodsOECFJOeKZwIQBX1c5rv3izUCTmNKjiQo0-H6JmOLFwLC8eDm3sH6z3goHZl0lUTR49ywBnACq91IUPG-ta0qzvu8VP2YiZjQ84Q1WRgmNvbxRtFk-Idy76npAiNsK9u_A9Ok17BHjGg3EXdfL7G27CecJwj5OLDJxX14fQ-CHvJa4N46-SRXOjf88tY_oEHWOfPT2nL3d0ffZ_JUsX1ud81hF6qV9GNXSJ6mX4g00H5Z7SpXAtVW3Bbcv6T0TMbwLPgKXqLs8gp_KpXn6XLLy-CPpsbPDVtbAnDlP54zsd9fhcKJaPmgiBjYUmwSu4vCUewYGqM6CMwmw5VACOezc2Xsu_UjuWJeobQVcJjwuvvQa2-dNOVoOj4kToVi4XbeCQM6aMLQ-BR1CIcRwWuJeMxBrQiJ4jW1PtaxqTA8nNUBI5F_Gvwao595pSctMYCmaP7AW9BpTVaJMrToQulP27u-XiDFQjgRQQNIDr3AvLpH3y6zbjj3axuENyfWoHHCr6imFWkNQOIHwlg-aiAnqi4w2Vt2giqrOBvYVYNUJI1M6RTCm3Pad4CmHWpeR2Q7WYsCden4JNkou6qILrFK_yaSAzgfz8berNMiqrFBte0VF_RdIllX9VqV6MeaT3xhAyjhinoAOzGcfSMsB8qx8E9F7SRZqDOmLSsQq9ICRAFOD3VgiEfLdEjFFSrsqr77AYtf9YbIzzCpWHRJWrMt00BB4KNTLyVqDn78PTSui9kdPxKi81RJbQNTsp2PQbvgAoD6LXy5beZtqtcaeezw05WAcuyUp4ldb75CIcYnW9CGVGlyQ2V0SAubg9T1ukeNouOi5l4pXrjR29cBKJLae550fMxFTCDVyiFtKHrcCXyEteF3B_wXcSwQmSXs7sH6pdfniF1VV4zhdnNPXRMyfJhGTA0xinLsdlnSPVNwK7u2MsnUnXObKWSf_hqgiA5e4QS13T4fDWYgfNBEj4YWgKojN2X8QUcLhgI4a3KJ1SzsANIFwGejY3E9rDW-BI_S9iOcvyzk4gbiJgyGvPjPogyhZj52wOby98_EPEeW0pO9x1X8KJqx8Gcoa7CkRs0KqKa0euxOYDAAXU9DUkqr4LE2y28udC_C7LHWLYRb4ByEDEMvbfE3E_BkkYkAoC6Zpg3Ok-Xd71t2xwA3da0VlZJhkVMH9wFQfVxTj-LRRIb-lfdyOGKyXcFUEtDMgI4HA3aL6wpquVjhhXf0-VSp0FYwxRc_Uh55VLraexA4KDPyATFYvnLC1b26NEDg4ojku1V9ygMYaFYjr82zC6ciHCPupGfkpA2rXVtWgoLZ3XpFIxFFcx4ZU2sKdqk8rOxKSell6DzDkpyuAHoKdr4mYnyYRU34-iRgYqA8NLOvDCXZmHqTmtM90KIJjsa3fXlWEx4H1ftoC3nu3sfuNafwMEjFjuY4BkT2smLIEImNoMc3abi1EnCUp7HWzeMvunuI-GXR9lnu81r8mYt4xEZNDG9JR2FvF-lljO1tet5Sy4ZlbZec5nceMIE8zRC_DgNe4xH0dI3nhXfG5bn0CgPJQRXKG5iZBc4ccysaNL1eXKBN7LzRlGt2T9-IYmKUnWOMdS7KlelSJpAUN1oNNA5OgBqco-XHKY5ieq4a4LG9lSUQNU_P3lCaTlLi4112aOliOQxbHFcVVq_7ues0y3VtYAehbw4nE5nl6Uv4x5I-ZiKl9qCY4PWi85kG2dSoBlX3YxwnIcZyeFTs9p163pryMTMjdwIywz8vbdoL2wp9cCZ01aRz3Tli9TbeoKu_VBBVgabSYbAwNbtBoWcAMEXTdpyJVxVirydVQl0tsqqmeB3yQZtMCC4hLoihWTyloqMDJWHjqulBjR95Rmqxh3Vu3jbvMIK2MhvvUFNgkAYCZVtaAol07_2n7Rk6ZW7GNecT-EFH7gImYEj1AdxIEouI9VlCNMI9fj98Rp0rrbm127klseRwXW_HV-dZLmIsvuTk_DjdZebcgJRdJ26WxSuUUwfKJOQtzW8rLzNTi6ZwqFSZ3XeOr7f-pWUqiTqmXqWzFsYJ55c5TJdaUmDbS8FE1rgWOVJRtxtkdDCTW1of1_m6JGGlC4t9mj21jHsMbHpbVawmEulwDheLKshPWNu8SVHPkQIE8sGn7oIkYc-P4EQfaivOwWZJzXxeHB8kdZ_s5LfV52GPiS94gnbUSp7eNekd52fXQH_3DKVeNUuUYTxB6LVhFA0LkORSVQBJl431ZlkNAMiJPxYl0DTR4StFtZUP547FrBK5baOm2qMmYQ54oz2lw_Lnwa5CnfLca7zMHNuq-tbxfIqblE7oSAVOTmpV6lLBWGxITstYwgz2SBGAH5KPFFnJyhe5wq58U-gdNxMQrYQBLxOroGMnEbUivEE9tKUknIEhHxFytTeXolrJe3toWMWbZk9alAsTU8GGolUd2TIJdmJ9lgCUsd_cab-0GuGSrBTRKwI-kvPby85S-TeumS0TLhbWtZU32AyMjRk2rARPw1CUggl-DXyYDbNQpdd3edzq_J1v3R0_E7KVSc86Xr7BYG0m1pz3urBWT5yF7U3mteZVc0OQp61BLxFDEzY-6xI-YiDHCRcToAUYGW25hlhWg5nNsD1FJucWZ3uluU9VjOXdjG-y5PZp97EnS0FiMWDKokwiCo3f11yz1d2tY2eEfzy4gpt8pmccGn1NvPao9euqkm1mdFjV3yelJ_EK114ZnIyap6LxsJ2hhDaMQ4UKBygS8G9CPr6T-fh0xcQ81THqOIy9dFTqBAi8B5coJrzZqT_hYKYDa5ZhctQlptqS3ZLd88EQMLFmFHBmEuOr1ojaNdeCSvLFudArWkJaospKTFT8-bzwcWZdW6Xt0xGwMLCR1hx_pj-mipxXCUcS0nJiXsy7iWpYsJoGd-DJMxR-xgQTHSB8zEaOqBomM7YbzziD-CJStPUudbOFmwNk7RI0lWAAttTbq3FXpf-JTc_-aiPnTJGKS2u5Tz1LvilZxrUOp-DEy7AJS3ZSzFHpawYun1RJuizhpJyVo-v2giZg5VbUGZewjEP1AhzVcFX1q7Dh4P-VTE-9KKCdKDCBRBkJ1YCd_nOLLnusJO-dYRoJ69Ln7wXjDavUs3eHy2yGZ2rpKwmVmgfJ-leP1qHuJnJ8iRXIcK4Q-wfQtlhvBtBGUI52Qpf5EL9ZHd1Oanp3qOF5YtYajlxr710TMn0EihtAf1oSDQk_UPg0h1RC25UPVG5Kc3kERq-Ito0GhxUI9Fc7g5aDmD5uIgSRBxLKwJA8NHoaQtSu5kZAbQHlg6zGsmi5UTMPWQBZAVWCbhqTel9Jk4TTC-e3NPYQ0GjwMZCUR6zWMk6srZMWZpd8vqEGT3tsEy6USc27PV4WxDw0BP9aAvSA_8Jp6TW_YIpsLpJeCtIYniMzgjUdC5FLWlKLa10TMT5SIqT-fREwAO-aUarTiY2OBMGeQZYJ6qJs-8jJl5IfkwOPYToxK_6uEizxS-RkkYtRmzamR1lp-zAaZcy2CmUnV1_yW5mA5B06vdjihI-GloF3rjjHC-NoR856JmG2lzIlfgqy0GeeQqJdN17VfrC5fBkg5KwelP3a5Aedm8qGl7ft8OQLNwMl5ljLXvGP2axqwC1lsNXioBU8IdNLgkQaAB-JUACBI6WylzZ-8a7t5Dwnb3XTWqkQB3GvDZ-calqbTT7zpinV3BYcKogrq-FVTZOUF3i8Rc6LExom6QbTfAXxup5sXTWkPrU_N19xlSGWH0MXJ2OL9apDILXzUGTHHapu7WIbFq78pXNUe3pWSFQDt2W3BuDTvIUmeQAOWmzqXcGPEzPZ0297dgRoqgN9Yh50hJaK4tJdd3alTBehZeaxtBr2uqW0_3fdMNp87YnSFp1ERrF9ME0O93cDm0zQZeLd8AbellVnWCh3icaTM2-Oa93YAdXu_RMzOrq5kC3WCA3bSLVLAtYIYNJVyrgB9gALsdv2GG3pekkctR3Wf8dGo_AETMY7_njtNGbVuHtjPoHLy-RDiX7lrSrrDiMYoplFoG0AKmY8aVT3my_pl3EzvKh4uKcAPAXhrdLYxgZ5mG9K8uAeUV9MQS-yaH5-MgHlrKX37H0x70GhF3fmeck_lMWLjUUPeuuCL-z4krtQkHt2NkKOBy1vtOHtYf7-OGCn6Ac3rlE5TkqIBb5dYCQ4b5tyvxj7oYg86GOUtUgUHuFSF19Xs7g-ZiAkq9ZOSD4R_HB_9HE3_iJarw4mBPX3WBT6YoymE4L55e97zJPY4v8zeHn7qbNUUl034iPNOCf_u8miNaZKnIwxwTNjlcIdpKhCkB3jF3tt-chpZvUdKC7VIKLKlKkTJww0P2JSmfgTL9yEvJNuyTEBZWfMGVtK8lvdLxJhpVsnpXTq1s0MrTrr9sgYHR2J4uevxjDM5VbpoZQlGXK3vxdFJ56NKk1kH6bUVy9SNO_F6Yh1Xw-jHloAHRlN0v9pgTdOC9A70rZtIMNncl1coOeUs7bK72bctdcNkpmGdSxkTzgh7WR8Ti3Q1E9oDfQRCs83YbTwxuZ3PtRx31aCqu2ZSbqhB4PKQntDKplkxmg8cqvB7aUn3LePENU7K5f0SMViwEf3CBpqJ4Wu0DYYPot6cDugD_vUWb_DjqEgTC_Cqq60Vy56PBNRHTMR4nZpzesooObXaiBVj4sJ1qV7WnWyz7F45fZGSfqVN3qyx0WJJTxGF3V5YFM5j4_ynJrLhM-I2jRfpRyozN0l-hvAkozTNth0avKSizKeCoHR2V1-mWgIkcjHSUBr4utr4CSBt-Wm9K2e6XZMF0tbE6avTuXqf75aIEQJz8FWTAizOIe0cB54sRNApSJvHHlkV3ivttKSvoNLcVtYxPO2jtfQDJ2IGhj5XYstu7j0b9k_YXA_d61znGg9J12p9aWqJSYot7p78-kh5fyrw_yMnYvAyh-W8q0_OH-56rzQcUBz3kuh4gBGBWWJUnUrCHpd62qTlrNLhcT9mIsalhhBPUxX0ylft_UUNzgB4fGSQ1qNu0GCSgn94IdDXre2oGy1O-zoj5k-UiMkd4FxHDu5q9g1jCEnagmHlY7jNeG4DboZ4jjIW0KTHhaeBP8FlH3VGTHMIRKt3C8VnKdxKuGgmXHldTW0FMWhOrcaJ8UtDAe9Sw_OuF9f9NNWWj7nbMOzIt6q2QYrssV1dDUPVAClnqb5F0zCkg5kjaFDtMWerBuRJxNJ5oJDXzKpSkxam7iaWhCBxWmPrEg1HUPFWageAnebkquloY4Wa1tdEzJ9BIgbWMEyyFqCvtNJ5qN1NNSmX6xMaSxTGnnzZgvKqqDWFhBXWUIJ63T9sIsZ8lFB2h3FEn9LFTZqH7G0eDTQYMNR7cSQOLptS2zS-KUKvNKLP90uxA7xNh-QXFfAmwOwZwcIDhlUgCdw4mRrZunrKHrpKQPN9x-Jkgbuex3_yZyDi7VlZ2jIWZykC1IC8dYQFW8QHTjdwf9Bks9X5aq4BMDh5oRO_JmJ-mkRM-xklYtQqDwWIGVbAf0ycNZCxSwR7pr6lizH2PLXBr-eUUEYi8tyLXank5-cwI0bFPqA23kf6a-ExU9c19iE85j2tc4PeVvO_-3Q_cXLE8tUdStA4iK-JmPdLxOgK_DEv_aH6uOPaJXiMPQwAD6hGbVglwoh7mq0fyAO8xvDA7BUw7uWMmCntsn1UUbwnaD3kLD3GoMkPBuqTcH9QwffQKOUIbgqz5Yo9cShaePKuqhuTkiT46GS19VvKo131xzi-dOF2NaW46qJKc2iaOaR01avb1bXfsyOmBwl2pThV2DIjZJCzEsBtCaJ4JSQuPRk1q-4hXtMN0y-KVhDIxwDdDylNpvlo505pXIfTtwNjY_Deum7BYtuPCiSVEasbos2kG7OlQoZ81njZLBXDMsBw6JLg5aiHDA5JCTYdNOlzG7EYAyzzdiXY7KG-ssvpwZREebo30yBG1S5J5QEHVZtLP89iWea5wzHS6kEaRaayd5f0O3iYl4mP-cXvmIgBWIC1Y1GLR8Dh4ej0HGeUm05eaeJTBSFiUUcR9P2GkzEg9TSLanzMRMzJ7Jf1x2SYR641QOUulrFnxpk8JMCCLY7-gXWkNcE9kj6MqrnjrD5VpZVxS3r01bS-D-fkpLpDAjJBWULmpOgOpmvaaAfz6jralbxXu40_JWL4ML7N0w2acQ5bSlApdUaXnA3Co0kxWZd5PGltRTVLfApcDSTI09V3S8R49qihCDxLxlSP2q8bKwUNrgqhFpdEl5KGQmiaWgJeAuVv7mOJEn7QGTGwiztw2TPW2QvuTx0LOU9Tt3js6w5TFz7HuxcoSoJG7zGUVDF8zMuyqRlMtSoaOtu6kjBb0xivOtpvarAdzVXSF22rV4To5WX6Y8BIqJ9mMP5-Yh-cZXM-VNF6Tan3DRTXeCaowLgaanTFollvj2D9iqOoPeoiP8z-qBB9p0TMccz2NjkCyW9wPBJeDYvAOyopLgWvNnvGGFwKB60pg313yAQXf0gef8REzDRdXw_AbyoukYO4a-qE8Yd6wEM5ZUcplm4NGcjFJfe0pddwNX88PXXEVELKXJzteHSox7So2XThbHit8lL8VXWo06kAXcqcxIFDVFai5fmyXbpmpxSVBYNVau8af2V1EjeUvUmfArgEse-dDtu7czgPGr078Gm_XyKmQR2IYgWm0B655ZikUja3ppzgN1WVsAq4C7shEru08l2ya6eleXx80ESMYkmt6pNaYLu0JKEzi7q1rRirfUB2ht2ocwrYGZXZVVdSk-SMrSe_sdeGY3HWOT-QqBqBAg79Zj1Ar7AY71HZbMmrFk0mA0Z028uwKZbrCWw0qDxf7mo0Ulslh019elETmUEy_mg3ajiUWzPvIUksDyVOH9jKO86IweQBUbrymyAxAPY5XhpoY4C2u0YRqJnHp9oUVCM0JsgrJVW65pxO_diJmKj2xTxH6eMhrFaJKntb6z1y4PF7Ujg-1VpN4AxcokQtLa8zOwvwaJD_Yydi2pmPKZs57MWzFYffqrlbOoam3vicQEmaQvEYoDZtDnVk2ixBcwT2x0zEbEJ0whWPGfAsU6rDtUzp3u1AOFJFv_qu6iO8pt4KcG3EyzcHzKuOr4mYP00ihtj4mDbpUmUEYOW4iAKqjNGITjDarmNoWjHHZVUJd5XYB0AHw8yfmvE_pDQZcRdjG9KQ9GrR1aoFbJug-Oq8yyYmRIhIBP1rYmtPUj9V-yQgOj0JNVSO68a0NdiypaGK1DMjH3oFs4UDCbIhjCoNjlski6zykYGzURPnS3Ps0jVL55iklCWmVDXLFJQCWEjwlkeBhIYnWtVzZYmbQUEmBPqqO-ZrIubPIBGzbtlZUg5pthYaMSH5ibGGI5nl3GPzYSN72eAceHFwFZWe3Op-CCV92ERMDkHqRta326mcqt4cK56jArHVjSK1mkctdxBmx_dLwsI1nGIoNf8yAzocxxPabpyVcapDBGcWTuE0x4lXKsqO5kdoT6xOihoWU49qLs54alr2EzQCDxBWlsEWdoO5SZ-DRxttSGAihsgz60aTn87j7OMRbJ_z-HS79DUR8xMkYvrPaEbMZPPV2XtVtKk63ZX9lJShjzEG8EzepZ82sPoIyD5LBfSgtwqHJfL8DBIxNdndEJqxpOO5YE5jPMQ1puS5O2FHV2pDRXzK1Vx1aB6o450dimz-NRHzjomYMLsl68tKDU3TrPKqPd-MhUobScNNkkZ1SOFC9Yl7WpmuGkTc2n2qLylzpJJXupoOCCWcIKOUvXbvq-yxcdfssZepcKRyzbGrCup9rpK9PWlzd93NtcfN041Sd08iXuazHl2txrR6g6FrEJ7VjZ2esbdaseA3PHt9v0TMMA4pbOqcVmvYLcYjSRvTysHMdar3znWteQjNQUIQaUrhrascPLSPmYjxElN0oGW5G0OQlEUgGFbpduK2WsY7wfMbON9OyktzH5vrAtwuaPhlIkaDHCy527ZVDj-omnN3HArUDo9nu_dQwKc7t0zU3BDYdLEbaHYHxT5Lk6041HTP06n_afJB1esmCmNTa45wD9G25Rgk_KXBD-CDlKqIIz7nPaXJorTlM-fI5rnHZoPY4vxHTADxgmEnztzR6CPdCgFBgnRNrW2-83Ex_QETMZp_OzQuXOI4e48laQO11q2cLPipK0r9QL-JU2e3SDfJeOkYb18viyQdTj9mgvxNXY-lzVmxDgXSBKUy26xxqzkGRqVQ2SR6AVfUsl2Q7dN1e380ZGlIcYhYhW-MSUWnW-mgVrMyMKHXCEMqaUryOUlEEKOYI8KL3i0Rg29sLfl03fcncGZUCT-MzHQH_xAzlNwhWME5VVUFzuV0XGDfB49dPmYipu7TfbHLU30qW-qCx89krdvGbRNtlFuXyk-9hwN5gpt6Pfj9ZjFeOo0KJkiOA70tRNP8jrUzzl2XzPBi5V81UorlCLBkyZtppoiyVhKAqE-Jfeyvqi-CXxKPNA1cmbqmjwt8MjW7WmQp3Y4Z48GCoiCMouFZYPz1_RIxSTNskq9rzksvG7YqHCOdgKObYCgVSlTNDDKvCe9YTBcDnBZwV-31gyZi4JrLe8016X6jqlBuV2z4OHFd_GgnIMdeVzORSsQv9Mqx11wgxzW8LKlLsFde_6ryvckf5BMJts13Cx1_K547JVeJ_92lalChl5Ekkqoum6fMvkQjCRqWuoNDgrTv1pKwWVPnCf77qgFFd4zJJIWXlgbC8VOhp5Zqfb9ETL8hJpP2D-Q9dTZ-XwLF5VScFDUqT9kFTz3WpkQGTtJ4lxw2MKyu_EETMXfAA3wWKb9k3sAMjKnGSNgCpyEu3AA2HvcmpBM-G8AwQ9BtVL32y0QM3pK9JpgUxZ8dN84o6N6r5a5inlrHqDANECQuRpO5CBS6gJT8bW9PnXRyEeqhahzDAvbEQavVpDya-7cKloCg4WoeNCE99o5fm-BAz-qiCe8nTXbrkYhvHTXUkb1LvlAVpISQuSwCv3jaK42djlnojfNVCzNcD5xq_sGlyVa_JbQygid1D0_e5jRPkjbB_xNtjvJLZxh0EhLNduciDWANiSuP7NofOxFjBws6N_WQ8BEEqTMf099ah0tLh9w0QWKoF96kGjVnjphVMLaopEdI_4gzYqQCzWm73ewxRWTCCVdQtB7njtqua4gTsdn4QhXYur4Pe7LLBSZ-TcT8aRIxDQ-oSVtEVVjXbURcEESrvYOwk4ASZ-eCzdtQerPXMDV0TicogY32B03E7Ar1xO6yLoVLLH73XV21FRNXHO6sErJUgzXhelkp95PwbHgMo7CXSnn48FHBdgWyINHXGIgEYQCyCaXggQAVCXCyngGOzQRrwSkBB-rLCBhP98Ez57k3XGe4lFOhOt2UzZHmhxrecQ8g1VkHkF7SItAhHtJG7nHO9lWa7M8hESM8dkOzdXxoRnyrmkKSW8IXgkeA-YSuZPaYdKf5sa3DZw5_uqto7cdNxKwVdE1Tzpb2r82ke2swuuc9YdbnSBXlHL9xeK1z3tpUxb1bHgoJLyHaUft7jdlbyRyNwjqVVIncNWpsxwP-L7DMAaplgE3YQohtRk5eeK6jKkOk8PaqzjNXoQw_x5q2pZGzc52jW_SFV9gZKhCqT9MltRGxLZ71NRHz0yRixs8nESMZAbUm997VNAp0SfV6yjl7GgWykSeu-waJZEshSmOdXV1tKq-_5ecgTaZiVVdffolVhVb5EC8b0X_5ESaV5M55lK_W1qyC3-5NI7VVdVnvXzti3jMRA_ZK6WbpI7Wk22L36D3r9n2q3mks07g7r58GA3ewnEYouyrSbnm6ONOo5AW-SFd_GMNYPkoZGoN9puSKsIpmp9a8NDN9RzHFrml4PhOo5am2vfYRNEKEE5MxQ05M2OfMI9kW50tLwo8D14zX34X3mIBK4sBuKa5q75eI6bur4esx_SNGXeKJ7d9FcGCdtobjqOun5ACFUaIKgLeS5z75zv4Y9_AREzGsM6RrwR4h78N1JbVD7BWoCwB-dLHommefskNOO0Xecm2V-JeBA3m6U42atpuXdbbtgpNz3dJmvPiFMvPpd_d5sQiCOQ7y7I2Z1bTO3Ytg_GQa8S4ArQqUNIPyJIkfAm7jgH-Dj7NBJiafcDUKhL0B9dYSo6QR23iAnXdKxMQKY4UpaADGo1FjHKnfp72w7L6VuLp8ywL-zwIcdwxoDGyDL2fNBPuYM2LsxAS3630NmElTQ7AHXbTHek5QOXa85ltF5RoBCkTL0wFLcL9o_SU33_Xs1DQGFvgmijMg80UdUksFvWX0zU5vCSRKWcq9H4xnxaX7OhjSkwDVKESYBqxN0Et8SNetFO6l7cHfLWuLhcXvCxPMXQNl4jlFOVQP5YZ3S8TgLA04floAzRbN1C61Y6RKI028JHw4unxtnlUdUleybxwS3YuU-qji-YCJmPMYV10n4IETf-z4eqiaWGgQ3pw1L3uxO_eWJLFtME1nozH__BjK8lJ-Cuht5awNNI9qTFlxEzAWe7brlLA--JqzNVbtF0DdN19feI4TYTzJn4Uj2hhNoxPq1XgNflaadZosAFQZq_R9chmJKNXz1AUtuyN_HcdVkeU7JmLwBCd73ZitRiTfrQtgWLt6M9bEI5-Am2UV1lBVgqSll1LMkzfwEz5oIqZlXMDCK0C-ulQvceFyhbcSuTkGTRHgPDic2iw3ezhrXdPWhryel-nbmtUQMR_jh9nQtE2iU6vUmGa_NVpaXYOKrWZsZkAiq7LFUL7FGQxPAUWnbudgUTfXElU4lolUKy9fpa0t9HFEqpckFaKrkDHcpjnASY78HaXJ0mghVqDOUvmK-kV4Cp-gng3wqRhBDG0TQ4NpTHBy695qqiAiOeMPmogpDfKNF77qitmVoAGSgH-DovIeUq9Qw7Gpn2AUjX7j6F6QKv8JAqxPgyiStChbnDXb0X5Kl115CQn2HwIR1qaZ7rpxKwDemJr0acRLlDx5StJx1jRH5iEhzR9n3TUrCeZgIeFP3fOnXIvEpoGAEdakCUNAHHVX_Z4A-R87EQOM4u3GAjd4CThBJefOCusQK6WK4tJwN78gZoU7DaYLZhpAJnWpDz4jRoqAsaatEZRgSWWUti5Z91aHZbLRF2_CIZUE_Lr9Mahkh_EY9dtyeodEzIxlKGUxqpCpScpL5WK4tX6kvV3n4fsxYmlGpfDo3XYs-PbNNtQPmojxq8N0wloaxqSJtmB-yVKvAFS1MmLC6WpwVsNNBbVTFF77mJUiFeuviZg_TSJmQLSOD2kyhApIT4TEtpu1m4U8JYFrjVjSNVaNEJzmlabz0N0ojsM_qjTZvn3gagloU63UXidGqQJfAsXS3EipiBE8EoFxxwwPrEFjLaTjecLLKYSq1gBJTcHFhoX3cfXPo7M4agrcGppfOaJGpJ6molDxjg7eHH30p1Kfac5pj0WjkVo4wjHBiQucD9j_2MQHtqEuifLyTdNSh_D5fJQsjVm_JmL-DBIxoAwcn4ogSvKRh6-J8wcIQEevmiH9AiFq0MSGuMK9kDrT6AZX_eH6uDNioGh1D2mAwk80N0m6FID5TEgYGsAYTw1HSmsSBm-aega-uH1yJlN6Uo_lXZNsP2xvQRLcDpwvrgjO4QgSJO4cF6l2zlDzjFnXx53gw4rFdp6lyUICuasae-UMMAP7Fo5Z6gSiJi24HB7jP6SE_-igC-fGDhVR-9LJXxMxP00ixn4-iRiHKvV81LM8NfiizAYHTHFrToLuseMSKgPyH2Ddhq42jbrXMOo-y90_g0RMTpoEUK0XnI1n4tpNpw6N5Ya6QrClc9-hUuXY0EBa6ZVIXmEtjWNcXxMx75iIgfJ6vWyY6kvDaGzKDPz_Eh5D8bBB8Mw5zUVFQ4R1zqEZ8zMUjfN42Xde0kjQ4pZXchxqVAaGuOSwIdWuSqTazumrjdAzRp01ujId4KEaH8rT6DvvNtULM26zlXu_tzmfEEqeJWxNnwizD9BYxOlWI6ilXNRc03mNU99xRsxMN64msbYNmNMU6am5YRMy3rppVHi1S8jp0q-3yCp41nFvaQ9V-n_MRAzu6Dzm8LTC8i9N0FXvesULDLMpSaU1fV02usRY2oKW7bh9AWR3ay9v2ydAEzwiapzZPV8aR94Sh16zOVqzIv31fh0GUMbVdPZHA-mSlO5-TsTMelbVzFb4-KrqU9_zPsYcLXWab_78rgZiTiCbqPkbMIu2ifRjeC7vOSNGumM-RNB7jwD16E1laQOMjomosDOPERvrfKHwwA1N-oM6wQuG7f5BEzGLZ4XUSWYVpoMFqNjuJl4naVaPyr_YZmXegKQrxtDX3KUSUC6v254EqELrmqiEjTxu7QvnBc9DjIyPEUQRE1NVdL7t1mwb6OoBTDVuBUj15_QtgHDUUq1hYfGOiX1pJmccHkte1mJTp4GF7riSBqzUVOMZN0t-T3q3REyxe_LN6aQ9rfbcQ3yIefiIj4plVx4m5xk0jmcfInJ3cHyrRNQxzgftiMHHwz81rFKT7pMEAfs-veheuyUANZRlQFBxFmqWDNJr0fX5wo_gXF5ett81W1Zh2Xw0t59tjo-Jt5QcNWYmetT17IAZq-iTo745P-CksVQIeZ9ubaBC3nFVAWByQ94KdLiy2asXDbkvDWCuK31MKESlT4t15f9L8XzX-yViCu-qfjnLox2OhKWgntVuKv-2nSwTPWEQnLC0w8C12VGtMv6V93l0dX3ERAzUaHdpqOcRllrR2srVe0gLS2htJM3U1ei4q2FNJWuKEm_MJkBaU3iZ2Q_zeOwVj5l7xI4SaPnW6Kurt6kCDvgL6krErtDbaWsXm-qG2Zr0Hp-YnGni9MBhTFZwJz4kq4-1qb6jTgBGMnXhmnRSb026t9JwEB7yIX-43i8Ro0pKHs4MHxwvSwd-cBupX0k5Nojy1YpWlq778n66FP-mhFAA3w_Zug-ZiIkpjl5gHRzGa_WWKOkwjD1fm8EfgrmS-ABYSIGQXeR97gh1qZ77pd9o8jiAl21Xhc5X7by7Xo4ydpWbHA4ut4WmfSY28FnZ031USUDXn_yGq6J6t46LAsetA-aYrPU63qQzi68OuqOQpqoCVSrTH6liCVAL4b2fNBkhcahCtaekcMvTdKX4W9Hl-OYUVatxp5OlWTA7wH7gUy52XeMntb4PnIiBfoSLw5BnCPh7jX2zq4p-wgALLmvHd7AVUBOl6gxqY7KhcQx3-R6JGM6ctsCuR8_NGgteokkRkefocKFVCI8E-WxVEgRxJjW5-sr31vJBZ8Rg4FdVLRpgXiEvS0UCIT1094Ku5PzRrapumL1gkrfrdhzDbO5wnPI1EfOnScQQ-NooCfhAqHQcaroajbhExfzRmE54BOyMO7wJfSRNvMIPg40hfOmDJmI0RrKpQqY2yxKh7HIHKtWadkrBS4AGcj0q-miE_LUxWkkU4hgGOOBJw4NQuggvQEvJInFEC5gzlgqDvZo7bjW35uqeHj5bcOLN2S3aXTafS0fbzvxJieDIDeGJedWj6tMEepkTvMbnZ0nCqFlgQ4WvNBYfdUNnlDy-JmL-DBIxawFNDbASwpZYCPwtL9V8ad4nDh8bOuUsVYdKNnNoftCdkt0EnX2CCx8zEXMkLZIPNI6nLB0fAstX_dQB-7SqqXhmkZgMIqn9HDmS2eBjYW6Vfbw8eMGz5g9v17TPCjoJ0hodTRNmp4b_gdBimh6kk9ElV7a2xFZ2mH3GJ_GcqVa1BlB3vofDW6Ce1XJUFcYqnDTwD9-iHDMsWvrIqZdhAHjdzq39NRHzkyRiWvgZSZPVFBJ2N2GNe7THGCIOZFunDqhC1NCUT6IHqibJ0tpQB_3BmvJpafwMEjGmqWnp2EmQwQkQKGs3P2MsdezBAGpbN2rMw90bKhV7DBKGWSI4a9yviZh3TMSAv6aBV45uiCFnKdVYwM_gmtoKrA0IofEFcZ0jNX_cq48MdDuXcGIvL0daCq5i1j2ku4UfnB2mJI2EnF0KXa2Vx9gOgJGrK3DnowbHtc_0HOpTbXsF-kSL9WpexBpdzGYFCVnVR-VzjKn3ce4sh6CgYp59JXQjaYJk8_0SMRzpGzu8UGMrYkljsWpmu0vjXFfQoz7U04xYVkfYVSOYV5yX_8Gaz8dMxORbdgosZjyxLdXkH0Awh9hK2qmDHauNnOcsJc24z-JkZ6VhtsWW_Nk0WIOUCYHgjbGg_lciS16wOglKhBEsg6E9lzUI7b0V1gqkemoJPe6ne7Mel9IXA9is59DgRMByBnKrbOnGENVpdPNwiYymoqnn3XeR2les6_0SMQf2I_00aaikFDkW1TWaPCddPoIrUhkrrjo5UENDoCvIPywpB2uawgedEcPb8ISjR9tqiKqqKZZeMwC0hHxO3JqB1HpoGuw6orsKDFZmi7cu6F_KAwDYJH7n0pM5LQ4-t2ji4dBMDONnF9awvGoctROXtqphr6c2cTDr6UpV9UdhwDH38nA9SMxc5nVjNX5fr7phVKgcPerWVRJZ8lixlA53e79ETJRYmwl_38qTziuedg4k-YQ2p0ZsHximEHyHN1e4tLTKNGhI2nb2MRMx4e4rAbiL09t5qWAfw9jJ0zxha9hDGcW92ENBm31WWhogxG5Za_Flik6EBd_Ql8mdclpc-sASq9ozewSQqIs9lpvVBwMLqMnWLnk-RgjEJ-EIk6xKXb2nMgFXar7LulySir_081qfp8rI9p4JtHMj4OsWjWvhP7e9XyKmnSYlspxVgCCtoVhyTLWxCndKHrLkDsMn7MzST69V88LzTqyPOdb9QRMxXrZLs5KAiM-fWeqduQ21JQAIenpkk3ivMDTXyZ0XdrZrJ_XR-XlpGwH7Ibyarpur9O8vrqfjMxZRyaBfEia8Fs7tquCOmg-2lfRUDrn4k21Ut_iYrA3Hy9p_bCI3TMT3PjXXnee6iR0CghAGx15SNjiqAF-p3fuOHTE361GtSOyktwgtwFLKLgsqC6lQlQwe5Sx1AkD8NfhoRV1OqYnsUYPwIaXJKvH5cuiAS5CFEjR_R-KAu3H6dUkxRzH4esslXny6xh1L40mTJ_t6HkhnGilbuhz9spzUjFvCZJPPSA7VGLkCyUZVBwzAcZ7TVT85JvzsPld9lIo5abB9TgekB_45HMfUVU6tG7l7ZcumFrsxSnZV30h7Gqvsc7R3S8RUoHpWR0IZ-NxxiHRKCQxc21Qn5QZbLdzHJlqrSVt-T3XyuJChq_SPnYjZx7CER5hQqlzDnjSNoW5NIXX4STOogKaHOWwDluIjgE3dZhh5lPfoiAG_wpzKwNx8KPWZdwCLgpRS0mSqMDCYs2qP6gmXHuNeKj66QrzhAUs_YCLGRgHRgzgKxA3qt9tYZdUY2BBnsWPSwEP4z6yppnAVtzAwWxKZXffrjJg_USImz3BHVoWv5RmJEStMGBKAC861Fv6pWJwaPgdUgoWq-rfHpH7gLpnXjypNdtSr3FRsCaNYycsk8qdW1OvcShY4fDi_ZGrVr_zrMX1RA-p6Oy9nxCw-alZgh6stt2vIFoQBJqMhFSXvQmy5rdXaHc95FwfZDVBbsubO-VPpKJC-5lXFWG_B4SqHk2AurR_dDQwVkT4GXRDtbtNT7nzU5AlAgRifr4mYP4NETFYve8QIprKZnjZAA0S2Q6pFdnrA8HUAdPKaXb1QvaiV2SQj6vXRjvsxEzGgyI6P0ACX_qlBGeoUBg--8p5dchJT41Yf42NAYJzIBAqqHAiOTnoZB7Iq9dNsgKltsySV50MODx-lAYFZt4gXQuYFkmBqZyP-EO7VMbNLeErE7Ab5CdZYZ1ZSmvoSYYDFNW9uJULheiXEwkxHnOnm1NJuvYAPr91H89_XRMxPkIiJP59EzD31YKyNUAK-z5rhldaAmSaY4qN7kX9rGLb3arAnGxCFrekLY8Zxz88gEYPVcwIhqms_BEjzglTHchpn6UqnqpcRswoKoAdey-5pSxJ8X_hWiONrIuYdEzGa0-OPpr3Wr80zjoMl8IcVYlNilS4LXmvheyXYD2ctwT3sWQEnK7-cEdNcnYMwaRgu4C4l9QgCxjWJukh_PTjI3NQcgSsvRSMSutTo7hh-nutLlFqXlDw_NyHpnAB_2NXQdJouVQjJ3AwVk_QU27kNEKqbmQBt_1Tz-k6JmNJKvFGK5k7s4NxwookfhFgoeybOStl8SAt0gbvmZemO5eGhF--wmY-ZiHm0FkgwS1cjUbN_edphANIUNeZGuoLhTuzFAMTXvUdNbod3hqvu8Bem0cMtmMQekq3vGJaKAmdffbEoUyNBgjpKG4BFwmfsZw-BzU3YT8VCX5rGANzOpuulbrXgQNeBDtaoEUaOx0pijWuVXjUhkS2wuZr0uVux0Lu_ozRZUdV_9IH5qksqZU27K2CXBmk3zXWQoqpuQ8D-DZIuLfpzptQ0UvygHTFZ9fdEKBXT6q60Ea_GSadKYmzA51TRnwLGXXUpPh65-ZGTKtJbfxKSiXuPI1GBKnUzYdFYOpwpdT608SE9ExlgW60sjEENWviKktfWDLLnLuVVd8Kodgo7ZBU5KLOjGndg4eT3Ju3EHMN-9IJnw4zmqhIxOnxlvJ80mcRusx6rygec8RDaOrx0njnHOATnh0vNSX1dN17VNLT_n703y5br1pluu8S6aA5Igv1vwj9j6T4o8zzd8VnplMc-hWztchUkEEEAEWmtVWWzsr-zEKM6yHEohULeJVAck_viXbJbFBtte5jC9mGdeFsLJB57KqFCV9cbWneXKthKjf3DvYZWCPpOOiK8g_cJHxKcIuJH8XHNPGV-Giw4jbF_NT_-vjJKa0GKThMEdjKh7U4pIOXSeA9Q5VCvxuykSdF11g4ky9JPSxECfdfnCjFjxKATVWkHpXokS6aRgO73yjEzd_JG4HlJbQkc6fqKrOYu9fLUOb-0EMP61gF1i6rf-lDnR7Rbpzze-Is0TNmF0lCRnCcQog3uZslfPCY_r6IiMkOvR_SvbQ1bdeUqEAYssZGzEkHFNAHhELRYpBLUp2WWWpInqL-d6EHyzozuaaxeNKamJoQ1ztz8yDrZqmlLxaQRi3p0nc7cCHvc0p5v54PSZO1w2-U09XtowLg1-X9rMVtJauaVGDaPefI3LWe2yyiV4MFOSreeLy3EhAgzVizWkphk_pLXIB6CJkcesCXvpYZzTeFazkImg6BQWz5RIgKvLH8W8CHJ1Rc7u4XuAM0wQorSoeg1xHpGlYgmaeVKNJE8YEeWAyyX-8bytRkh8UGdRS7tCwmQleqBSMLizbK2ib5VXI4ya_IuHjMBqbAlj_Fz0mRVvgVHjnOS_GV_rCbzzcMTJK-mPNPobbR6AO5A-y2vzkZGnPJhes4Bv7gQA7qvBHYglA-4w-KvSf2tfE2T7Mgt9xBfyhq2x_AlK2ENu0rP1OYz0PinCzGDbSnjqqAp7n5VKWe5iVYRy6AmQH1XD8kd3RLARGo1Dp3S0A7pKn1nIUbuBQRAnW4Vwo5ZrgrjU0fTo0uAvCyyOvhqFnL4CQPeRtIG6B12p_8UYv6dQsztNUKpFuByHnkW2TjS5JFoIijq7u2hkBskzlLkedYH_EsAPMdC1v3SQoznlRYoSG1KTTMwkoUsW34ce9dhycl1J6vJc2TZP44xWq7qGKw1jFeEyXY9Vye_bNlwtiQ_-NNmNjZwJTyCqfaQT6GVHA-AUWKkEreN6hZ5K8TcNYJFfm-XKlWvRF6eZ97SxHWCkhzcy7TIw96tS5T7jEGsULcZBDX_FGL-A4WYVuRX_ww7ahRG8sLrjhslMS19stmrSbJhdI14hAWx7cA41Qg7K-OLJ2LULgmvBvqmYwB4VTFLHaJNp5J0CSjN7wSuz_XY9T49NOVYa_HsV2mynQhE40qK4Ko0PK9EO8FU8ymWdCgZCUcuMjpOdtVDK09qtqAuDHvrlamZRyw8XJtXHayzDfdd7eoxy_bCWiWF9d3s3GaZCAcaFCSsd5-zfgox_0whJv1F0mRmXVONDiori_APpJEXXkpxWGiaZs8g5KDK3na53Z6WoY7wVCjYvH9BIWYJ7IdoyUosN-ydPASw886Ws5MrV8oXQFdHk9RAUI_8iSU3EHPv4ccj5pOFmNRyLssTxNI87mUwuId52mhzxNqs1NnXbTA3sWL1d7Rx4pAPNujmLbqemZcVYmCXOFnoVSPDMaVgW2OKRNSnXH28wKiI0nFK1cEP-6E9B0i_w5rd1k5ysQwragh5F4XkpK7V2LdasfhBfE2RBLgO32o6am4-y_j1-XOFmCwzYEutSk58wsZi5H4rO52NDGnZ8xkigfC7PYoYG4C4OvhwcCf3fGkhZs6c9nlsVmYjvTZguqZHs9fhbZrajMEWLId2d0mj7SpdOqmVtbJfz81ySDlMof7aWloJBJ00wQ2ohtrdqh6s6XFlZ81pPnXokNR61CO89U0t5EzNV50S1VKUM999_Mg4l6dcVYcbJ6zU65UTVTu7ZNaXMrwUa4Z9UJpsyzYnsDFkS6ffzQL2okn5vqLKBT22dC0V-VGzDTX_yKo2NqUOdtJ3FmLAlp5lejeHLC4BVYG9Hs8Ic6YBK1orFcJ8kZRg0OlgZ-e6HVDoivYaNYq3nVO6qfe-Vebd7GoZv6qupiJOdUUpFoGcqeXn3ZK1nuvuz4Z7OzaDWppmvfORs_NdQQYRpR8DmAWAImAOcKdmOejmWDWa79Aa9xLi2OtjhZggn4NcR7m5xV5GlnrnUYmCi9bMRk5rWEyEkcKO2TdWlors7V1l0fSlEzEuSSTeTZDmT9Wutu6l3ZsT9ETnNxNOS2jhLQkRw0rgINzcSDO1_Ga63bZDaXnpnV2bVu-pgbOjnB9M85GrBhAECSHdVttMoS17nC_bIhi855OqFyBlsq45uzRji7lr3KJWBWjVFBv4vOcwZizOImaxscYJ7L-OyT5UiInpDJW_AUSTTBwtqkI7bzLC7onkaajKSDK9b00W7BpMJKOG1Nghj7zeNxZiAikkWehJygzHHch316rrtJJ1nprilFu7MsgqGdYVUp9AjxE9ASve_OiSxn-C6XSdENtNwnUhpfPAhsFyq3JYkja0yVj0jmyKwi2NXct-n4g5xAc23nOkswDdcVRCNwtky3m3SM1MXtuViK3Lhuqpc-X0SRQ6zT9XiImPGkuSOMoaBIk1LoHLh0tom1SpXpQoEcOgZsCguZjFsuDRdzZOLt9aiGl7KdjLjmLummvQlJc6NsjxXa81ubqRfWjAZWgmhj17ytGHZ3kTu-SRdGmPbfV1aHzm8DaBGLKmV21TdoK5S4Qcxj8FccAbIcpaMMz6Hjdm05G_hNPgbDC2Kl3VIBMjn7KcWYWcfqtGtVXKAdcdNYYqbml643PSZJqTWkHYSD1TNz9DleYsaJcW5LphyIhMc2NjhzlMpo1q8tsHTJq_uxDD_rUlm7SkyuzdEnWVF0k29mAJUbda8pryUBqnZ9L69KwuMF5pnfUDhZjbQoDA88DLYl-6jwuM7Y_NMBvQTwKzHuuTJaMZhFAJLVYdvp93iF9aiPEKaJMMtaD6jlMqgVsWWSDVHgfk4KbZJ6H2GaRIK2rasdTwpNP14xHzLxVixs6txSRr-JQviZU0EcHl4bGltCgF76TMxjvqUNMsM76omfycu4X5pYWYUOMuFk4_Ryld7oFhwaJaFAdMvuKF_sGtsuRuK8AysBmlhB7O3G9Oc6pOxb6W3GEaKCrUDuQj5UA7mnUgagcilnzAYZIpOATWJQpjbsTXt0IMzKKL6UAA1Uo4p4CwWjA3qGdkoFqS1QV7pYbq8YJvusF5Ja92Z9o_hZj_QCEG5nPaqPxXJCI-CWjAT_oIjyq7-qXYXlL00tCk_I0tu5T2I1ngKQp86UQMER4WVqRxYwFKV4BRRy1AvSZJmPR-u4BEfUqjMz5myM6XEFj2fm3MJQYFq3mAXtmC21XlsSoH8dXBr3uuerv06nX2vSU_TOYnp7hVHRm95QGLB2geQlIGhvY3yRaHDp_wDK-HhI7awf0mpyZA_Q0HYHgkssC2PD8TMf9QISb_RRMxM0ZCehM0A7PBosGVMRZrmZUcljs4DuI_dEQLnQhF6sSmnlHwdP0bpMkkIEt6hGiD07JONLjPJYte6C6JSts1iGVnOMCaN8yo6f_bA7dLevspxHywEHMlFnIz7DFI31HijrDHNFJ3v6p5V53srKXAOFcZueyUhuaYGuBuv07EVLmAlNAi9CONuPxOtWs2dbqa3c7vWSccJ_mk5JKnLguIAydW7Xy_dSLq2HaDqHppSwp92XQOuS58cm52Q1zL2iI6e_Hk8E5vzSANpDiNJX_QI8aT7Tvkt1ZkB9649RW3WrlVJTjJltnZFaQHHYvqUtzs9Svr6CvBme8sxBQecZhsYw2oaIgjJmmMb9kQ1LzGauTh7s1kh1gyWVAaMCrrBUllv2pDr-fma1Ynd4X5s3TOXvznTJVptxozFxiYhVH0e_jDvKQT5TSZ8vvYQwe-pkO6lb-5Z64NyFPDuFnHDUCdYr34ZHnd8EwaEmHj7O2XX-0HCzHVNDcGKusSrbiXGBhK1GxyJ87PU6XPDHDpdZc6NysI7p4Dm4M7HE874RcWYlaSE-u8nm42AGeO0qTg7ko9mqYbMcCIrrfTmuwK-x5dA8zktJPBpq-FGA37Fze1Jkj7PTW5WnZ-XHnmZ2SW4WAmn1X672V3Pb0eqw5xY36zVma3FTkCFFGmzEIEh_Edh9Uy2JGeJcbQ6gUe60Qu9yBhMAJWi5mrOB8rxNwRdy9SVhmTm68JIGpHSgmFi79DnJLlaiFo8ixGUwVLJ-9jm1RavrMQc2bdTfZB1jU2CZM9N7EK2o3lsrHLCRJalWdlg3iMUhQ2zsryEbupvqwMeGw5q4nL7NLKkJCcLLAD35CmJMralph3alWGB6URb7WKtkoU08rbyigL8FE0y8WDtq5ZG3hxLjXvCeEe6lDmjx153oXAJ7cSvqEdNTaX8rlCzJJaSFbHv2Li9lYJY-pD1tnzIITy0HpT2ya34122lDr4ytHzmemR-PzGQozzWuTkkHlJnj2cCgWTSPo6WTZfVqRROBT7ivm2SzBZeTb2fgk2XrHGyF5uX5J2XDpz1hxuVIcG23v6kkYb-NP5tvy4dUuHOvBzdwnk6vF2hBJSJH-w8foKHiJMeaWb_LIJ5fVGFlomE-pa4YsSZ90hzNZYxRHQEcrnCjG3Zc3uQE8TSCrkXOAK3iR0f4BwY_XEInatjv74_sVL8mk8mNhAePFLCzGs80aKVuAAJWyi_SHGZZj4nea29oi3tVIycXEoIG55hbMAlg2yS3pdG_uXlvneckQPo_ekHmRrErUEkXo5Bf5VD6kHMNtHmqrpE7aqtbbf2i3Hjay-xuIoIvA2WZUFpKfi1y5HE9DyijlZnXWxFyPpRFBpPNe6lfuxQoyk3I73wJKENOaTIixuBFZ-G3C5YmyqDHi7RGBz8EjvzZxwCCe9u365NNkOLP1M4CjhQke44BMzQPQ2me-5VEevzJDHkIzpJeH3XFIAfYm6zE9MxFTpjA1YyXbTGNZMMlZc_AtUKaexeD18aN6nNDzK1KVK1d7HLveRS_zCQsxJEJYwhjzTFlRAY_2aUBagI_9cQk3dsj0uI5RWyV05eeMOj8lp7ccj5l8qxKynQWpnEI4d2KnBrAJBL43Tr7MYNWheVR00l_54qeRh-SgAxYpH-9aJmNuuS3Wo1iSRmKPUt8o-pWk09ByJH8OoiABpSL3yVkmhStoj9BFez4NhrzVNDW2HBZRSDUdt-XD3NS_QJJ45zt4a8Q4tlBN3vcbCH_BNtWW8nVgAPCTZzYav_Go2v0zMZb2T_dSbnp4IrncPc5WG4H4HBLHVKLBmXD-FmP9AIaaf2AMk1ataxSA5C8iZW0pTNmbRZVTIF1jRGVpc2wI81pLwWiysiq8txJxYT18Q0zV3BkuSgAHEN9ReZoOrsLJh-RN-ms-KC-LOVgFpGEwGKvg2iralIrjD5ItWPIfgA9WXdwF_10maH69TfVipBo9zL3i8-jhD8NS7v9v4HScTjSV5lXTmgicFn1kOk9Y3iUgFTwPRu4bkM6itAenUkpXGjv2nEPPPFGLK31OImbVlKSx5TBl-BFVYOqCtllWrKC7OMckaxhKCYORVk_SAQtrz6lj6LyjERNIXaWqC_3e8pJ01pP8fg5rB-ZE9her1qndevsyHzUXOvHnpHCX0-lOI-WAhpkJqfQsKlC03l16Irl1Gi7CZPnRiltVOU4ZdGc2H1mQyJgjxWBy_FmKiPzCPGD0eK_ZujWjNMkjER2jUZpmnu3YngrfjeRYAIjmplw20fzsc0ZFIaJJoHSWTwHJuGpBg-fQTUuq3jhznki-4dPJzjqctEsFkS5UZPliIyVJdg_eX7lvmvbfWltSB2qttOVGv2peFmmfInu8Z26JLtKA-8vZfWoipj-qGDtfX6lE1lJrB7Ozd7YEbmNLbArBOcDG5zYAbvPow5pQU8X0blpIknS_1T6yWu0NM_awdkj22HXFVMjvcjTCQSivp7suTSWEUSfG8T8TwO0mkM8hN8Yr3ec06wPMpUbhT1uLH3Fp21SQFi7gXzZrIvVGFoA9Kk6mbO6hmxD1cB68dAh_7IV_Ced3XNMArs9PkHbYUhUK6-iSfBnH7UmkytdRm33k6ecKFjQA7tUQg2hwpaMhnSYYqhAtTEjlKZ2TYRtTdt9dhqZmk4dGFyeYm27Xou2scikdX9xT_7zDGrnM0y5vHIoVdd5YRnOxpZ_mdmxPLmiTO1uNMs4B75s9x-tlNMounyUFBEUxFGChnIZCMIguslL1-rBAj5WDZFAbCFdgxyMY3eumHHXd87LNlmqG9MQiCuQQBhwm5jDzq_Tiyf-NETN4si9BbllI2K3lLqJxoVyVuyeaHwbYl4TpN6m8do827WfOeifTlzZD99Jn51r2PqtbqrvIaNw-plfHLv76wCokYWeL1mpM5t_CIz5FX87uQcMrZ4UUwhszi4YV7lHBNNM3N9XvVwBhWeqwhZUIzYdXNwtQ_128-IH9-IsbFGmR44OSKBa2HyyxNyRyPVyK1N9ZgPWhgqDTJhJJlS9vLTsnPWcNXSpNJ5a32K8Eg57ZqtFh4Ta7Soqx2ffTKmwveCCfqe51VQx9NSvY3vp7oQWSlJkiGmveZsvMrn85B9rUNaK5Nx082giptxJ4E6_P6ONgFnY-_VfbbkHuhaTJLhA6eRtAScy7kOxaTfleS9NdehJTMSovXoyCy-tvr5woxRTpjAKsQyXplzAGRn-Z3az6jJzX5ES7kWEqkqEEePKTGlLghYvMzSPeNhRhZxa0Ovos8ZNaI5KX4y7k15itdUZ13W9Ouhy5pSKWFGOXDymso8W3GMlVSR89XvaiRGF8erVhQ6SMZxZog55JZGr9Abkup-ZCPgCzVVeR9U_6PadxKDj-2Rl9bRRYpSsvyw_mk5dMknq4G6rKVzFYuXVaABI1Q7GOFGHKsqehYR5IJASD57DWz5OlAJKer4cEzwNQAV87eA6Yp5pEDU_0l4v7FhRhTkLAsFQWFfzlDLalbQiYGr8738BHk370J8RpICkCKy_8M7Ld_WV786YkYOSCqkwhiE3IHxRroM6skXOBDZ94MsJURbJKus40RZCJOGAcwkuq-sxCj3rEs66xxpdpZNEXqgH5P2lGLbAm9mYOsqunEtEnDk5R81LW1avwpxPxLhRi5xt9qPUN9hHKuXDdbZ7-QB1mCWeXp2bI0v3mbBWhBuJB-6zhwsm8txMQ1psSvNWywls4HNH6v8U-XSyg7PqklPrEVSYmE5SUT3HGs9TbfFZLIIlXl6aipSB2bAxCzVFxgAlXGLSPERYbIVsMOQMK0gYUkEADsmOtNw4PV734zyaZtW73YlktYq3MVn8KriYxkaZNCJhRV2gzSN60yQNPB9E8h5j9QiAFtxNmfloE4Uxxlacb6rDjz2WHLiggctjTCLcvYMwmSYYB6YaPyj_naQkyqsP5TK7ytamn3xwp5ychgJpb59nNAEmdKoqCqBzZNyT73VNLc_bX7UvMvgA7JtleHAxN0Slyx5xnbUQdOFPF3kLfMxdmNt0SlTnZT4QreVKfZqMBbqdeONG9ckOYZpUUDROeK4fdRzUjbs50O0SRZRQ-gwsz17n5_CjH_TCGm_j2FmCRbiAxRbEVSGbDqemOfEwC_idhB0wiZPFmtrZ1BmcniLEOobXdA3l9QiCk37Vt7l75V8R27zDAkiBjiAf8n87Qs8C-1GaSIPVxqh2SxqV396j-FmA8WYkREpyxoSRZ3SHweGB2uhhDGOEEORatsB7qNdgOR03OoVQOIOjI8r24Pdky-Hep8PnGPR6wdQgj7laD7rvKyy8tSXHOoN0dt7xuy0uTKnO9bf0k_aVSLKqxI61G-MMVvyfAWeFchyJbsADIV1ovN4INEl1t3NhLfuj9XiGEFX2j9sQv3980T3VKJlV5CjBdYqCp8jRKslOjW0iBHu94vzyf3Pr-zEHMhV1NCmzsrS86U5pC_4-SVysF6VEKYbThxkqKUctpIoNAQziWhvY2iDtPwO4ElsQp6ZmFBouNpgUWg5kwW1xlSKxqkUR7OiZK7jy43CVba69JoJsK7nlP7x2bGWx2NUEKm3XlrDgbuy2JdS-e--WrtRbegDqUyzucKMXecXA7k4DGNXtw8RF3PAIp-uTFvGqHfarfexRvP5NaoqRHuxcLThv-FhZjrEh_MsjcXJ7pusqGVEGVm40sh6pTDK76FjZAlJs7SmHzxXCa09bo0LojUJC3Fs-CFwnbyysd6CL738lHOhanktePc02rsRbj3tvl0OL8dqVZdjM4-NIic2XbDZ9Q0jN_qGvLSKXCwEXTlt6iH1q2xqpd64vrnPGIA3wLlew4bRSPaqk83UDnhU4LBSZNj8L6mTlN4mrBmmsSVQjQpjwfuFxZiCA2wjiI3xO11yD0hAwhkrCz1cWJ5X4DwZmVa0whVWAT_uo5fz7-EyX-r3gaVYNgY1wmv7qfJUkmTLYGMEoxdv2as16p19XoA4ZNm3iP0ppXzJjEkjYpa2X6Pd0YDxbRrcy9Vb0aGQkSucue8U09iU_KUUCvJYJuOWj9YiJFPSWWvEP48yOygp9T20WgZ-ZBc0y3JwhIYKT-TfrhZAjThVOJTw7-0ENMhVhKh9Vp4zOw78kklD5BKggppxqvwmgM71K5GF4IEqHplo5NQ0qv8FPtWklAsB5UZuloQYYc9wNfWjdd4hScfEM3TySStP-mWauzpdu3117Whb99nF2KDSXcExnl466qA2o3PWCa4A6LMVWdJTrC6Omtv6nC-nvy5QgyBgUtsiTC5ocBWZa8TTkmP7qlpeHUa1DfwZTplO2wbvrTPq3L2_tZCzExEidRYIHZE6K0ugp6TAe4CIkp3fbInCeBLpVHezImQijA1wNYffcrfcGhPrR87Jy3gg-ZQk7HP61y8-UQO4eNV3zjmJepDzOfpWoyhBc1svU3EwFiclbaj-v6XLzBbAtid24oO5ohQ3XneJ9VJoMoEOX6wbJzm2DbyxwoxPW5bGwRhwCfevBSvjo41DplYIEpm8XfJXaq0MCvI1HLXGSPPPTxNH19ciNHqTkCr1uQAJQ27E8doJS1SjEThO6G7k1UmT2GRYmJuQW1_wIVYHjnUP12IUTd8k-rljYDjEHbo6ktRI3QukcBCmJqdHdr6rJkA13o_LF6yVgEd7-8sxIwrr5eiIuYQYCKqCj_1cG1DEDb78rDeQ1Ens3T-565p76Edym3_SJP9S4WYXuYBB0l2MTbY9kijnxUaULsbxK4ALiqBzM7c8qTLRMIioc2QM8Hwa6XJgBGkNCLx2mU2cKYMVws8EN65NBej2VdwXbvJctqCoAYLU8vn8PHaBKYRlQxTX0SWk8HjlgDbnrsZ6NxBnjXIfyqRbGoL0nyYvUnUI66V_Y2WjtV0-HGvhmbj6bEAbQ70E-AbwCd5VpfdHOyILWQtyxrkxiwdDweh_BRi_gOFmBxUe9Oyct8yWgbzjtp8VelrpeRjD9bDAJpGA1SCVUnGoTqJTc01X1uImXVDrm-Oo8DApk4pJKZ91h1--9Cg8Nlyhk3FolSgzaLAeCbU7JnimznTEbeJEVwFv88wl8jGIGf6kU-Czo5rBtjzvErZGkrTUeSBQJ863s-D9soDsFc0mwNfJrRZbiQf9dJxWWP5BRK3nLm4qrOXWCUVeI96UZaPn0LMP1OIaX9PIaYAzdc-pXRSo3Tvk07gruUyw5CjmvZl1LHzGCXtKOP7xHq6pJp4iNV_wUQM1DSDTqXVO4VGT5qXIJTDyHyOdNev7DE835lthqODFokWy-8pn_tTiPlgISbdnBO0QCBm966e7N0IgcXshKgmKHVi3xBlsn7UeJwthuJRIszx7reDMzW0Av1CCksOt3KScEtHKMSdN-_8bAcjwQdNv5dXbvOATQ6_sb8djhTNDyQzme21PsyTd3gzMPLOk3Jse0sCDEAmTxDCdl0NHnaug3PSBwsxsg9jedfps8g8ZfqakrOZ_bIHoIgmxdkmb_nTIcVtqt8aXpaUyuxLPWJsF_h6iupW6KsWHUC5prwHmCKrKb9f9R7Cg2fL9Sxe_ggaIPXK-34be-DmJ6vmXgAtvHhJs7AHHR2WqQNb253lcaTBCGjVcWm6K7F0SiNhvw1LdR6a2qnPktPplfqnV8Kn1hUvwiPxcl-QQColyUc3bCIQ8PhmMY8PTsT4UZtWKerbWtx5CqrD9VoTAIDnGo7VKbE-66yfHVtz7QgWBwyXD39pISbYcyzmUY4lU_1fm_cGZkr9xB09kd9Cj4-iUg03jBs16pFh8m3U1yPVvLrcK4M8btvRzEJtV8J3N2qYmO_1VHkIPBYZkpc6XM0JMg5onvrbkWqobcBAb1H_tCcWSV-PmjSga-gf17tlVrGt3jpATHGF-DKaPJzH5yZiuoZ8jo-H9EkdJx-ZFMTVRxlbegXztjb5u04M-3202tLsp3QprsXvLMRAWJfLHgHqu5MBX6wvQh8UeMF3AQSEewCNQrq59O0O8UUG5Gd4KeOtbSqpGtlTLqqlytidh-Y7Wz_yi4DWkp0gAvDloupETqClp0-ejXTf-hV51fsWuWuHtchUZ_U6YtOKLLdowmrNRCrZm1_j0rjxUI4R81pqs-_PFWK2pn7OGEaEjStkayv2TcwIqsQcYgSEgmuKCoLqTtjWdQxmfXKLjwrPNxZidt3kDzlBwYukuqtCe4zEiZWeUblaZBC2ZKOmYF1luEmsXFubM78etncV_KRnKNsQk32Skm7lQ-GwmbenSDyd7DFJ_ZKhBUpjn1qfly96q-zHZWkH6Jp0v7g2IgVolUUm4zJWxbIjdNpX6hKbr4OFfYaUJYbcrD5XiAlzSS42iyr4LQPumXmGGl2uzzkosbnESzLRrJQ3aCgfTjL_C9fX_tJCDAEwjWMLlBjOY7KchUxFOEjlHfpwQ7IywE5bIqcEeOfGt2uA0tvb2pg3SWPQUwjkItZQJB_MkUd4mj00WneIG8ZLJPBKxx84SvQvvY1x6ttktmDb4wS01T2uTjqyjmWTdMYGIVVwLAkq9B4Lf97UuSq-Z-Qic72PFWLU-XYqOZnntgmQJLp6VZcJz1w2QEhzUh6rgZZi4T9rpFXZjiHm28uXS5PFs7cGnDR1G0pQMzxEAGAZ1f2lEs2VvvyUQn1qcAp9tcZj9lzjEa_704WYVfZi5capSjLgE77i9U4gaFmzLV5IyJLUrRJy0Vx0OlP9bFtqtPnxaf7CQgwbUvOX8RmbgLoleW-1NGVhno5Li3jIYu_e9Ui_pkbcdlMH15Q6808h5t8pxBSCYJHN_F116DRHks9q10jwsxFBk6BgtdnxTgtUGqJSBJF0pMpHv3UiBvSu1qxmnUDnnQRMKI46JdZINaS0615UCZX00ByXXHkJ1k7evP19Iia4Wt6SDMBEzh6P72WzC2vprALsCeoEWxJPrTXJqU4pCcc01r3vGh7W5RDS19gEoVOgeFWi2VzrFah4HOvOGhCToAb-G8oGJmdyXiZb359CzH-gEKPpS5NjcEianyYdQVJ3m7CIw8pxlgMITWk7HRaoer6geKuDeIR78tcWYnRAAeoGpl3Y_-zDm6bnqoyo9j3rrAQo3YmFD53e8O85CiTFTJ5Ifb7Z-J2aNTjZzg4ejwRVI9nwbHjZIm2XuIexo0cnx0SfESowQhvPaXJ8FzuYbLrSXKcFZxdQTYIDTPbyTfw0v4ckdaRVxrYjR6ci-Y1UpGO2wgg_hZh_phDT_55CjJo1U5QzUdLAIuB4QRx1QBsrS1nuKrLvirOoz7zFbHm5RqpKIuOU_BcUYm5Ooa5e0inXSEMFLPoMwQTLBzZVQ2K3Zk2ODWsSKVMVFfgKSc88l59CzAcLMb2PR91jm2oH4ywFwgN1gJ9auTeOvc2zZH5yvvv4uipLq8YiHarXsYcJQt-1W1ojqx2KNSS1B37YkIDrOimtqcbuPK7UNKBQEnDcXrzFut8OzprDXnKJ4w4V1atGHEuI0lGbmn4pXsOSbOs4gk8ngXLgYDOGoW7s9LlCjCa-27yal5ZyUGGfJnkW1LNjLM2y9L24Qs_NYpqjSXB-9yrK3Otj-feFhRhwQY6a42C7riDj-zZO5bHDtE5kkcy5eUVLYw5ZIuvg1TiGPIFyHfP1TDVBQQEecT46DI_4vzw_xpiCKHIWSeO6jrWun8aDIYKwcIh_Z7Zmb0YghZ-kely31k-rtWd5aclnArRT1XIdueoxoLhcsFWTF3AceQCMKvT4c4WY7jMs-cTdnI8sHM_Rwa57P4WnGfbmoS1upK8rnDPHM10k3bLMTX1pIWaW0ItdQFK1VaNquanIYBt8GXOIxv2NBepsvTZYOS_nqAvIVIklhLyp1i2CT3okftZYkJ8TlqYdyB9LVfouTOY6aPecS-5jlquwEQP46rxxc0kAlFlrVFe9ZvceAeb2WAl53JVlqiGqmJqKfTInApqNabONUWr4XCGG1RpDbZPf38mPO6l-XAge6qnrkjNUC-MK8alxppXznrnfAq4cINEvLcSQPJ6JxNylJHzDzOcQHIETddbBO7aW09qSkwQbrHgkzKqa_Txqbn71iCH23H1Dz9tCdLk6LPZvBb5PaSvzTJxFUVgTm0ibb4brAiTijbDeXzpnv6-MLglYHf6XJfOZ2Fgkj987y2sRMI57ZjFKQI_fu4Y61sYClxFm7icnYoh8fYykDvdBADtXYbWY6lnnsvprcx5wnjyzqLoi3K6ReHzvEzRd-qWFGL3zewlwydkGkhsc7e4Ue7BlvfD6RiazJCUadb1HtwrKGJcIMI-_TktNMomxQ1hMAhaV1RPJvlOhlu_squMOY8mRbcxWkTFhnUXGnwFO94Y1apDS2AoT_neICRI1CIt0Yqt3C-QUADkwBERbJ5lsJ1Z0J3asaWnUD0qTab5c3UyWehnzcEkjSpYs2zOPHJskZjxwbR5Ux06EuNbYQTwhQJ1_aSEmLO5HPsh1qmbUE7yD8JDL1HF2CSfCGrIU6HwUFXeTHAx58dBpUIW9TsSAJuu5-QK-1pUcGW9K48oAj7uaztFa2TyUGe14qictVphO8bLLz_6t3RIIYoIlm8yWZP7c1ibFw4XmAPZoSpv_LrXTqStzywvAryDeVEH9c4WYyjPpez9ivw7Z6l2F5yAVBvBnjDYWH1FPgrURH6OcrUaYrf63pxf3iwsxfbOId2dlzwRSUN2c578yeYZYD0_2rqNR9jjM9QQZZ0txdDhvrfszOPunCzGEtdViVJ-ExinVH9DJNdD41NR3ESOYpdxiUGPNiEKRpV8vUb6bVgrfWYjh0kH5XOHUmVWs3F3gFvfWXLpEAnOShtMhRUIPqqeH4ZBBt5zb3mzffgoxHyvE2D3ysL96MarKZyD1nWBgmD0RT9CmXVnL1w5F8bLhZxICOlHtYKt_aSEGsCMJpGZqUNHM2-jSW4NfbxmISW8hakD2nmZqr5m5wQZB2BL1mPmVlkqd0i40di25eoyg3h95euYu2UbyfEr8FhleAtYhlhDdWIFjHRaX81vraKvh8k2SRdRwZSgZggKJhcaCLIqQQrYIKAbUkMNIzLsPOeQF9YE9Ou8_hZi_vRAj0irztaoh6tzPUn9YUAtzhw8tCe2rW3hohhZIwdqC6u8rF7srBv21hRh1QkoiRaX3G5v6WGrvJSYg1siEjsrWqaEbiF38u0MDZ73mAKI452sFdEmO4GmtyfPIPBcqE24PG67Y4tL880pzq8mpSly4BGtifXVC1nJ5EztI4w62v1-_gUyqiYbdiGphdq4R6NfV0ZdPhlpkXRRxLg-SMZv_EPB-CjH_TCFm_D2FGBU--3OoPAHLGVIdjmbiqyRu75Ussvp-3KvcU5xMeuJIxpa2brPdv6AQ47HbWlIjCWlBWO4CEUCVx7jRh91lsVXulxsaEmAf6vkFphb1G7btP4WYDxZi_GTR8xJLCkuHn6lX3oKMTWFoeZ14Rw5VHRxdjiD3NlYCAfiMHsdsbxMxpY0xUyTnhODWHpHGdnjBl5gXw8xSVgHTtBVHJXaGeFMFjfB7g79FV2vhDttHCjZSL1PiGus2MkH2I6_rudysTIGw0HSqmaZGirmbkX5z4PrzhZhVSjxyFCt9dE2BlRb4680wYmi889CaHOqhZGqZPGraV24Z8k5s9p2FmPlLbGDCraYE52FcJ4Ajqw_2dZ-P_CAItswbp5_gvMhz_MwG8NzrNfFG07Gh6nDdSKSXyFf2owe67gRJe5FY-dbBrAxW1VRxrfURxyre55sRCO9e9nd3jyEvmAmVaM7_YR2tWNHiur2A1UHSwViB8N5RIJMhjJnz-qRHzG1TxaqiZly20j73ZGEM9epOmyNI7V9nTnJSIgUs9tBa0lztqc3vLMT0sh6qEyE5Uo3KzZOBzioRROdlp-8k4ZyTshv4cxRVZK7PJCHn-tokSbKALsFWJu_eJeKnBhdfRSU-uerJEaEEL8lmUn-pOmhGIHNKUDG9HalKre76gxTX9N1Xb1L6qqn5LFbjUk9Nmb9IVlnj3NLZi0c_HWQ5PlaIWXAw-HFaTvgb0DQrJ8DtCJdRjT829oJhVvZcHUmjPNU2CNMgi60t-85CTKp9QZO5Ddd8b5EBT4us7NkPO7wmzb1tKLPX8niXSrNh8-a3xlPSq-l2cDYEK2NLBsiJlZL8VdohJKygepXfeWK0X5btBWQt8cORyz6CVO8rg8cv9avuGlIzYhCAvNfKey8yriNgTKK3RiNIX_k5nzTeDOEv7t_Q-h8vxAAIrwoRvc5zJY51E7yB68pZI36EvVxJsovVc8aMkUce2GCVyLdDDfdLCzE-pf0lUaEEwXLfUNRQgRiDqFG7BsS6DkHKmrJTiKXy9xTLJnT2lV-bPgQzwiAQ2DkdaDrj0zjgQb6EELMcOiihHDa8Riv4tenaAjzsKvuot6ghsyHT7O0tUqOTrcQz4bDr3jrX3XcAaTTHnTSuJ7myMacOXuPg_fjnCjFbdiSyJlnrHFlkEDlgC6YOAxBRPlWVzxJ34GJXLt4Iy1mhZqhIaV9aiDnkQoLa4kYaGzHZODWdXeM9piJdVUl32yZkEC8ntD9f0kbrfRQHdb0er03eGPmB-K5tBDp04gGbvD71YM1uQkdU-pOs4QV7gdZYITYvmKy9TcQ4Lz2qw0O1lsqC4qkfjfqpeXypgmfLL9m6P8sbPDPzJqqsZV1eLZ_ziMkl-nM4EgBDfQKxAPdPO0LuPLYV5UQHPJZFl2uYlExL6uXh9PRrf31xIeZImkRu8HORLqv0OVUNIzdAI8GKQy3q8xDLp2lQmiWjukDZpB0bT_PHny7EgPxZWeTn0Mhbh913CSIsg5GdT7ELc2N9ly7nN1ZhDhoQhVqNLOT3pRMxBt4itpJl1m5L4wAjX-VjHY9dMS2HvYUlbA1Ci2vB6vmnyZ6vvIkm_BRiPlaIkc_ZkAebBFzZPe2QwOSgeQl6sNAOlgB-duC75SVtpCKtkiVsZb9C6jcWYnJJM4OF4yX05-0zm6sYXyUyRCQeV3l73Z7b7DkqNhAPJU09LMz1mikIjwNGECUDUqoTsuGQRJKoTp00qsRMWfEsdtJtzk7-uffI9FtD-e8Ik6yrUbc8oSwaxwst9t7UXhfhfTz4sInOWSVNqSpmIoUOv0gnY-s87KcQ8x8oxEBzpIm7io2uRdRUTTDWrBiLJlWlD0O4t21qsILeSJeBxSqN29K_txDDxbuNC8oUj_LTi8qh7JFQNnGkXM2DSRg0sxHV9daayDucrWdywut5kICb152TxccKfRd1S-0i8eJ9XR_32IrsqtgbRfPn8rrKdRfpvL25GEAypbooIyfSr2sCh5h3H81PExaLl9i2ziIgDshBsab2y6pmvvv7KNpPIeb_VIiZf08h5j72kqypJmMO1QOHxXonKAdEM9czl3VrVrVuycbeCdSgzcxKJcX8DR4xB4YEr1aHr69dXWf3tecD2ZGJmRJT18luyFFHahZVzKxNssuRe-4_hZgPFmL2PRMAohHlFvMFjqk_eSyAjOfc1tZJKP-DBsNpl0kE6sro9BYf-755xOQOtvBLiCXYrZlzXY8nTPYFPsnXUyskJH1-d-M31Ss1NKHHtcrb2INHGC_oXh5bkG0_CqrZNP3vBs7KaUpIjUs0Yf5l4ciflKxGLgi_uT38eWkyj-v0mXaVCnuE-ppnPrLUzmkWn2Cf7zMUti3V5CDdlbfNnUbzL52IkWBFIqFOk4X6NYnOj7GrpNQFdWU3uJKtuUvp60r4Ih1rtY4DQQuvp-2rHFPfnARUNMVyR2zqXJ92Zlk3x-nTpbhT-wwp9FpXDy637L1yiv4_w1IA77JS1_nHnDmVvfu48q8B3eQj9wera_stgTycGrCA-MNN2Cl7fNAj5oqs5lnr3TKF5xFWzb5XMoAD2LMDK6acMlfRCfuU9AYruh0Cb3ia-r-wENM8AB-BUXuc3ouG-6F5V2s-q52fq49SoQy81dttzG4rnciX6FRzvMqGF7WhV4CYFOm6yTg55gmenYCwap3wI3Whq3k3Nt2A8csceY7Le139_bi9KJ3uNhrveYZWpUh1mgbm1JaU5Lweo4PBYFS78kIujx7AqPqRp_SxQoz3fkh7ZL8bxo6n6XApmG3IHCujVS-Sa2wWY0vwNY2YVnuqGX4JwN9ZiAmmU71IOlGJ9cyj96Cu9TmlJNxmJWaPLWf1lYgGOiaNi3QS2iYnvE491Aval7V2B6O3pkmyS4KCb1f5SEG-k9QmWu37yC9iZHUs3gibzgVu89Y2xXplIYxKoNkZlhTL2mZCOcSIKs2v24tXSXtDmwZbM8hVwl2SHSt8UJrsEAwiz4UIWSOrnJtT3Sk-jTlN_d6wiJRiMC4X9jOk-1r5-IaxxG8txJD-I8mCZB0J-Np7it8jkQP6tqLhNZ70IpWPrEa45PtIuS4SY6qVV22DGUdPE1J7pW8qZ15yh6Z2nxYI1kudNzlMeAjdxMbLj-DM0uOdMsd90zbo0MFHPrUIiUstkxU8JLI7pKwYV4syAKnlrgmtu2Dy1SRqlcBEtj4oTQZ4PiMcmDCJmKga9hruYDaepR-TjnZMhFOWBxfs-RJU4LWzTijH_daJmLuygqHoPDnxkNKBmrwnjcns_XgqA5WAS8BG6UmWtA28kLKwXm6vayM1MCfpVJOYqrYQ23nppdzJG17SzgC8muugHETAfhLQuUfHkUPb6S1u9EIsk4Yvy_Hyo9tYTux4fFYasUYnEWm6nRzI2rH3m2Ez_CLiTfHzsULM4h5CH5qw4EFye1PFgDo1MOaSjhqkY0JG4gF7S1VibJE1DdxI_VdD9hcXYkI0lsBtwnqwC3Dilmi3GmZbbcoRCoh12qOsoMn7Uqxl87kI9r8OuP5wIeb26MLJYaarZoPO3tQ49LYd7YburNgYSIbbOoF73jKKeYdsS5_3GXL9wkJMTRYbl8-urJ2bXQsEd0FsUiHTYLKssshYLPUc3KeazKyvXfgkOPZnIuZfKsTAPk8XJV7NC5EwSr_nOW9hXQ7Pgz20fMm0RMLQOfogsdR6pNgAgf7SQsx1mN-8XH4ktnWp-FtaqxLxTjkyumnyIdQAIMTEzgBhACOzjIvBkPF9rnZAW5aqKuOOLvt0GNgB_0FL1LnXowRL_URipB8izi61QoqXrNjeUMSWvUwyy94ef9i-VL-E8DayhtSoCF-xnweMPNtflSMQMGlt8vvspxDzXyjE7Cy7oq6BzXBNKimPDPuG5ycwwmYtDa8r1gM8TXDSCdHYML9lN97v9YiRsDfkIkbZ3myrAOuruevGH0XnFRKfz0GzYzN2oFVP5jbqhm8flv7rUaFVh-bftdU51Y73O26SHSREJmQS6SFvlyCXKwiZtV41kZlldAelfOujUueDpwbU4-LWmFuTfxarad4vkZG0bbkYyIK0QYuGyQspKyV1if8G0X4KMf-XQkwPf08hZk_QCjRqih11H_0xeDxyp5bB0C7jhgunFshhu7alKdGU5SNMZP__Oy7yrxRiyOHHVPmfOnFZ06A8oQD9t3r01ihrQqVmKmWswINQGSbAvOrIThKtP4WYDxZizk3gtT14M8S8EM8lPh2fc4NmIKi-V009tVoh-Zb2gPHsUwF4d_WdXsvcQ7b0QS6hCWieJNKYKiHRq3qO-SyrPRdIsnlhhUB94-4C7vy43MOb4ip5q5cgd-LnVHKLfcvF9WhOXj6uKexUIWX8KvmBj73Mh8Yn5yUxfnAihn0ylC7qtQi0XSdUORo8RrDnbHv8LfseOgqQj7j8zcLslYzS6i3fKk0W8hor3iwfdfkPSoi-RVkFyeiGhBhS5zGE4wMcn--BzIfHhuNKZuq1SdWVqLv0_9WgRAKGwIGAVdTT25MHglpS1atZ63Vh58nDYc1kh_m9ecS4Fw-rbpaO1HYBuld9SzmS2I8Wio5_97HBIiTT3hNjntnzJtHvOT9YiIka-Moyf9kyTd4yNl0-4AXdgPjj1gQRqsMIi1Yj-20Bd_j4Shu08qXSZJEAoQI6ABJEk6N8oFjux6M6IaVOBhtqEgQahx1hS8obvNrciu7sNWqAu4Bi-cbT5ehOuhuDm4-pJpPqVJHyq50eZx-Lt9zjimOXCPsKPM03nWauaYITWZ436BAPwnQv8YZlELMOR9KV0aCMTMLKU-JqMXavBKX4i1p9qBDTao-3rzaHdJTIrRLuv_A09fqGMBubZJSgQ6W72mK_yA9JvX-aEUtfKk12NOxCzEueUijcUSv9sMKh-Rp_aUUUhEDRQRe1gjFKujJrqa1q2bxVb2cIJq3JLhPVHVly3fiJMHFX8UaRVv3qpRoxqbPehrQ2rFnM_9M2FVqqsjjvOSWpUPCTpywoIkttsVrmaGLHEVJASCP63ZRiS_KRhF00_1whJlhP5-4DobmNmCYPiiJLqXuHq8eMDxr0J2XSHl9cWiijXR4yybzH9qWFmGGkENAGu3lkOePOeQZ3uSVoM-XUAwWT2NRqBgfzw-KXK6gy503ntXybiqoMrY10w3k6QAgyfJUMl7q0qdSaXmSYe5dcDAAkrCL1gfSa9_sRCuAz6EAKTK6hqhSaii41QO0IIDIFHkN9UjJhnyJ4aUfiVDEi1v8nu_ChQowqKvCAu9TuAScgNpBTdKCk0_S8RyRe9NG2dCHYUDFEwfAUB6u-1PmlhRhN9Cx4tzRQF1n89tllSaeSws5sWmCSx0r6boEk2XQ_IW64RnSp3L1mFNkGgUrg3CZJ_inVmkyg0UlYyZCba1WSdZVksdNJgLGskrgFIPD72uA9kz6KJgJYq-pRkhmd_DCMiFxWVMsZnGhVstgEHheTpIFKpgNg8zlpst2l81e9knlHW16d1bt9cs_7XGAZeIvoDDPryfms5zytA4dIsL08Gp9fXIgpxPSeY5eoC3BJhVcwh1StV4Ncsn7UFQJh6J0tUR8BMAI5mb7sdJ-g-KcLMZFVHD0O1l5TxRduwmohR8k4UWOtoGbSnJrmYVmjAPbGkXlVe25mfmchJpCmia8kWR7xzi6oVgi8RCH4FlSesCsVT0uwGdJp7322mCW4lKRV_VOI-XcKMTluEl8ZrLF4Q46PrCYJVVNlMHs2C6_V8oxpL4FNQITSXasweHXUfWkhxlsqchiMt1w_6pPXedQzpipbPyJdbVul0DEI8WGF0dLIq1qS8kB6bR3NhO1Rp-wiQKJ-h0D46qf20q_3OzUHa0P9DW3wb7YcANCn2rtOXfddqCG1CvSF81dSjib4JLqvHPbIVLJFziSR9G3nzDvgQ72Iwm4ptaf6U4j5DxRipGcx812nyp80pD4lwAurV12maVoQXOkQ12JtpVOTrAFsdOBqi3Hv7y3EXI3iy4xS2ptTk7Z7HnXMAqlLlXxw3myjdAq3uTYEt1Y1DIUb4NivpL-ry6n7HAPGFmohUoFhbwH6Q8Hgujfw3Fq4c5sXSV8cTXrnayFqOu1tIobHtxqkqZyUbunNm-rOV0k1PAqgEERScGgzst1rYps3kKCsFgkbP9Jk_1AhJv49hZiWrslzcld5g5Iiq7XsK7AngynIV2kxtLagGCzLpTmqvPny2Q5A7vwFhZj0tAPNKatm63L5KxrJlLVfh0Wp10DKbNe59SuPzKu-2Sv3qqK981OI-WAhxrtM73WqpSoH4GaEEudOsnYIoR_IbprAHAm6W58yvV0tOuS2Ge9wv0VXI26GnqKI84GRaioEvlchTGpOi_DjltM---RENqpg-p0aBByu3Me7JETj16-pY4Yoe-ZroxK6qzrro9RnxljwYMvVmw6yV9gth1SPTCLHBz1igH49l2PskehwPtIpbPAelnXnQbHQ97Q1Sre2j7xjlIpbS3UC0myM7yzExAukSHPOpuOxNHclh42ePfGgARCg1jw9lTRjLWBMUETXEXHXAMi9b61HYH_ezmUhhCJDxECKrN4FlvPqkrZvZjeoncSta3AlpKyZqapC3DsBay77KZlRgXp1nCqhnk0enoBwGbWDgJdstxJLuiWby0pdTRWeCG3_XCFmVY3AuJokEwBmLekdtVoWEZYHNxeIIYa9HJC_uxpVZwCcZ9AGq6Ot7yzE7JRk3Qd3460DQqEUjewUZCBt48qUWzbFHaK3c63q9oSWq0iXSqi3vdXoLElMLo7Thhyme9LRGotg7DF3kR2MjPxYg-Wu3NeqWa5CuZD5WT1v8gC3wH9kmZAzP0O-yeMELVOiXBlHGTWGnNXAuRIxbjfPsFMd94Ir_WOFGJ3WRHmWLFaszRMyq6AFWKOfCNV0UKOGgK7HME4mispGHmyv5n9P_qUTMVWc5MJZNzHeAciZBU_SuJL10aS4jdIOhPmpu7E_WSfRs9R8pDr5qmd4dILFcwIptANGJ6KySMylrFNDnuqXDwSckHmzroLedAJB6qCp9V6iK2d1dSrKgok8ovPIdXrtM41VNR3P8rtqX4OxH3USsDha5ataXqym9blCjBhO6WMlQlaYJhmMGkzTQZtEoiEgkxZ5DWTVRbRcEq6dHpyFIT3bb52ICaXZvtJsMIAhmY89YJvHzcYcAM4VOgunxp3tVJl5lKbTc-LlUr_7Wy8reOTRpg5BlYe-e9CoBAiSfZ9JTjZrBmpXKR4cOXRpaCQ3Cf9ZeYsak4gtD9Kx_KqVkYccJPAdH-1q4AvpAwQi1ytWs9bQ1Gm3iYCSJPfnCjGlg256hWd2UuUmoGpQx2WeocIMe0LTgmvsVcnGo0h4JZ_BrWjEbpcvLcREnxCiaQS9Ry_HYRH8vxPMTQN0sAMxorFBEUHDpUFzYr331peKu684tHa-jO9LFdwSZCaU1SbtxPu1lYvqviSVFGT7M3QEIO0lthE_6vjb2mjHZKZ-PQxNLo5I8G1WCdCByx0JlDFYBwVoCvqNZtAkYp3ciOJYj3rvZwoxTWZiYAjyaQedQrAkTDYjUY3EstKaYQPbz2SzJZOaK4-4Rw-FZ5zS-e5CDFz5aV9NIGzx0BjkEgWqyHOxwtsR7jeyPPF6QKDB4IATjYG5ytnjA4UYqRfFB-ECPOVUCaYp-ZDqrhTiUpN9wNHTJtnAq2JWSc9ZLyShde07CzGSiNwHQO9rE236LotQbdbcYWFb4umxrWIzhQTNaRDIc1Luko4llJ-fQsy_U4hRA6ylq_aUvI7D3jQ1C2huZZMqB7gYPN0lHwy8luVWOaluksdUK1X-0kIMIC009T-AKCdI32Zbof7yjPUCwRrxEfRVO_CcJMoSYYqApNYj0fAVYTaQZIKBqVWo55RjMRmNB-D2Gg36UmGlsBtyUwdtT5Y5C77vO_ktyd5ELK_ABSwDfls1vk-ayUPyU6GpzavYNc127yBWdOcmLER-A5ynSAG-_RRi_gOFGPnXXlNPhzjJzL2cySacZAIwpssVMc_GLvRwj4FAJA3C7puioMCTry3EzDC9B4mg-DRW9SmWvAw2xhwQ9qFhyNUhf-TeXkX4ZjqQWpmentFez4MgAj01no9QGj-m111r6CkA4YfOw6VgvmDCqYQpdZRxKwhx5wAZDuvNxWCf8hgIrhPOqeqpY0u3rmnTzu5ecALp_AS1Xpa0YddRcoOzAId6-G0U7acQ838qxKS_pxCT7lrCYkuaTBCOFnZ3FitLAoxmpc0m5ndcvhrxZijWYWHtG6CBQOe_YSJG7syiIgu6InG_s1fTXlQDxtRJP_fElg5y3k1wg3RctpqaKd1QqJ9CzAcLMbHMVckPHegyZF6663RpoBz1fawG4VkrlJIl-FLVzR562BAIzUCU-tqJ6E0durzDEWUKYWCRXAJru25NvoDiCdu9AHnWhUSDE_kjQaZOvdv3O8q2q9Pp44GF5HynjQjEMVN1INbW-zye4JipyUJhpKWO-763S8Pb6-cKMUtyaZDxce7hfk-S2bD6lHNtoUFIbg_rFJI1zzDKD-nc2fmSWpKGx7-zEJMmlKrK9SVF9mqoEra1nJo6-G5cOdRUDiz-NFOqazKzkAzE4cWt-Vqjq4BPYoCvITeg6Y3cGiP_OMs0qTt4fUPy8PDsYTOzfjKsrpT0eJO_LQ1dzTlT59J8ww4eeOWgZk1DnZzg7GDg5kK3A9BjsTe5dzuLmgVZz-cKMboK3x1-GtWTtgmrWTYZ9wzuQIoo5AHZq1iqo_XlfA0fTdXJAOE5mP7CQkwGvMiyr6_sCUIi-f5-pC2d4TwVKKq-SCl9qQLV_IxRu-pjHexGhnudUi4xqFCWovzdh7yyp6s0XHmpAmHy1jaXbUfPtjeshbdJyGgpbr757dhsrVX5rrqHfADimKnkOfv1OgHGxzNpdNhNkq9LVxZZPUrVSm42e96PFWKmAKJ8uGrLcGZpq0_JyNgYZdr0Enapzl-i-4ZAE6GzRZf2CtdpXypNtk_UkGJTwVMVOSJe4Q6mjqvKiM0S0X7cfndp7jmBdIgzAOSwxzyP_tdvne3w3900CXSludxGSptY1JsMXZzVxGvNcAFZC2kAU3M3M8upSg2e-a0Q85hVA8a5OMB5mWNGWYMXYtkNxOOx5XclB7JxoBJqkAf3l1taPMNn_6A0mTQgbyFvZMKipDb7tgh54xGSrcnNPGQ5Ke1Zw1kEZhZ7mpv7kjfl_NaJGJ45iTG2UtMqKzVN_m3P0mq8YIIKNDQJbYGPvcoK59SUJEwAaDyhvSeUUAZkavLuSCVRTiGVN1lvJebLv8xuaLzRVGyeofHeRCa7vPYb37UuN6AcIH5dWlbkD5fXXZV8Zkuwu1Xl3liDlkjropjz6uimQ-qyBi0-V4jpV-NEvrmU05Z6DnWuQ9xQypwd5KHhqS41V0W6Szztcmafc02v91sLMZKaJZqTCGTqomGCxRqovMSgcewFInDwHtneJBqXj9zlQxysID2HV-V_2cYtomdrQSXWOBuLq0lDvC84e5e5C7EpQWh8q2HGhnTtWEEskjjeWP7IgMwSeZp5erdyATFKVyVKKH1Kd4rsdriBqYWbeTuEbc06ShR17I8VYiBbVRJpN--6lUB8dDLg4-jYdyZbAJgWATWoqkjws7h3AfPzXUL4Xy5NVkSIiQNH3uJmReKRXDuhX_PZYVmQoDfbgZBOpDEy6uELgR4Q7fSJiZgTm1yA7UR14ZLobln5liMxXhVuiRbBNTY8H2fYuED_QUZxGhm2x9vrCwsxcuas-ZIWZ9k7tXyeGNumtCNrkPrMJaKfCIgyqcAk6Y-qClMqAdp-CjH_ViEmz7gnPFluWGEsuXu6ig6-XDZ0LQGjGzDYBB5S4JO8MBAPKCJ-bSEGpnir5lPqsViclHFZh00HFOYHDE0UDBLsTF41DU-M0ImxswfVA3beZifzvb32vEUkJQRyVm4lAEKgbfyQVoiZZxcle-838qQI_HNlmM1a_zNX28G0odmBawht3qYeiCslEYA9VxjStSyufO7hJuKVqXsnkdVh7fwUYv4DhZgh1KLYGMEfAJYCFn1E3FOIwo5AUlhI3fCh1Z-yg4-H68L2Z3lQ7ncWYuokb8HxT8hsv1Nr0jxkIL4cWFwBP5HKWMfqhB1HTp7Emw0rcZH4_qoeWzVZI1OBcFvzDdQHwdZTcrdk8REbLPJAdI2WebhXyuEBZCWPVG9vnsHqnDuxELoABzxnaWNbk1w-fPSWqNpRWpfg5uDLLQ_RvFdJsL_meYefQsw_U4jJf08hRrNd7NNikOyzmgRWItEe2G53aVikQaeMvXrdYs6AmpNkqbgDhI8l9BcUYppLgMnWhEqlcrTFDuseknxD5E4uKY-dAKOcLYTeAvdYYgEe_GpL_SnEfLAQszSeZ1UC66BqNRgfiY_zL9mkhZdqsxEyuKSAGGq6rNWQYtcJOpH2VfiRmHbjrrnxttuAPe8NyptlFk0Zeh55Z7Xh1aV-R3eDLd8Kc1Vny2jvXapd1RxYcgU0kgKsAIqI_mCn0av8QAA5zdq0sqNxB5rwHalPllec5XyuEDPOIX8U39NIvzyiosfVhtqyT7HDvc-qJuwbihSykvMZHrLVaSPf-p2FGADq1HE6KdaGLZdzWp-h8rZONJXHUrmbpVCXpIMIGNxf5AP1ENvSa5OqtLZKyqTumtYZrBMgrhqUIc8j5pSGhOBvai7T5nj8Nh7lvjw0r7WWt7ORCzXUcFGb0EIr6pYog9_aontfUI0jYd9IYo7qx8hBMk_cBPxc8fVzhRgZ4gK-yf2ySbKYJsxbM7RsiZNnU_vZgTxdOSFJcQoqALgZe2tlhy8txFhL54atYXtCeoak3NrJVGXuaqCxCi7Lu7H92Y9FpXjQ1FYNwdi267VG56O2BAQlDkXNN6hbKJzYnNjDKslNZuWdxycZrzWsdHZVqzGVXaRG-0aG-uguQ_N5sxqn6yVggYtXIsVyDRcWCtNsjxYSAHFD2x5H081D97M_V4ipbJ4tgaHYt_HYCJxTsmk8TpaIW055HIlIm6aIapRrwpFQWyxwvC8txICZkzZ4HDk8d0YslkAd8b427udozKertMR-HOOkFOXVyDuTmvCbxFAPYZTjauXmIazS5zP74JIb7E3T5Qq2DWIbYmY57jStk37kI5B_2ej8nk-upLEWyF9n84QbkH8H5Kcot8kT1t2q1smgiLwHg1pjn_vrpNd_71f844WYnPpm0ULnd77gQtuLh8cq3mq2q0AluaeUJvdZecY7j2hZg_QU1Z_KlxZiYJqsjT5zJNRFNmDONY7BNkjxiCzFAOCc1fZtvBJu2vlEmXJL3ym-JpSWetorZ7kn9QCOviwKFsJMwb2Qmvp55ErZL_IxIzG0ZEnNp8dBtm-iInvMC0kjEEgc8JiKNYAOr6WqhlxVmYlc5sjVvJycpCUkpT32J99XP1eIuebSNoXZx7R7T-nMyIKvQB4b5UrAG-apJnVguIQfThoJ0DFmlNvQ_dJCTNERbr8we7YgzDnsrNnHKHEt8skAAhQn-jdLErM7KkuWTrYMh_BxX097yQG8eIvqxeSVEe_Zw1mluAmi3D52m3uRdlmQx-XHNOyER39uknzf-py5lhX5CnkZpgS1v-cEvmGz0HqsEjI4FcqW2YNcJZezTpBzVSR2p_k5abI2JPG5xyJjngBE8xoaeaXNW0xa9JUHNnwW3bUUO8oRagJ8H2LM_fKJGPNEsg91SG6_LpiiWhMBc31eZytIKVC6yoWXHbqrDttGmL4UGcYzAvenCzGSY21stlFaZJkkyQa0TkKfU5aAh9VrxwEjcv0ujdg9VLnrdn1Motm3FmI0X7ULIRjGUk4ZcAD44llRuv09yJBxlSO9QFYRtyeb5PPMmFUC5U8h5t8pxCTQUK2yMDvQZLhyhbMV6JqEJLb1tCyxHsmdTd4UM2Qi3ajsorBieMxUvtIjBvDmWS19K9emJtJE_AIYyS52F9YnnwEgsQ8lTJgkJ9NzKMPgYjmnt5lrkqWmEoQgho6CIYware1zs-olvaGpyj6KRJfzXrfd07uReyLk9Y17CIkpeflIUizNk90gB01lDfIDfKPKibcNwjS_Tk1n17mFoZ6730v2P4WYv7cQAwQFkli4EHsNx8yqnkF1lD0e8gVw2UKMZbGwWjzAmdyLW1s-zpy9f20hppi851jjtanj2o9k5lXIlLmL7G8hJD1Xgv8SY7pHdtC1n_yog4_XHjtrbKxQLOwoewqdiqjj8sQ5Sd_p1A0HUtv-rewMzxNex5f4JIb9L7WDTsCA5BbVfI_m0LUpap00LD5yV7cvu6xlCHcpwouj3bn9gu2llv9TiPlnCjHl7ynEqP3IN8stetF41NToFEsvq1t0kRfWIA-MqQPoXU9LlQwCgbC6J9H9_hWFmOyw1ixtDe6WfaR0OHtqC86Ydfb-WML1rmEz4FwkUeps1oAFl-T2U4j5XCFGpxS5VyhLO1W2totkUOUSPJI0PsglToyNALxTT9cpxZFTniQAJEn1irLTktFHyAuAITGRmRuZh-g381UrVY-P7GNb90qGqkp8orc-K4CRz74N-sYCwypmk5_nGswh2ktWs0nt1gQcZ9Ghr-YH1tQsWVaDNF8xUzb_XCGmRu5xhisvbeg69GuCxnSsDn1ZMXOvPJYcYkmAYh4bZL3U1JuMBOdDwL6wECMzwpR0ntEkv3lDv2Q59R6Ma6EAeY9a-Fdi96rsMWfO_6-9c0tyLLeB6P-shiRIglwOn7uYvfvknXC4S57Hh22N2lEdHd2qkqqkywsCmQSQkKVgQtzZel4k_UuHMydrkiuamn-gJekx5KBi3n6IjIXw7isCT3WXlZbljmteTHgpbm9H1JW_sw9-_t6VMLFeIz5F9asJqBwmIXnvOtwGsdf2bKvmsyzW8r5EjJ3R941-Wt5A7bXLKE2CGXzGfkqBIXgavkE0pcTTZ7pFglUS4Mr2nBd-YiJGQpNSDmTzmqT6VYtfT7pZWnLKpsQYBZiUuYVx9BykSZRrSBqw_Tow0w8s6izuFj8YMm6mpunTwpIsE_akaTqRoHn24Us2ebESPd1juc0XTEZEUX421yjxIkgn7uDo1NpLnK2LhQLA2lMW4wELxiZ1eht1DSm8LRFjOgk8bUiXjvCoEveeR9Rx5G5EW5fM_-krHkxcSl8EUValqQr8hmcG7gcmYqJE4mfYUAzTyJa5e9NNysp0ifvrLDOtuiDO-JOmlwYV1F61zJ2vCt5LR_IuiTqNH1-nqNh54IqkzdZOl2rb8nsB_YtfKP-L68il7acNbb2c2iQgP0ziJoKbWrauCnsnGAffg5NJ8PqpuupRcd4j49PM6w0Q51MB9O_siIG_VHWTKTVNhONj-V6wuz0t4-6IKcuDaf68dVZilRyVVFTF6_bxqYkYXHPfEnqDoQF7h600MPkjaZRaAAR1EkHP0547B7x0qs0xXLG03L8yue6raExdJ7yWAlLdGEBSbipAx66rGFjDpQhZyufhcIEkOdzl4O4SX4eObRFGHJr42ZWkFHa7cs3su2AsbOzYbYdcp6lGzqWxRZnfDLDBGd43zojRaRCrJjG2nPYEVaepNkIu0VLDyJtY_pMKH4Nla0AvPr2EUwuX8amJmKJOaem94JwFJM7UyEyiOECD-7YTex3nwm5_6ixg-oaNYDg6Algvyv-KIBccW5Pu2lrYnARMl4E2ibtZeHMFB4FlrC3FcsbmlfsRdauvLF_Nd0fzdZZXvqiNvVk7vmxO8f6p-puraUczYpX3jIFhViyd2zGzvS0RM7IX1bnlFJIk0tSYnVIBlAFPd9hnCHxbyRiJ9TLGGlaedlbT4aV_diJGPfRAfHZeP08uDpfv0W6rMhzQOBfI2i8dBln1dliGQoQBdXDd2d-RiIkYRE1Kj6u7ik_CXsVDHR3IaZY9zxQJ3eFXJN9rd4JNNVFywKwsfmYiJklcCWPa2-psBwbYLiHY65L-a5bT6RssDhtMqmAQSdCoTBx48FH9OxHz9yRiOugbsLOGpLo12aQS4SDbSWfEReeVI2lCSfZn6sMIODNstF_17Lc8PlWazO5W_zhEOttJVg1WWSMhOi6451TvD3hf45q6jovx0gfKUJ8BF72n195Jl2DUghFkSHpJmii2b5dIR9FEsZlYMCw-EVjxp-CviufJEtW_96UwP0iFGpPftUJ2NS0c9xykFLk0foBHEbrMeoPmrBN-rqRweqgrr2jnuyPm_yER00IZPjf-H1sEEaTxDALLSR3utxGRsJDmYDd4r8cCLcWilKSJKeZnlMqHJmL4iOdO6LM6zgFMqqofxFaAj2RHFr7m3A6aL8D8uMMAi29TE3GLwV5mhSX47S6qm8sxSt--q5pa8bOXtavmuK6pRP6VmCLxO56ZWrm7J3DMS63MKGz_EdnKuRbNhmGRdYINtjm7rNCvs-kAjNaCepU0xQeIvKSAAKL7TsT8lxIx5edJxCgfVxNuWRLw16bEoC6g3uLIGeKkyRd9XJ8FSrkt1Qw_0LRFz6C4Xn4GabKQrzbYmFdlAbAbtYeB9TNYei61wGgwCBG_hzmnEqvr5J66BujCbL8TMW9MxFjAry0DiJ2R1Tebw92Jm57uzlVqTj52LXjHfnMsFnrQAYja0wEUL-VOw0ETveNfLfJsPrdIS1iA5ACZoH-NcHNNR89Hg3RbjRIlimuVWV5r22uzU-JJGh-TbOcYpD0pUUrpiJfbLF2ZmUn0Oswzpnx4hCPMfu4Puu3_-0TM4eOlGdRnDHJrAXYI3eUaJeMf2bf8GacSl6DshOgbXVobt0RYm9fPTMT4GHDGFSG5GdhOxCIID23aJeAOuOQhPF8lcZr1UFKoqi5XJo4w-BXxjqDq_4glLF-tGQsjcXWp0xFgYaLZaiLwqgliqqOqyCID0NcMYvfaAy6BfN7GQmHNQdx3q-GeYN5wqxoXEZZFeHqd-2C2eM-7Ym9BcnHb7vsSMapHxjRLb7lInh100dPNKXRV85tO4c9Tid3KVrV7jAHa0MJIZcT0JKA-MBHTimbmrZs11wFIVdo5mEMcjxAZN0DqsZXgVWeZ3PWWwWJzcL3sfV9f--ieuX-gODYtkcCuw3B2YxG0QaAz03A-aoEJ7KCW28rWx0o6ItUcoJdEDOsNa5pmpmM4lcCsqFaYrRO0Y6bPMs01OhEovCwNbcyDC5ot36eb4F3SZKfB8gj8oal6zrgus9qaZsYEdXEBB-YavOjGqXGi4FooYQtq-Bv5MxMxXarBkFY-o9ca9o5mAHduIBg9z3F1LnjSkzqHvsBqN_HENZpjzzS_JvbrxtV7EIQO-MrIZa8Bm8lgpOwrh-YS10nqcJhbqnNgbohQXawzoeYlnuBlBm8hiZusIl6LLmHxPIPUg29uGfqkuT2lzCjBIaysxgtLHoabfl8iZo9Z_N5O7KyYe-riGGFCUzb-LahuDATlA2KiJtHGhZSOtZ8S9FPnQxMx5ZF-nk3nqaHGtnB3frRPieRNA01X9Ts9W8OfJ8z--q7PWBdph3zN7D-0cGkaUIuu8450rqjXjNw4rCoQfmcY3hIY5t5QYIe5niU1-XFf5adghKO3tfVG3reGNUm7pcXjEkbrWIbdhlnB8fJkb2KZ6RJu2Lp31vy-RAwuo1SuaE3ln-bGo8Jdg-y6s8HYKY6Du0PCpw5BVi_V3N66KvXvI-T4kYkYzxG_XouVfTtMHwBZMYO4bEXgVVNb8QF8qhUWlo7rh_Dzclh8iy9Tx_hdaqSKwXnhcchIE33CuT75tkb4bSsRftPYwVW2yftpfCXhAMD50pnNe51dhwCQjhZu8RNbK6nWiqm5lRtXJbwAYKRevoI2pKQ3QEMbX_W2RIxOBwvuQMmgpW2QlCkssfVeVUpKhA2OH10VLKLC7eFgt9LKUn760ZD64ESMsrh3-6xBdo_HD012oYpWcOPxlfbUNF4f8FTQZHdXKkqzCwO7IL4hEdNYY48Ft6VpzYM4lPKuCoMEFAxMQ8PxdeDAJY2IofPeCQLgLqnlfH9mIqYBjCKhB-6yLFxwK1QwZHWe9iEJvokvh87VXfpRpC9qFOSG7Hrbb8oE34mYvyERM9MzUEkFk6FrvFabXQPN0gH4btdALEC5tx3wUxq66RpTXG9YqpJ4xlN-YiJGXaAS2Zgao2dXVVSnQyPZdxhpqux9YsfgWlTiMXbGLZQjNm5ceO4vpT5JvY4ee8TzK91fNcjuQGs1gnIPzdchyK5DwE2gridVhVM97rbPi8Cp9w54CK1aS0XNqsmOhUTAiar0iWkOIKVGQs0b3JtULkNIVg6BL_1QH_idiPl5EzFHM0_7AVOYAQhM5hCvJjCcWZsUZgIOtcJV9mqS9ZYwTIj5qMAOjvqxiRhwzVD3MBbdpLanOdLz5iEZzik4_Cj1ZsiaJxtJZ4BDNHCsMzR36aWO6njhyiUJ3XuPSuPHjFvKptMxmDl8ZmTVV5ckvQQIY7iAqhs7fO8Fvg_4sWuc09EMAJV3tXPBxTovBBzm0lWkd2NZS4rDI9qQKM_uOctF-nci5j951V-_5q9e8efP_9mzf_zcHz3z-9__ve_--_dev_P16x-_-tfjfz767X_9--svv_7yDxcRyA3tpQIA">open interactively in the visualizer ↗</a></summary>

```text
GroveDBProofV1 {
  LayerProof {
    proof: Merk(... root-level descent, identical to every other chapter query ...)
    lower_layers: {
      @ => { ... contract_id descent ... }
      // L2..L4 byte-identical to G3 / G5 / G7 (the @/contract_id/0x01/widget chain)
    }
  }
  // L5 widget doctype: brand queried (same as G3 / G5 / G7)
  // L6 byBrand merk-tree: 25 outer-key matches inlined as KVValueHash items
  //                       (brand_051 ... brand_075), each descending into its
  //                       continuation. Boundary commitments cover the
  //                       brands_outside_the_limited_window.
  // L7 brand_NNN's value tree: single key `color` with NonCounted(ProvableCountTree)
  //    — repeated 25 times, once per resolved outer brand
  // L8 brand_NNN's byBrandColor color subtree:
  //    proof: Merk(
  //      ... 36-37 ACOR boundary ops over color > color_00000500,
  //          summing to count = 499 per brand ...
  //    )
  //    — repeated 25 times in parallel, each with its own per-brand boundary hashes
}
```

The 1 426-line full verbatim is available via the bench's `[gproof] G8` output. The schematic compresses the 25 parallel L7+L8 descents — they share the same template (single-key continuation + 37-op ACOR boundary walk), differing only in per-brand kv-hashes and the resulting subtree commits. Each per-brand L8 contributes ~1 700 B of ACOR boundary commitments — exactly the predicted `Q8 - L1..L5` overhead per outer match, scaling linearly: `43 638 B ≈ shared upper layers + 25 × ~1 700 B ≈ 43 KB` (matches the per-In slope from G7 vs Q8).

**Cryptographic guarantee** (via grovedb PR #663 + PR #664): every per-brand count is independently committed to the merk root via `node_hash_with_count`. The `SizedQuery::limit` is part of the serialized PathQuery and is part of the merk-root reconstruction the verifier performs — a malicious prover can't truncate the outer walk at a different point without breaking the hash chain.

</details>

```mermaid
flowchart TB
  WD["@/contract_id/0x01/widget"]:::tree
  WD ==> BR["brand: NormalTree"]:::path
  BR ==> B051["brand_051: CountTree count=1000"]:::path
  BR ==> BMore["… 23 more in-range brands (brand_052 … brand_074) …"]:::path
  BR ==> B075["brand_075: CountTree count=1000"]:::path
  BR -.-> BCapped["brand_076 … brand_099<br/>(beyond platform cap — opaque subtree commitments)"]:::faded
  BR -.-> BBelow["brand_000 … brand_050<br/>(below range floor — boundary commitments)"]:::faded

  B051 ==> B051_C["brand_051/color: NonCounted(ProvableCountTree)<br/>ACOR boundary walk (color > color_00000500)"]:::target
  BMore ==> BMore_C["23 parallel ACOR walks"]:::target
  B075 ==> B075_C["brand_075/color: NonCounted(ProvableCountTree)<br/>ACOR boundary walk (color > color_00000500)"]:::target

  SDK["Entries(25 groups, sum=12 475):<br/>(&quot;brand_051&quot;, 499)<br/>(&quot;brand_052&quot;, 499)<br/>…<br/>(&quot;brand_075&quot;, 499)"]:::sdk
  B051_C -.-> SDK
  BMore_C -.-> SDK
  B075_C -.-> SDK

  classDef tree fill:#21262d,color:#c9d1d9,stroke:#1f6feb,stroke-width:2px;
  classDef path fill:#6e7681,color:#fff,stroke:#1f6feb,stroke-width:2px;
  classDef faded fill:#21262d,color:#6e7681,stroke:#484f58;
  classDef target fill:#39c5cf,color:#0d1117,stroke:#39c5cf,stroke-width:3px;
  classDef sdk fill:#21262d,color:#39c5cf,stroke:#39c5cf,stroke-width:2px,stroke-dasharray: 4 2;

  linkStyle 0 stroke:#1f6feb,stroke-width:3px;
  linkStyle 1 stroke:#1f6feb,stroke-width:3px;
  linkStyle 2 stroke:#1f6feb,stroke-width:3px;
  linkStyle 3 stroke:#1f6feb,stroke-width:3px;
  linkStyle 6 stroke:#1f6feb,stroke-width:3px;
  linkStyle 7 stroke:#1f6feb,stroke-width:3px;
  linkStyle 8 stroke:#1f6feb,stroke-width:3px;
```

### Diagram: per-layer merk-tree structure (Layer 5+)

L5 is identical to G7's L5 (widget doctype with `brand` queried). L6 differs: G7 inlined 2 `KVValueHash` targets for the In-bearing brands; G8 inlines 25 KVValueHash targets for the in-range brands the carrier walks (`brand_051` through `brand_075`), with boundary commitments covering both the below-floor and beyond-cap portions of the byBrand merk tree. L7 + L8 fork into 25 parallel descents, each shaped exactly like G7's L7 + L8 — same `NonCounted(ProvableCountTree)` continuation, same 37-op ACOR boundary walk over `color > color_00000500`.

```mermaid
flowchart TB
  subgraph L5["Layer 5 — widget doctype merk-tree"]
    direction TB
    L5_q["<b>brand</b> (queried)<br/>kv_hash=HASH[68b6...]"]:::queried
  end

  subgraph L6["Layer 6 — byBrand merk-tree (25 outer-range targets)"]
    direction TB
    L6_t051["<b>brand_051</b><br/>CountTree count=1000"]:::queried
    L6_tmid["… 23 more in-range targets …<br/>(brand_052 … brand_074)"]:::queried
    L6_t075["<b>brand_075</b><br/>CountTree count=1000"]:::queried
    L6_capped["Beyond-cap commitments:<br/>brand_076 … brand_099<br/>(opaque KVHash / Hash ops)"]:::sibling
    L6_floor["Below-floor commitments:<br/>brand_000 … brand_050<br/>(opaque)"]:::sibling

    L6_t051 --> L6_tmid
    L6_tmid --> L6_t070
    L6_t070 --> L6_capped
    L6_t051 --> L6_floor
  end

  subgraph L7L8["Layers 7+8 — per-brand continuation + ACOR walk (×25)"]
    direction TB
    L7L8_each["For each of brand_051 … brand_075:<br/>L7: single-key `color` continuation (NonCounted(ProvableCountTree))<br/>L8: 37 merk ops — ACOR boundary walk for color > color_00000500<br/>committing one `u64 = 499` per brand"]:::target
  end

  L5_q -. "byBrand" .-> L6_t051
  L6_t051 -. "continuation × 20" .-> L7L8_each

  classDef queried fill:#1f6feb,color:#fff,stroke:#1f6feb,stroke-width:2px;
  classDef sibling fill:#6e7681,color:#fff,stroke:#6e7681;
  classDef target fill:#39c5cf,color:#0d1117,stroke:#39c5cf,stroke-width:3px;
```

The slope vs G7 is the proof's whole story: G7's `k = 2` outer matches → ~4 KB; G8's `L = 25` outer matches → ~43 KB. The per-outer-match cost (~1 700 B) is the same; only the outer-walk count changes. The platform cap of 25 is hardcoded to keep the worst-case proof under 50 KB; larger windows are unreachable without changing the constant (and the structural contract that goes with it).

## Future Work

This chapter now mirrors chapter 29's per-query structure: every section above carries a path query, verified payload, proof size, verbatim or schematic proof display, narrative, conceptual flowchart, and per-layer merk-tree diagram.

Two pieces of infrastructure made this possible:

- `query_g1_*` … `query_g6_*` criterion `bench_function` calls in [`document_count_worst_case.rs`](https://github.com/dashpay/platform/blob/v3.1-dev/packages/rs-drive/benches/document_count_worst_case.rs) — produce the **Avg time** column in [Queries in this Chapter](#queries-in-this-chapter).
- `display_group_by_proofs` (a sibling of `display_proofs` in the same bench file) — emits each `group_by` shape's verbatim merk-proof structure via bincode decode + `GroveDBProof::Display`. Tagged with `[gproof]` prefix in stderr so reviewers can grep deterministically.

Open follow-ups:

1. **Inline the full G4 / G5 / G6 verbatim** rather than the schematic-with-elision form. The bench captures every byte; the chapter's `<details>` blocks currently summarise the 100-target enumerations because reproducing 100 near-identical `KVValueHashFeatureTypeWithChildHash` lines per case is more noise than signal. If a reader needs byte-exact output, they can run the bench and grep `[gproof]`.
2. **Wire path-query reconstruction + verified-payload printing into `display_group_by_proofs`**. Today it only dumps the proof-display block; chapter 29's `display_proofs` also reconstructs the `PathQuery` and prints the verifier's structured result (the `verified:` block). Adding that to the group_by side would give the chapter parity with chapter 29's `verified:` sections — currently rendered manually from the `[matrix]` output's `Entries(len=N, sum=M)` figures.
3. **A high-fanout byColor variant of G6** (`color IN [100 values]`, `group_by = [color]`) — captured implicitly in the bench's existing `group_by_color_in_proof_100_rangecountable_branches` (10 512 B) but not given its own G* section, since it's structurally G6 with `ProvableCountTree` overhead.

## Cross-Reference to Chapter 29

For background on the building blocks every query in this chapter uses:

- [Document Count Trees](./document-count-trees.md) — `CountTree` / `ProvableCountTree` / `NormalTree` mechanics.
- [Count Index Examples § How To Read The Proofs](./count-index-examples.md#how-to-read-the-proofs) — the four-section per-query template plus the `LayerProof` / `Merk` / `Push` / `Parent` / `Child` op grammar.
- [Count Index Examples § Worked Example: How `node_hash_with_count` Rebuilds the Merk Root](./count-index-examples.md#worked-example-how-node_hash_with_count-rebuilds-the-merk-root) — exact Blake3 formulas underpinning every count proof in either chapter.

The path-query builder (`packages/rs-drive/src/query/drive_document_count_query/path_query.rs`) and verifier mirror (`packages/rs-drive/src/verify/document_count/`) live in the same modules for both chapters' queries — the only difference is which `point_lookup_*` / `aggregate_*` / `group_by_*` function the dispatcher calls based on the `CountMode` carried in the request.
