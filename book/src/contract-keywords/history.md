# History

Platform can keep two kinds of history for a document type. `documentsKeepHistory` keeps every version of each document in Drive, under the document itself, so an application can read what a document said at any earlier time. The three `keeps*History` flags record events instead: each transfer, purchase or price update of a document becomes a record in the document history system contract, where it can be queried by document, by contract, by identity and by time.

## `documentsKeepHistory`

Keeps every version of every document of the type, not only the latest.

| | |
|---|---|
| **Where** | document type |
| **Value** | boolean |
| **Default** | the contract config's `documentsKeepHistoryContractDefault`, which is `false` unless the contract says otherwise |
| **Since** | protocol version 1 |
| **On update** | Fixed (`DocumentTypeUpdateError`, 40212). Adding or removing the key without changing its value is refused too, as a schema change (`IncompatibleDocumentTypeSchemaError`, 10246). |
| **Errors** | `InvalidDocumentTransitionActionError` (10404) for a delete of a document of the type, from protocol version 14 |

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

Every edit of a profile adds a version, and every earlier version stays readable. `canBeDeleted: false` has to be written out, since a type that keeps history can never delete and the default is `true`.

### How it works

- Each document is stored as a small tree of its own: every version the document has had, keyed by the block time it was written at, and a pointer to the current one. A query by id or through an index sees the current version.
- Every write that changes the document adds a version: a replace, and also a transfer, a price update or a purchase.
- Nothing is ever removed. Drive refuses to delete a document whose type keeps history, so its owner's delete is refused (10404 from protocol version 14; before it, the delete failed inside Drive as an internal error), and the type can have neither moderator deletion nor a `ttl`.
- The `getDocumentHistory` query returns a document's versions from a given time on, each with the block time it was written at, at most 10 per request, and with a proof when asked.
- Every version stays stored, paid for by the write that added it.
- A doctype-level sum or average (`documentsSummable`, `documentsAverageable`) counts only each document's current version. See [Counts, Sums and Averages](aggregates.md).

### Rules at registration

All refusals below are `InvalidContractStructure` (10231).

- From protocol version 14, the type must set `canBeDeleted: false`. A contract registered earlier with both flags on stays readable, and its next update must turn `canBeDeleted` off on that type. See [Deletion](deletion.md).
- Refused together with `ttl`, with `moderatorAbilities.delete` and with `indexOnly`.

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
- [Deletion](deletion.md) and [Time To Live](ttl.md), for why a type that keeps history can never lose a document
- [Counts, Sums and Averages](aggregates.md), for the aggregates of the history records
- [Contract-Level Keys and config](contract-config.md), for `documentsKeepHistoryContractDefault` and the contract's own `keepsHistory`
