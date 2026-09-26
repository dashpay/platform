# Document Time To Live

A document type may declare a **time to live**. Every document of such a type is deleted by
the platform once its time to live has passed, whoever owns it and whatever the type says
about who else may delete it. Its owner pays, when the document is written, for the time
the document will occupy the state rather than for perpetual storage, and prepays its
deletion. Available from protocol version 14.

```json
"note": {
  "type": "object",
  "ttl": 1209600,
  "properties": { "text": { "type": "string", "maxLength": 280, "position": 0 } },
  "required": ["$createdAt", "text"],
  "additionalProperties": false
}
```

Documents of `note` above are deleted two weeks (1,209,600 seconds) after their creation.

This is a different feature from the `ttl` key of a `timeRange` index (see
[Time-Range Index TTL](../drive/time-range-ttl.md)), which expires index entries and leaves
the document in place.

## Semantics

- A document expires at its `$createdAt` plus `ttl` seconds. `$createdAt` is set from block
  time when the document is created, so nothing the writer sends moves the expiry, and every
  document of a type created in one block expires at the same time.
- A replace, a transfer, a purchase or a price update never moves the expiry: `$createdAt`
  never changes. A buyer of an expiring document buys what is left of its life.
- After each block's state transitions the platform deletes the expired documents, oldest
  first, at most `max_document_expirations_per_block` per block (128 at protocol version
  14). The rest wait for the next block. A state transition in the block a document expires
  in still sees it; a document can therefore outlive its expiry by the part of a block
  before the cleanup, and by more when a backlog builds.
- The expiry belongs to the document: it is its own `$createdAt` plus the type's `ttl`, both
  fixed, and the platform reads it from there. The expirations tree is only the index the
  cleanup finds documents through.
- From its expiry on, a document can no longer be replaced, transferred, bought or repriced,
  and a moderator can no longer restore it: each is refused, paid, with
  `DocumentExpiredError` (40140), whether or not the cleanup has reached the document. Its
  owner may still delete it, which only removes it sooner. Until the cleanup deletes it, it
  can still be queried and referenced, and it keeps its values in the type's unique indexes:
  a create of the same unique value is refused as a duplicate until the cleanup has run. The
  cleanup deletes a fixed number per block, so a sustained flood of creations with a short
  time to live builds a backlog it drains at that rate, and that lag grows with it.
- The document may be deleted earlier as usual: by its owner when `canBeDeleted` allows it,
  by the contract's moderators when `canBeDeletedByModerators` does. `canBeDeleted: false`
  only stops the owner; the platform still deletes the document when it expires.
- The deletion is an ordinary deletion: the document and every index entry go, count and sum
  trees are decremented, and nothing is left behind.

## Where a time to live is refused

The keyword is refused, on every parse of the contract (registration, update and a stored
contract read back), when:

| The document type | Why |
|---|---|
| does not list `$createdAt` in `required` | The expiry is computed from it. |
| sets `documentsKeepHistory: true` | Drive refuses to delete a document whose type keeps history. |
| sets `indexOnly: true` | There is no stored row to delete by id. |
| has a contested index | A contested document waits in its vote poll until the poll awards it, keeping the `$createdAt` of its create, so it could expire before it is stored. |
| declares `ttl: 0` | A time to live lasts at least a second. |

When a contract is registered or updated, a `ttl` below `min_document_ttl_seconds` (one hour at
protocol version 14) or above `max_document_ttl_seconds` (one year) is refused too. The floor
keeps a document in state well past the moment its writer fetches the proof of its create,
which proves the document present; a document the cleanup had already deleted would fail that
proof although the create succeeded. A contract update may not add, remove or change the `ttl`
of an existing document type: every stored document carries the expiry it was written and paid
with. A document type added by an update declares it freely.

The same holds for any write close to a document's expiry: a replace, transfer, purchase or price
update accepted in the last block before the expiry proves the document present, and a proof
fetched after the next block's cleanup finds it gone.

References treat a type with a `ttl` as deletable, like one with `canBeDeleted` or
`canBeDeletedByModerators`. A `permanentDocument` reference, a lookup one included, and a list
element reference may not target it; a `deletableDocument` reference may, a lookup one included.
The check is `DocumentTypeV2Getters::documents_can_disappear`.

Everything else composes: mutable types, `transferable`, `tradeMode`,
`canBeDeletedByModerators` (a moderator's restore puts the document back with its original
`$createdAt`, and is refused once that document has expired),
`creationRestrictionMode`, countable, summable and ranked indexes, `timeRange` indexes with
or without their own `ttl`, `refersTo` declared on the type, action fees and token costs.

## Storage

Nothing a document of such a type writes carries storage flags: its primary item, its index
entries and the index trees it creates are flagless, and its deletion refunds nothing.
Drive drops the writer's flags when the document is inserted or changed
(`DocumentAndContractInfo::without_storage_flags_if_expiring`), before any element is sized,
and the re-tagging described below strips any left.

Each document also gets an entry in the **documents expirations tree** under `Misc`:

```text
Misc (104) / E / <expires at, u64 big endian ms> / <document id> -> contract id (32 bytes) ++ document type name
```

The entry is written with the document and removed with it, whoever deletes it (the hook is
in `force_delete_document_for_contract_operations`, which the owner's, the moderators' and the
cleanup's deletions share). The last entry of an expiry time takes the tree of that time with
it, so every tree of an expiry time holds at least one entry, and a document deleted early
leaves nothing the cleanup would have to read. The expirations tree itself is created with the
initial state structure of protocol version 14 and on the first block of protocol version 14,
through one helper (`Drive::insert_documents_expirations_tree`).

## Fees

The fee schedule's `document_ttl` group (`FeeDocumentTtlVersion`, `FEE_VERSION3`) prices a
document of such a type:

- **Bytes.** Every byte the document writes, its expirations tree entry included, costs the
  price of the lifetime it has left. Up to seven days a tier applies; past that a price per
  `pricing_period_seconds` spanned, rounded up. The period is part of the schedule (788,400
  seconds, mainnet's epoch length), not the node's epoch length, so a network configured with
  short epochs, like testnet's hour, prices a lifetime as mainnet does.

  | Lifetime | Credits per byte (protocol version 14) |
  |---|---|
  | up to 1 hour | 1 |
  | up to 1 day | 4 |
  | up to 2 days | 8 |
  | up to 4 days | 15 |
  | up to 7 days | 26 |
  | longer | 34 per 9.125 days spanned |

  The values are the first year's share of the perpetual storage price (27,000 credits per
  byte, 5% of it paid out in the first year) pro rata, rounded up. A one-year `ttl` pays
  40 × 34 = 1,360 credits per byte, about what a permanent document deleted after a year
  keeps paying net of its refund.
- **Route.** A document of a type whose `ttl` is shorter than `processing_route_below_epochs`
  epochs (two) of the network pays that amount into the current epoch's processing fees; one
  of a longer `ttl` into the storage fee distribution pool, which spreads it over future
  epochs like any storage fee. Here the epoch is the network's: the node's
  `epoch_time_length_s`, handed to Drive through `DriveConfig`. The route follows the declared
  `ttl`, not the lifetime left, so every write of a document takes the same one.
- **Deletion.** Creating the document prepays, as processing, what its deletion will cost:
  `cleanup_base_processing_cost` (1,200,000) plus `cleanup_processing_cost_per_index_level`
  (400,000) per index level of the type, where an index counts its properties, times the
  overlapping windows of a `timeRange` index. The cleanup itself bills nobody.
- **Changes.** A replace, transfer, purchase or price update prices the bytes it adds by the
  lifetime left at that block and pays no second deletion fee. A deletion by the owner or a
  moderator pays its own processing like any deletion; the prepaid deletion is the
  platform's, and is not refunded.

The price never decreases with the lifetime and the route depends on the `ttl` alone, so an
estimate made at an earlier block time (check_tx) stays an upper bound of the execution. A dry
run estimates a replace as an insert, the entry and the prepaid deletion included, which only
raises it. In Drive the document's grove operations are
re-tagged `EphemeralGroveOperation(_, EphemeralPricing::DocumentTtl { .. })` and applied as
their own GroveDB batch, so their added bytes can be priced apart from the rest of the
transition (see `apply_batch_low_level_drive_operations`); operations already tagged for a
`timeRange` index's `ttl` keep that rule.

## Expired documents

`validate_document_not_expired` (drive-abci, `state_transition/common`) is the one rule: a
document of a type with a `ttl` has expired when block time is at or past its `$createdAt`
plus the `ttl` (Drive's `document_expires_at`, the time its entry is keyed by), the same
boundary the cleanup deletes at. The batch's state validation (v0, in place, unreachable before
protocol version 14) calls it once per action through `validate_batched_action_not_expired`,
an exhaustive match that refuses a replace, transfer, purchase or price update of an expired
document before the action's own checks. The moderator restore calls it too, judging the
`$createdAt` of the document the removal record's hash pins. A document deletion by its owner
does not.

## Cleanup

`Platform::expire_documents` runs after the block's state transitions, right after the address
balance cleanup in `run_block_proposal`, and calls `Drive::remove_expired_documents` with
`max_document_expirations_per_block`:

1. `fetch_expired_documents` reads, in one query, the entries of the expiry times at or before
   the block time, oldest first, at most the limit of them. Every tree of an expiry time holds
   an entry, so the query visits at most the limit of trees.
2. Each expired document is checked against state (its contract, document type, `ttl` and
   stored document, and that the document expires when its entry says) and deleted through
   `DocumentOperationType::ForceDeleteDocument`, the moderators' deletion, each in its own batch
   so every index tree the next deletion reads is final. Its entry, and the tree of its expiry
   time when it was the last, go with it. The fee result is discarded.
3. An entry without a document to delete is logged and removed; the block's orphans share one
   batch. None is expected, and failing the block over one would halt the chain.

## Versioning

Everything rides protocol version 14, unreleased when this landed: the keyword joined document
meta-schema v3 and the generation 3 parser, the limits joined `SYSTEM_LIMITS_V4`, the fee group
joined `FEE_VERSION3`, the update rule joined `validate_update` v1, and `expire_documents` is
`Some(0)` in `DRIVE_ABCI_METHOD_VERSIONS_V10` only. Four shipped generations were edited in
place, each inert before 14: the deletion hook in `delete_document_for_contract_operations` v0
and the expiry check in the batch's state validation v0 (`documents_ttl_seconds` is only ever
`Some` on a document type parsed from the keyword, which no earlier protocol version reads),
the `expire_documents` call in `run_block_proposal` v0 (the method is `None` before 14), and
one batch per pricing rule in `apply_batch_low_level_drive_operations` v0 (nothing is tagged
ephemeral before 14).
