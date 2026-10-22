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
| `Tradeable` on a [non-transferable token](non-transferable-tokens.md) | `NonTransferableTokenTradeableError` (10281) | Contract create and update before anything is charged, and the advanced structure validation of a `TokenConfigUpdate`, which charges the signer and runs in check_tx too, so the mempool refuses the update |

A non-transferable token cannot be tradeable because filling a listing or an offer moves the token
from the seller to the buyer, which is the transfer the token refuses. `transferable` is fixed
when the token is created, so the trade mode is the only half of the rule an update can change.

## Where orders and escrow live

Open orders are documents of the token market system contract
(`AE6yYq65VdfGCg2MSyV9vV7cWGiFWjbwBuy7THesDh4e`), registered at genesis from protocol version 15
and inserted by the upgrade to 15. It has two document types, `ask` (a listing) and `bid` (an
offer), each with three indexes:

| Index | Properties | Serves |
|---|---|---|
| `book` | `tokenId`, `price`, `$createdAt` | A token's order book, by price and then by age |
| `depth` | `tokenId`, `price`, summed | The total open at each price, or over a price range, in one proven sum query: the base units of the asks (`remaining`) and the credits of the bids (`escrow`) |
| `byOwner` | `$ownerId`, `tokenId`, `$createdAt` | An identity's open orders |

Each side sums what it holds in escrow, so the sums cannot overflow: the tokens listed never exceed
the token's supply, and the credits offered never exceed the credits on Platform. Every integer of
an order is a signed 8-byte value, so a partial fill rewrites an order without changing its stored
size.

An order's price is in credits per whole token, and each order records the token's decimals when
it was placed (`decimals`). Every fill prices the order at that scale, so a later change to the
token's decimals (a `Conventions` update) changes neither what an open listing costs nor what an
open offer pays. The `book` and `depth` indexes order by price alone: after a token changes its
decimals, orders placed before and after the change are priced at different scales until the older
ones close, and a client comparing them scales each by its own `decimals`.

Only the marketplace transitions write orders. A document transition cannot create one
(`creationRestrictionMode` 2), and a replace or a delete of one is refused by a data trigger. The
types stay mutable and deletable so the documents carry a revision and storage flags: the order's
owner pays its storage and gets it back when the order closes.

What an order holds is kept outside the contract:

- The tokens of a listing sit in `[Tokens, TOKEN_ORDER_ESCROW_KEY]`, a BigSumTree with one SumTree
  per token, keyed by order id. The token conservation check counts it beside the identity
  balances and the shielded pools.
- The credits of an offer sit in `[PreFundedSpecializedBalances, PREFUNDED_BALANCES_FOR_TOKEN_ORDERS]`,
  a SumTree keyed by order id, which the credit conservation check already counts.

