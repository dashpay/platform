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
set or a preallocation to apply to, so a flat index admits none of
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
There is no document beyond that. One kind of property may sit in no
entry-keeping index: one a `summableOffCountIndex` index's source fixes
through a reference's `where` (a like's `postAuthor`, kept only by
`byAuthorPost`'s counters). Such a property, when no entry-keeping index
holds it, is lacking from a document read back, and its value is the
referenced document's, so a client rebuilding the row for a delete reads it
from there.

## The row commitment

Each entry's 32-byte payload is
`hash_double(owner ‖ (name ‖ length ‖ raw index bytes)* ‖ [$createdAt])`
over the document's PRESENT properties in sorted-name order
(`index_only_row_commitment`), `$createdAt` included unless only indexes
whose entries outlive a delete involve it
(`index_only_row_commits_created_at`, see
[below](#entries-that-outlive-a-delete-outlivesdelete)) — every required property must be present,
and an optional property (a skip property of a `skipIfAbsent` index, the
only optional kind) contributes nothing when absent, not even its name. It binds the
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
| every property in `required` — except a skip property of a `skipIfAbsent` index; every ancestor of an indexed dotted path required | the index path is the storage; no null layout exists — the one sanctioned hole removes the whole entry instead |
| every index holding an optional property skips on it (its skip set is all of its optional properties) | an absent value that did not skip would need the null layout this mode has no equivalent of |
| every required property appears in ≥ 1 **non-skip** index (prefix or terminal); every optional property appears in a skip index whose skip set is that property alone | only indexed values exist, and a skip index carries no value for a document it skips — covered only there, a property would be validated and committed yet written nowhere |
| **every index embeds `$ownerId`** (prefix or terminal) | entries are self-authorizing: a delete computed with owner = signer can only ever address the signer's own entries |
| ≥ 1 index is `$createdAt`-free AND non-`skipIfAbsent` — the **proof index** | executed-transition proofs locate entries from the transition's values alone: they can neither reproduce a block timestamp nor anchor on an entry that may not exist |
| every terminal component is `$ownerId` or a schema property passing the indexed-shape limits (no arrays or objects; byte arrays ≤ 255 bytes, strings ≤ 63 characters); every component but the last is fixed width; the whole key ≤ 255 bytes | the member key is the components' tree-key encodings concatenated, derived by the same functions the prefix levels use; a leading component must be splittable back and rangeable; grovedb caps keys at 255 bytes; other system properties are refused because the `$createdAt` rules walk the prefix properties |
| no index declares `integerRange` | an entry is keyed by the index's values and terminal, and a bucketed level holds window starts: rows differing only in the bucketed integer would claim the same entry in every window they share |
| a flat index (no `properties`) admits no countable / summable / ranked / `timeRange` / `integerRange` / `skipIfAbsent` / `preallocated` keyword | there is no prefix level for them to apply to |
| every `entryPayload` property is a required, bounded, top-level scalar in no index | the entry value has no representation for an absent property, estimation sizes the item by the bounds, and a property is either a key or a value |
| indexed `$createdAt` requires `$createdAt` in `required` | creation only assigns timestamps for required system times |
| `documentsMutable: false`, no transfers/trading/history/transient | no stored row, no revision |
| non-unique, non-contested, `nullSearchable` default | v1 scope |
| `preallocated` requires a fully reference-determined, non-bucketed path | see [Preallocated index paths](#preallocated-index-paths) |
| a skip property is an optional, top-level schema property; no ranking sits above the index's deepest skip property | see [Conditional participation](#conditional-participation-skipifabsent) |
| a `summableOffCountIndex` index sums a source holding every document once, holds every source property, and fixes its other properties through unchanging `where` values | see [Counters (summableOffCountIndex)](#counters-summableoffcountindex) |

`indexOnly` and the index set (terminals included, `preallocated` and
`skipIfAbsent` flags included) are immutable across contract updates — a
later-added index could never be backfilled, and the walkers derive the
skip from each index's skip set, which therefore cannot drift from
historical entries either. (Respelling `skipIfAbsent: true` as the array of
the same properties is no change: both parse to the same skip set.)

## Conditional participation (skipIfAbsent)

An index may declare `skipIfAbsent`: a document that omits a property of
the index's **skip set** writes no entry into that index at all, and a
delete recomputes the same skip from its carried values. `true` makes the
skip set every optional property of the index (on an indexOnly type it
must be that set anyway, so the array spelling, naming it, parses to the
same index); a skip property may sit at any position, a `timeRange` window
included. The skip properties are the only properties that may leave
`required`, and the rules keep three views provably equivalent: the write
walkers write an index's entry only for a document carrying its skip set
(`document_takes_part_in_index`), the probes derive zero entry paths for
the same documents, and the row commitment pins the exact present-set so
a delete with a different absence pattern fails every probe — a skip
index can neither be force-pruned nor left with an orphan entry.

**No stranded trees.** The merged index structure shares levels across
indexes and prunes empty trees only upward from an entry, so the walkers
build a level only when an index the document takes part in ends at or
below it (`level_reaches_entry`). An untagged like under
`[$createdAt window, hashtag, postId]` still builds the day window shared
with `byDayPost`, but no `hashtag` branch under it; under a grid that only
a skip index uses, it builds no window at all. On an indexOnly type every
index holding an optional property skips on it, so a level keyed by an
absent value is never reached.

The semantics are a **sparse projection**: the index holds exactly the
documents carrying its skip set. Counts, ranked reads and absence proofs
over it answer "among documents with these properties" — a proved empty
position means "no *tagged* like", not "no like". The query router makes
that opt-in (`index_admissible_for_skip_if_absent`, in every index picker,
compiled into the verifier too): a skip index is admissible only when the
query binds every skip property (equality, `in`, range, order-by, or the
ranked property); the generic matcher alone would admit an unbound query
within its difference budget, and an aggregate picker's prefix match could
stop above a deep skip property, silently omitting every skipped row. A
ranking never sits above a skip property (refused at registration: no
query could read it). Structural uniqueness still spans the skip boundary
— a skipped and a taking-part document colliding on any shared non-skip
entry cannot coexist. An absent skip property is distinct from a
present-but-empty value, which indexes normally under its encoded key.

**Coverage.** A document carrying one optional property but missing
another skips every index holding both, so each optional property needs a
skip index whose skip set is that property alone: its value is then
written whenever the document carries it, and the document stays
deletable by values.

The economics are the point: each ranked index costs roughly the same on
every write, so a per-hashtag ranked index on a like doctype used to tax
every like — tagged or not — and forced a `''` sentinel onto the
referenced post's hashtag. With `byHashtagPost` as a skip index, an
untagged like pays only for the indexes it actually appears in, and the
sentinel disappears (see the absence-aware `where` below).

## Lifecycle

- **Create** reuses `DocumentCreateTransitionV0` unchanged. State
  validation probes every index's entry: ANY existing entry is a duplicate
  (`DuplicateUniqueIndexError`), which is also what makes a shorter index a
  uniqueness constraint over its value projection plus owner — for likes,
  the `[postId]` index is the one-like-per-(post, owner) rule. `refersTo`
  validation runs unchanged (it reads transition values, not storage), so a
  like on a nonexistent post is rejected, and a `where` declaration on the
  reference (`{ "hashtag": "hashtag" }`) binds the referenced post's
  property to the like's own: the referenced document is
  already fetched for the existence check, so the equality comparison adds
  no reads, and a like whose hashtag disagrees with its post's is refused.
  Absence is part of the comparison, strictly: both sides absent agree, one
  side absent is the same mismatch a differing value would be — a like may
  omit its hashtag exactly when its post has none (anything laxer would
  let likes on tagged posts silently deflate per-tag aggregates), which is
  what lets a compared property double as a `skipIfAbsent` skip property
  with both sides of the reference optional. The key of an entry, the
  referenced side, may also name the referenced document's `$ownerId` or
  `$creatorId` (the referring side must then be an identifier property):
  `{ "$ownerId": "authorId" }` binds a like to its post's current owner, so an `[authorId, postId]`
  index can be preallocated and ranked per author. `$ownerId` follows the
  post through transfers and `$creatorId` never changes; either is checked
  when the like is written, not when the post later moves. `$creatorId` is
  only recorded by transferable or tradeable types of a format-1 contract,
  which contract registration checks before accepting the declaration.
  The referring side, an entry's value, may in turn be the like's own
  `$ownerId`, the writer: `{ "$ownerId": "$ownerId" }` lets only the post's
  current owner create or replace a like on it, `{ "$creatorId": "$ownerId" }`
  only its original creator. That is a write gate, checked on create and on every
  replace of the like, not only when its reference changes, since the post
  may have been transferred in between; a transfer itself is not
  re-checked, so on a transferable referring type it governs writing, not
  holding. A writer gate does not make an owner-prefixed index
  preallocatable.
- **Delete** is its own transition kind,
  `DocumentIndexOnlyDeleteTransition { base, data }` (`$action:
  "indexOnlyDelete"`), carrying the full value tuple (`$createdAt` under
  its system key exactly when the row commits to it:
  `index_only_row_commits_created_at`). Delete-by-id and
  delete-by-values are different operations — different payload,
  authorization model and validation pipeline — so the factory picks the
  KIND from the doctype's storage mode. Validation and the storage layer
  both require every entry to exist AND match the row commitment. A by-id
  delete on an index-only type (and an indexOnlyDelete on a stored type)
  is rejected by the structure gates; below PV14 the kind is rejected at
  basic structure, keeping check_tx behavior aligned with pre-4.2
  software.
- **Replace / transfer / purchase / price** are structurally impossible.

## Entries that outlive a delete (outlivesDelete)

A delete-by-values recomputes every entry of the document from its values,
and a time-window entry is keyed by the bucket starts of `$createdAt`, which
only the block that included the create assigned. A client that did not keep
that timestamp could not delete the document. `outlivesDelete: true` on a
`timeRange` index with a `ttl` takes the window out of the delete:

- **Delete.** Neither the state validation probes nor Drive's
  row-integrity gate check the index, and the delete walkers do not descend
  into it: `level_removes_entry` skips a level whose indexes all outlive the
  delete (`IndexLevel::outlives_delete_at_or_below` keeps every other
  contract on the old path), and the terminating level of such an index
  removes nothing. Its entries stay until their window is dropped by the TTL
  cleanup, which drops whole buckets.
- **Commitment.** When every index involving `$createdAt` outlives deletes,
  the row commitment leaves the timestamp out, and a delete carries none
  (structure validation refuses one that does). The executed-transition
  proof verifier computes the same commitment, so it no longer needs the
  block time for such a type.
- **Create.** State validation does not probe the index for a duplicate,
  and the terminal insert writes over an entry already standing at the same
  key (left by a deleted document with the same key) without reading it: the
  count does not move, and the entry carries the commitment of the row
  writing it. Registration makes the index's key hold the key of an index a
  delete clears that skips nothing, so no two documents in state share an
  entry, and the within-batch collision tracker leaves these entries to that
  index.
- **Proofs.** `index_only_proof_index` never picks such an index, and the
  parser requires another one to exist.

Registration admits the keyword only on an indexOnly `timeRange` index with
a `ttl`, without a sum and on a type without `entryPayload` (a kept entry
holds the first document's amount or payload), and requires every schema
property to sit in an index that neither skips nor outlives deletes, and the
index's key to hold the key of such an index. The flag is fixed with the
index (`find_first_outlives_delete_change`). The index structure caches the
decisions a delete reads (`IndexLevel::cleared_on_delete_at_or_below`, and at
its root `created_at_indexed_only_by_outliving`), and Drive's delete refuses a
document carrying a `$createdAt` its row does not commit to.

The cost is on the aggregates: a deleted document still counts in the
windows it wrote until they move past it.

## Preallocated index paths

The first entry under a fresh value tuple pays for every tree on its path
— for a like that is the hashtag value tree, the `postId` property-name
tree, the post's value tree and the `0` member bucket — while the second
entry pays for one item insert. When the index path is a pure function of
a refersTo-referenced document, that lopsidedness is avoidable: an index
may declare `preallocated: true` iff every index property is either the
referring property itself (its value is the referenced document's `$id`)
or a referring value of that reference's `where` (consensus-equal to a
referenced-document property, its `$ownerId` and `$creatorId` included),
and the reference is a `permanentDocument` or `moderatedDocument` one
targeting a document type of the **same contract**. A `deletableDocument`
reference shapes the path the same way but does not qualify: its target can
be deleted without a record, and the trees created alongside it would
outlive it with other owners' entries inside and nothing left to say what
they were keyed by. A `moderatedDocument` target leaves state only through a
moderator's removal, whose record is never deleted, so its trees outlive it
the way the record does, and a restore (which puts the document back through
the create path) finds them in place. Through such a reference a binding
counts only when the record keeps every key it binds: the referenced
`$id` or `$ownerId`, or a property the referenced type lists under
`moderatorAbilities.deleteKeepsFields` (`PreallocationBinding::is_kept_on_removal`).
Registration refuses a preallocated index with no such binding
(`validate_preallocated_indexes_kept_on_removal`, once every document type
of the contract is parsed), and the insert path preallocates only through
one (`Index::preallocation_bindings_for_target`, given the referenced
type). `byHashtagPost` (`[hashtag, postId]`) qualifies:
`hashtag` through `where`, `postId` as the reference;
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
**prefix pivot**: one `in` clause may sit on the index's last prefix
property instead of the terminal (`hashtag == h AND postId IN [p, q] AND
$ownerId == me`), with everything above it equality-bound, the terminal
clause an equality, and a limit of at least the number of `in` values.

A range pivot (`postId > p` in the same query), or an `in` pivot with
prefix properties below it, is refused, and the error names the index
shape that serves the query: one that lists the equality-bound
properties, the terminal's included, before the ranged property. A
pivot walk opens one branch per pivot value, and grovedb charges a
branch that holds no row one slot of the limit, so a page of such a
query could hold fewer rows than exist, and the response carries no
cursor to say where it stopped. These shapes stay refused until the
storage layer can report where a page stopped. An `in` pivot on the last
prefix property opens at most one branch per value, so a limit that
covers its values is never used up early. When another index serves the
same query without an incomplete pivot, index selection prefers it over
a pivot index that would win the name-order tie-break.

All shapes prove and verify through the same shared path-query builder.

Not supported on the read surface: by-`$id` fetches (no primary tree —
rejected with guidance) and `startAt` cursors (rejected with the keyset
guidance above); ranked / count / range-aggregate queries work unchanged
since they never open value trees.

## Counters (summableOffCountIndex)

A `summableOffCountIndex` index keeps no entry per document. At the value
position of its last property, where another index grows a value tree, a
`0` bucket and one entry per document, it keeps one `Element::SumItem`
holding the number of entries its source index keeps for that group:

```text
before: byAuthorPost → postAuthor → <author> → postId → <post> → 0 → <liker> = Item(commitment)
after:  byAuthorPost → postAuthor → <author> → postId → <post> = SumItem(likes)
```

The tree of the last property is a count-and-sum tree (`rangeSummable`,
plus `rangeCountable` for the group count an average divides by), so each
counter counts one group and adds its value to the sum. A level a `{ "at": ... }` ranking
names, and every level between it and the counters, carries those totals
up: its value trees are `CountSumTree`s from the shallowest average
ranking down and `SumTree`s above it (a `rankedCountable` on such an index
is its sum ranking, so no count-only chain arises). The property-name tree
of a level a ranking names is the indexed tree for the axes ranked at it,
`ProvableCountProvableSumIndexedTree` for `[Sum, Avg]`; a level between two
ranked levels keeps a plain count-and-sum or sum tree. Grovedb admits a bare
`SumItem` under that indexed tree from grove version 4.

The write path (`add_summable_off_count_counter_operations`):

- **Create**: reads the counter and writes it back one higher, or inserts
  it at one for the first document of the group. The create is refused
  before it gets here when the source already holds the entry, so the
  counter equals the source group's entry count.
- **Delete**: writes it back one lower once the entries of the indexes
  that keep them matched the row commitment. A preallocated index keeps
  the counter at zero; any other removes it with its last document and
  prunes the trees it leaves empty, up to the document type.
- **Once per batch**: a counter is written at most once per batch, because
  a documents batch carries one transition. A source keyed by more than its
  owner (a terminal such as `["$ownerId", "emoji"]`) holds several entries
  of one owner in one group, and each document is converted on its own, so
  raising that cap needs the counter moves folded across documents first.
  A second write within one conversion is refused as corrupted code
  execution.
- **Storage**: a `SumItem` is charged a fixed 11 bytes plus flags whatever
  its value, so a rewrite stores nothing new, and it keeps the flags of the
  first document that paid for it.
- **Preallocation**: creating the referenced document creates the counter
  at zero, in place of the value tree and its empty `0` bucket.

The state probes and the duplicate check skip the index (it decides
nothing about a create), it is never the proof index, and document
queries never read it. The count, sum, average and ranked queries do: a
sum query names the source index (`sum(byPost)`), a count query takes a
counter's sum (its group's documents) where another index's read takes a
count (a ranked or `HAVING` count walks the Sum secondaries, and its
entries come back as counts), and a point query may stop at a level
carrying the sums, reading that value tree's element. A range count reads
the range sums (`DriveDocumentCountQuery::counter_sums_query`): every range
count executor and verifier hands the same index and clauses to the sum
surface's counterpart, summing the source index, and reads the sums back as
counts, since grovedb's range count over the counters would count them, one
per group. A range total over an index whose path passes through a ranked
level (its own, or one another index ranks at a shared level) is refused with
a hint to group by the last property, as everywhere: a ranked level's tree is
indexed, and grovedb neither totals a range over an indexed tree nor proves a
range total through one, so the unproven read refuses it too and the two
agree.

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
