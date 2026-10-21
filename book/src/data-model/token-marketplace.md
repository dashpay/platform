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


## Listings

Three token transitions of a batch work with listings. Each is refused before protocol version
15, unpaid, by the batch's `is_allowed` gate (`StateTransitionNotActiveError` 10603, as
`TokenPlaceAsk`, `TokenFillAsk` or `TokenCancelOrder`).

| Transition | Fields | Effect |
|---|---|---|
| `TokenPlaceAsk` | `amount`, `price`, `minFill`, optional `expiresAt` | Moves `amount` base units from the seller's balance into escrow and records an `ask` |
| `TokenFillAsk` | `orderId`, `minAmount`, `maxAmount`, `maxTotalPrice` | Moves part or all of the listing from escrow to the buyer, and its cost from the buyer's credits to the seller's |
| `TokenCancelOrder` | `orderId`, `side` | Returns what an order holds to its owner and deletes the order |

A listing's id follows from the token, the seller and the transition's identity contract nonce
(`TokenOrderSide::order_id`), so a client knows it before the listing is placed.

### Price and partial fills

`price` is in credits per whole token, 10 to the power of the token's decimals base units. A fill
takes as much as it may: everything open, up to `maxAmount`. A listing never keeps less than
`minFill` open, so a fill that would leave less takes less, leaving exactly `minFill`; a fill
that does not take everything open takes at least `minFill`. A `minFill` equal to `amount` makes
the listing all-or-nothing. The fill is refused when the amount it would take is under
`minAmount`, or when it costs more than `maxTotalPrice`.

Each fill pays the change in the listing's running total rounded up, so splitting a purchase
never changes what it costs in all:

```text
2 decimals, 333 credits per token (3.33 per base unit), a listing of 3 base units
one fill of 3:          10 credits
three fills of 1:  4 + 3 + 3 = 10 credits
```

The rounding favours the seller and is never more than one credit per fill.

### Rules

| Rule | Refused with |
|---|---|
| `amount` and `price` from 1 to `i64::MAX`, `minFill` from 1 to `amount` | `InvalidTokenAmountError` (10458), `InvalidTokenOrderPriceError` (10462), `InvalidTokenOrderFillRangeError` (10463) |
| A fill's `minAmount` from 1 to `maxAmount`, `maxAmount` up to `i64::MAX` | `InvalidTokenOrderFillRangeError` (10463) |
| The token is tradeable (to place or fill) | `TokenNotTradeableError` (40750) |
| The order is open, for this token | `TokenOrderNotFoundError` (40751) |
| A listing's `expiresAt` is after the block, and a fill comes before it | `TokenOrderExpiredError` (40752) |
| The fill range meets the listing | `TokenOrderFillAmountError` (40753) |
| The fill costs at most `maxTotalPrice` | `TokenOrderPriceLimitError` (40754) |
| Only the order's owner cancels it | `TokenOrderNotOwnedError` (40755) |
| A seller does not fill their own listing | `TokenOrderSelfFillError` (40756) |
| A listing is worth at most `i64::MAX` credits | `TokenOrderValueTooLargeError` (40757) |
| The seller holds `amount` | `IdentityDoesNotHaveEnoughTokenBalanceError` |
| The token is not paused (to place or fill) | `TokenIsPausedError` |
| The seller's account is not frozen (to place or fill), nor the buyer's unless the token allows transfers to frozen balances | `IdentityTokenAccountFrozenError` |

The state rules are paid: a refused transition bumps the signer's nonce and charges the work that
ran. The rules that read nothing beyond the contract and the block (a tradeable token, a listing's
value and its expiry) run with the batch's advanced structure, so check_tx refuses them too. A cancel has no rule beyond ownership, so an order can always be withdrawn, also when the
token is paused, no longer tradeable or the owner's account is frozen. The buyer's credits are
checked with the batch's fees, which count a fill's cost like a direct purchase.

### Storage and proofs

The seller pays the storage of the `ask` and gets it back when the listing closes, by a fill that
takes everything left or by a cancel. A partial fill rewrites the `ask` without changing its size,
so the buyer pays no storage for it beyond their own token balance. Selling a listing charges the
seller nothing.

The proof a node returns for each transition:

| Transition | Proves |
|---|---|
| `TokenPlaceAsk` | The `ask` document, holding the listing's terms |
| `TokenFillAsk` | The buyer's token balance |
| `TokenCancelOrder` | That the order's document is gone |

## Offers

Two more token transitions work with offers, refused before protocol version 15 like the listing
ones (as `TokenPlaceBid` and `TokenFillBid`). A `TokenCancelOrder` with the `bid` side withdraws
an offer.

| Transition | Fields | Effect |
|---|---|---|
| `TokenPlaceBid` | `amount`, `price`, `minFill`, optional `expiresAt` | Moves what the offer is worth from the buyer's credits into escrow and records a `bid` |
| `TokenFillBid` | `orderId`, `minAmount`, `maxAmount`, `minTotalReceived` | Moves part or all of the amount still wanted from the seller to the buyer, and its price from the escrow to the seller |

An offer holds what it is worth, its amount at its price rounded up, and its `escrow` property
says what is left. Each sale is paid the change in the offer's running total rounded down, so the
payments never add up past the escrow; when the offer closes, by a sale that takes everything
still wanted or by a cancel, whatever is left goes back to the buyer:

```text
1.5 credits per base unit, an offer for 7 base units holds 11 credits (10.5 rounded up)
sales of 3, 3 and 1:  4 + 5 + 1 = 10 credits to the sellers, 1 credit back to the buyer
```

The rounding favours the buyer and is never more than one credit per sale. The fill range works
as it does for a listing, and a sale is refused when it would be paid less than
`minTotalReceived` (`TokenOrderPriceLimitError`, 40754).

| Rule | Refused with |
|---|---|
| The token is tradeable (to place or sell into) | `TokenNotTradeableError` (40750), in check_tx too |
| An offer is worth at most `i64::MAX` credits | `TokenOrderValueTooLargeError` (40757), in check_tx too |
| An offer's `expiresAt` is after the block, and a sale comes before it | `TokenOrderExpiredError` (40752) |
| A seller does not sell into their own offer | `TokenOrderSelfFillError` (40756) |
| The seller holds the amount the sale takes | `IdentityDoesNotHaveEnoughTokenBalanceError` |
| The token is not paused (to place or sell into) | `TokenIsPausedError` |
| The seller's account is not frozen, nor the buyer's unless the token allows transfers to frozen balances | `IdentityTokenAccountFrozenError` |

The buyer's credits are checked with the batch's fees, which count the escrow like a purchase. The
buyer pays the storage of the `bid` and gets it back when the offer closes. A `TokenPlaceBid` is
proven by its `bid` document, and a `TokenFillBid` by the seller's token balance.
