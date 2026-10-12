# Deletion

A document can leave the state three ways: its owner deletes it, the contract's moderators delete it, or the platform deletes it when its time to live runs out. `canBeDeleted` rules the first, `moderatorAbilities.delete`, `moderatorAbilities.deleteWithin` and `moderatorAbilities.deleteSettled` the second, and `ttl` the third (see [Time To Live](ttl.md)). Each is independent of the others: a type may let moderators remove what its authors cannot retract, or expire documents that nobody may delete by hand.

## `canBeDeleted`

Whether a document's owner may delete it. Set it to `false` for records that other documents or other people rely on staying put, and to `"onlyWhenConsumed"` for records only a create that consumes them may remove.

| | |
|---|---|
| **Where** | document type |
| **Value** | boolean, or `"onlyWhenConsumed"` |
| **Default** | the contract config's `documentsCanBeDeletedContractDefault`, which is `true` unless the contract says otherwise |
| **Since** | protocol version 1; `"onlyWhenConsumed"` protocol version 14 |
| **On update** | Fixed (`DocumentTypeUpdateError`, 40212), except that a type which keeps history before and after the update may change it from `true` to `false`. Adding or removing the key without changing its value is refused too, as a schema change (`IncompatibleDocumentTypeSchemaError`, 10246). |
| **Errors** | `InvalidDocumentTransitionActionError` (10404) for a delete of a type set to `false` or `"onlyWhenConsumed"`, or, from protocol version 14, of a type that keeps history; `DocumentOwnerIdMismatchError` (40102) for a delete by anyone but the owner |

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
- An identity that is banned or suspended on a moderated contract may still delete its own documents. See [Contract Moderation](../data-model/contract-moderation.md#the-model). On a type set to `false` it can retract them instead, when the type declares [`retractedWhen`](#retractedwhen).
- `false` binds only the owner. The contract's moderators, when the type allows them, and the platform, when the type has a `ttl`, still delete such documents.
- `"onlyWhenConsumed"` binds the owner as `false` does, and lets a create consume the document (see [below](#deleted-only-when-consumed)).
- Drive never deletes a document whose type keeps history (`documentsKeepHistory`). From protocol version 14 a delete of such a document is refused with 10404 whatever `canBeDeleted` says; before it, the delete failed inside Drive as an internal error.
- Documents of an `indexOnly` type are deleted with an index-only delete transition that carries their values, since there is no stored row to name by id. A delete by id of such a document is refused (10404). See [Index-Only Types](index-only.md).

### Rules at registration

- From protocol version 14, a type with `documentsKeepHistory: true` must set `canBeDeleted: false` (`InvalidContractStructure`, 10231). The default is `true`, so it has to be written out. A contract registered earlier with both flags on stays readable, but its next update is checked like a new contract, so that update must turn `canBeDeleted` off on the type. That is the one change to `canBeDeleted` an update may make.
- For references, a type whose owner may delete its documents is deletable: a `permanentDocument` reference, `inList` included, may not point at it (`ReferencedDocumentTypeDeletableError`, 40122), and a `deletableDocument` reference may. See [References](refers-to.md).
- `"onlyWhenConsumed"` is refused on a type that keeps history or is `indexOnly` (`InvalidContractStructure`, 10231): the storage layer never deletes a document that keeps history, and an `indexOnly` type has no stored row a reference finds, so nothing could consume either.

### Deleted only when consumed

`"onlyWhenConsumed"` says the owner can not delete a document, as `false` does, but a create of the same contract whose `refersTo` declares [`consume`](refers-to-lookup.md#commit-and-reveal) can. Its owner never removes it: it leaves state when a create consumes it, or, as with `false`, when the contract's moderators delete it where the type allows them (`moderatorAbilities.delete`) or the platform deletes it when its `ttl` passes.

```json
"preorder": {
  "type": "object",
  "documentsMutable": false,
  "canBeDeleted": "onlyWhenConsumed",
  "indices": [
    { "name": "saltedHash", "properties": [{ "saltedDomainHash": "asc" }], "unique": true }
  ],
  "properties": {
    "saltedDomainHash": {
      "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32, "position": 0
    }
  },
  "required": ["$createdAtBlockHeight", "saltedDomainHash"],
  "additionalProperties": false
}
```

A preorder stays until the name registration that reveals it consumes it (the `preorderSalt` declaration in [Commit and reveal](refers-to-lookup.md#commit-and-reveal)). Its owner can not take it back with a delete.

- A delete transition of such a document is refused (`InvalidDocumentTransitionActionError`, 10404), as for `false`.
- A consume deletes it as a delete by its owner would, its storage refunded to the owner.
- For references the type is deletable, as with `true`: its documents can leave state without a record. A `permanentDocument` reference to it is refused (40122), and so is a `moderatedDocument` one (40143), even when the type also lets its moderators delete with records; a `deletableDocument` reference is the one that points at it.
- Fixed on update: an update may neither set nor remove it (`DocumentTypeUpdateError`, 40212). Setting it would let documents leave state under the `permanentDocument` references made to a type that promised they never would; removing it would leave the references that consume its documents nothing to delete.
- Before protocol version 14 the meta-schemas accept only a boolean, so a contract carrying the string is refused (`JsonSchemaError`, 10101).

## `retractedWhen`

The replace that retracts a document: what a banned or suspended author may still do on a type whose documents it can not delete. A barred identity can write nothing new, but deleting what it wrote is never refused. On a type set to `canBeDeleted: false` there is no delete, and the author's only way to take a document back is a replace that blanks it, which a bar refuses like any other write. `retractedWhen` names that replace, and lets it through.

| | |
|---|---|
| **Where** | document type |
| **Value** | one condition, in the grammar of an [`immutable`](mutability.md#immutable) entry's `when` |
| **Default** | none: a barred author's replaces are all refused |
| **Since** | protocol version 14 |
| **On update** | Fixed (`DocumentTypeUpdateError`, 40212): an update may not add it, remove it or change it |
| **Errors** | `ContractUserBannedError` (41107) or `ContractUserSuspendedError` (41108) for a barred author's replace whose written document does not meet the condition |

### Example

```json
"post": {
  "type": "object",
  "canBeDeleted": false,
  "properties": {
    "text": { "type": "string", "maxLength": 500, "position": 0 },
    "deleted": { "type": "boolean", "position": 1 }
  },
  "additionalProperties": false,
  "retractedWhen": { "present": "deleted" },
  "propertyConstraints": {
    "retractedIsBlank": { "anyOf": [{ "absent": "deleted" }, { "absent": "text" }] }
  },
  "immutable": [
    { "property": "deleted", "when": { "present": "$old.deleted" } }
  ]
}
```

Replies point at posts, so a post stays in place. Its author retracts one by replacing it with `{ "deleted": true }`. The three keywords split the work. `retractedWhen` says that a post carrying `deleted` is retracted, so a banned author may still write one. `retractedIsBlank` says that a retracted post carries no text, for every author. The `immutable` entry says that a post stays retracted once it is. A banned author's edit of its text is refused with 41107, and so is a replace that drops `deleted` again.

### How it works

- Only replaces are let through, and only those of a banned or suspended owner are judged. An identity that is not barred replaces its documents under the type's other rules alone.
- The moderation gate lets every replace of a barred owner on the type through. Once the stored document is fetched, the transformer judges the condition on the document the replace writes, as an `immutable` condition is judged: its properties as the replace sets them, the replace's block as `$updatedAt`, and the stored document under `$old.`. A replace for which the condition does not hold is refused with the bar's error and its nonce bumped, in a block and in the mempool alike. A condition that faults, dividing by zero or overflowing, counts as not holding: a fault never lifts a bar.
- The condition only picks the replaces a barred owner may make. Every other rule of the type still judges them: the schema, `propertyConstraints`, `immutable`, references, token costs and action fees. What a retracted document may hold is for those rules to say. Without a rule like `retractedIsBlank` above, a barred author could write anything into a document that meets the condition.
- Creates, transfers, purchases and price updates by a barred owner are refused as before, and deletions pass as before.

### Rules at registration

All refusals below are `InvalidContractStructure` (10231), checked on every parse.

- Only on a type whose documents are mutable: without a replace there is nothing to retract with.
- Only on a contract that keeps a banlist or a suspension list: otherwise no owner is ever barred.
- The condition reads what an `immutable` entry's `when` may read: declared properties of the right kind, neither transient nor inside a transient object, the system times and heights the type lists in `required`, and the stored document through `$old.`. It reads no `countOf` or `sumOf`. When a contract is registered or updated, it also stays within the node limit of a rule and lists no condition twice.

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

- The window is measured from the document's `$updatedAt`, or from its `$createdAt` on a type that does not record `$updatedAt`. A deletion at exactly that time plus the window still passes; after it, every moderator is refused, the contract owner included. On a type that sets [`deleteSettled`](#moderatorabilitiesdeletesettled), the seated team may still delete it together.
- A replace or a price update moves `$updatedAt`, so new content opens the window again. A transfer, a purchase or a moderator's [field change](moderator-abilities.md#changefields) does not move it.
- A restored document comes back with its old `$updatedAt`, so it may already be settled.
- The window says nothing about the document's own owner, whose deletion `canBeDeleted` rules at any age.

### Rules at registration

- Needs `delete: true` (`InvalidContractStructure`, 10231).
- A type whose documents can be replaced must list `$updatedAt` in `required`: measured from creation alone, an author could wait the window out and then rewrite a post into something no moderator can remove. A type with `documentsMutable: false` must list `$updatedAt` or `$createdAt`. Both refusals are 10231.
- A window of 0 is refused by the meta-schema (`JsonSchemaError`, 10101). A type that moderators may never delete from simply leaves `delete` out.

## `moderatorAbilities.deleteSettled`

Who must agree to delete a settled document: one past its `deleteWithin` window, which no moderator deletes alone. It lets a contract keep settled content safe from any single moderator while leaving its elected moderation team a way to remove it when the team agrees: so many members together, the leader among them only when the rule says so (`leader: true`), which may be the leader alone.

| | |
|---|---|
| **Where** | `moderatorAbilities` of a document type, with `delete: true` and `deleteWithin`, in a contract whose moderators are an elected team |
| **Value** | object with `leader` (boolean, default `false`: whether the team's leader must be among the approvals), `approvals` (integer, default 1: how many members of the seated team must approve, the leader counted, at least 1 and at most the members the declared team can hold) and `approversPredateDocument` (boolean, default `true` when `approvals` is above 1 and `false` otherwise: whether a member the leader added counts only for documents created after its addition), at least one of them given |
| **Default** | absent: nobody deletes a settled document |
| **Since** | protocol version 14 |
| **On update** | Fixed (`DocumentTypeUpdateError`, 40212), in both directions: fewer approvals would reach content written under more |
| **Errors** | `DocumentTypeNotDeletableOnceSettledError` (41204), `ContractModerationTeamNotSeatedError` (41205), `DocumentNotSettledError` (41206), `ContractTeamActionDoesNotExistError` (41207), `ContractTeamActionAlreadySignedError` (41208), `SettledDeletionNotRestorableError` (41209), `ContractTeamActionAlreadyCompletedError` (41210), `ContractTeamActionDocumentChangedError` (41211), `ContractTeamMemberAddedAfterDocumentError` (41212), and those of a moderator's deletion (41101, 41102, 41201, 41203) |

### Example

```json
"post": {
  "type": "object",
  "documentsMutable": true,
  "moderatorAbilities": {
    "delete": true,
    "deleteWithin": 86400,
    "deleteSettled": { "leader": true, "approvals": 3 }
  },
  "properties": {
    "text": { "type": "string", "maxLength": 280, "position": 0 }
  },
  "required": ["$createdAt", "$updatedAt", "text"],
  "additionalProperties": false
}
```

For a day after a post is written or edited, any moderator deletes it. After that, it is deleted only when the team's leader and two other members approve: one of them proposes the deletion, and the others approve the proposal. A member the leader added counts only for posts written after the leader added it. `{ "leader": true }` would let the leader alone delete a settled post; `{ "approvals": 2 }` any two members; `{ "leader": true, "approvals": 3, "approversPredateDocument": false }` the leader and two others, whenever added.

### How it works

The team deletes a settled document the way a token group carries out an action: one member proposes it, the others approve it by its id, and it runs once the approvals meet the rule.

- **Proposing.** A member of the seated team sends the contract user moderation transition's `deleteSettledDocument` action, naming the document type, the document id and a reason. The proposal is its own approval. It is kept under the contract as a team action, naming the document as it is (its last modification and revision) and the reason, by an id the proposer's client computes from the contract, the proposer, its nonce, the document and the reason. A rule the proposer meets alone (`{ "leader": true }` proposed by the leader) deletes the document at once.
- **Approving.** The other members send the `approveTeamAction` action with that id. What is deleted and why is the proposal's: an approval carries nothing else. The team's actions and who approved each are readable with `getContractTeamActions` and `getContractTeamActionSigners`, which is how a member finds what the others proposed.
- **Running.** The approval that meets the rule deletes the document as a moderator's `deleteDocument` would: its removal record (unless the type sets `deleteKeepsRecord: false`, with the proposal's reason and the member whose approval deleted it), and the owner's refund as `deleteRefundsOwner` says. The action then moves, with the approvals that counted, from the contract's active team actions to its closed ones, for good.
- **Nothing lapses.** A proposal stays active until it runs. Once the document changes (the author edits it, it changes hands, or a moderator changes its fields: anything that moves its `$revision`), an approval is refused (41211): the team would be approving the deletion of content it never saw. A member proposes afresh instead, a new action. Two proposals may name one document; once one runs, the other takes no approval (the document is gone, 40101) and stays active, as a token group's action that never gathers its power does. An edit by the author opens the `deleteWithin` window again anyway, in which any moderator deletes the document alone; a moderator's change of its fields does not.
- **Members the leader added.** The leader names whom it adds (`addedModerator`), so without a limit it could add members who approve whatever it proposes, and take them off again once they had. Unless the rule sets `approversPredateDocument: false`, a member the leader added counts only for documents created after its addition (the default whenever more than one approval is needed; a rule one approval meets, the leader meets alone, so it dates nobody by default): its `addedModerator`'s `$createdAt` is earlier than the document's `$createdAt`. One added in the same block as the document does not count. Its proposal or approval of an older document is refused (41212). The leader and the elected members always count, for documents older than the team too: the election seated them, not the leader. What the rule needs is not lowered for it: a team with too few members from before a document never deletes that document once settled. Every addition comes after the seat, so a document written before the seat counts only the leader and the elected members: under a rule asking for more approvals than those, no document older than the seat is ever deleted once settled, and the rule can not change. Pick `approvals` no higher than the leader plus the members a charter is expected to elect, or set `approversPredateDocument: false`.
- **Members who left.** A member who left the team since approving no longer counts. When the approvals given could meet the rule, an approval reads the team and drops the approvals of members who are gone, refunded to them: one who comes back after that approves again, while one back before any such read still has its approval counted. A member the leader takes off and adds again sits in the new addition: under `approversPredateDocument` its approval of a document created before the new addition is dropped the same way, and it approves that document no more. A dropped approval no longer proves: the proof of that member's proposal or approval fails from then on.
- **The checks of a proposal**, in order, each refusal paid: the document type exists (10406) and sets `deleteSettled` (41204); a team is seated (41205): the interim moderators and the contract owner never delete a settled document; the signer is on the team (41101) and the declaration gives the team `deleteDocuments` on the type (41201); the reason names a reason document the team's proposal lists (41203); the document exists (40101) and its owner is not protected (41102); the document is settled (41206: within the window, use `deleteDocument`); a signer the leader added was added before the document was created (41212).
- **The checks of an approval**, in order, each refusal paid: the contract keeps team actions and the team proposed this one (41207); the action has not run (41210); a team is seated (41205); the signer is on the team with `deleteDocuments` on the type (41101, 41201); the document still exists (40101), its owner is not protected (41102), and it has not changed since the proposal (41211); a signer the leader added was added before the document was created (41212); the signer has not approved it already (41208): a member taken off and added again too late is told so, not that its earlier approval, which no longer counts, stands.
- A deletion the team approved stands: no moderator restores it, the leader included (`SettledDeletionNotRestorableError`, 41209). A single moderator undoing what the leader and the members agreed on would defeat the rule; a deletion within the window is restored as before.
- A deletion counts toward the moderators pot's action share for every approver whose approval counted, once it happens. Approvals that fall short count for nobody.
- Each member pays for the transition and for its approval (the proposer for the action too), and is refunded its approval (the proposer the action's info too, but not the two trees that hold the action, which carry no storage flags and refund nobody) when the action runs and moves to the closed actions, or when a later approval drops it. The member whose approval runs the action pays for its closed copy, which nothing deletes.

### Rules at registration

All refusals below are `InvalidContractStructure` (10231).

- Needs `deleteWithin`: without a window nothing is ever settled.
- Needs a contract whose `moderation` declares an elected team. The elected declaration must give the team `deleteDocuments` on the type (`InvalidContractModerationConfigError`, 10900), or no team could ever use the rule.
- `approvals` is at least 1 and at most the members the declared team can hold: its leader, the 15 members a charter elects and the `maxAddedModerators` of the elected declaration (31 at most). A rule no team could meet would leave settled documents undeletable for good, since neither the rule nor the declaration can change. A seated team whose charter elects fewer than 15 members holds fewer: a rule asking for more than it can hold asks for all of them. A member the leader removed still counts toward what the team can hold: the leader can not lower the bar by removing members who would not approve, and gets the seat back by deleting the removal.
- While `approversPredateDocument` is on (the default when `approvals` is above 1), the type must list `$createdAt` in `required`: who of the team predates a document is read from it. A type that does not record `$createdAt` sets `approversPredateDocument: false`. Checked at registration only, as the bound on `approvals` is: a stored type is read back as it is, and a document without `$createdAt` admits no added member.

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
- They are read wherever the record is: `getContractDocumentRemovals`, by document id or by page, the proof of the deletion, and a join through a [`moderatedDocument`](refers-to.md#moderateddocument) reference. The records are not indexed by them: no query finds a record by a kept value.
- A `moderatedDocument` reference to a removed document checks a `where` pair on a kept property against the kept value, as it would against the document.
- An index of a referring type may hold a kept value, read through a `moderatedDocument` reference: a [derived index property](derived-index-properties.md), such as a reply's `postId.hashtag`. The value must be one an index can key and fixed once written: the example above is not, since its documents are mutable and `hashtag` is not listed under [`immutable`](mutability.md). Once the document is removed, Drive reads the value from the record.
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
| The seated moderation team, together | `moderatorAbilities.deleteSettled`, once `moderatorAbilities.deleteWithin` has passed | As for the moderators |
| The platform | `ttl`, once it has passed | No |

Which reference may point at a type follows from which of the three it allows. A type that allows none of them is the target of a `permanentDocument` reference, with `inList` or without. A type that allows only the moderators' deletion, with removal records kept, is the target of a `moderatedDocument` reference: its documents never leave state without a record. Any other type is the target of a `deletableDocument` reference.

## See also

- [Deleting Documents](../data-model/contract-moderation.md#deleting-documents), [Deleting Settled Documents](../data-model/contract-moderation.md#deleting-settled-documents) and [Restoring Documents](../data-model/contract-moderation.md#restoring-documents), for the moderation transition, the approvals and the removal record
- [Moderator Abilities](moderator-abilities.md), for the `moderatorAbilities` object and the fields only moderators write
- [Time To Live](ttl.md), the third way a document leaves the state
- [History](history.md), for why a type that keeps history can never delete
- [Mutability](mutability.md), for `immutable` and the conditions `retractedWhen` shares with it, and [Creation, Transfers and Trading](ownership-and-trading.md)
- [References](refers-to.md), for `permanentDocument`, `moderatedDocument` and `deletableDocument`
- [Contract-Level Keys and config](contract-config.md), for `documentsCanBeDeletedContractDefault` and `moderation`
