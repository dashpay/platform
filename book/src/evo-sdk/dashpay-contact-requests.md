# DashPay Contact Requests

DashPay contact requests are `contactRequest` documents in the DashPay data
contract. The document links the sender identity to `toUserId` and carries the
sender's DIP-15 contact payment public key encrypted for the recipient.

The Evo SDK exposes the DIP-15 derivation primitive through
`wallet.deriveDashpayContactKey`. Applications still need to encrypt the
derived contact xpub before submitting the document.

## Document fields

The `contactRequest` document requires these fields:

```typescript
type DashpayContactRequestDocument = {
  $createdAt: number;
  $createdAtCoreBlockHeight: number;
  toUserId: Uint8Array;
  encryptedPublicKey: Uint8Array;
  senderKeyIndex: number;
  recipientKeyIndex: number;
  accountReference: number;
};
```

The current DashPay contract schema requires the system field
`$createdAtCoreBlockHeight`. Older external references may use
`coreHeightCreatedAt`; do not submit that name to the current contract.

`encryptedPublicKey` is exactly 96 bytes:

- 16 bytes: AES-CBC initialization vector
- 80 bytes: AES-CBC ciphertext for the sender's 78-byte serialized contact xpub

The sender derives the contact xpub from the sender identity, recipient
identity, account, and address index. The sender then encrypts that xpub with an
ECDH shared secret from the sender's identity encryption private key and the
recipient's identity decryption public key.

## Derive the contact payment xpub

```typescript
import { wallet } from '@dashevo/evo-sdk';

const senderKeyIndex = 0;
const recipientKeyIndex = 0;
const addressIndex = 0;

const contactKey = await wallet.deriveDashpayContactKey({
  mnemonic: senderMnemonic,
  network: 'testnet',
  senderIdentityId,
  receiverIdentityId: recipientIdentityId,
  account: 0,
  addressIndex,
});

// contactKey.xpub is encrypted into contactRequest.encryptedPublicKey.
```

`senderKeyIndex` and `recipientKeyIndex` identify the identity public keys used
for ECDH. `addressIndex` is the DIP-15 child index used for contact payment key
derivation and is independent from those identity key indexes.

## Build the encrypted public key

The following helper shows the byte-level encryption shape. It uses the same
secp256k1 primitives exposed by `@dashevo/dashcore-lib` and Node.js `crypto`
for AES-CBC.

```typescript
import crypto from 'node:crypto';
import dashcore from '@dashevo/dashcore-lib';

function fixed32(value): Buffer {
  return Buffer.from(value.toArray('be', 32));
}

function deriveSharedKey({
  privateKeyWif,
  publicKeyBytes,
}: {
  privateKeyWif: string;
  publicKeyBytes: Uint8Array;
}): Buffer {
  const privateKey = dashcore.PrivateKey.fromWIF(privateKeyWif);
  const publicKey = dashcore.PublicKey.fromBuffer(Buffer.from(publicKeyBytes));
  const sharedPoint = publicKey.point.mul(privateKey.toBigNumber());

  if (sharedPoint.isInfinity()) {
    throw new Error('ECDH shared point is invalid');
  }

  const x = fixed32(sharedPoint.getX());
  const y = fixed32(sharedPoint.getY());
  const compressedPrefix = Buffer.from([2 | (y[31] & 1)]);

  return crypto.createHash('sha256').update(Buffer.concat([compressedPrefix, x])).digest();
}

function serializedXpubPayload(xpub: string): Buffer {
  const payload = dashcore.encoding.Base58Check.decode(xpub);

  if (payload.length !== 78) {
    throw new Error(`Invalid DashPay contact xpub length: ${payload.length}`);
  }

  return payload;
}

function encryptContactXpub({
  contactXpub,
  senderEncryptionPrivateKeyWif,
  recipientDecryptionPublicKeyBytes,
}: {
  contactXpub: string;
  senderEncryptionPrivateKeyWif: string;
  recipientDecryptionPublicKeyBytes: Uint8Array;
}): Uint8Array {
  const aesKey = deriveSharedKey({
    privateKeyWif: senderEncryptionPrivateKeyWif,
    publicKeyBytes: recipientDecryptionPublicKeyBytes,
  });
  const payload = serializedXpubPayload(contactXpub);
  const iv = crypto.randomBytes(16);
  const cipher = crypto.createCipheriv('aes-256-cbc', aesKey, iv);
  const encrypted = Buffer.concat([
    cipher.update(payload),
    cipher.final(),
  ]);

  const encryptedPublicKey = Buffer.concat([iv, encrypted]);

  if (encryptedPublicKey.length !== 96) {
    throw new Error(`DashPay encryptedPublicKey must be 96 bytes, got ${encryptedPublicKey.length}`);
  }

  return encryptedPublicKey;
}
```

## Submit the document

```typescript
const encryptedPublicKey = encryptContactXpub({
  contactXpub: contactKey.xpub,
  senderEncryptionPrivateKeyWif,
  recipientDecryptionPublicKeyBytes,
});

const accountReference = 0;

const document = {
  $createdAt: Date.now(),
  $createdAtCoreBlockHeight: platformCoreHeight,
  toUserId: recipientIdentityIdBytes,
  encryptedPublicKey,
  senderKeyIndex,
  recipientKeyIndex,
  accountReference,
};
```

`accountReference` above is a placeholder for the current Platform field
accepted by the `contactRequest` schema. It is not a complete implementation of
any ASK/HMAC-based account-reference obfuscation described in older DIP text.

When querying received requests through the JavaScript SDK, pass identity IDs in
the representation expected by the SDK call being used. The contract stores
`toUserId` as a 32-byte identifier, while some high-level JavaScript query
helpers accept the base58 identity string and perform the conversion
internally.

## Current SDK boundary

`wallet.deriveDashpayContactKey` handles DIP-15 path derivation. It does not
currently submit DashPay documents or encrypt/decrypt `encryptedPublicKey`.
Applications need to combine the wallet helper with identity encryption keys
until a higher-level DashPay contact request helper is added to the JavaScript
SDK.

Treat the example as a byte-level reference. A production application should add
contract validation, decrypt round-trip tests, and checks that the selected
identity keys are active secp256k1 keys bounded for DashPay contact requests.
