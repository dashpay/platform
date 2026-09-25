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

The **terminal** is the member key, sitting exactly where a normal
non-unique index keys by document id; the element is an `Item` instead of
a `Reference` because there is nothing to point at. It is a per-index
keyword defaulting to `$ownerId`. It may name any schema property a prefix
position could carry (an identifier with or without a `refersTo`, a
bounded byte array or string, an integer, a boolean, a date), or an
ordered **list** of such properties (a *composite* terminal, whose member
key is their encoded values concatenated). The `0` storage marker,
value-tree types, and the count/sum/ranked tree derivation are
byte-identical to the ordinary non-unique layout, which is what lets the
protocol v14 ranked machinery (see
[Document Ranked Trees](./document-ranked-trees.md)) serve index-only
types unchanged: "the five most-liked posts in `#dash`" is an
O(log n + k) read with an O(log n + k) proof, and Items count in
count/ranked trees exactly as References do.

The member key is the terminal value's **tree-key encoding**, produced by
the same functions the prefix levels use (the walkers and probes through
`get_raw_for_document_type`, queries and executed proofs through
`serialize_value_for_key`, synthesis through `decode_value_for_tree_keys`),
so nothing about it is specific to a 32-byte identifier: a 33-byte public
key, a short string or an integer keys the `0` bucket exactly as it would
key a prefix level, and fee estimation sizes the member key by the
terminal property's declared bound (`index_only_terminal_max_key_size`)
rather than by a fixed 32. Structural uniqueness spans the terminal value:
one entry per (prefix values, terminal value), so two documents by one
owner that differ only in a scalar terminal are two entries under the
same prefix.

**Composite terminals.** `"terminal": ["kind", "$ownerId"]` keys the
member by `encode(kind) ‖ owner`. Every component but the last must be
fixed width (a byte array with `minItems == maxItems`, an identifier, an
integer, a boolean, a date), so equality on the leading components is a
clean key range and synthesis can split the key back; a string can only
be the last component; the whole key is capped at 255 bytes. Uniqueness
spans the whole key. Queries bind the components in order: equality
clauses on the leading ones, then at most one range or `in` clause on the
next (ordered by it), nothing on the rest. The lowering pads the bound
prefix with `0xFF` to the key cap for the upper bound of "every key under
this prefix", and addresses the key itself when the bound component is
the last one. After equality-bound components are ignored, `orderBy` must
start at the first remaining component and follow component order without
gaps, with the same direction for every listed component. A single member-key
walk cannot sort by a later component alone or mix ascending and descending
components.

**Flat indexes.** An index with no `properties` at all is *flat*: its
entries live directly under a level of their own, keyed by a zero byte
followed by each terminal component name preceded by a zero byte
(`"\0appEphemeralPubKeyHash\0$ownerId"`), which no property-name tree can
collide with since property names never contain a zero byte. This level key,
including its separators, must also fit within 255 bytes:

```text
[DataContractDocuments, contract_id, 1, <doctype>, "\0<c1>\0<c2>…", 0, <c1 ‖ c2 ‖ …>]
      → Item(<row commitment> [‖ <entry payload>], flags)
```

The flat level is registration-time structure, created with the
property-name trees and kept when the last entry goes (the prune stops at
its `0` bucket, as on a preallocated index), so every entry costs the same.
There is no prefix level for an aggregate, a ranking, a time grid, a skip
trigger or a preallocation to apply to, so a flat index admits none of
those keywords. A clause-free query on a type with a flat index scans the
flat level (every other indexOnly type refuses the by-id shape). Non-proof
responses require this index to cover every property, including optional
ones, just as filtered queries do; otherwise use a proved projection.

**The entry payload.** `entryPayload: ["walletEphemeralPubKey",
"encryptedPayload"]` on the document type names top-level properties that
live in no index: every entry's item carries them after the 32-byte row
commitment, each length-framed (`u16` big-endian), in property-name order:
the type's value slot. Byte arrays store their raw bytes and strings store
UTF-8; other scalars use their tree-key encoding. The length frame preserves
empty byte arrays and strings without null sentinels, and distinguishes an
empty string from a NUL string. A payload
property must be required, scalar and bounded (the sum of the bounds is
capped by the field value limit), and appears in no index as a property
or a terminal component. It is still committed (the commitment hashes
every present property, a payload value through the uncapped payload
encoding), so the delete probes and the executed-transition verifier keep
comparing the item's first 32 bytes only, and synthesis decodes the rest
of the proved element. With more than one index the payload rides in
every entry; fee estimation sizes the item by the commitment plus the
payload bound. Together, a flat composite terminal and an entry payload
make a key-value table:

```json
"indices": [{ "name": "byRequest", "terminal": ["appEphemeralPubKeyHash", "$ownerId"] }],
"entryPayload": ["walletEphemeralPubKey", "encryptedPayload"]
```

lands at `[…, "\0appEphemeralPubKeyHash\0$ownerId", 0, hash ‖ owner] →
Item(commitment ‖ len ‖ ciphertext ‖ len ‖ wallet key)`, and a query on
the hash returns every responder's owner id with the payload decoded off
the item, as one proof.

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
over the document's PRESENT properties in sorted-name order
(`index_only_row_commitment`) — every required property must be present,
and an optional property (a `skipIfAbsent` trigger, the only optional
kind) contributes nothing when absent, not even its name. It binds the
independently stored index projections of one document back into one
logical row: a delete recomputes the commitment from its submitted
values, and every probed entry must carry it. A values tuple spliced from
two different creates — even two creates by the same owner — fails the
comparison on whichever entry belongs to the other row. Entry existence
alone cannot make that distinction; the commitment is what does. The
present-set is part of what is committed: absent (no name emitted) and
present-but-empty (`name ‖ 00000000`) hash differently, and the variable
present-set stays unambiguous because property names never contain a
zero byte while every length prefix starts with one.

## Constraint matrix (parse-time, `apply_index_only`)

The on-disk layout depends on every one of these, so they run regardless
of `full_validation` — the same untrusted-boundary rule the doctype
aggregate keywords follow:

| Constraint | Why |
|---|---|
| every property in `required` — except a `skipIfAbsent` index's trigger; every ancestor of an indexed dotted path required | the index path is the storage; no null layout exists — the one sanctioned hole removes the whole entry instead |
| every non-trigger property appears in ≥ 1 **non-skip** index (prefix or terminal) | only indexed values exist, and a skip index carries no value for trigger-absent documents — covered only there, a property would be validated and committed yet written nowhere |
| **every index embeds `$ownerId`** (prefix or terminal) | entries are self-authorizing: a delete computed with owner = signer can only ever address the signer's own entries |
| ≥ 1 index is `$createdAt`-free AND non-`skipIfAbsent` — the **proof index** | executed-transition proofs locate entries from the transition's values alone: they can neither reproduce a block timestamp nor anchor on an entry that may not exist |
| every terminal component is `$ownerId` or a schema property passing the indexed-shape limits (no arrays or objects; byte arrays ≤ 255 bytes, strings ≤ 63 characters); every component but the last is fixed width; the whole key ≤ 255 bytes | the member key is the components' tree-key encodings concatenated, derived by the same functions the prefix levels use; a leading component must be splittable back and rangeable; grovedb caps keys at 255 bytes; other system properties are refused because the `$createdAt` rules walk the prefix properties |
| a flat index (no `properties`) admits no countable / summable / ranked / `timeRange` / `skipIfAbsent` / `preallocated` keyword | there is no prefix level for them to apply to |
| every `entryPayload` property is a required, bounded, top-level scalar in no index | the entry value has no representation for an absent property, estimation sizes the item by the bounds, and a property is either a key or a value |
| indexed `$createdAt` requires `$createdAt` in `required` | creation only assigns timestamps for required system times |
| `documentsMutable: false`, no transfers/trading/history/transient | no stored row, no revision |
| non-unique, non-contested, `nullSearchable` default | v1 scope |
| `preallocated` requires a fully reference-determined, non-bucketed path | see [Preallocated index paths](#preallocated-index-paths) |
| `skipIfAbsent` requires its first property to be an optional, top-level schema property | see [Conditional participation](#conditional-participation-skipifabsent) |

`indexOnly` and the index set (terminals included, `preallocated` and
`skipIfAbsent` flags included) are immutable across contract updates — a
later-added index could never be backfilled, and the walkers derive the
skip from `required` membership, which therefore cannot drift from
historical entries either.

## Conditional participation (skipIfAbsent)

An index may declare `skipIfAbsent: true`: a document that omits the
index's FIRST property — the **skip trigger** — writes no entry into that
index at all, and a delete recomputes the same skip from its carried
values. The trigger is the one property that may leave `required`, and
the rules keep three views provably equivalent: the write walkers skip a
top-level branch keyed by an unrequired property (every index through
such a branch is a skip index — the parser admits an optional property
only at position 0 of skip indexes, never as a terminal), the probes
derive zero entry paths from the parsed flag, and the row commitment
pins the exact present-set so a delete with a different absence pattern
fails every probe — a skip index can neither be force-pruned nor left
with an orphan entry.

Why the FIRST property: the merged index structure shares prefix levels
across indexes and prunes empty trees only upward from a terminal. A
deeper skip would leave the prefix levels above the absent property
inserted but unterminated — silently charged, never reclaimed. At the
top of the branch, absence writes nothing at all. (This also makes
`skipIfAbsent` + `timeRange` structurally impossible: a bucketed source
is `$createdAt`, which is required whenever indexed.)

The semantics are a **sparse projection**: the index holds exactly the
documents carrying its trigger. Counts, ranked reads and absence proofs
over it answer "among documents with this property" — a proved empty
position means "no *tagged* like", not "no like". The query router makes
that opt-in: a skip index is admissible only when the query binds its
trigger (equality, `in`, range or order-by); the generic matcher alone
would admit a trigger-unbound query within its difference budget and
silently omit every trigger-absent row. Structural uniqueness still
spans the skip boundary — a trigger-absent and a trigger-present
document colliding on any shared non-skip entry cannot coexist. An
absent trigger is distinct from a present-but-empty value, which indexes
normally under its (possibly empty) encoded key.

The economics are the point: each ranked index costs roughly the same on
every write, so a per-hashtag ranked index on a like doctype used to tax
every like — tagged or not — and forced a `''` sentinel onto the
referenced post's hashtag. With `byHashtagPost` as a skip index, an
untagged like pays only for the indexes it actually appears in, and the
sentinel disappears (see the absence-aware `propertyAgreement` below).

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
  Absence is part of the agreement, strictly: both sides absent agree, one
  side absent is the same mismatch a differing value would be — a like may
  omit its hashtag exactly when its post has none (anything laxer would
  let likes on tagged posts silently deflate per-tag aggregates), which is
  what lets an agreement key double as a `skipIfAbsent` trigger with both
  sides of the reference optional. The referenced side of a pair may also
  name the referenced document's `$ownerId` or `$creatorId` (the referring
  side must then be an identifier property): `{ "authorId": "$ownerId" }`
  binds a like to its post's current owner, so an `[authorId, postId]`
  index can be preallocated and ranked per author. `$ownerId` follows the
  post through transfers and `$creatorId` never changes; either is checked
  when the like is written, not when the post later moves. `$creatorId` is
  only recorded by transferable or tradeable types of a format-1 contract,
  which contract registration checks before accepting the declaration.
  The referring side may in turn be the like's own `$ownerId`, the writer:
  `{ "$ownerId": "$ownerId" }` lets only the post's current owner create
  or replace a like on it, `{ "$ownerId": "$creatorId" }` only its
  original creator. That is a write gate, checked on create and on every
  replace of the like, not only when its reference changes, since the post
  may have been transferred in between; a transfer itself is not
  re-checked, so on a transferable referring type it governs writing, not
  holding. A writer gate does not make an owner-prefixed index
  preallocatable.
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
referenced-document property, its `$ownerId` and `$creatorId` included),
and the reference is a `permanentDocument` one targeting a document type
of the **same contract**. A `deletableDocument` reference shapes the path
the same way but does not qualify: its target can be deleted, and the
trees created alongside it would outlive it with other owners' entries
inside. `byHashtagPost` (`[hashtag, postId]`) qualifies
— `hashtag` through the agreement, `postId` as the reference;
`byLiker` (`[$ownerId]`) cannot, since no referenced document determines
the liker. An `[authorId, postId]` index whose `authorId` agrees with the
post's `$ownerId` qualifies too: the poster is the one owner a referenced
post does determine.

Three things change, all bit-compatible with the fallback layout:

- **Insert side** (`insert/add_preallocated_index_tree_operations`):
  inserting the referenced document also emits if-not-exists creations of
  the referring index's dynamic trees, down to the empty `0` member
  bucket, derived through the same tree-type helper the entry walkers use
  — so a preallocated tree is byte-identical to the tree the first
  entry's create-on-insert path would have made. The poster pays for the
  structural bytes; storage flags ride only when the contract itself is
  deletable, because that is a preallocated tree's one deletion path —
  entry deletes retain it by design, so entry-level flags would be
  unrefundable dead weight. Shared prefixes (a second post under the same
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

`preallocated` composes with `skipIfAbsent`: every bound key is resolved
before any operation is emitted, and an absent bound value (an untagged
post's hashtag, under the absence-aware agreement) bails without emitting
anything — so a tagged post preallocates the skip index's trees, an
untagged post preallocates only its id-bound indexes, and an untagged
like skips exactly the trees that were never built.

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
(the first `$ownerId`-bearing, non-`skipIfAbsent` index not involving
`$createdAt` — contract admission guarantees one exists) and a delete by
its absence, with the
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
