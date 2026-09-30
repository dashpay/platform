# Deletion

A document can leave the state three ways: its owner deletes it, the contract's moderators delete it, or the platform deletes it when its time to live runs out. `canBeDeleted` rules the first, `moderatorAbilities.delete` and `moderatorAbilities.deleteWithin` the second, and `ttl` the third (see [Time To Live](ttl.md)). Each is independent of the others: a type may let moderators remove what its authors cannot retract, or expire documents that nobody may delete by hand.

## `canBeDeleted`

Whether a document's owner may delete it. Set it to `false` for records that other documents or other people rely on staying put.

| | |
|---|---|
| **Where** | document type |
| **Value** | boolean |
| **Default** | the contract config's `documentsCanBeDeletedContractDefault`, which is `true` unless the contract says otherwise |
| **Since** | protocol version 1 |
| **On update** | Fixed (`DocumentTypeUpdateError`, 40212), except that a type which keeps history before and after the update may change it from `true` to `false`. Adding or removing the key without changing its value is refused too, as a schema change (`IncompatibleDocumentTypeSchemaError`, 10246). |
| **Errors** | `InvalidDocumentTransitionActionError` (10404) for a delete of a type set to `false`, or, from protocol version 14, of a type that keeps history; `DocumentOwnerIdMismatchError` (40102) for a delete by anyone but the owner |

### Example

```json
"comment": {
  "type": "object",
  "canBeDeleted": true,
  "properties": {
    "postId": {
      "type": "array",
      "byteArray": true,
      "minItems": 32,
      "maxItems": 32,
      "contentMediaType": "application/x.dash.dpp.identifier",
      "position": 0
    },
    "text": { "type": "string", "maxLength": 500, "position": 1 }
  },
  "required": ["postId", "text"],
  "additionalProperties": false
}
```

A commenter may take a comment down at any time. Since `true` is the usual default, the key could be left out; writing it makes the intent plain.

### How it works

- The owner deletes a document with a delete transition that names its id. Anyone else is refused (`DocumentOwnerIdMismatchError`, 40102), and a document that does not exist is `DocumentNotFoundError` (40101).
- The owner is refunded the part of the document's storage fee that has not yet been paid out to past epochs. A document of a type with a `ttl` refunds nothing. See [Refunds](../fees/overview.md#refunds).
- A delete may carry a token cost or an action fee, like any document action. See [Token Costs](token-cost.md) and [Action Fees](action-fees.md).
- An identity that is banned or suspended on a moderated contract may still delete its own documents. See [Contract Moderation](../data-model/contract-moderation.md#the-model).
- `false` binds only the owner. The contract's moderators, when the type allows them, and the platform, when the type has a `ttl`, still delete such documents.
- Drive never deletes a document whose type keeps history (`documentsKeepHistory`). From protocol version 14 a delete of such a document is refused with 10404 whatever `canBeDeleted` says; before it, the delete failed inside Drive as an internal error.
- Documents of an `indexOnly` type are deleted with an index-only delete transition that carries their values, since there is no stored row to name by id. A delete by id of such a document is refused (10404). See [Index-Only Types](index-only.md).

### Rules at registration

- From protocol version 14, a type with `documentsKeepHistory: true` must set `canBeDeleted: false` (`InvalidContractStructure`, 10231). The default is `true`, so it has to be written out. A contract registered earlier with both flags on stays readable, but its next update is checked like a new contract, so that update must turn `canBeDeleted` off on the type. That is the one change to `canBeDeleted` an update may make.
- For references, a type whose owner may delete its documents is deletable: a `permanentDocument` reference, `inList` included, may not point at it (`ReferencedDocumentTypeDeletableError`, 40122), and a `deletableDocument` reference may. See [References](refers-to.md).

## `moderatorAbilities.delete`

Lets the contract's moderators delete documents of the type, whoever owns them. It is how an application takes down content that breaks its rules, where a ban only stops an identity from writing more. It is one key of the [`moderatorAbilities`](moderator-abilities.md) object, which also names the fields only moderators write.

| | |
|---|---|
| **Where** | `moderatorAbilities` of a document type, in a contract whose config declares `moderation` |
| **Value** | boolean |
| **Default** | `false` |
| **Since** | protocol version 14 |
| **On update** | Fixed (`DocumentTypeUpdateError`, 40212) |
| **Errors** | `DocumentTypeNotDeletableByModeratorsError` (41115), `IdentityNotContractModeratorError` (41101), `ContractModerationTargetNotAllowedError` (41102), `DocumentNotFoundError` (40101), and `DocumentModerationWindowElapsedError` (41116) with a window |

### Example

```json
"post": {
  "type": "object",
  "documentsMutable": true,
  "canBeDeleted": false,
  "moderatorAbilities": { "delete": true, "deleteWithin": 604800 },
  "properties": {
    "text": { "type": "string", "maxLength": 280, "position": 0 }
  },
  "required": ["$createdAt", "$updatedAt", "text"],
  "additionalProperties": false
}
```

Authors cannot retract a post, but the contract's moderators can remove one for a week (604,800 seconds) after it was written or last edited. The contract around it must declare `moderation` in its config.

### How it works

- A moderator deletes a document with the contract user moderation transition, naming the document type, the document id and a reason. The moderators are the ones the contract's `moderation` config declares; see [Contract Moderation](../data-model/contract-moderation.md#the-model).
- The transition is checked in this order, each refusal paid: the document type exists (`InvalidDocumentTypeError`, 10406); it sets `delete` (41115); the signer is the contract owner or a moderator (41101); the document exists (40101); its owner is neither the contract owner nor a moderator (41102); and, when the type sets `deleteWithin`, the window has not passed (41116).
- The document and all its index entries are deleted as an owner's delete would delete them, without the `canBeDeleted` check. A removal record is written under the contract: whose document it was, which moderator removed it, the reason, the block time and a hash of the document, and the values of any fields the type keeps public (see [`deleteKeepsFields`](#moderatorabilitiesdeletekeepsfields)). The record is never deleted. A type may leave no record: see [`deleteKeepsRecord`](#moderatorabilitiesdeletekeepsrecord).
- The document's owner gets no storage refund unless the type says otherwise (see [`deleteRefundsOwner`](#moderatorabilitiesdeleterefundsowner)), and the moderator pays neither the type's delete token cost nor its delete action fee.
- For a week after the deletion a moderator may restore the document exactly as it was. See [Restoring Documents](../data-model/contract-moderation.md#restoring-documents).

### Rules at registration

All refusals below are `InvalidContractStructure` (10231).

- The contract's config must declare `moderation`. Moderation cannot be added by a later update, so without it nobody could ever delete anything. In return the `moderation` block may keep no banlist, suspension list or warning list at all when a document type gives its moderators an ability.
- Refused on a type that keeps history (Drive never deletes those documents), on an `indexOnly` type (there is no stored row to name), on a type with `creationRestrictionMode` 1 or 2 (its documents are the contract owner's or the platform's), and on a type with a contested index (a restore could not go through the vote the index requires).
- For references, the type is no longer permanent even with `canBeDeleted: false`: a `permanentDocument` reference, `inList` included, may not point at it (40122). With `canBeDeleted: false`, no `ttl` and removal records kept (the default), its documents leave state only on a moderator's record, and a [`moderatedDocument`](refers-to.md#moderateddocument) reference is the one that points at it, resolving to the document or to its removal record; a `deletableDocument` reference is refused (40144). Otherwise a `deletableDocument` reference points at it.

## `moderatorAbilities.deleteWithin`

Limits the moderators' deletion to a window after a document's last change. Once the window has passed the document is settled: moderation acts on what was just written and does not reach back into what has stood unchallenged.

| | |
|---|---|
| **Where** | `moderatorAbilities` of a document type, with `delete: true` |
| **Value** | integer, seconds, 1 to 4294967295 |
| **Default** | absent: no limit |
| **Since** | protocol version 14 |
| **On update** | Fixed (`DocumentTypeUpdateError`, 40212), in both directions: a longer window would reopen documents that had settled |
| **Errors** | `DocumentModerationWindowElapsedError` (41116) |

### How it works

- The window is measured from the document's `$updatedAt`, or from its `$createdAt` on a type that does not record `$updatedAt`. A deletion at exactly that time plus the window still passes; after it, every moderator is refused, the contract owner included.
- A replace or a price update moves `$updatedAt`, so new content opens the window again. A transfer, a purchase or a moderator's [field change](moderator-abilities.md#changefields) does not move it.
- A restored document comes back with its old `$updatedAt`, so it may already be settled.
- The window says nothing about the document's own owner, whose deletion `canBeDeleted` rules at any age.

### Rules at registration

- Needs `delete: true` (`InvalidContractStructure`, 10231).
- A type whose documents can be replaced must list `$updatedAt` in `required`: measured from creation alone, an author could wait the window out and then rewrite a post into something no moderator can remove. A type with `documentsMutable: false` must list `$updatedAt` or `$createdAt`. Both refusals are 10231.
- A window of 0 is refused by the meta-schema (`JsonSchemaError`, 10101). A type that moderators may never delete from simply leaves `delete` out.

## `moderatorAbilities.deleteKeepsRecord`

Whether a moderator's deletion leaves a removal record under the contract. The record is what explains a missing document (who removed it, whose it was, why, when) and what a restore brings it back from. A contract that wants its moderators' deletions final and unrecorded, or does not want to pay for the records, turns it off.

| | |
|---|---|
| **Where** | `moderatorAbilities` of a document type, with `delete: true` |
| **Value** | boolean |
| **Default** | `true` |
| **Since** | protocol version 14 |
| **On update** | Fixed (`DocumentTypeUpdateError`, 40212) |
| **Errors** | `ContractDocumentRemovalNotFoundError` (41119) for a restore when `false` |

### How it works

- With `false`, the deletion writes no record, and the type gets no removal records tree: `getContractDocumentRemovals` refuses it as a type that keeps none. The document is gone for good: a restore is refused (41119), and a document id is never produced twice, so it cannot come back another way.
- The proof of such a deletion is the document's absence, which the SDKs report as no record (`delete_contract_document` resolves with `None`, `contractDeleteDocument` with `undefined`). The verifier reads the type's setting from the contract, so for such a type it needs the contract, as a restore's does; a deletion that leaves a record is proved by the record alone.
- The moderator pays less: no record is written and no hash computed.

### Rules at registration

- Needs `delete: true` (`InvalidContractStructure`, 10231).

## `moderatorAbilities.deleteKeepsFields`

Which fields of a deleted document stay public in its removal record. The document is gone, but some of what it said may still matter to everyone else: the hashtag of a removed post keeps the hashtag's timeline honest ("a post here was removed"), the thread a removed reply belonged to, the time it was written. The record keeps a copy of those values; everything else leaves with the document.

| | |
|---|---|
| **Where** | `moderatorAbilities` of a document type, with `delete: true` and a record (`deleteKeepsRecord` not `false`) |
| **Value** | array of property paths, at least one, none twice |
| **Default** | absent: the record keeps no field of the document |
| **Since** | protocol version 14 |
| **On update** | Fixed (`DocumentTypeUpdateError`, 40212), in both directions: which fields stay public is what an author was told when writing |

### Example

```json
{
  "type": "object",
  "properties": {
    "text": { "type": "string", "maxLength": 500, "position": 0 },
    "hashtag": { "type": "string", "maxLength": 61, "position": 1 },
    "meta": {
      "type": "object",
      "position": 2,
      "properties": {
        "tags": { "type": "array", "items": { "type": "string", "maxLength": 20 }, "maxItems": 5, "position": 0 },
        "note": { "type": "string", "maxLength": 100, "position": 1 }
      },
      "additionalProperties": false
    }
  },
  "required": ["$createdAt", "text"],
  "canBeDeleted": false,
  "moderatorAbilities": {
    "delete": true,
    "deleteKeepsFields": ["hashtag", "meta.tags", "$createdAt"]
  },
  "additionalProperties": false
}
```

A moderator deleting a post of this type leaves a record that still says which hashtag and tags it carried and when it was written, while its text and its note are gone:

```json
{
  "documentId": "…",
  "documentOwnerId": "…",
  "moderatorId": "…",
  "reason": { "text": "spam" },
  "removedAt": 1759200000000,
  "documentHash": "…",
  "keptFields": {
    "$createdAt": 1759100000000,
    "hashtag": "dash",
    "meta.tags": ["privacy", "payments"]
  }
}
```

### How it works

- The values are copied from the document as stored at the deletion, each under the path the type lists: a top-level property, a property inside an object (`meta.tags`), or a whole object (`meta`). A path the document holds no value at is left out of the record.
- The record stores them as the document stores its properties, so they are read, as the document is, under its document type, and come back typed exactly as the document's values: the SDKs do this for you (`keptFields`). An object kept whole shows members an update added after the removal as absent, as an older document does.
- They are read wherever the record is: `getContractDocumentRemovals`, by document id or by page, the proof of the deletion, and a join through a [`moderatedDocument`](refers-to.md#moderateddocument) reference. They are not indexed: no query finds records by a kept value.
- A `moderatedDocument` reference to a removed document checks a `where` pair on a kept property against the kept value, as it would against the document.
- A restore brings the document back and leaves the record, marked restored, with what it kept. A later deletion of the restored document writes a fresh record, with the values the document then held.
- The moderator pays for the record, kept values included.

### Rules at registration

All refusals below are `InvalidContractStructure` (10231).

- Needs `delete: true`, and a record: refused beside `deleteKeepsRecord: false`.
- Each entry is a declared property at any depth, stepping through objects by `.`, or one of the timestamps and block heights (`$createdAt`, `$updatedAt`, `$transferredAt`, and their `BlockHeight` and `CoreBlockHeight` forms) listed in `required`, without which no document carries it.
- Refused: a transient property (no stored document holds it), `$id` and `$ownerId` (every record holds them already), any other system property, and a path inside another listed path (the object around it is kept whole already).

## `moderatorAbilities.deleteRefundsOwner`

Whether the owner of a document a moderator deletes is refunded its storage. By default the owner forfeits it: removed content costs its author what they paid to store it. A contract whose moderation is housekeeping rather than sanction (clearing handled reports, expired listings) gives it back.

| | |
|---|---|
| **Where** | `moderatorAbilities` of a document type, with `delete: true` |
| **Value** | boolean |
| **Default** | `false` |
| **Since** | protocol version 14 |
| **On update** | Fixed (`DocumentTypeUpdateError`, 40212) |

### How it works

- With `true`, the owner is refunded as for their own deletion: the part of the storage fee not yet paid out to past epochs. The refund goes to the owner, not to the moderator, who still pays for the transition and the record. A document of a type with a `ttl` refunds nothing either way.
- With `false`, the credits stay in the storage pools they were paid into.

### Rules at registration

- Needs `delete: true` (`InvalidContractStructure`, 10231).

## How they combine

| Who deletes | Allowed by | Refund to the owner |
|---|---|---|
| The document's owner | `canBeDeleted: true`, on a type that does not keep history | Yes, except on a type with a `ttl` |
| The contract's moderators | `moderatorAbilities.delete: true`, within `moderatorAbilities.deleteWithin` when set | Only with `moderatorAbilities.deleteRefundsOwner: true`, except on a type with a `ttl` |
| The platform | `ttl`, once it has passed | No |

Which reference may point at a type follows from which of the three it allows. A type that allows none of them is the target of a `permanentDocument` reference, with `inList` or without. A type that allows only the moderators' deletion, with removal records kept, is the target of a `moderatedDocument` reference: its documents never leave state without a record. Any other type is the target of a `deletableDocument` reference.

## See also

- [Deleting Documents](../data-model/contract-moderation.md#deleting-documents) and [Restoring Documents](../data-model/contract-moderation.md#restoring-documents), for the moderation transition and the removal record
- [Moderator Abilities](moderator-abilities.md), for the `moderatorAbilities` object and the fields only moderators write
- [Time To Live](ttl.md), the third way a document leaves the state
- [History](history.md), for why a type that keeps history can never delete
- [Mutability](mutability.md) and [Creation, Transfers and Trading](ownership-and-trading.md)
- [References](refers-to.md), for `permanentDocument`, `moderatedDocument` and `deletableDocument`
- [Contract-Level Keys and config](contract-config.md), for `documentsCanBeDeletedContractDefault` and `moderation`
