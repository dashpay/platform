# Moderator Abilities

`moderatorAbilities` says what the contract's moderators may do to the documents of a type, whoever owns them: delete them, and write the fields that only they write. It is how an application gives its moderators a say over content without making them its authors: a report its moderators mark as handled, a post they flag, a ticket they assign.

| | |
|---|---|
| **Where** | document type, in a contract whose config declares `moderation` |
| **Value** | object with at least one of `delete` (boolean), `deleteWithin` (seconds) and `changeFields` (array of top-level property names) |
| **Default** | absent: the moderators can do nothing to documents of the type |
| **Since** | protocol version 14 |
| **On update** | Fixed (`DocumentTypeUpdateError`, 40212): a type can neither gain, lose nor change it. A type the update adds may declare it. |

The keys:

| Key | What it allows | Details |
|---|---|---|
| `delete` | The moderators delete documents of the type, leaving a removal record, and may restore them within a week. | [Deletion](deletion.md#moderatorabilitiesdelete) |
| `deleteWithin` | Limits `delete` to so many seconds after a document's last change. | [Deletion](deletion.md#moderatorabilitiesdeletewithin) |
| `changeFields` | The listed properties are written only by the moderators. | [below](#changefields) |

The moderators are the ones the contract's `moderation` config declares: the contract owner, the identities it appoints, or the members of the seated team of an elected contract. A seated team holds an ability on a type only when the declaration's `moderatedDocumentTypes` gives it: `deleteDocuments` for `delete`, `changeDocumentFields` for `changeFields`. See [Contract Moderation](../data-model/contract-moderation.md).

## `changeFields`

The top-level properties of the type that only the contract's moderators write. A moderator sets or removes them on any document of the type with the contract user moderation transition. A document's owner can neither set them when creating the document nor change or remove them when replacing it, unless the owner is a moderator of the contract.

| | |
|---|---|
| **Where** | `moderatorAbilities` of a document type |
| **Value** | array of top-level property names, at least one, none twice |
| **Default** | absent: nobody but a document's owner writes its properties |
| **Since** | protocol version 14 |
| **On update** | Fixed (`DocumentTypeUpdateError`, 40212) |
| **Errors** | `DocumentFieldNotChangeableByModeratorsError` (41123), `DocumentModeratorFieldNotWritableError` (41124), `InvalidContractModerationDocumentFieldsError` (10905), `IdentityNotContractModeratorError` (41101), `ContractModerationAbilityNotGrantedError` (41201) |

### Example

```json
"report": {
  "type": "object",
  "documentsMutable": false,
  "moderatorAbilities": { "delete": true, "changeFields": ["status", "resolution"] },
  "indices": [
    { "name": "byStatus", "properties": [{ "status": "asc" }, { "$createdAt": "asc" }] }
  ],
  "properties": {
    "postId": {
      "type": "array",
      "byteArray": true,
      "minItems": 32,
      "maxItems": 32,
      "contentMediaType": "application/x.dash.dpp.identifier",
      "position": 0,
      "refersTo": { "type": "deletableDocument", "documentType": "post" }
    },
    "reason": { "type": "integer", "minimum": 0, "maximum": 8, "position": 1 },
    "status": { "type": "integer", "minimum": 1, "maximum": 3, "position": 2 },
    "resolution": { "type": "string", "maxLength": 200, "position": 3 }
  },
  "required": ["$createdAt", "postId", "reason"],
  "additionalProperties": false
}
```

A user files a report and can never edit it. A report starts with no `status`: the reporter cannot file one already marked handled. A moderator sets `status` and `resolution` when the report is dealt with, and queries the open ones through `byStatus`. The report stays, so the reporter and everyone else can see what became of it.

### How it works

- A moderator changes the fields with the contract user moderation transition's `changeDocumentFields` action, naming the document type, the document id, the new value of each field (`null` removes one) and a reason.
- The transition is checked in this order, each refusal paid: the document type exists (`InvalidDocumentTypeError`, 10406); every field named is listed under `changeFields` (41123); the signer is a moderator of the contract (41101), and for a seated team the declaration gives it `changeDocumentFields` on the type (41201) and the reason names a reason document its proposal lists (41203); the document exists (40101) and has not expired (`DocumentExpiredError`, 40140); and the document as changed is still one of its type: its schema (`JsonSchemaError`, 10101), its `propertyConstraints` and its unique indexes (`DuplicateUniqueIndexError`, 40105). A change naming no field, or a system property, is refused before any state is read (10905).
- Whoever owns the document may have it changed, the contract owner and the moderators included: the fields are the moderators', not the owner's, so the protection that keeps moderators from deleting each other's documents does not apply.
- The document is updated in place. Every other property stays as its owner wrote it, `$updatedAt` among them, so a change never opens a [`deleteWithin`](deletion.md#moderatorabilitiesdeletewithin) window again. `$revision` goes up by one, so a replace its owner built on the earlier revision is refused (`InvalidDocumentRevisionError`, 40106) rather than writing over the change.
- References are not checked again: no field a moderator writes holds one or is read by one (see the rules below). A report whose post a moderator has already deleted can still be marked handled.
- The moderator pays for the transition and the bytes it adds. Storage the change frees, or index entries it moves, are refunded to the document's owner, who paid for them. Nothing the type prices is charged: a moderator's change is no action of the owner's.
- A member of a seated team who signs a change has it counted toward the team's action share, as a deletion is.
- The proof of the change is the document as it now stands, holding the values the change set.

### Owners who moderate

A create that sets a listed field, and a replace that changes, adds or removes one, is refused unless its signer is a moderator of the contract (41124). The contract owner is one under the appointed and owner-only kinds and during an interim that names it; once an elected contract has a seated team, only the team's leader and members are, and only where the declaration gives the team `changeDocumentFields` on the type. A replace carries the whole document, so an owner who is not a moderator must carry the listed fields exactly as the moderators left them.

### Rules at registration

All refusals below are `InvalidContractStructure` (10231).

- The contract's config must declare `moderation`, as for `delete`. A `moderation` block may then keep no list at all.
- Refused on an `indexOnly` type: there is no stored row to change.
- Every entry must name a top-level property the type declares (list the object around a nested one), and that property must be:
  - optional: nobody but a moderator can set it, so it starts absent;
  - stored, so not `transient`;
  - not listed under `immutable`;
  - neither a `refersTo` reference nor read by one: not the referring side of a `propertyAgreement`, not a lookup key's source, not the identity property of a key id reference;
  - neither `generatedFrom` another property nor a parameter of one;
  - in no contested index.
- A type that lists any keeps `$revision` on its documents, even when `documentsMutable` is `false`, because a moderator's change is stored as an update.
- A `refersTo` lookup key, or a `listElement` reference's list, may not read a listed field of the type it refers to: such a field can change after the reference was checked.

## See also

- [Deletion](deletion.md), for `delete` and `deleteWithin`
- [Contract Moderation](../data-model/contract-moderation.md#changing-document-fields), for the transition, the checks and the proof
- [Mutability](mutability.md), for what a document's own owner may change
- [Contract-Level Keys and config](contract-config.md), for `moderation`
