# Token Market Contract

The token market system contract holds the open orders of the token
marketplace: the listings (`ask`) holders make to sell a tradeable token at a
price, and the offers (`bid`) anyone makes to buy one. It activates at protocol
version 15, registered at genesis by chains born at 15 and inserted by the
upgrade to 15, and has the same ID on every network:
`AE6yYq65VdfGCg2MSyV9vV7cWGiFWjbwBuy7THesDh4e`.

Only the token marketplace transitions write these documents. Users cannot
create them (`creationRestrictionMode` 2), and a replace or delete sent as a
document transition is refused. The order's owner is its maker, who pays the
order's storage and gets it back when the order closes. What the order holds
in escrow (the listed tokens of an ask, the offered credits of a bid) is kept
by Platform under the order's id, outside this contract.

## `ask`

Tokens a holder offers for sale at a price.

| Property | Type | Meaning |
| --- | --- | --- |
| `tokenId` | identifier, required | The token for sale |
| `price` | integer, required | The credits the seller asks per whole token (10 to the power of `decimals` base units) |
| `amount` | integer, required | The base units listed when the order was placed |
| `remaining` | integer, required | The base units still for sale; the order is deleted when it is filled entirely |
| `decimals` | integer, required | The token's decimals when the order was placed, generated from `tokenId` (`generatedFrom` `sys.tokens.decimals`) and kept after. Every fill prices the order at this scale, so a later change to the token's decimals does not change what an open order costs or pays |

## `bid`

An offer to buy a token at a price: the same properties as an `ask`, read from
the buyer's side, and `escrow`.

| Property | Type | Meaning |
| --- | --- | --- |
| `escrow` | integer, required | The credits still held in escrow for the order; what is left once it is filled goes back to the buyer |

## Indexes

Both types have the same three:

- `book` (`tokenId`, `decimals`, `price`, `$createdAt`): the order book of a
  token, by price and then by age, for one scale of price. Prices are per whole
  token at each order's own `decimals`, so a query names the token's current
  decimals; after a token changes its decimals, the orders placed before the
  change sit apart, under the old decimals, until they close.
- `depth` (`tokenId`, `decimals`, `price`): keeps a sum per price, and over any
  range of prices at one scale, of what is open: the base units of the asks (`remaining`) and the
  credits of the bids (`escrow`). Each side sums what it holds in escrow, which
  the token's supply bounds for asks and the credits on Platform bound for bids,
  so the sums cannot overflow.
- `byOwner` (`$ownerId`, `tokenId`, `$createdAt`): the open orders of an
  identity.

## Install

```sh
npm install @dashevo/token-market-contract
```

## License

[MIT](LICENSE) © Dash Core Group, Inc.
