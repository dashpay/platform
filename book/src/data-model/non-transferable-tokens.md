# Non-Transferable Tokens

From protocol version 14 a token can be non-transferable: once an identity holds it, no one can
move it to another identity, directly or through a contract. The token can still be minted,
claimed, bought, burned, frozen and paused as its rules allow, and it can still pay for the
documents of its own contract, by being burned. Reach for it when a token is a usage allowance,
points or a reputation score rather than a currency: an app hands it out, users spend it on the
app's own actions, and it never becomes something to sell or give away.

## Configuration

`TokenConfiguration` format version 1 carries `transferable: bool`, `true` when absent. A
version 0 configuration is always transferable, so only a format version 1 configuration can set
it to `false`, and protocol versions before 14 refuse that format at contract create and update.

```json
{
  "$formatVersion": "1",
  "baseSupply": 0,
  "transferable": false,
  ...
}
```

The flag is fixed when the token is created. No `TokenConfigUpdate` item changes it, and a
contract update cannot change an existing token's configuration.

## What a non-transferable token still does

- **Minting.** The minting rules decide who mints and whether they may choose the destination,
  as for any token. This is how an issuer hands the token out; a base supply would sit with the
  contract owner, who cannot send it on either.
- **Distributions and claims.** Perpetual, pre-programmed and once-per-identity distributions mint
  to their recipients.
- **Direct purchase.** A purchase mints the tokens to the buyer and pays the price in credits.
- **Burning, freezing, unfreezing, destroying frozen funds and pausing**, under their own rules.
- **Paying for documents by burning.** A document type of the token's own contract may charge it
  with `effect: 1` ([`tokenCost`](../contract-keywords/token-cost.md#effect-transfer-or-burn)).

## What is refused

| Operation | Refused with | Where |
|---|---|---|
| A `TokenTransfer` of the token | `TokenNotTransferableError` (40726) | Batch advanced structure validation, from the contract the action carries: the mempool refuses it, and a block charges the signer and bumps the nonce |
| A document type of the token's own contract charging it with `effect: 0` (pay the contract owner) | `NonTransferableTokenPaymentMustBurnError` (10280) | Contract create and update, when the document type is parsed |
| A document type of another contract charging it | `TokenNotTransferableError` (40726) | Contract create and update state validation, which already reads the token's contract to check the token exists. Such a cost always pays the contract owner, since another contract's token cannot be burned (10261) |
| `hasShieldedPool: true` | `NonTransferableTokenShieldedPoolError` (10279) | Contract create and update, before anything is charged |

A shielded pool is refused because a note spent inside the pool can be unshielded to any
identity, so shielding and unshielding would move the token between holders.

Every rule is checked when the contract is registered or when the transfer arrives, and the flag
cannot change afterwards, so a document payment never has to look the flag up: a document type
that could pay the owner in a non-transferable token is never registered.

## Why not pause the token

A paused token refuses transfers, but from protocol version 14 a paused token also refuses every
document payment, burns included (`TokenIsPausedError`, 40711). Before protocol version 14 a
paused token could still pay for documents, and since any contract may charge another contract's
token and pay it to its own owner, anyone could move a paused token by registering such a
contract. A non-transferable token closes that path at registration and keeps its document
payments working.
