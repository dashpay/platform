# System Properties

Every document carries a few values the platform manages rather than the writer: its id, its owner, and, on some types, its revision, its creator and the times it was created, updated and transferred. Their names start with `$`. A document type does not declare them in `properties`. It names them where it wants to use them: in `required`, to have a timestamp recorded, in `indices`, to query by them, and in the keywords that accept one, such as a reference's `propertyAgreement`.

| Property | Holds | Recorded |
|---|---|---|
| [`$id`](#id) | the document's id | always |
| [`$ownerId`](#ownerid) | the identity that owns the document now | always |
| [`$revision`](#revision) | how many times the document has changed, plus one | on types whose documents can be replaced, transferred or sold |
| [`$createdAt`, `$updatedAt`, `$transferredAt`](#timestamps) | block times of the creation, last update and last transfer | when listed in `required` |
| [`$createdAtBlockHeight` and the other heights](#block-heights) | Platform and Core block heights of the same events | when listed in `required` |
| [`$creatorId`](#creatorid) | the identity that created the document | on types whose documents can be transferred or sold |

## Example

```json
"listing": {
  "type": "object",
  "documentsMutable": true,
  "transferable": 1,
  "tradeMode": 1,
  "indices": [
    { "name": "byCreator", "properties": [{ "$creatorId": "asc" }, { "$createdAt": "asc" }] },
    { "name": "byOwner", "properties": [{ "$ownerId": "asc" }, { "$updatedAt": "asc" }] }
  ],
  "properties": {
    "title": { "type": "string", "minLength": 1, "maxLength": 100, "position": 0 }
  },
  "required": ["$createdAt", "$updatedAt", "$transferredAt", "title"],
  "additionalProperties": false
}
```

Every listing records when it was created, last updated and last transferred, because `required` lists the three times. Listings can be replaced, transferred and sold, so each one also carries a revision and the id of its creator, and the two indexes find them by who made them and by who holds them now.

## `$id`

| | |
|---|---|
| **Where** | Every document |
| **Value** | An identifier: 32 bytes |
| **Recorded** | Always |
| **Since** | protocol version 1 |
| **Errors** | `InvalidDocumentTransitionIdError` (10405): a create whose id is not the one derived for it. `SystemPropertyIndexAlreadyPresentError` (10208): an index that names `$id`. |

A document's id is derived from the contract id, the owner's id, the document type's name, entropy chosen by the client and, from protocol version 14, the identity contract nonce of the create transition. Consensus derives it again for every create and refuses a transition that carries another. The id never changes. See [Document ID Generation](../data-model/documents.md#document-id-generation).

Documents are already stored by id, so an index may not name `$id`. A reference's `propertyAgreement` may name it on the referenced side (see [References](refers-to.md)).

## `$ownerId`

| | |
|---|---|
| **Where** | Every document |
| **Value** | An identifier: the id of an identity |
| **Recorded** | Always |
| **Since** | protocol version 1 |
| **Errors** | `DocumentOwnerIdMismatchError` (40102): a replace, transfer, price update or delete signed by an identity that does not own the document |

The owner is the identity that created the document, until a transfer or a purchase hands it to another. Only the owner may replace, transfer, reprice or delete it with a document transition. A document can also leave by other paths, a moderators' deletion or an expired [`ttl`](ttl.md); see [Deletion](deletion.md).

`$ownerId` may be indexed, and several keywords read it, where it usually stands for the writer of the document:

- [`distinctFrom`](distinct-from.md): `"$ownerId"` makes a property differ from the owner.
- [`propertyConstraints`](property-constraints.md): a rule may compare an identifier property with `$ownerId`.
- [References](refers-to.md): a `propertyAgreement` pair, a lookup key and `identityProperty` may name it, and [`ownerRefersTo`](owner-refers-to.md) checks the owner itself.
- [`encryptedFor`](encrypted-for.md): `"recipient": "$ownerId"` marks a message the writer encrypts to themself.

## `$revision`

| | |
|---|---|
| **Where** | Documents of a type whose documents can be replaced (`documentsMutable`, true by default), transferred (`transferable: 1`) or sold (`tradeMode: 1`) |
| **Value** | An integer, from 1 |
| **Recorded** | On those types; absent on every other |
| **Since** | protocol version 1 |
| **Errors** | `InvalidDocumentRevisionError` (40106): a transition whose revision is not the stored one plus one |

A new document has revision 1. Every replace, transfer, price update and purchase raises it by one, and the transition must state the new revision: the stored revision plus one. A transition built against an older copy of the document is refused, rather than silently overwriting a newer one. A type whose documents can never change after creation carries no revision. `$revision` is not one of the system properties an index may name.

## Timestamps

| | |
|---|---|
| **Properties** | `$createdAt`, `$updatedAt`, `$transferredAt` |
| **Value** | A block time, in milliseconds since the Unix epoch |
| **Recorded** | Only when listed in the document type's `required` |
| **Since** | protocol version 1 |
| **On update** | The set is fixed: a contract update may not add one to `required` or remove one (`DataContractInvalidRequiredFieldsUpdateError`, 10276) |

The platform sets these from the block that processes the transition; the writer never supplies them. A timestamp that is not in `required` is never recorded, and documents of the type do not have it.

| Event | Sets |
|---|---|
| Create | every listed timestamp, so `$updatedAt` and `$transferredAt` start at the creation time |
| Replace | `$updatedAt` |
| Price update | `$updatedAt` |
| Transfer | `$transferredAt` |
| Purchase | `$transferredAt` |

Timestamps may be indexed. Some keywords need one in `required`, since they read it:

- [`ttl`](ttl.md) counts from `$createdAt`.
- `moderatorAbilities.deleteWithin` counts from `$updatedAt`, or from `$createdAt` on a type whose documents cannot be replaced (see [Deletion](deletion.md)).
- A [time-range index](time-range.md) needs the timestamp it buckets.

## Block heights

| | |
|---|---|
| **Properties** | `$createdAtBlockHeight`, `$updatedAtBlockHeight`, `$transferredAtBlockHeight`, `$createdAtCoreBlockHeight`, `$updatedAtCoreBlockHeight`, `$transferredAtCoreBlockHeight` |
| **Value** | The `BlockHeight` forms: the Platform block height. The `CoreBlockHeight` forms: the Core chain height recorded with that block. |
| **Recorded** | Only when listed in the document type's `required` |
| **Since** | protocol version 1 |
| **On update** | The set is fixed, as for the timestamps (10276) |

The same three events as the timestamps, measured in blocks instead of time. Each is set on the same events as the timestamp of its name, and may be indexed.

## `$creatorId`

| | |
|---|---|
| **Where** | Documents of a type that sets `transferable: 1` or `tradeMode: 1`, in a contract of format 1 whose config is version 1 or later |
| **Value** | An identifier: the id of the identity that created the document |
| **Recorded** | On those types, from protocol version 10 |
| **Since** | protocol version 10 |
| **Errors** | `UndefinedIndexPropertyError` (10209): an index naming `$creatorId` on a type that does not record it |

On a type whose documents can change hands, `$ownerId` follows the document while `$creatorId` stays with the identity that created it: a transfer or a purchase never changes it. On a type whose documents never change hands the creator is always the owner, and no `$creatorId` is recorded. Neither is it on a contract of format 0 or with a config of version 0, whatever its types allow.

`$creatorId` is not written in `required` or `properties`. It may be named:

- in an index;
- on the referenced side of a reference's `propertyAgreement`, and as a key reference's `identityProperty`;
- by [`creatorRefersTo`](owner-refers-to.md), which checks the creator. A type that does not record creators may not declare it (`InvalidContractStructure`, 10231).

## See also

- [What Lives Inside a Document](../data-model/documents.md#what-lives-inside-a-document), for the fields of a document
- [Document Shape](document-shape.md#required), for the `required` list that records timestamps
- [Document Serialization](../serialization/document-serialization.md#high-level-structure), for where each system property sits in the stored bytes
- [Creation, Transfers and Trading](ownership-and-trading.md), for the flags that decide `$revision` and `$creatorId`
