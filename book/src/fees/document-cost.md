# What a Document Costs

`drive::document::cost` computes what creating one document costs, from the
contract alone. The JavaScript SDKs expose it as
`documentCreateCost(contract, documentTypeName, options, platformVersion)`,
which is what the contract visualizer shows per document type.

Amounts are in credits: 1 Dash is 100,000,000,000 credits.

## Storage

The storage fee is the bytes the insert adds times
`storage_disk_usage_credit_per_byte` (27,000 credits from protocol version 14).
It is usually almost all of what a document costs.

The estimate is exact. It lists every element the insert writes (see
[GroveDB Structure](../drive/grovedb-structure.md) for the layout), built with
the functions the insert walkers use:

- the document by id, or with `documentsKeepHistory` a tree of revisions and a
  pointer to the newest;
- for each index, the value trees along its properties (created only when no
  earlier document has the value), the `[0]` terminal and the document's
  reference in it, or an indexOnly type's entry;
- the rows a ranked index keeps for the value in its secondary trees, one per
  ranking axis;
- the trees preallocated for other document types' `preallocated` indexes
  whose entries will reference the document, charged to its creator.

Each element is priced with GroveDB's byte formulas: the key with the 32-byte
subtree prefix, the serialized element (or a fixed size standing for a tree),
the value and node hashes, the aggregate feature of the tree it sits in (8 or
16 bytes in a count or sum tree), and the link its parent keeps to it. A
document create's elements carry 35 bytes of storage flags (the owner and the
epoch); index trees carry them only when the documents are mutable, the
contract can be deleted, or the type is indexOnly and its documents can be
deleted.

The storage depends on what is already stored, so it comes in two scenarios:

- **every value new**: the first document with these index values creates
  their trees;
- **every value known**: a later document with the same values adds only its
  own entries. A unique index still adds its value, which no earlier document
  can hold, as does every tree keyed by the document's own id (a preallocated
  index's, say) and a `ttl` document's expiration entry; a ranked index's row
  for an existing value only moves, which GroveDB bills as replaced bytes.

The estimate also splits the storage by index: the layers an index shares with
other indexes (a common prefix of properties, paid once for the document) and
the layers only it uses, so an index's cost on its own is its shared layers
plus its own. A `skipIfAbsent` index that skips the document writes nothing,
and adds no layer, just as Drive builds none for it; the rows of a ranked
level under a time window with a `ttl` are priced like the window, as
processing.

The test `should_price_what_drive_charges` inserts documents into 21 contracts
covering every index shape and requires the estimate to equal the storage fee
Drive charged, byte for byte, with the trees already stored read from GroveDB
before each insert.

## Processing

- **Exact:** verifying the signature (15,000 credits for an ECDSA key, 300,000
  for BLS) and fetching the signing key and the identity's balance (18,000).
- **Estimated:** the work of the writes (seeks, hashing and rewriting the path
  to each new element in its tree and above), for an assumed number of stored
  documents, each with its own values (1,000 by default); and the small reads
  and writes around the insert (the document id check and the identity's
  contract nonce). The test
  `should_estimate_the_processing_of_the_writes_within_a_factor_of_two` holds
  the write estimate to Drive's processing fee. Processing is a few percent of
  a document's cost.
- A `userFeeIncrease` raises the processing fee only.

The per-document "minimum fee" of a batch (`document_batch_sub_transition`) is
a balance check before processing, not a charge, and the unique index checks
and the fetch of the batch's own contract are not billed.

## What the contract adds

- **Action fees** (`actionFees`): the create's fee, in credits, scaled by the
  epoch's fee multiplier unless priced as fixed, paid into the contract's fee
  pots.
- **Token cost** (`tokenCost`): tokens transferred to the contract owner or
  burned.
- **Contest fund**: a contested index's vote fund (0.1 Dash, doubling past 250
  contenders), paid when the value is contested.

## Refunds

Deleting a document refunds the storage fee of its flagged elements, less what
the epochs already passed were paid: about 99.9% in the epoch it was created,
about 95% a year later, then less each year for fifty years.

## Documents with a `ttl`

A document whose type declares a [`ttl`](../contract-keywords/ttl.md) is
stored without flags and pays for its bytes by the lifetime it has left, all of
its `ttl` when it is created, at the schedule's tier for that lifetime (from 1
credit per byte for an hour to 26 for a week, then 34 per 788,400 seconds)
instead of the 27,000 of storage kept for good. It also adds its entry to the
documents expirations tree, and prepays its deletion as processing (a base
cost, a cost per index level and one per document byte). Nothing of it is
refunded.

## Not covered

- A create whose value starts a contest (a DPNS name matching the contest
  rule, say) is stored in the contest's vote poll until the contest ends, not
  in the index. The estimate prices an uncontested create and lists the contest
  fund; the vote poll's storage is not priced.
- A platform version whose insert methods differ from protocol version 14's is
  refused: other versions write other elements for some shapes.
