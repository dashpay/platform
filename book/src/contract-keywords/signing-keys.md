# Signing and Keys

These keywords tie a document type to identity keys. `signatureSecurityLevelRequirement` sets how strong a key must be to sign transitions on documents of the type. `requiresIdentityEncryptionBoundedKey` and `requiresIdentityDecryptionBoundedKey` let identities register encryption and decryption keys bound to the type, for applications that encrypt data between users, and say how many such keys an identity may hold.

Every identity key has a purpose (authentication, encryption, decryption and others) and a security level. The levels are, from strongest to weakest, `MASTER` (0), `CRITICAL` (1), `HIGH` (2) and `MEDIUM` (3): a lower number is a stronger key. See [Identity Keys](../sdk/identity-keys.md#security-level).

## `signatureSecurityLevelRequirement`

The weakest key security level that may sign a transition on documents of the type. Raise it to `1` for documents whose forgery would be costly, so that a weaker everyday key cannot write them.

| | |
|---|---|
| **Where** | document type |
| **Value** | `1` critical, `2` high, `3` medium |
| **Default** | `2` (high) |
| **Since** | protocol version 1 |
| **On update** | Fixed (`DocumentTypeUpdateError`, 40212). Adding or removing the key without changing its value is refused too, as a schema change (`IncompatibleDocumentTypeSchemaError`, 10246). |
| **Errors** | `InvalidSignaturePublicKeySecurityLevelError` (20004) |

### Example

```json
"payout": {
  "type": "object",
  "documentsMutable": false,
  "canBeDeleted": false,
  "signatureSecurityLevelRequirement": 1,
  "properties": {
    "recipient": {
      "type": "array",
      "byteArray": true,
      "minItems": 32,
      "maxItems": 32,
      "contentMediaType": "application/x.dash.dpp.identifier",
      "position": 0
    },
    "amount": { "type": "integer", "minimum": 1, "position": 1 }
  },
  "required": ["recipient", "amount"],
  "additionalProperties": false
}
```

Only a critical key may sign a transition that creates a payout. A high or medium key of the same identity is refused.

### How it works

The requirement admits the level it names and every stronger level except `MASTER`:

| Value | Keys that may sign |
|---|---|
| `1` critical | critical |
| `2` high (the default) | critical, high |
| `3` medium | critical, high, medium |

- It applies to every document transition on the type: create, replace, delete, transfer, price update and purchase. A purchase is signed by the buyer, so the buyer needs a key at that level.
- A batch transition signs all its transitions with one key. When it holds transitions on several document types, the key must satisfy the strictest of their requirements. A batch that also holds a token transition needs a critical key.
- The key must be an authentication key. A master key never signs a document batch, whatever the requirement: the batch is refused at the signature check (20004).
- A key whose level the requirement does not admit is refused with `InvalidSignaturePublicKeySecurityLevelError` (20004) after the signature has been verified. The failure is paid: the identity's nonce for the contract is bumped and the fees are charged.

### Rules at registration

- The meta-schema admits only `1`, `2` and `3` (`JsonSchemaError`, 10101 otherwise). `0`, master, cannot be required.

## `requiresIdentityEncryptionBoundedKey`

Lets identities add encryption keys bound to this document type, and says how they are kept. Use it when documents of the type carry data encrypted to their readers, and each identity publishes the key others encrypt to.

| | |
|---|---|
| **Where** | document type |
| **Value** | `0` unique, `1` multiple, `2` multiple with a pointer to the latest |
| **Default** | absent: no encryption key may be bound to the type |
| **Since** | protocol version 1 |
| **On update** | Fixed (`DocumentTypeUpdateError`, 40212) |
| **Errors** | `DataContractBoundsNotPresentError` (10515) for an encryption key bound to a type that does not declare it; `IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError` (40211) for a second key under `0` |

## `requiresIdentityDecryptionBoundedKey`

The same for decryption keys.

| | |
|---|---|
| **Where** | document type |
| **Value** | `0` unique, `1` multiple, `2` multiple with a pointer to the latest |
| **Default** | absent: no decryption key may be bound to the type |
| **Since** | protocol version 1 |
| **On update** | Fixed (`DocumentTypeUpdateError`, 40212) |
| **Errors** | `DataContractBoundsNotPresentError` (10515) for a decryption key bound to a type that does not declare it; `IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError` (40211) for a second key under `0` |

### Example

Trimmed from the DashPay contract's `contactRequest`:

```json
"contactRequest": {
  "type": "object",
  "documentsMutable": false,
  "canBeDeleted": false,
  "requiresIdentityEncryptionBoundedKey": 2,
  "requiresIdentityDecryptionBoundedKey": 2,
  "properties": {
    "toUserId": {
      "type": "array",
      "byteArray": true,
      "minItems": 32,
      "maxItems": 32,
      "contentMediaType": "application/x.dash.dpp.identifier",
      "position": 0
    },
    "encryptedPublicKey": {
      "type": "array",
      "byteArray": true,
      "minItems": 96,
      "maxItems": 96,
      "position": 1
    },
    "senderKeyIndex": { "type": "integer", "minimum": 0, "position": 2 },
    "recipientKeyIndex": { "type": "integer", "minimum": 0, "position": 3 }
  },
  "required": ["toUserId", "encryptedPublicKey", "senderKeyIndex", "recipientKeyIndex"],
  "additionalProperties": false
}
```

An identity may bind any number of encryption and decryption keys to `contactRequest`, and the platform keeps a pointer to the newest of each. The request records the ids of the sender's and the recipient's keys in `senderKeyIndex` and `recipientKeyIndex`.

### How it works

- An identity key may carry contract bounds: a contract, or one document type of a contract. An encryption or decryption key bound to a document type is only accepted when the type declares the matching keyword; otherwise it is refused with `DataContractBoundsNotPresentError` (10515). The check runs when an identity is created with such a key, or updated to add one. The bound contract and document type must exist (`DataContractNotPresentError`, 10400; `InvalidDocumentTypeError`, 10406).
- The value says how the identity's keys of that purpose, bound to the type, are kept:
  - `0`, unique: the identity holds at most one, and it can never be replaced. A second is refused (`IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError`, 40211).
  - `1`, multiple: the identity may hold any number.
  - `2`, multiple with a pointer to the latest: any number, and the platform keeps a pointer to the most recently added one, so a client reads the current key in one step.
- Clients read keys bound to a contract or a document type with the `getIdentitiesContractKeys` query, which takes the identities, the contract, an optional document type name and the purposes.
- Consensus reads these keywords only when keys are added. Nothing requires the writer of a document to hold such a key, and nothing checks which key encrypted a property. See [encryptedFor](encrypted-for.md) for what consensus does check about encrypted values.
- Encryption and decryption keys are `MEDIUM` keys. See [Identity Keys](../sdk/identity-keys.md#what-security-level-controls).
- Keys bound to the whole contract, rather than one document type, are governed by the contract config keys of the same names. See [Contract-Level Keys and config](contract-config.md).
- Before protocol version 12, a decryption key bound to a document type was checked against `requiresIdentityEncryptionBoundedKey` by mistake. From 12 each purpose reads its own keyword.
- From protocol version 14 an authentication key may also be bound to a contract or a document type, to limit what it may sign. That needs neither keyword. See [Contract Bounds](../sdk/identity-keys.md#contract-bounds).

## See also

- [Identity Keys Deep Dive](../sdk/identity-keys.md), for purposes, security levels and contract bounds
- [encryptedFor](encrypted-for.md), for declaring how an encrypted property was made
- [Creation, Transfers and Trading](ownership-and-trading.md), for the actions a key signs
- [Contract-Level Keys and config](contract-config.md), for the contract-wide key requirements
