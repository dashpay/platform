# Writer and Creator References

A property's [`refersTo`](refers-to.md) judges a value the writer chose. `ownerRefersTo` and `creatorRefersTo` judge an identity of the document instead: its writer (`$ownerId`) or its creator (`$creatorId`). Each takes the same declaration a property's `refersTo` takes, limited to the targets an identity's id can be. Reach for them to say who may write a document of a type: "only a member of this team", "only someone with a profile". A type declares at most one of the two: `ownerRefersTo` when its documents stay with their owner, `creatorRefersTo` when they can be transferred or traded.

Both are checked the way a property's reference is: the same targets, keys, errors and replace rules, with the identity as the value. They count as one reference each (times the leaves of an expression) against the [reference budget](refers-to.md#the-reference-budget), and the check runs before the properties' references.

## `ownerRefersTo`

| | |
|---|---|
| **Where** | The document type, at the top level of its schema. Only on a type whose documents can be neither transferred nor traded. |
| **Value** | A `refersTo` declaration whose value is the writer: `identity`, a `permanentDocument` or `deletableDocument` found by [`findBy`](refers-to-lookup.md), a `permanentDocument` with [`inList`](refers-to-list-element.md), or an [`anyOf` or `allOf`](refers-to-expressions.md) whose leaves are all of these. |
| **Since** | protocol version 14 |
| **On update** | Fixed: adding, removing or changing it is refused (`IncompatibleDocumentTypeSchemaError`, 10246). |
| **Errors** | The error the target reports for a property, at the path `$ownerId`: `ReferencedEntityNotFoundError` (40120) when `findBy` finds nothing or the writer is not in the list, `ReferencedDocumentPropertyMismatchError` (40127) for a `where` entry. |

### Example

```json
"post": {
  "type": "object",
  "ownerRefersTo": {
    "type": "deletableDocument",
    "contractId": "Bwr4WHCPz5rFVAD87RqTs3izo4zpzwsEdKPWUT1NS1C7",
    "documentType": "profile",
    "findBy": { "$ownerId": "." }
  },
  "properties": {
    "text": { "type": "string", "minLength": 1, "maxLength": 280, "position": 0 }
  },
  "required": ["text"],
  "additionalProperties": false
}
```

Only an identity with a DashPay profile may create a post. In `findBy`, `"."` is the writer: DashPay's unique index on `$ownerId` must find a `profile` owned by them. Profiles can be deleted, so the target is a `deletableDocument` one and every replace asks again: a writer whose profile is gone can no longer edit their posts, though they can still delete them.

The moderation charters contract's `resignationRequest` combines two targets: its writer must be one of the elected `members` of the charter the request names, or have an `addedModerator` document for it that exists now. The declaration is shown under [`anyOf`](refers-to-expressions.md#anyof).

### How it works

- **On create**, the writer is checked against the target exactly as a property's value would be. An `identity` target always holds and reads nothing: the transition has already proved that the writer exists.
- **In `findBy`**, `"."` is the writer, and so is a `"$ownerId"` source. In a `where`, an entry valued `"$ownerId"` names the same writer.
- **On replace**, the declaration is checked again when a property its `findBy` or `where` reads changed, and on every replace when it has a `where` entry valued `"$ownerId"` or a `deletableDocument` found by `findBy` (alone or as a leaf), unless a `findBy` function computes the key, which is judged on the create alone. Otherwise nothing is read: the writer is always the owner, a permanent target is never deleted and its key never moves.
- **Transfers and purchases** cannot happen on such a type, so the owner of every document is a writer that was checked.
- A refusal names `$ownerId` as its path. Registration errors name the declaration `<documentType>.$ownerId`.

### Rules at registration

- The document type's documents can be neither transferred nor traded (`transferable` and `tradeMode` absent or `0`). Otherwise a transfer or a purchase, which is not a write, would hand a document to an owner the declaration never checked; declare `creatorRefersTo` instead.
- The target is one the writer's id can be. `contract`, `token` and a document by id are refused, since an identity's id is never one of those ids, and so is `identityPublicKey`, which pairs the value with a key id the writer does not carry. The same holds for every leaf of an expression.
- Every other rule is a property reference's: those of [`findBy`](refers-to-lookup.md#rules-at-registration), the [list](refers-to-list-element.md#rules-at-registration) and each [`where`](refers-to.md#where), checked against another contract's stored type where the declaration names one.

A malformed declaration is refused by the meta-schema (`JsonSchemaError`, 10101) or the parser (`InvalidContractStructure`, 10231); one that cannot hold, with the reference errors of its target (see [Errors](refers-to.md#errors)).

## `creatorRefersTo`

| | |
|---|---|
| **Where** | The document type, at the top level of its schema. Only on a type that records creator ids: a transferable or tradeable type of a format-1 contract (see [System Properties](system-properties.md)). |
| **Value** | A `refersTo` declaration whose value is the creator: `identity`, a `permanentDocument` found by [`findBy`](refers-to-lookup.md) or with [`inList`](refers-to-list-element.md), a `deletableDocument` whose `findBy` holds a [function](refers-to-lookup.md#commit-and-reveal), or an [`anyOf` or `allOf`](refers-to-expressions.md) whose leaves are all of these. |
| **Since** | protocol version 14 |
| **On update** | Fixed: adding, removing or changing it is refused (`IncompatibleDocumentTypeSchemaError`, 10246). |
| **Errors** | The error the target reports for a property, at the path `$creatorId`: `ReferencedEntityNotFoundError` (40120), `ReferencedDocumentPropertyMismatchError` (40127). |

### Example

```json
"moderatorBadge": {
  "type": "object",
  "transferable": 1,
  "creatorRefersTo": {
    "type": "permanentDocument",
    "contractId": "EG7RGfV8fDTayC2FyVr8HwdpJh3fXDbVztcfE94UmN88",
    "documentType": "electedCharter",
    "findBy": { "$id": "electedCharterId" },
    "inList": "members"
  },
  "properties": {
    "electedCharterId": {
      "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
      "contentMediaType": "application/x.dash.dpp.identifier",
      "position": 0
    }
  },
  "required": ["electedCharterId"],
  "additionalProperties": false
}
```

Only an elected member of the charter `electedCharterId` names, in the moderation charters contract, may mint a badge. Once minted, the badge can be transferred to anyone (`transferable: 1`), and it keeps its creator.

### How it works

- **On create**, the creator is the writer, and is checked as `ownerRefersTo` checks the writer. An `identity` target reads nothing.
- **On replace**, the value is the stored creator, whoever writes. The declaration is checked again when a property its `findBy` or `where` reads changed, and on every replace when it has a `where` entry valued `"$ownerId"`. Such an entry still names the writer, not the creator.
- **Transfers and purchases** need no check: they do not change the creator.
- **In `findBy`**, `"."` is the creator.
- A refusal names `$creatorId` as its path. Registration errors name the declaration `<documentType>.$creatorId`.

### Rules at registration

- The document type records creator ids. Such a type's documents can be transferred or traded, so `ownerRefersTo` is refused on it, and a type declares at most one of the two.
- The targets are those of `ownerRefersTo` except `deletableDocument`: the creator never changes, and a document a transfer handed on could not be replaced by its new owner once the document `findBy` found was deleted. A `deletableDocument` whose key a `findBy` function computes is the exception, since it is judged on the create alone.
- `findBy` may not read `"$ownerId"`: on a type whose documents can change owner, a transfer or a purchase would move that key part without a write.
- It can only be declared on a type when the type is created, since an update may not add it. So every document of the type records its creator.
- Every other rule is as for `ownerRefersTo`.

## Choosing between them

| The type's documents | Declare | Judges |
|---|---|---|
| stay with their owner (`transferable` and `tradeMode` absent or `0`) | `ownerRefersTo` | whoever writes the document, who is always its owner |
| can be transferred or traded | `creatorRefersTo` | the identity that created the document, whoever writes it later |

For a rule about the writer and a document one of its own properties already names, a [`where`](refers-to.md#where) entry valued `"$ownerId"` on that property's reference is enough: `{ "$ownerId": "$ownerId" }` requires the writer to own the referenced document. The type-level keywords are for rules no property carries, such as "the writer has a profile" or "the writer is on this list".

## See also

- [References (refersTo)](refers-to.md) for the targets, keys and errors.
- [findBy](refers-to-lookup.md), [List Elements](refers-to-list-element.md) and [Expressions](refers-to-expressions.md), the targets a writer or creator reference usually takes.
- [On the writer or the creator](../data-model/documents.md#on-the-writer-or-the-creator-ownerrefersto-creatorrefersto) in the Documents chapter, with the internals.
- [System Properties](system-properties.md) for `$ownerId` and `$creatorId`, and [Creation, Transfers and Trading](ownership-and-trading.md) for `transferable` and `tradeMode`.
