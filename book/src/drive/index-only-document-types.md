# Index-Only Document Types

> **Status:** implemented and gated at protocol version 14 (meta-schema v3 /
> parser generation 3). The storage layout is pinned end-to-end against a real
> grovedb by
> [`index_only_e2e_tests`](https://github.com/dashpay/platform/blob/v4.2-dev/packages/rs-drive/src/drive/contract/insert/insert_contract/v0/tests/index_only_e2e_tests.rs),
> which runs against the yappr-likes fixture at
> [`packages/rs-drive/tests/supporting_files/contract/yappr-likes/yappr-likes-contract.json`](https://github.com/dashpay/platform/blob/v4.2-dev/packages/rs-drive/tests/supporting_files/contract/yappr-likes/yappr-likes-contract.json).
> The full ABCI pipeline (transitions, validation, executed-transition
> proofs) is exercised by the `index_only` test modules in rs-drive-abci's
> batch tests.

## The problem

A minimal social interaction — a like — is fully expressed by *where it
sits*: which post, which hashtag, which identity. Storing it as an ordinary
document costs a serialized body in primary storage (~90–150 bytes plus
element flags and tree-node overhead), a primary-tree insertion, and a
~70–90-byte reference per index, for a fact whose entire content is its
index position.

An **index-only document type** (`indexOnly: true` on the doc-type schema)
stores nothing in primary storage. The index entries ARE the rows:

```text
[DataContractDocuments, contract_id, 1, <doctype>,
   <prop 1>, <val 1>, …, <prop K>, <val K>, 0, <terminal value>]
      → Item(<row commitment>, flags)
```

The **terminal** — a per-index keyword defaulting to `$ownerId`, or any
refersTo-typed identifier property (identity, contract, token, permanent
document) — is the member key, sitting exactly where a normal non-unique
index keys by document id; the element is an `Item` instead of a
`Reference` because there is nothing to point at. The `0` storage marker,
value-tree types, and the count/sum/ranked tree derivation are
byte-identical to the ordinary non-unique layout, which is what lets the
protocol v14 ranked machinery (see
[Document Ranked Trees](./document-ranked-trees.md)) serve index-only
types unchanged: "the five most-liked posts in `#dash`" is an
O(log n + k) read with an O(log n + k) proof, and Items count in
count/ranked trees exactly as References do.

**`timeRange` buckets** compose too: a bucketed indexOnly index writes
one commitment entry per containing bucket under the grid-qualified
level, exactly as stored types do — the walkers' bucket fan-out, the
probes' path derivation (`entry_keys_for_raw`, shared so probe and write
paths cannot drift), and the `IN_TIME_RANGE` count aggregates are all the
same machinery ("how many likes under `#dash` this hour"). The source can
only be `$createdAt` (the prefix rule admits no other timestamp), which
`required` must carry, so a delete's values reproduce the exact bucket
set. A bucketed index involves `$createdAt` and therefore never serves as
the proof index; and document synthesis over bucketed entries is refused
with guidance — the bucket level carries bucket-start granularity, not
the document's timestamp, and the raw entries are served by the type's
non-bucketed indexes.

The **sum axes** compose the same way: a `summable: "<prop>"` index
stores `ItemWithSumItem(<row commitment>, <amount>)` terminals — the same
commitment payload, plus the summed property's value — so entries
contribute to ancestor sum trees exactly as stored types'
`ReferenceWithSumItem` references do ("total tipped to this post", "top
posts by total tipped" via `rankedSummable`). The doctype-level summable
cross-checks (one canonical summed property, i64-safe integer type,
`required` membership) apply unchanged, and on delete grovedb reads the
amount off the stored element and propagates the subtraction — the
falsified-amount case dies on the commitment probe first, since the
amount is one of the committed properties.

**Governing principle: only what is in the indexes exists and is
recoverable.** Prefix property values live in the path, the terminal id in
the member key, `$ownerId` and `$createdAt` wherever an index carries them.
There is no document beyond that.

## The row commitment

Each entry's 32-byte payload is
`hash_double(owner ‖ (name ‖ length ‖ raw index bytes)* ‖ [$createdAt])`
over ALL of the document's properties in sorted-name order
(`index_only_row_commitment`). It binds the independently stored index
projections of one document back into one logical row: a delete recomputes
the commitment from its submitted values, and every probed entry must
carry it. A values tuple spliced from two different creates — even two
creates by the same owner — fails the comparison on whichever entry
belongs to the other row. Entry existence alone cannot make that
distinction; the commitment is what does.

## Constraint matrix (parse-time, `apply_index_only`)

The on-disk layout depends on every one of these, so they run regardless
of `full_validation` — the same untrusted-boundary rule the doctype
aggregate keywords follow:

| Constraint | Why |
|---|---|
| every property in `required`; every ancestor of an indexed dotted path required | the index path is the storage; no null layout exists |
| every property appears in ≥ 1 index (prefix or terminal) | an unindexed property would be silently dropped |
| **every index embeds `$ownerId`** (prefix or terminal) | entries are self-authorizing: a delete computed with owner = signer can only ever address the signer's own entries |
| terminal is `$ownerId` or a single-id refersTo property | the member key must alone be a referable entity id (`identityPublicKey` is compound and rejected) |
| indexed `$createdAt` requires `$createdAt` in `required` | creation only assigns timestamps for required system times |
| `documentsMutable: false`, no transfers/trading/history/transient | no stored row, no revision |
| non-unique, non-contested, `nullSearchable` default | v1 scope |
| `preallocated` requires a fully reference-determined, non-bucketed path | see [Preallocated index paths](#preallocated-index-paths) |

`indexOnly` and the index set (terminals included, `preallocated` flags
included) are immutable across contract updates — a later-added index
could never be backfilled.

## Lifecycle

- **Create** reuses `DocumentCreateTransitionV0` unchanged. State
  validation probes every index's entry: ANY existing entry is a duplicate
  (`DuplicateUniqueIndexError`), which is also what makes a shorter index a
  uniqueness constraint over its value projection plus owner — for likes,
  the `[postId]` index is the one-like-per-(post, owner) rule. `refersTo`
  validation runs unchanged (it reads transition values, not storage), so a
  like on a nonexistent post is rejected — and a `propertyAgreement`
  declaration on the reference (`{ "hashtag": "hashtag" }`) binds the
  like's own property to the referenced post's: the referenced document is
  already fetched for the existence check, so the equality comparison adds
  no reads, and a like whose hashtag disagrees with its post's is refused.
- **Delete** is its own transition kind,
  `DocumentIndexOnlyDeleteTransition { base, data }` (`$action:
  "indexOnlyDelete"`), carrying the full value tuple (`$createdAt` under
  its system key exactly when the type requires it). Delete-by-id and
  delete-by-values are different operations — different payload,
  authorization model and validation pipeline — so the factory picks the
  KIND from the doctype's storage mode. Validation and the storage layer
  both require every entry to exist AND match the row commitment. A by-id
  delete on an index-only type (and an indexOnlyDelete on a stored type)
  is rejected by the structure gates; below PV14 the kind is rejected at
  basic structure, keeping check_tx behavior aligned with pre-4.2
  software.
- **Replace / transfer / purchase / price** are structurally impossible.

## Preallocated index paths

The first entry under a fresh value tuple pays for every tree on its path
— for a like that is the hashtag value tree, the `postId` property-name
tree, the post's value tree and the `0` member bucket — while the second
entry pays for one item insert. When the index path is a pure function of
a refersTo-referenced document, that lopsidedness is avoidable: an index
may declare `preallocated: true` iff every index property is either the
referring property itself (its value is the referenced document's `$id`)
or a key of that reference's `propertyAgreement` (consensus-equal to a
referenced-document property), and the reference targets a document type
of the **same contract**. `byHashtagPost` (`[hashtag, postId]`) qualifies
— `hashtag` through the agreement, `postId` as the reference;
`byLiker` (`[$ownerId]`) cannot, since no referenced document determines
the liker.

Three things change, all bit-compatible with the fallback layout:

- **Insert side** (`insert/add_preallocated_index_tree_operations`):
  inserting the referenced document also emits if-not-exists creations of
  the referring index's dynamic trees, down to the empty `0` member
  bucket, derived through the same tree-type helper the entry walkers use
  — so a preallocated tree is byte-identical to the tree the first
  entry's create-on-insert path would have made. The trees carry the
  referenced document creator's storage flags: the poster owns the
  structural bytes. Shared prefixes (a second post under the same
  hashtag) deduplicate through the if-not-exists semantics.
- **Delete side**: removing the last member entry stops the
  empty-tree-pruning climb at the member level, keeping the whole
  apparatus — the group stays in the ranked secondaries at count 0, and a
  re-entry is again a plain item insert. (Non-preallocated indexes of the
  same type keep pruning as before.)
- **Nothing else**: entry insertion keeps its create-if-missing behavior,
  so correctness never depends on preallocation. Referenced documents
  created before a contract update introduced a referring type simply
  hand the first entry the old price, and their trees — created by the
  fallback — are retained on delete exactly like preallocated ones.

The economics: the referenced document's creator pays for the trees
whether or not anyone ever references it (which is why the flag is an
explicit opt-in, per index), every entry from the first on costs the
same, and "no entries yet" becomes a present-but-empty member bucket —
provable as zero results, rankable as a zero-count group — instead of an
absent tree.

An entry's proved `(path, key)` position IS the document, so queries and
proofs **synthesize** documents through one shared builder
(`query/index_only_synthesis.rs`, compiled for server and verify): prefix
properties decoded from the path via `decode_value_for_tree_keys` (the
inverse of the write path's key encoding), the terminal from the member
key. A query through a subset index yields a documented *projection*. The
synthesized `$id` is deterministic over the proved position (a
domain-separated, length-framed hash covering every non-owner component,
`$createdAt` included) — nothing on chain is ever addressed by it.

Executed-transition proofs (waitForStateTransitionResult) prove a create
by the presence of the entry its values produce under the **proof index**
(the first `$ownerId`-bearing index not involving `$createdAt` — contract
admission guarantees one exists) and a delete by its absence, with the
proved entry's payload checked against the transition-derived row
commitment (and, when the proof index is summable, the proved sum
contribution against the created document's amount); prover and verifier
build the same single-entry path query from the transition. The outcome is always `AffectedState`, never
`ExecutionProved`: the commitment carries neither id, entropy nor nonce,
so a snapshot cannot bind one specific transition's execution.

Where clauses on the **terminal property** lower directly onto the entry
level's member keys once every prefix property carries an equality clause:
an equality answers "did I like X" in one query, and a range ordered by
the terminal (`terminal > <last seen>`, with a limit) walks the entries
page by page — **keyset pagination**, the indexOnly replacement for
id-shaped `startAt` cursors, which cannot address a position whose
synthesized id is a one-way hash. Mixed shapes are served through a
**prefix pivot**: one range or `in` clause may sit on a prefix property
instead of the terminal (`hashtag == h AND postId > p AND $ownerId ==
me`), with everything above the pivot equality-bound, everything below
it unconstrained, and the terminal clause an equality. All shapes prove
and verify through the same shared path-query builder.

Not supported on the read surface: by-`$id` fetches (no primary tree —
rejected with guidance) and `startAt` cursors (rejected with the keyset
guidance above); ranked / count / range-aggregate queries work unchanged
since they never open value trees.

## What it costs and what it saves

Registration skips the `[0]` primary-key tree. Each document is exactly
one `[…values, 0, terminal] → Item(32-byte commitment)` per index — no
primary row, no references — cutting storage well past half against a
minimal stored document, with deletion refunds flowing from each entry's
own element flags (the index walkers pass flags for
immutable-yet-deletable index-only types specifically). Estimation pads
the dry-run item above the real payload so estimated fees keep
upper-bounding applied fees across the indexed-tree layers' documented
under-count.
