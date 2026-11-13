# History

Platform can keep two kinds of history for a document type. `documentsKeepHistory` keeps every version of each document in Drive, so an application can read what a document said at any earlier time. The three `keeps*History` flags record events instead: each transfer, purchase or price update of a document becomes a record in the document history system contract, where it can be queried by document, by contract, by identity and by time.

## `documentsKeepHistory`

Keeps every version of every document of the type, not only the latest.

| | |
|---|---|
| **Where** | document type |
| **Value** | boolean |
| **Default** | the contract config's `documentsKeepHistoryContractDefault`, which is `false` unless the contract says otherwise |
| **Since** | protocol version 1 |
| **On update** | Fixed (`DocumentTypeUpdateError`, 40212). Adding or removing the key without changing its value is refused too, as a schema change (`IncompatibleDocumentTypeSchemaError`, 10246). |
| **Errors** | `InvalidDocumentTransitionActionError` (10404) for a delete at protocol version 14, or when `canBeDeleted` refuses it |

### Example

```json
"profile": {
  "type": "object",
  "documentsKeepHistory": true,
  "canBeDeleted": false,
  "properties": {
    "displayName": { "type": "string", "maxLength": 25, "position": 0 },
    "bio": { "type": "string", "maxLength": 140, "position": 1 }
  },
  "required": ["displayName"],
  "additionalProperties": false
}
```

Every edit of a profile adds a version. This example explicitly prevents owner deletion; the usual default is `true`. Protocol version 14 requires `canBeDeleted: false` on a history type. From version 15, a deletable history type may remove the current document while retaining its revisions, and may separately allow their erasure with [`canBeErased`](deletion.md#canbeerased).

### How it works

- Through protocol version 14, each document is stored as a small tree of its own, with versions keyed by block time and a pointer to the current one. From protocol version 15, versions live in a separate history tree for the document type and retain both time and revision, including edits sharing a block time. A query by id or through an index sees the current version.
- Every write that changes the document adds a version: a replace, and also a transfer, a price update or a purchase.
- Through protocol version 14, Drive refuses to delete a document whose type keeps history (10404 at version 14; before it, the delete failed inside Drive as an internal error). From version 15, an allowed owner delete removes the current pointer and indexes but retains every revision. Erase transitions may then remove retained revisions if the type declares `canBeErased: true`. History types still allow neither moderator deletion nor a `ttl`.
- The `getDocumentHistory` query returns a document's versions from a given time on, each with the block time it was written at, at most 10 per request, and with a proof when asked.
- Each retained version is paid for by the write that added it. Erasure refunds removed storage to its recorded payer.
- A doctype-level sum or average (`documentsSummable`, `documentsAverageable`) counts only each document's current version. See [Counts, Sums and Averages](aggregates.md).

`getDocumentHistory` uses request and response version 0 through protocol version
14. Its `start_at_ms` is exclusive, `offset` skips entries, and response entries
contain `date` and document `value` bytes. Proved requests require an absent or
zero offset.

From protocol version 15, the query accepts version 1 only. It requires exactly
one filter: inclusive `start_at_ms`, exclusive `start_after` with both time and
revision, inclusive `start_at_revision`, or exact `revision`. Unproved responses
carry entries with time, revision and document bytes, plus lifecycle metadata;
proved responses carry one signed proof object containing one GroveDB proof of
the page and lifecycle.
The Rust SDK and its WASM/JavaScript bindings send version 1 and require proofs.
On an updated node running protocol version 14 this returns the standard
unsupported-query-version error; the SDK does not fall back to version 0.

### Rules at registration

All refusals below are `InvalidContractStructure` (10231).

- At protocol version 14, the type must set `canBeDeleted: false`; an older contract with both flags on stays readable but must turn deletion off on its next update. From version 15, deletable history is admitted. Older types whose deletion flag is true become soft-deletable at activation without an update. See [Deletion](deletion.md).
- Refused together with `ttl`, with `moderatorAbilities.delete` and with `indexOnly`.
- From protocol version 15, refused together with a contested index.

## `keepsTransferHistory`

Records every transfer of a document of the type in the document history contract.

| | |
|---|---|
| **Where** | document type |
| **Value** | boolean |
| **Default** | `false` |
| **Since** | protocol version 13 |
| **On update** | Fixed (`DocumentTypeUpdateError`, 40212) |
| **Errors** | none of its own |

## `keepsPurchaseHistory`

Records every purchase of a document of the type in the document history contract.

| | |
|---|---|
| **Where** | document type |
| **Value** | boolean |
| **Default** | `false` |
| **Since** | protocol version 13 |
| **On update** | Fixed (`DocumentTypeUpdateError`, 40212) |
| **Errors** | none of its own |

## `keepsPricingHistory`

Records every price update of a document of the type in the document history contract.

| | |
|---|---|
| **Where** | document type |
| **Value** | boolean |
| **Default** | `false` |
| **Since** | protocol version 13 |
| **On update** | Fixed (`DocumentTypeUpdateError`, 40212) |
| **Errors** | none of its own |

## The document history contract

### Example

```json
"ticket": {
  "type": "object",
  "documentsMutable": false,
  "canBeDeleted": false,
  "transferable": 1,
  "tradeMode": 1,
  "keepsTransferHistory": true,
  "keepsPurchaseHistory": true,
  "keepsPricingHistory": true,
  "properties": {
    "event": { "type": "string", "maxLength": 63, "position": 0 },
    "seat": { "type": "string", "maxLength": 10, "position": 1 }
  },
  "required": ["event", "seat"],
  "additionalProperties": false
}
```

Each time a ticket is given away, listed or sold, a record of it is written to the document history contract, so anyone can look up who held a ticket, what it was offered at and what it sold for. The DPNS `domain` type sets all three flags from protocol version 13.

### How records work

The document history contract is a system contract registered at protocol version 13, with the id `6voHRaoiPcfmMhbqCA9dixH98xcgPQ9UEcuaXjpVu3LD`. It has one document type per flag:

| Flag | Record type | Written for | Properties | `$ownerId` of the record |
|---|---|---|---|---|
| `keepsTransferHistory` | `transfer` | each transfer | `dataContractId`, `documentTypeName`, `documentId`, `toIdentityId` | the sender |
| `keepsPurchaseHistory` | `purchase` | each purchase | `dataContractId`, `documentTypeName`, `documentId`, `sellerId`, `price` | the buyer |
| `keepsPricingHistory` | `priceUpdate` | each price update | `dataContractId`, `documentTypeName`, `documentId`, `price` | the owner who set the price |

- The platform writes the record as part of the transition that made the change, so a record exists exactly when the transfer, purchase or price update succeeded. Each record also carries `$createdAt` and `$createdAtBlockHeight`, the block's time and height.
- The identity that signed the action owns the record, and writing it is part of that transition's fees.
- Records can never be changed or deleted, and nobody can create one directly: the record types set `documentsMutable: false`, `canBeDeleted: false` and `creationRestrictionMode: 2`.
- The record types are indexed for lookups by document (`byDocument`: contract, document, time) and by contract (`byContract`: contract, time). Transfers are also indexed by sender (`from`) and recipient (`to`), purchases by buyer (`buyer`), seller (`seller`) and price (`byPrice`).
- They also keep provable aggregates. `purchase` keeps a count, total and average of `price` over all its records, and per contract or per document over a time range. `priceUpdate` keeps a count of all its records, and a count, total and average of asking prices per contract or per document over a time range, where a document listed three times counts three times. `transfer` keeps a count of all its records, and per contract over a time range. See [Counts, Sums and Averages](aggregates.md).
- A flag only records the action the type allows. `keepsTransferHistory` on a type that is not transferable records nothing; no rule ties the flags to `transferable` or `tradeMode`.
- The flags are fixed on update, so an existing type cannot start or stop recording. A type added by an update may set them.

### Rules at registration

- An `indexOnly` type may keep no history of either kind. See [Index-Only Types](index-only.md).

## See also

- [Creation, Transfers and Trading](ownership-and-trading.md), for the actions the three flags record
- [Deletion](deletion.md) and [Time To Live](ttl.md), for owner deletion, erasure and the expiry restriction
- [Keep-History Documents](../drive/keep-history-documents.md), for lifecycle states, temporal reads and proofs
- [Counts, Sums and Averages](aggregates.md), for the aggregates of the history records
- [Contract-Level Keys and config](contract-config.md), for `documentsKeepHistoryContractDefault` and the contract's own `keepsHistory`
