# encryptedFor

`encryptedFor` marks a byte array property as ciphertext that one identity can read, and writes the recipe into the contract: who the message is for, which identity keys were used, and which encryption scheme made the bytes. Wallets and SDKs read the recipe from the contract instead of from per-app documentation. Reach for it when a document carries a private message, a private note to self, or any other value only its recipient should read. Consensus cannot see inside the ciphertext: it checks only that the bytes have the length the scheme produces.

| | |
|---|---|
| **Where** | A byte array property (`byteArray: true`) that is not an identifier, at the top level or inside an object. Not on the elements of a typed array |
| **Value** | An object with exactly four keys, all required: `recipient`, `recipientKey`, `senderKey`, `scheme` (below) |
| **Default** | Absent: the property is plain bytes |
| **Since** | protocol version 14 |
| **On update** | Fixed: adding, removing or changing it is refused (`IncompatibleDocumentTypeSchemaError`, 10246). Documents already written could not be read under another recipe |
| **Errors** | `InvalidEncryptedPropertyShapeError` (10420) on a document; at registration `JsonSchemaError` (10101) or `InvalidContractStructure` (10231) |

The four keys:

| Key | Value |
|---|---|
| `recipient` | The dotted path of an identifier property of the same document type, whose value is the recipient identity's id; or `"$ownerId"` for a message the writer encrypts to themself |
| `recipientKey` | The dotted path of an integer property of the same document type that carries the id of the recipient's identity key. Its schema must declare `minimum` of at least 0 and `maximum` of at most 4294967295 |
| `senderKey` | The same for the sender's identity key, a key of the document's owner (`$ownerId`) |
| `scheme` | `"ecdh-secp256k1-aes256-cbc"`, the only scheme today |

The three paths are 1 to 256 characters each.

## Example

```json
"directMessage": {
  "type": "object",
  "documentsMutable": false,
  "properties": {
    "recipientId": {
      "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
      "contentMediaType": "application/x.dash.dpp.identifier",
      "position": 0
    },
    "recipientKeyId": { "type": "integer", "minimum": 0, "maximum": 4294967295, "position": 1 },
    "senderKeyId": { "type": "integer", "minimum": 0, "maximum": 4294967295, "position": 2 },
    "encryptedMessage": {
      "type": "array", "byteArray": true, "minItems": 32, "maxItems": 1040,
      "encryptedFor": {
        "recipient": "recipientId",
        "recipientKey": "recipientKeyId",
        "senderKey": "senderKeyId",
        "scheme": "ecdh-secp256k1-aes256-cbc"
      },
      "position": 3
    }
  },
  "required": ["recipientId", "recipientKeyId", "senderKeyId", "encryptedMessage"],
  "additionalProperties": false
}
```

A message is written by its owner to the identity in `recipientId`. The owner used their key `senderKeyId` and the recipient's key `recipientKeyId`, and the ciphertext is 32 to 1040 bytes, room for a plaintext of up to 1023 bytes.

## The scheme

`ecdh-secp256k1-aes256-cbc` is the scheme the DashPay contact request already uses for its encrypted fields:

1. The shared key is the secp256k1 ECDH of the sender's private key and the recipient's public key: the SHA-256 of the product point's parity byte and x coordinate. The recipient derives the same 32 bytes from their own private key and the sender's public key.
2. The writer draws a random 16-byte IV.
3. The stored value is the IV followed by the plaintext encrypted with AES-256-CBC under the shared key and that IV, with PKCS7 padding.

A ciphertext is therefore `16 + 16 * ceil((plaintext length + 1) / 16)` bytes: at least 32, and always a multiple of 16. There is no authentication tag, so a reader with the wrong key usually fails the padding check, but about once in 256 attempts gets garbage instead. Readers should treat a value that does not decrypt as a bad message, not as a protocol error.

The SDKs do this from the declaration. The Rust SDK's `dash_sdk::platform::encrypted_for` module has `encrypt_property` (which also fills in both key id properties) and `decrypt_property`; the JavaScript SDK has `sdk.encryptedFor.encrypt`, `decrypt` and `envelope`.

## How it works

- **Create and replace.** After the JSON schema validation, each property that declares `encryptedFor` and is present in the transition is checked for its shape: at least 32 bytes (the IV and one block) and a multiple of 16. A value that is not refuses the transition with `InvalidEncryptedPropertyShapeError` (10420), which names the property, the scheme and the lengths. The schema's own `minItems` and `maxItems` are checked first, so a value outside them gets the schema's error instead.
- **Nothing else is checkable on chain.** Consensus does not know whether the bytes decrypt, whether the key ids exist on the identities, or whether those keys have an encryption purpose. A writer can store any 32 bytes.
- **Checking the keys.** To have consensus check that the keys exist and are of the right kind, add [references](refers-to.md) of type `identityPublicKey` next to the declaration. The moderation charters system contract does this: the recipient's key must be a decryption key and the sender's an encryption key.

```json
"recipientId": {
  "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
  "contentMediaType": "application/x.dash.dpp.identifier",
  "refersTo": {
    "type": "identityPublicKey",
    "keyIdProperty": "recipientKeyId",
    "keyRequirements": { "purpose": "decryption" }
  },
  "position": 0
},
"senderKeyId": {
  "type": "integer", "minimum": 0, "maximum": 4294967295,
  "refersTo": {
    "type": "identityPublicKey",
    "identityProperty": "$ownerId",
    "keyRequirements": { "purpose": "encryption" }
  },
  "position": 2
}
```

`encryptedFor` neither requires nor duplicates these references: it describes the recipe, and the references hold the keys to it.

## Rules at registration

- The keyword is allowed only on a byte array that is not an identifier. On an identifier or any other property the meta-schema refuses it (`JsonSchemaError`, 10101), and so it does an unknown key, a missing key or another `scheme`.
- `recipient` must name an identifier property of the document type (an identifier that carries `refersTo` counts) or be `"$ownerId"`. No other `$` name is accepted.
- `recipientKey` and `senderKey` must name integer properties of the document type whose schemas declare `minimum` of at least 0 and `maximum` of at most 4294967295, the range of a key id. System properties are refused.
- None of the three named properties may be `transient` or sit inside a transient object: a transient value is never stored, so a stored ciphertext would lose its recipe.
- The byte array's `maxItems` must be at least 32, the shortest ciphertext the scheme produces.

A registration refusal from the parser is `InvalidContractStructure` (10231).

## See also

- [Encrypted Properties](../data-model/documents.md#encrypted-properties-encryptedfor), the deep dive, with [the scheme's layout](../data-model/documents.md#the-ecdh-secp256k1-aes256-cbc-layout) and [what consensus checks, and what it cannot](../data-model/documents.md#what-consensus-checks-and-what-it-cannot)
- [References (refersTo)](refers-to.md), for `identityPublicKey` references
- [Signing and Keys](signing-keys.md), for encryption and decryption keys bound to a document type
- [transient](transient.md)
- [Contract Keywords overview](../contract-keywords.md), for the conventions of these tables
