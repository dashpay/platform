# Expressions

A [`refersTo`](refers-to.md) declaration may combine several targets instead of naming one. `anyOf` holds when at least one of its operands holds, and `allOf` when every operand holds for the same value. Reach for an expression when a value may point at one of several kinds of thing ("an elected member or an added one"), or must satisfy several references at once ("a member of the team who also has a profile"). Both combinators take the same operands, follow the same limits and are checked the same way; they differ only in when they stop.

## `anyOf`

| | |
|---|---|
| **Where** | In place of a single target: in `refersTo` on an identifier property or on the `items` of a typed array of identifiers, in `ownerRefersTo` and `creatorRefersTo`, and as an operand of an `allOf`. Not on a key id property. |
| **Value** | `{ "anyOf": [ ... ] }`, the declaration's only key: 2 to 4 distinct [operands](#operands). |
| **Since** | protocol version 14 |
| **On update** | Fixed (`IncompatibleDocumentTypeSchemaError`, 10246), including a change of operand order. |
| **Errors** | None of its own: when no operand holds, the write is refused with the last operand's error, for example `ReferencedEntityNotFoundError` (40120). |

The operands are checked in the order they are listed, and the first that holds decides: the rest are not read. When none holds, the write is refused with the error of the last one.

The moderation charters contract lets a member of a seated team resign with a `resignationRequest`. Its writer must be on the team, either elected or added later:

```json
"ownerRefersTo": {
  "anyOf": [
    {
      "type": "listElement",
      "documentType": "electedCharter",
      "propertyAgreement": { "electedCharterId": "$id" },
      "inList": "members"
    },
    {
      "type": "deletableDocument",
      "documentType": "addedModerator",
      "lookup": {
        "index": "byElectedCharterMember",
        "keys": { "electedCharterId": "electedCharterId", "memberId": "." }
      }
    }
  ]
}
```

The writer must be one of the `members` of the `electedCharter` this document's `electedCharterId` names, or have an `addedModerator` document for that charter that exists now. Here the expression is the writer's reference (see [Writer and Creator References](owner-refers-to.md)); the same declaration works on an identifier property, where it judges the property's value.

## `allOf`

| | |
|---|---|
| **Where** | In place of a single target: in `refersTo` on an identifier property or on the `items` of a typed array of identifiers, in `ownerRefersTo` and `creatorRefersTo`, and as an operand of an `anyOf`. Not on a key id property. |
| **Value** | `{ "allOf": [ ... ] }`, the declaration's only key: 2 to 4 distinct [operands](#operands). |
| **Since** | protocol version 14 |
| **On update** | Fixed (`IncompatibleDocumentTypeSchemaError`, 10246), including a change of operand order. |
| **Errors** | None of its own: the write is refused with the first failing operand's error, for example `ReferencedEntityNotFoundError` (40120). |

The operands are checked in the order they are listed, and the first that fails decides: the rest are not read, and the write is refused with that operand's error.

```json
"memberId": {
  "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
  "contentMediaType": "application/x.dash.dpp.identifier",
  "refersTo": {
    "allOf": [
      {
        "type": "listElement",
        "contractId": "EG7RGfV8fDTayC2FyVr8HwdpJh3fXDbVztcfE94UmN88",
        "documentType": "electedCharter",
        "propertyAgreement": { "electedCharterId": "$id" },
        "inList": "members"
      },
      {
        "type": "deletableDocument",
        "contractId": "Bwr4WHCPz5rFVAD87RqTs3izo4zpzwsEdKPWUT1NS1C7",
        "documentType": "profile",
        "lookup": { "index": "ownerId", "keys": { "$ownerId": "." } }
      }
    ]
  },
  "position": 1
}
```

This property of a hypothetical moderator directory, next to an `electedCharterId` identifier property, must be one of the elected members of that charter in the moderation charters contract, and must have a DashPay profile when the document is written. The list check comes first, so a value that is not a member is refused without the profile lookup being read.

## Operands

An operand is a leaf or a nested expression.

**Leaves.** A leaf is an ordinary target declaration with its own keys, one of:

- `identity`;
- `permanentDocument`, by id or with a [`lookup`](refers-to-lookup.md);
- [`listElement`](refers-to-list-element.md);
- `deletableDocument` with a `lookup`.

The first three are existence checks against things that are never deleted, so an expression made only of them holds for good once it holds. A deletable lookup may find nothing later, which is why an expression holding one is checked on every replace.

The other targets are refused as operands, since they do not compose with other operands:

- `deletableDocument` by id: once its document is deleted a replace may clear the property, which assumes the property refers to that one target.
- `identityPublicKey`, in either form: it pairs the value with a key id that no other operand reads.
- `contract`: its requirements are gates judged against the block time and the writer, not an existence check, and a contract id is never also an identity or document id.
- `token`: a token id is never also an identity or document id.

**Nesting.** An operand may be an expression of the other combinator: an `allOf` inside an `anyOf`, or an `anyOf` inside an `allOf`. An `anyOf` directly inside an `anyOf`, or an `allOf` inside an `allOf`, is refused, since it says what one flat list says.

A [`propertyAgreement`](refers-to.md#propertyagreement) or a `lookup` belongs to its leaf, inside it: the expression itself holds nothing but its combinator.

## How it works

- **Order decides the error.** A refusal is always the error a leaf declared alone would give, naming the property (or the element) as a single reference would. So the author's order decides which error a writer sees: put the most general operand of an `anyOf` last, and the cheapest or most telling operand of an `allOf` first.
- **Every read is billed.** A value the second operand of an `anyOf` holds for also pays for the first operand's read. An `allOf` whose first operand fails reads nothing more.
- **Agreements are per leaf.** A `propertyAgreement` is checked only against its own leaf's document. A value whose first leaf fails its agreement can still be accepted through a second leaf that has none.
- **Typed arrays.** On the `items` of a typed array, each element meets the expression on its own.
- **Replace.** An expression is checked again when any of its leaves would be checked again alone (see [On replace](refers-to.md#on-replace)), and then it is evaluated whole, since which operands hold may have changed. An expression holding a `deletableDocument` lookup is therefore checked on every replace.
- **Budget.** Every leaf counts against the [reference budget](refers-to.md#the-reference-budget), since each may be read for each value. An `anyOf` of two leaves on a typed array of `maxItems` 15 counts 30.
- **Queries.** An expression cannot be the join property of a chained query or of a composite join by id, and a `preallocated` index is never bound through one.

## Rules at registration

- The combinator is the declaration's only key, and lists at least two operands: a single one is declared on its own. No two operands of one list may be alike; a leaf naming the declaring contract's id in `contractId` is the same as one leaving it out.
- A list holds at most **4** operands, and any path from the declaration to a leaf passes through at most **4** combinators. The `anyOf` holding an `allOf` is 2 deep.
- Every leaf is one of the four admitted targets, and is checked exactly as the same target declared alone: its document type, the type's deletability, its `propertyAgreement` and its `lookup` or list. Every leaf must pass, since each has to be a declaration that could hold. The errors name the leaf by where it sits: `refersTo anyOf[1].allOf[1] lookup: ...` from the parser, `resignation.memberId.anyOf[1].allOf[1]` from the registration check.
- An `immutable` property may not hold an expression with a `deletableDocument` lookup leaf (`InvalidContractStructure`, 10231).
- Every leaf counts against the reference budget of 256.

A malformed expression is refused by the meta-schema (`JsonSchemaError`, 10101) or the parser (`InvalidContractStructure`, 10231); a leaf that cannot hold, with the reference error it would get alone (see [Errors](refers-to.md#errors)). On an update, any change is refused (10246): an operand added, removed, changed or moved, `anyOf` swapped for `allOf`, or a single target turned into an expression or back.

## See also

- [References (refersTo)](refers-to.md) for the targets and the replace rules.
- [Lookups](refers-to-lookup.md), [List Elements](refers-to-list-element.md) and [Writer and Creator References](owner-refers-to.md), whose declarations are the usual leaves and holders of an expression.
- [Reference expressions](../data-model/documents.md#reference-expressions-anyof-allof) in the Documents chapter, with the internals.
- [propertyConstraints](property-constraints.md), whose rules also combine with `anyOf` and `allOf` but compare the document's own values instead of reading other state.
