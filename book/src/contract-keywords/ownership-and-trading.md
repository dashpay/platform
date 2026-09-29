# Creation, Transfers and Trading

A document belongs to the identity in its `$ownerId`: at first the one that created it. These three keywords decide who may create documents of a type, and whether a document may later change hands. `creationRestrictionMode` limits who creates. `transferable` lets an owner give a document away. `tradeMode` lets an owner put a price on a document and anyone else buy it at that price.

The example below is used throughout the chapter:

```json
"ticket": {
  "type": "object",
  "documentsMutable": false,
  "canBeDeleted": false,
  "creationRestrictionMode": 1,
  "transferable": 1,
  "tradeMode": 1,
  "properties": {
    "event": { "type": "string", "maxLength": 63, "position": 0 },
    "seat": { "type": "string", "maxLength": 10, "position": 1 }
  },
  "required": ["event", "seat", "$createdAt"],
  "additionalProperties": false
}
```

Only the organiser, the identity that owns the contract, can issue tickets. A ticket's holder may give it to a friend, or list it for sale; anyone may then buy it at the listed price, with no approval from the seller. Nobody can edit a ticket or delete it.

## `creationRestrictionMode`

Who may create documents of the type.

| | |
|---|---|
| **Where** | document type |
| **Value** | `0` anyone, `1` the contract owner only, `2` nobody |
| **Default** | `0` |
| **Since** | protocol version 1 |
| **On update** | Fixed (`DocumentTypeUpdateError`, 40212). Adding or removing the key without changing its value is refused too, as a schema change (`IncompatibleDocumentTypeSchemaError`, 10246). |
| **Errors** | `DocumentCreationNotAllowedError` (10416) |

### How it works

- `0`: any identity may create documents of the type.
- `1`: only the identity that owns the contract may. A create signed by anyone else is refused (`DocumentCreationNotAllowedError`, 10416). Once created, a document may still pass to other identities if the type is transferable or tradeable.
- `2`: no create transition is ever accepted. This is for system contracts whose documents only the platform writes, such as the document history contract (see [History](history.md)) and the keyword search contract. On a user's contract the type would stay empty for good.
- The mode rules creation only. Replaces, deletes, transfers and sales are decided by each document's own owner and by the other keywords.

### Rules at registration

- A type with mode `1` or `2` may not carry `moderatorAbilities.delete` (`InvalidContractStructure`, 10231): its documents belong to the contract owner or the platform, and no moderator may delete those. See [Deletion](deletion.md).

## `transferable`

Whether an owner may give a document to another identity.

| | |
|---|---|
| **Where** | document type |
| **Value** | `0` never, `1` always |
| **Default** | `0` |
| **Since** | protocol version 1 |
| **On update** | Fixed (`DocumentTypeUpdateError`, 40212). Adding or removing the key without changing its value is refused too, as a schema change (10246). |
| **Errors** | `InvalidDocumentTransitionActionError` (10404) for a transfer of a type set to `0` |

### How it works

- The owner sends a transfer transition naming the document, the recipient and a `$revision` one higher than the stored one (`InvalidDocumentRevisionError`, 40106). Only the owner may transfer (`DocumentOwnerIdMismatchError`, 40102). The recipient does not have to agree.
- The document gets the recipient as its `$ownerId` and a new `$revision`. A price it was listed at is removed, so a transferred document is no longer for sale. When the type lists `$transferredAt` in `required`, it is set to the block's time, and likewise `$transferredAtBlockHeight` and `$transferredAtCoreBlockHeight` to the block heights.
- A transfer carries no property values, and `$creatorId` keeps naming the identity that created the document. The one change the platform makes itself: from protocol version 13, a DPNS `domain` that is transferred or sold has its `records.identity` pointed at the new owner.
- The document is checked as it will be stored, with its new owner: against the type's unique indexes (`DuplicateUniqueIndexError`, 40105), against a `distinctFrom: "$ownerId"` property (`DocumentPropertyNotDistinctError`, 10419), and against `propertyConstraints` rules that read `$ownerId` (`DocumentPropertyConstraintViolatedError`, 10422).
- On a moderated contract, a transfer to a banned or suspended identity is refused (`ContractModerationCounterpartyBarredError`, 41114).
- A transfer of a document past its `ttl` expiry is refused (`DocumentExpiredError`, 40140).
- With `keepsTransferHistory: true`, each transfer is also recorded in the document history contract. See [History](history.md).

## `tradeMode`

Whether documents of the type can be sold through the platform's built-in marketplace. With `1`, direct purchase, an owner sets a price and any other identity may buy the document at that price, with no approval.

| | |
|---|---|
| **Where** | document type |
| **Value** | `0` none, `1` direct purchase |
| **Default** | `0` |
| **Since** | protocol version 1 |
| **On update** | Fixed (`DocumentTypeUpdateError`, 40212). Adding or removing the key without changing its value is refused too, as a schema change (10246). |
| **Errors** | `InvalidDocumentTransitionActionError` (10404) for a price update or a purchase on a type set to `0`, and for a purchase by the document's own owner; `DocumentNotForSaleError` (40108); `DocumentIncorrectPurchasePriceError` (40109) |

### How it works

- **Listing.** The owner sends a price update transition with a price in credits and the next `$revision`. Only the owner may set the price (40102). The price is stored with the document, the revision goes up, and `$updatedAt` is set when the type lists it in `required`.
- **Buying.** Any identity other than the owner sends a purchase transition naming the document, the next `$revision` and the price. A document with no price set is not for sale (`DocumentNotForSaleError`, 40108), and the price in the transition must equal the listed price exactly (`DocumentIncorrectPurchasePriceError`, 40109). An owner cannot buy its own document (10404).
- **What a purchase does.** The price moves from the buyer's credit balance to the seller's. The document gets the buyer as its `$ownerId` and a new `$revision`, the listing is removed, and `$transferredAt` is set when the type lists it in `required`. The buyer's credit balance must cover the price.
- The new owner is checked exactly as for a transfer: unique indexes (40105), `distinctFrom: "$ownerId"` (10419) and `propertyConstraints` rules that read `$ownerId` (10422).
- On a moderated contract, a banned or suspended buyer is refused like any barred writer, and so is a purchase from a banned or suspended seller (`ContractModerationCounterpartyBarredError`, 41114).
- A price update or a purchase of a document past its `ttl` expiry is refused (`DocumentExpiredError`, 40140).
- With `keepsPricingHistory` and `keepsPurchaseHistory`, each price update and each purchase is also recorded in the document history contract. See [History](history.md).

## How they combine

- `transferable` and `tradeMode` are independent. The DPNS `domain` type sets both, so a name can be given away or sold. A type may allow sales without gifts, or gifts without sales.
- Neither needs `documentsMutable`. A document that cannot be replaced can still change owner and price, as the ticket above does.
- A type that is transferable or tradeable stores a `$revision` on each document even when its documents cannot be replaced, and, from protocol version 10 on a format-1 contract whose config is version 1 or later, records each document's creator in `$creatorId`. See [System Properties](system-properties.md).
- Every action has its own optional token cost and action fee: `transfer`, `update_price` and `purchase`, besides `create`. See [Token Costs](token-cost.md) and [Action Fees](action-fees.md).
- `signatureSecurityLevelRequirement` applies to all of these actions, so a buyer signs a purchase with a key at the level the type requires. See [Signing and Keys](signing-keys.md).

## Rules at registration

All refusals below are `InvalidContractStructure` (10231).

- `ownerRefersTo` is refused on a type whose documents can be transferred or traded: a document could end up with an owner the declaration never checked. Such a type uses `creatorRefersTo`, which checks the creator, who never changes. `creatorRefersTo` is only accepted on such a type. See [Writer and Creator References](owner-refers-to.md).
- An `indexOnly` type can be neither transferable nor tradeable. See [Index-Only Types](index-only.md).
- On a transferable or tradeable type, an `immutable` property may not hold a `contract` reference with an `owner` requirement. See [Mutability](mutability.md).
- `creationRestrictionMode` `1` or `2` is refused together with `moderatorAbilities.delete`.

## See also

- [History](history.md), for recording transfers, purchases and price updates
- [Mutability](mutability.md) and [Deletion](deletion.md), the other keywords on what may happen to a document
- [System Properties](system-properties.md), for `$ownerId`, `$creatorId`, `$revision` and `$transferredAt`
- [distinctFrom](distinct-from.md) and [propertyConstraints](property-constraints.md), which also judge a new owner
- [Contract Moderation](../data-model/contract-moderation.md), for barred counterparties
