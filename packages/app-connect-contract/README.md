# App Connect Contract

The App Connect system contract carries the wallet's encrypted response to an
app login request. It activates at protocol version 14 and has the same ID on
every network: `H8F9mP1BM55TE1ShsxPZHzhyinaMdY9bMmP85mkDhcJJ`.

Its only document type is `loginKeyResponse`, written by the wallet's identity
after the user approves:

| Property | Bytes | Meaning |
| --- | --- | --- |
| `appEphemeralPubKeyHash` | 20 | `hash160` of the ephemeral public key in the app's request |
| `walletEphemeralPubKey` | 33 | Compressed ephemeral public key used to derive the shared secret |
| `encryptedPayload` | 60–572 | Encrypted granted private keys: 28-byte envelope plus 32 bytes per key, for 1–17 keys (session key plus up to eight bindings with both purposes) |

All three properties are required. The contract checks their byte lengths;
clients validate the public key, ciphertext framing and decrypted grant.

The type is `indexOnly`, immutable and deletable. Its `byRequest` index is a
flat composite terminal `(appEphemeralPubKeyHash, $ownerId)`. The wallet key and
ciphertext are `entryPayload` values, so there is no primary document row or
per-request tree. Each identity may answer a request once; another identity's
answer cannot reserve the wallet's entry. Apps query by request hash and
validate each candidate's ciphertext and granted keys against its `$ownerId`.

On re-login the wallet deletes its old response **by values** and creates the
new response. It retains the previous request hash, wallet key and ciphertext
for deletion; retaining only a document ID is insufficient for an index-only
type. See [the protocol guide](../../docs/protocol/app-connect.md) for details.

## Install

```sh
npm install @dashevo/app-connect-contract
```

## License

[MIT](LICENSE) © Dash Core Group, Inc.
