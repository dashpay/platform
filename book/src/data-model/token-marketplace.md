# Token Marketplace

From protocol version 15 a token can be tradeable: its holders can sell it to each other on
Platform at a price they set, and anyone can make an offer for it. Nobody has to trust the other
side. The tokens a seller lists and the credits a buyer offers are held in escrow until the order
is filled or cancelled, so a fill always delivers what the order promised.

This is a resale market between holders. It is separate from direct purchase
(`TokenSetPriceForDirectPurchase` and `TokenDirectPurchase`), where the token's issuer sets a
price and each purchase mints new tokens and pays the contract owner.

## Making a token tradeable

A token opts in through its `marketplaceRules`:

```json
"marketplaceRules": {
  "$formatVersion": "0",
  "tradeMode": "Tradeable",
  "tradeModeChangeRules": {
    "$formatVersion": "0",
    "authorizedToMakeChange": { "$type": "contractOwner" },
    "adminActionTakers": { "$type": "noOne" },
    "changingAuthorizedActionTakersToNoOneAllowed": true,
    "changingAdminActionTakersToNoOneAllowed": false,
    "selfChangingAdminActionTakersAllowed": false
  }
}
```

| `tradeMode` | Meaning |
|---|---|
| `NotTradeable` (the default) | Holders cannot list the token or make offers for it on Platform |
| `Tradeable` | Holders can list the token and make offers for it |

The mode is set when the token is created, or later by a `TokenConfigUpdate` with the
`MarketplaceTradeMode` item, which `tradeModeChangeRules` governs like every other configuration
item. A token whose `tradeModeChangeRules` let no one make the change keeps the mode it was
created with. That includes every token created with the default rules, so a token that was not
created tradeable becomes tradeable only if its issuer kept the right to change it.

## Rules

| Rule | Refused with | Where |
|---|---|---|
| `Tradeable` before protocol version 15 | `StateTransitionNotActiveError` (10603), as `TokenTradeModeTradeable` on a contract and `TokenConfigUpdateMarketplaceTradeModeTradeable` on an update | Contract create and update, and the batch carrying the update, before anything is charged. Software older than protocol version 15 cannot decode the mode, so this is the refusal it gives too |
| `Tradeable` on a [non-transferable token](non-transferable-tokens.md) | `NonTransferableTokenTradeableError` (10281) | Contract create and update before anything is charged, and the state validation of a `TokenConfigUpdate`, which charges the signer |

A non-transferable token cannot be tradeable because filling a listing or an offer moves the token
from the seller to the buyer, which is the transfer the token refuses. `transferable` is fixed
when the token is created, so the trade mode is the only half of the rule an update can change.
