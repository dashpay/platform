# App Connect login responses

The App Connect system contract gives wallet-to-app login responses a fixed
home on every network. It activates at protocol version 14, both at genesis
and when an existing chain upgrades from version 13.

- Contract ID: `H8F9mP1BM55TE1ShsxPZHzhyinaMdY9bMmP85mkDhcJJ`
- Owner: the all-zero system identity
- Registry entry: `SystemDataContract::AppConnect = 9`
- Schema version: 1
- Document type: `loginKeyResponse`

## Response schema

A wallet publishes a response under the identity whose keys the user approved
sharing with the app. Ordinary identities may create responses.

| Property | Type | Meaning |
| --- | --- | --- |
| `appEphemeralPubKeyHash` | 20 bytes, required | `hash160` of the ephemeral public key in the app's request; identifies that request |
| `walletEphemeralPubKey` | 33 bytes, required | Compressed wallet ephemeral public key; the app combines it with its own ephemeral private key to derive the shared secret |
| `encryptedPayload` | 60–572 bytes, required | Granted private keys encrypted to the app: a 28-byte envelope plus 32 bytes per key, for 1–17 keys (session key plus up to eight bindings with both purposes) |

Additional properties are rejected. The schema enforces lengths, but does not
parse secp256k1 points, validate an encryption tag, or enforce 32-byte increments
inside the payload range. Those are client protocol checks. The app's contract
ID, requested permissions and any manifest are outside this response schema.

## Storage and lookup

The type is immutable, deletable and `indexOnly`:

```json
{
  "indexOnly": true,
  "documentsMutable": false,
  "canBeDeleted": true,
  "creationRestrictionMode": 0,
  "indices": [{
    "name": "byRequest",
    "terminal": ["appEphemeralPubKeyHash", "$ownerId"]
  }],
  "entryPayload": ["walletEphemeralPubKey", "encryptedPayload"]
}
```

The flat index stores one entry at
`[…, loginKeyResponse, "\0appEphemeralPubKeyHash\0$ownerId", 0, hash ‖ owner]`.
Its value contains the row commitment followed by the framed payload
properties in name order (`encryptedPayload`, then `walletEphemeralPubKey`).
There is no primary document row, document-ID lookup, or tree per request.

The composite key enforces one response per request **per identity**. The
request hash is public, so uniqueness on the hash alone would allow an
observer to block the wallet. Including `$ownerId` lets other identities
respond without reserving the wallet's entry.

Apps query `appEphemeralPubKeyHash == requestHash` to retrieve candidates and
use `$ownerId` as the next ordered component for pagination. Once the identity
is known, equality on both fields selects one response. Query proofs
reconstruct all three properties and the owner from the entry. A synthesized
`$id` is not a primary row identifier or sufficient input for deletion.

## Approval and re-login

1. The app creates a fresh ephemeral key pair and keeps the request context.
2. After user approval, the wallet provisions the granted identity keys and
   waits for them to be confirmed before publishing the response. The wallet
   and app agree on encryption and how the grant binds to the request context.
3. The app checks each candidate's encryption tag and framing, then verifies
   that the granted keys are live on that candidate's `$ownerId`, with the
   approved purposes, bounds, expiry and budget. ECDH to a public ephemeral key
   alone does not authenticate the responding user; pairing and approval must
   establish which identity the app intends to log in.
4. The wallet retains its previous response values per identity and app. On
   re-login it deletes that response using an index-only delete carrying
   **all three original values**, then creates a response for the fresh
   request. An ordinary document replace is not supported. Deletion checks
   the stored commitment, so a changed ciphertext cannot delete the old entry.
5. If local tracking is lost, the wallet can create the fresh response. An old
   entry remains until its owner deletes it with the original values. There is
   no owner-only or per-app secondary index and no automatic cleanup.

Platform treats responses as per-request entries. The wallet manages their
per-app lifecycle and any identity-key revocation separately; deleting a
response does not revoke keys that an app has already received.
