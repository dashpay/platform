# Time To Live (ttl)

`ttl` gives every document of a type a lifetime. The platform deletes each document once that many seconds have passed since it was created, whoever owns it and whatever the type says about who else may delete it. Use it for content that is meant to disappear: stories, invitations, offers, session records. Such documents are also cheaper: they pay for the time they occupy the state rather than for storage forever.

This is the document type keyword. An index can carry a `ttl` of its own inside `timeRange`, which expires index entries and leaves the documents in place; see [Time-Range Indexes](time-range.md).

| | |
|---|---|
| **Where** | document type |
| **Value** | integer, seconds, 3600 (one hour) to 31536000 (one year) |
| **Default** | absent: documents live until someone deletes them |
| **Since** | protocol version 14 |
| **On update** | Fixed (`DocumentTypeUpdateError`, 40212): an update may not add, remove or change it |
| **Errors** | `DocumentExpiredError` (40140) for a replace, transfer, purchase or price update of a document past its expiry |

## Example

```json
"story": {
  "type": "object",
  "ttl": 86400,
  "canBeDeleted": true,
  "properties": {
    "caption": { "type": "string", "maxLength": 200, "position": 0 },
    "mediaUrl": { "type": "string", "maxLength": 500, "position": 1 }
  },
  "required": ["$createdAt", "mediaUrl"],
  "additionalProperties": false
}
```

Each story is deleted one day (86,400 seconds) after its creation. Its author may delete it sooner. `$createdAt` must be in `required`, since the expiry is counted from it.

## How it works

- **When a document expires.** At its `$createdAt` plus `ttl` seconds. `$createdAt` is the block time of the create, so nothing the writer sends moves the expiry, and documents created in the same block expire together. A replace, transfer, purchase or price update never moves it either: a buyer of an expiring document buys what is left of its life.
- **When it is deleted.** After each block's state transitions, the platform deletes expired documents, oldest first: at most 128 per block, and at most 1,024 in weight, where a document weighs 1 plus the index levels of its type (the values at protocol version 14). The rest wait for the next block. The deletion is an ordinary one: the document and every index entry go, and counts and sums are brought down.
- **Between expiry and deletion.** A document past its expiry that the cleanup has not reached yet can still be queried and referenced, and it still holds its values in the type's unique indexes, so a create with the same unique value is refused as a duplicate until the cleanup has run. It can no longer be replaced, transferred, bought or repriced, and a moderator can no longer restore it: each is refused, paid, with `DocumentExpiredError` (40140). Its owner may still delete it where `canBeDeleted` allows.
- **Earlier deletion.** The owner may delete a document before it expires when `canBeDeleted` allows it, and the contract's moderators when `moderatorAbilities.delete` does. `canBeDeleted: false` only stops the owner; the platform still deletes the document when it expires.
- **What it costs.** The document is stored without storage flags and refunds nothing when it is deleted, by anyone. Instead of the price of permanent storage, each byte it writes pays a price for the time it will live: five tiers up to seven days, then a price per 9.125 days spanned. Creating it also prepays, as processing, the cost of its later deletion. A replace, transfer, purchase or price update pays for the bytes it adds at the price of the lifetime left. See [Fees](../data-model/document-ttl.md#fees).
- **Proofs near the expiry.** A write accepted in the last block before a document expires proves the document present. A proof fetched after the next block's cleanup finds it gone. The one-hour minimum keeps a newly created document in the state well past the moment its writer fetches the proof of the create.

## Rules at registration

The refusals below are `InvalidContractStructure` (10231) unless a bullet says otherwise. They hold on every parse of the contract, except the bounds, which are checked when a contract is registered or updated.

- `$createdAt` must be in `required`.
- Refused together with `documentsKeepHistory: true` (Drive never deletes a document whose type keeps history), with `indexOnly: true` (there is no stored row to delete by id), and on a type with a contested index (a contested document waits in its vote poll, and could expire before it is stored).
- A summed property ([`summable`, `averageable`, `documentsSummable` or `documentsAverageable`](aggregates.md)) declares a `minimum` of at least 0, or a `minimum` of at least -134217728 and a `maximum` of at most 134217728 (±2^27, `max_expiring_signed_summed_value_magnitude`). Deleting an expired document takes its value out of the type's sums at the end of a block, with no transition to refuse: removing values that are never negative only lowers the sums, and values this small keep them in the signed 64-bit range short of 2^36 documents. Checked when a contract is registered or updated, like the bounds below.
- At least `min_document_ttl_seconds` and at most `max_document_ttl_seconds` of `SystemLimits`: 3600 and 31536000 at protocol version 14. The meta-schema itself admits 1 to 4294967295, so a `ttl` of 0 is a `JsonSchemaError` (10101) and one outside the narrower bounds is 10231.
- For references, a type with a `ttl` is deletable. A `permanentDocument` reference may not point at it, one found by `findBy` or with `inList` included (`ReferencedDocumentTypeDeletableError`, 40122); a `deletableDocument` reference may. See [References](refers-to.md).

Everything else combines with a `ttl`: mutable types, `transferable`, `tradeMode`, `moderatorAbilities.delete`, `creationRestrictionMode`, count, sum and ranked indexes, `timeRange` indexes with or without their own `ttl`, references declared on the type, action fees and token costs, with a summed property bounded as above.

### On update

An update may not add, remove or change the `ttl` of an existing document type. Every stored document has the expiry it was written and paid with: adding one would leave stored documents that the cleanup cannot find, removing it would leave entries that delete documents the type says live forever, and changing it would move expiries away from what was paid for. A document type added by an update may declare a `ttl` freely.

## See also

- [Document Time To Live](../data-model/document-ttl.md), for the expirations tree, the fee tiers, the payout to epochs and the cleanup
- [Deletion](deletion.md), for the other two ways a document is deleted
- [Time-Range Indexes](time-range.md), for the index `ttl`
- [History](history.md), for why a type that keeps history cannot expire
- [System Properties](system-properties.md), for `$createdAt`
