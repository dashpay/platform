# Token Costs (tokenCost)

`tokenCost` makes an action on a document cost tokens: creating a card costs 10 gems, deleting one costs 1. The tokens are taken from whoever signs the transition, and either go to the contract owner or are burned. Reach for it when an app has its own token and wants documents to be paid for with it, or wants to hand out tokens that let users act for free, with the contract owner paying the gas.

| | |
|---|---|
| **Where** | Document type |
| **Value** | An object keyed by action: `create`, `replace`, `delete`, `transfer`, `update_price`, `purchase`. Each value is a cost object with the keys below. Actions left out cost no tokens |
| **Default** | Absent: no action costs tokens |
| **Since** | protocol version 9. `gasFeesPaidBy` is accepted from 9 and acted on from 14; `optional` is 14 |
| **On update** | Fixed: a cost may not be added, changed or removed on an existing document type (`DocumentTypeUpdateError`, 40212) |
| **Errors** | On a document transition: `RequiredTokenPaymentInfoNotSetError` (40115), `IdentityHasNotAgreedToPayRequiredTokenAmountError` (40116), `IdentityTryingToPayWithWrongTokenError` (40117), `IdentityTokenAccountFrozenError` (40702), `IdentityDoesNotHaveEnoughTokenBalanceError` (40700), `GasFeesPaidByNotAllowedError` (40129), `InconsistentGasFeesPaidByInBatchError` (40130), `GasSponsorInsufficientBalanceError` (40222). At registration: `InvalidTokenPositionError` (10451), `RedundantDocumentPaidForByTokenWithContractId` (10275), `TokenPaymentByBurningOnlyAllowedOnInternalTokenError` (10261), `DataContractNotFoundError` (40008), `InvalidTokenPositionStateError` (40009) |

The keys of each cost object:

| Key | Value | Default | Meaning |
|---|---|---|---|
| `tokenPosition` | integer, 0 to 65535 | required | Which token is charged: its position in this contract's `tokens`, or in the contract `contractId` names |
| `amount` | integer, 1 to 281474976710655 | required | How many tokens the action costs |
| `contractId` | identifier (32 bytes) | this contract | The contract whose token is charged, when it is not this one |
| `effect` | `0` transfer to the contract owner, `1` burn | `0` | What happens to the tokens paid |
| `gasFeesPaidBy` | `0` document owner, `1` contract owner, `2` prefer contract owner | `0` | Who the contract owner offers to have pay the gas of this action (acted on from protocol version 14) |
| `optional` | boolean | `false` | `true` lets a transition skip the token and pay the gas in credits instead (protocol version 14) |

## Example

```json
"card": {
  "type": "object",
  "documentsMutable": true,
  "canBeDeleted": true,
  "transferable": 1,
  "tradeMode": 1,
  "tokenCost": {
    "create": { "tokenPosition": 0, "amount": 10, "gasFeesPaidBy": 2, "optional": true },
    "replace": { "tokenPosition": 1, "amount": 1 },
    "delete": { "tokenPosition": 1, "amount": 1, "effect": 1 }
  },
  "properties": {
    "name": { "type": "string", "minLength": 1, "maxLength": 63, "position": 0 },
    "attack": { "type": "integer", "minimum": 0, "maximum": 100, "position": 1 }
  },
  "required": ["name", "attack"],
  "additionalProperties": false
}
```

The contract has two tokens. Creating a card costs 10 of token 0, which go to the contract owner. A player who pays with the token and asks for it has the gas paid by the contract owner, when the owner's balance covers it; a player may also skip the token and pay the gas in credits. Replacing a card costs 1 of token 1, also to the owner. Deleting one burns 1 of token 1. Transfers, price updates and purchases cost no tokens.

## Paying: `$tokenPaymentInfo`

A document transition on an action with a token cost carries a `$tokenPaymentInfo` in its base, saying which token the signer agrees to pay with, how much at most, and who they ask to pay the gas:

```json
"$tokenPaymentInfo": {
  "$formatVersion": "0",
  "tokenContractPosition": 0,
  "maximumTokenCost": 10,
  "gasFeesPaidBy": "PreferContractOwner"
}
```

| Field | Meaning |
|---|---|
| `paymentTokenContractId` | The contract of the token paid with. Leave it out for a token of the document's own contract: it must match the cost's `contractId` exactly, and a cost on the contract's own token has none |
| `tokenContractPosition` | The token's position in that contract |
| `minimumTokenCost`, `maximumTokenCost` | Optional bounds on the amount the signer agrees to pay; a cost outside them refuses the transition |
| `gasFeesPaidBy` | `"DocumentOwner"`, `"ContractOwner"` or `"PreferContractOwner"`: who the signer asks to pay the gas (see [Who pays the gas](#who-pays-the-gas-gasfeespaidby)) |

When the transition is processed:

1. A required cost with no `$tokenPaymentInfo` is refused (`RequiredTokenPaymentInfoNotSetError`, 40115).
2. A payment info naming another token than the cost is refused (`IdentityTryingToPayWithWrongTokenError`, 40117).
3. A cost outside the signer's `minimumTokenCost` and `maximumTokenCost` is refused (`IdentityHasNotAgreedToPayRequiredTokenAmountError`, 40116).
4. The signer's gas request must be one the cost offers, and the whole batch must name one payer (40129, 40130).
5. Against state: a signer whose account for the token is frozen is refused (`IdentityTokenAccountFrozenError`, 40702), and so is one whose balance is below `amount` (`IdentityDoesNotHaveEnoughTokenBalanceError`, 40700).

The signer pays: the creator for a create, the owner for a replace, delete, transfer or price update, and the buyer for a purchase.

## `effect`: transfer or burn

- `0`, the default, moves the tokens from the signer to the owner of the contract that holds the document type. When the contract owner performs the action themselves nothing moves, though their balance is still checked.
- `1` burns the tokens from the signer's balance, lowering the token's supply. Only a token of the contract's own can be burned (`TokenPaymentByBurningOnlyAllowedOnInternalTokenError`, 10261, at registration).

## Tokens of another contract: `contractId`

A document type may charge a token another contract defines, for example a shared currency. `contractId` names that contract and `tokenPosition` the token in it. The effect must then be `0`: the tokens go to the owner of the contract holding the document type, not to the token's issuer. At registration the named contract must exist (`DataContractNotFoundError`, 40008) and have a token at that position (`InvalidTokenPositionStateError`, 40009), and it must not be the contract itself: leave `contractId` out for the contract's own tokens (`RedundantDocumentPaidForByTokenWithContractId`, 10275).

## Who pays the gas: `gasFeesPaidBy`

From protocol version 14 the contract owner can pay the gas (the storage and processing fees) of an action paid with a token. The cost states what the contract owner offers; the transition's `$tokenPaymentInfo.gasFeesPaidBy` states what the signer asks for. The two resolve like this:

| Cost offers / signer asks | `DocumentOwner` | `PreferContractOwner` | `ContractOwner` |
|---|---|---|---|
| `0` document owner | signer pays | signer pays | refused (40129) |
| `2` prefer contract owner | signer pays | contract owner, if their balance covers it | refused (40129) |
| `1` contract owner | signer pays | contract owner, if their balance covers it | contract owner |

- A signer can always pay for themself, can always state a preference, and can insist on the contract owner only where the cost offers `1`. A request the cost does not cover is refused (`GasFeesPaidByNotAllowedError`, 40129). An action without a token cost offers `0`, and a transition without `$tokenPaymentInfo` asks for `DocumentOwner`.
- A batch has one payer. Transitions of one batch that resolve to different payers are refused (`InconsistentGasFeesPaidByInBatchError`, 40130); a token transition in the batch is never sponsored, so it counts as the signer paying.
- When the contract owner's balance does not cover the gas (and any action fees they would owe), a batch that insisted is refused and charged nothing (`GasSponsorInsufficientBalanceError`, 40222), and a batch that only preferred falls back to the signer.
- A transition that fails validation is never sponsored: its signer pays for the work that ran.
- Storage refunds still go to the document's owner, whoever paid for the storage. Each token the contract owner hands out is therefore worth up to the storage fee of the largest document the type allows, so a type that offers to pay should bound its documents' size (`maxLength`, `maxItems`, [maxBytes](max-bytes.md)) and price the action to match.
- A sponsor also pays any [action fee](action-fees.md) the action charges.

Before protocol version 14 the key was accepted and stored but not acted on: the signer always paid.

## Optional costs

With `optional: true` a transition may leave `$tokenPaymentInfo` out. It then pays no token, its signer pays the gas in credits as on an action without a token cost, and no sponsorship applies. With `$tokenPaymentInfo` present the token is charged exactly as for a required cost, sponsorship included, and an insufficient token balance is a rejection, never a fallback to credits: the client chooses between token and credits before signing.

Together with `gasFeesPaidBy` this gives a "free usage" pattern: an app hands out tokens, users act for free while their tokens last, and keep going on credits after.

## Rules at registration

- The meta-schema checks the shape (`JsonSchemaError`, 10101): only the six action keys; `tokenPosition` and `amount` required in each cost; values in the ranges above; no other key. Before protocol version 14 it also refuses `optional`.
- Without `contractId`, `tokenPosition` must be a token of this contract (`InvalidTokenPositionError`, 10451).
- With `contractId`: not this contract's own id (10275), no burn (10261), and a contract that exists with a token at that position (40008, 40009).

## See also

- [Gas paid by the contract owner](../fees/overview.md#gas-paid-by-the-contract-owner) and [Optional token costs](../fees/overview.md#optional-token-costs), the deep dive
- [Action Fees (actionFees)](action-fees.md), fixed credit fees on the same six actions
- [Creation, Transfers and Trading](ownership-and-trading.md), for the actions a type allows
- [Contract-Level Keys and config](contract-config.md), for the contract's `tokens`
- [Contract Keywords overview](../contract-keywords.md), for the conventions of these tables
