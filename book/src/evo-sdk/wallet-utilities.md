# Wallet Utilities

The Evo SDK exports a standalone `wallet` namespace with offline cryptographic
utilities. These functions do **not** require a connected SDK instance — they
initialise the WASM module on first call and work independently.

```typescript
import { wallet } from '@dashevo/evo-sdk';
```

## Mnemonic management

```typescript
// Generate a new 12-word mnemonic
const mnemonic = await wallet.generateMnemonic();
// "abandon ability able about above absent ..."

// Validate an existing mnemonic
const valid = await wallet.validateMnemonic(mnemonic);

// Convert to seed bytes (with optional passphrase)
const seed = await wallet.mnemonicToSeed(mnemonic, 'optional-passphrase');
```

## Key derivation

### From seed phrase

```typescript
const keyInfo = await wallet.deriveKeyFromSeedPhrase({
  mnemonic,
  network: 'testnet',
  derivationPath: "m/44'/1'/0'/0/0",
});
// keyInfo.privateKeyWif, keyInfo.publicKeyHex, keyInfo.address
```

### From seed with path

```typescript
const seed = await wallet.mnemonicToSeed(mnemonic);
const key = await wallet.deriveKeyFromSeedWithPath({
  seed,
  network: 'testnet',
  path: "m/44'/1'/0'/0/0",
});
```

### Standard derivation paths

The SDK provides helpers for Dash-specific derivation paths:

```typescript
// BIP-44 paths
const bip44 = await wallet.derivationPathBip44Testnet(0, 0, 0);
// "m/44'/1'/0'/0/0"

// DIP-9 Platform paths (identity authentication keys)
const dip9 = await wallet.derivationPathDip9Testnet(0, 0, 0);

// DIP-13 DashPay paths (contact encryption keys)
const dip13 = await wallet.derivationPathDip13Testnet(0);
```

### Extended public key operations

```typescript
// Convert xprv to xpub
const xpub = await wallet.xprvToXpub(xprv);

// Derive child public key
const childPub = await wallet.deriveChildPublicKey(xpub, 0, false);
```

## Key pair generation

```typescript
// Generate a random key pair
const keyPair = await wallet.generateKeyPair('testnet');
// keyPair.privateKeyWif, keyPair.publicKeyHex, keyPair.address

// Generate multiple key pairs
const pairs = await wallet.generateKeyPairs('testnet', 5);

// Import from WIF
const imported = await wallet.keyPairFromWif('cPrivateKeyWif...');

// Import from hex
const fromHex = await wallet.keyPairFromHex('abcd1234...', 'testnet');
```

## Address utilities

```typescript
// Derive address from public key
const address = await wallet.pubkeyToAddress(pubkeyHex, 'testnet');

// Validate an address for a network
const ok = await wallet.validateAddress('yWhatever...', 'testnet');
```

## Message signing

```typescript
const signature = await wallet.signMessage(
  'Hello Dash Platform',
  privateKeyWif,
);
```

## DashPay contact keys

For DashPay encrypted messaging, derive contact-specific keys:

```typescript
const contactKey = await wallet.deriveDashpayContactKey({
  mnemonic,
  network: 'testnet',
  senderIdentityId: '...',
  recipientIdentityId: '...',
  account: 0,
  index: 0,
});
```
