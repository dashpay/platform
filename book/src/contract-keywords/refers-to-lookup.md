# Lookups

A `lookup` lets a document reference find its target through a unique index of the referenced document type instead of by id. The property's value is then one part of the index key, the rest comes from the referring document or its writer, and the reference holds if the index finds a document. Reach for it when the natural value is an identity (a member, an author, a recipient) and the rule is "this identity has a document of that kind": with an id reference the writer would have to find the document's id first, and the id would say nothing about who it belongs to.

| | |
|---|---|
| **Where** | Inside a `permanentDocument` or `deletableDocument` [`refersTo`](refers-to.md) declaration: on an identifier property, on the `items` of a typed array of identifiers, as an operand of an [expression](refers-to-expressions.md), or in [`ownerRefersTo` and `creatorRefersTo`](owner-refers-to.md). With a [`propertyAgreement` function](#commit-and-reveal), also on a string or byte array property. |
| **Value** | `{ "index": ..., "keys": { ... } }`. `index` is the name of a unique index of `documentType`, 1 to 32 characters. `keys` maps every property of that index (1 to 10) to where its value comes from: `"."` (the reference's own value), `"$ownerId"` (the writer) or a property path of the referring document type, except the one a [`propertyAgreement` function](#commit-and-reveal) fills. With a function the lookup may also declare `minimumAgeBlocks` and `consume`. |
| **Since** | protocol version 14 |
| **On update** | Fixed, like the rest of `refersTo`: adding, removing or changing a lookup is refused (`IncompatibleDocumentTypeSchemaError`, 10246). |
| **Errors** | `ReferencedEntityNotFoundError` (40120) when the index finds no document; at registration `ReferencedDocumentLookupInvalidError` (40137) for an index of another contract, `InvalidContractStructure` (10231) for one of the same contract. A function adds `DocumentReferencePreimageInvalidError` (10423) and `ReferencedDocumentRequirementNotMetError` (40142). |

## Example

The moderation charters contract's `joinRequest` type, which is immutable and can never be deleted, has this unique index: one join request per proposal and owner.

```json
"indices": [
  {
    "name": "bySubmittedCharter",
    "properties": [{ "submittedCharterId": "asc" }, { "$ownerId": "asc" }],
    "unique": true
  }
]
```

The same contract's `electedCharter` lists its team in `members`, each of whom must have asked to join:

```json
"members": {
  "type": "array", "minItems": 0, "maxItems": 15, "uniqueItems": true,
  "items": {
    "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
    "contentMediaType": "application/x.dash.dpp.identifier",
    "distinctFrom": "$ownerId",
    "refersTo": {
      "type": "permanentDocument",
      "documentType": "joinRequest",
      "lookup": {
        "index": "bySubmittedCharter",
        "keys": { "submittedCharterId": "submittedCharterId", "$ownerId": "." }
      }
    }
  },
  "position": 2
}
```

Every member must be the owner of a `joinRequest` whose `submittedCharterId` equals this elected charter's `submittedCharterId`. For each element the key is `submittedCharterId` read from the elected charter and `$ownerId` filled with the element itself (`"."`). The same form works on a single identifier property, where `"."` is the property's value.

## How it works

### Assembling the key

When the referring document is created or replaced, a key is assembled for each value (each element of a typed array), one part per entry of `keys`:

- `"."`: the value being checked, the property's value or the element.
- `"$ownerId"`: the referring document's owner, the writer.
- a property path (`"submittedCharterId"`, `"meta.charterId"`): the referring document's value at that path.

The index is queried for at most one document, billed as a document fetch. If it finds one, the reference holds, and any [`propertyAgreement`](refers-to.md#propertyagreement) pairs beside the `lookup` are checked against that document (`ReferencedDocumentPropertyMismatchError`, 40127). If it finds none, the write is refused with `ReferencedEntityNotFoundError` (40120) naming the property, or the element by its list path (`members[1]`); the error's target reads "found through unique index" and the index name.

### Permanent and deletable lookups

A `permanentDocument` lookup never dangles. Its referenced documents are never deleted, and registration makes sure the key of every one of them is fixed once written (see [below](#rules-at-registration)), so the document a key found stays there. A replace checks it again only:

- when the property itself changed (for a typed array, only the elements the stored list did not hold);
- when a property a key reads changed, and then every value, every element included;
- when an agreement's referring property changed, or on every replace for a pair keyed by `$ownerId`.

A key part read from `"$ownerId"` never triggers a check: only a type whose documents keep their writer may read it.

A `deletableDocument` lookup promises less. Once the document a key found is deleted, a new document filed later under the same key makes the reference hold again, with different content, where an id reference to a deleted document stays dead. So a deletable lookup means "a document with this key exists now", which is what a membership gate needs. Every replace checks it again, whether or not the replace touched it, and no `immutable` property may hold one. The moderation charters contract's `resignationRequest` uses one to accept a writer who has an `addedModerator` document for the charter at the time of writing, as the alternative to being an elected member (see [Writer and Creator References](owner-refers-to.md)).

A lookup can also reach into another contract. This `recipientId` must be an identity that has a DashPay profile when the document is written:

```json
"recipientId": {
  "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
  "contentMediaType": "application/x.dash.dpp.identifier",
  "refersTo": {
    "type": "deletableDocument",
    "contractId": "Bwr4WHCPz5rFVAD87RqTs3izo4zpzwsEdKPWUT1NS1C7",
    "documentType": "profile",
    "lookup": { "index": "ownerId", "keys": { "$ownerId": "." } }
  },
  "position": 0
}
```

DashPay's `profile` type has a unique index `ownerId` on `$ownerId`, and profiles can be deleted, so the reference is a `deletableDocument` one.

### What a lookup cannot do

The value of a lookup reference is a key part, not a document id. So a lookup reference cannot be the join property of a chained query or of a composite join by id, and a `preallocated` index is never bound through one (see [Index-Only Types](index-only.md)). A lookup counts as one reference per value against the [reference budget](refers-to.md#the-reference-budget), like any other.

## Commit and reveal

A `propertyAgreement` pair may hold a function: `"<referenced property>": { "function": "sys.hash.sha256d", "params": [...] }` says the referenced document's property holds the SHA-256 of the SHA-256 of the params' bytes, joined in order. It is keyed by the referenced property, since a function cannot be a key. That property must be in the lookup's index, and the platform fills it with the hash to find the document, so the lookup may leave it out of `keys`, and leave `keys` out entirely when it is the whole index. The document it finds is a commitment made earlier, one that stored that hash, so the document being created may exist only while a commitment to values it carries exists. This is how a name registration that preorders a salted hash works, written as a declaration on the salt:

```json
"preorderSalt": {
  "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32, "position": 4,
  "refersTo": {
    "type": "deletableDocument",
    "documentType": "preorder",
    "lookup": { "index": "saltedHash", "minimumAgeBlocks": 1, "consume": true },
    "propertyAgreement": {
      "$ownerId": "$ownerId",
      "saltedDomainHash": {
        "function": "sys.hash.sha256d",
        "params": ["preorderSalt", "normalizedLabel", { "const": "." }, "parentDomainName"]
      }
    }
  }
}
```

The salt must reveal a `preorder` whose `saltedDomainHash` is the sha256d of the salt, the normalized label, a dot and the parent. The `$ownerId` pair makes it the writer's own preorder, `minimumAgeBlocks: 1` one created in an earlier block, and `consume: true` deletes it with the create.

- **Params.** A property path reads the document being created, the property carrying the reference included; `{ "const": text }` is fixed text of 1 to 64 bytes; `"."` is the value carrying the reference where it has no path (each element of a typed array, the writer, the creator); 1 to 16 params. Strings count as their UTF-8, byte arrays as their bytes, identifiers as their 32 bytes. Two values of no fixed length must be separated by a one-byte `const`, and a value holding that byte is refused, so the joined bytes split back one way only.
- **Carrier.** A string or byte array property may carry a `refersTo` only this way, and keeps its type. The value carrying the reference fills the key exactly once, as a `"."` key or a param; a param names a property by its path, never as `"."`. In `ownerRefersTo` and `creatorRefersTo` the value may be left out beside a function.
- **Judged once.** A function's key is checked when the document is created, never on a replace, so every stored value it reads must be fixed once written, the carrier included (listed under `immutable` on a mutable type). Its plain `propertyAgreement` pairs are judged with it, so each property they name must be fixed once written too, or transient. On a type whose documents can be replaced it may not be an operand of an `anyOf`, which would then hold on every replace. Params may be transient: they are read from the create. The hash is billed as the double SHA-256 it is, by the blocks it hashes, beside the document fetch.
- **`minimumAgeBlocks`**: the commitment's `$createdAtBlockHeight` is at least that many blocks below the create's height. The commitment type must list `$createdAtBlockHeight` in `required`.
- **`consume: true`**: the create deletes the commitment, its storage refunded to its owner. Only with the `"$ownerId": "$ownerId"` pair, into a type of the same contract whose owner may delete its documents, that declares no delete token cost or delete action fee (no delete transition charges them), and that requires no stricter signature security level than the revealing type. A contract-bound key signing the create must be allowed to act on the consumed type too (`ContractBoundedKeyOutOfBoundsError`, 20014).

A create missing a param, or with a value holding its separator, is refused before any read (`DocumentReferencePreimageInvalidError`, 10423). No commitment is `ReferencedEntityNotFoundError` (40120), another identity's `ReferencedDocumentPropertyMismatchError` (40127), and one too young `ReferencedDocumentRequirementNotMetError` (40142). The [Documents chapter](../data-model/documents.md#commit-and-reveal-a-propertyagreement-function) has the full rules.

## Rules at registration

**On the referring side**, checked for every contract by the parser (`InvalidContractStructure`, 10231):

- `lookup` is only allowed on `permanentDocument` and `deletableDocument` references.
- The reference's own value is read exactly once, as a `"."` key part or as a param of a [`propertyAgreement` function](#commit-and-reveal). Without it every value would find the same document. Only the lookup of `ownerRefersTo` or `creatorRefersTo` may leave it out, beside a function.
- Every other source is `"$ownerId"` or a property path. No other `$` name is accepted, and a path may not name the reference property itself: write `"."` for that.
- A property a key reads must exist on the referring type, be required (and so must every object around it), not be `transient` or inside a transient object, and hold a single value, not an object or an array. A lookup never runs with a missing key part, and a reader can assemble the same key from the stored document. A function's params follow [their own rules](#commit-and-reveal): they may be transient or optional, since they are read from the create alone.
- A `"$ownerId"` key part needs a referring type whose documents can be neither transferred nor traded: a transfer or a purchase would move the writer part of the key without a write.
- A `deletableDocument` lookup may not sit under an `immutable` property, alone or as an operand of an expression: every replace checks it again, so once its document is deleted the property would have to change. A lookup whose key a [`propertyAgreement` function](#commit-and-reveal) computes is the exception: it is checked on the create alone, and its carrier must be fixed once written.

**On the referenced side**, refused with `InvalidContractStructure` (10231) for a document type of the declaring contract and with `ReferencedDocumentLookupInvalidError` (40137) for one of another contract:

- The index exists and is `unique`, so a key finds at most one document. It does not bucket its first property by a `timeRange` or an `integerRange`, the referenced type is not `indexOnly`, and no index property is `transient`.
- `keys` maps every property of the index exactly once, by its name on the referenced side (system properties such as `$ownerId` included), in any order, and nothing else.
- Each source holds the same type of value as the index property it fills. `"."` and `"$ownerId"` are identifiers.
- The key stays with the document it found. Every schema property of the index must be fixed once written: the referenced type is immutable (`documentsMutable: false`), or the property's top-level property is listed under `immutable`. `$ownerId` may be a key part only where the referenced documents can be neither transferred nor traded. `$updatedAt` and its block height forms may be one only where they can be neither replaced, transferred nor traded, and `$transferredAt` and its forms only where they can be neither transferred nor traded. `$id`, `$creatorId`, `$createdAt` and its forms never change.

**The referenced type** must exist (`ReferencedDocumentTypeNotFoundError`, 40121). A `permanentDocument` lookup into a type whose documents can disappear is refused with `ReferencedDocumentTypeDeletableError` (40122), and a `deletableDocument` lookup into one whose documents cannot with `ReferencedDocumentTypeNotDeletableError` (40131).

Index definitions and the flags these rules read cannot change on a contract update, so a lookup that registered keeps resolving.

## See also

- [References (refersTo)](refers-to.md) for the targets, `propertyAgreement` and the replace rules.
- [Expressions](refers-to-expressions.md), where a lookup may be an operand, and [Writer and Creator References](owner-refers-to.md), where `"."` is the writer or the creator.
- [Resolved through a unique index](../data-model/documents.md#resolved-through-a-unique-index-lookup) in the Documents chapter, with the internals.
- [Indexes (indices)](indexes.md), [Time-Range Indexes](time-range.md), [Mutability](mutability.md) and [transient](transient.md) for the keywords the rules read.
