# State Transitions

State transitions are the write operations of Dash Platform. Unlike queries
(which are free and instant), state transitions modify on-chain state, cost
credits, and must be signed with a private key.

> **API reference**: For the full list of state transition methods with
> parameters and examples, see the
> [State Transitions section](https://dashpay.github.io/evo-sdk-website/docs.html)
> of the Evo SDK Docs.

## How state transitions work

1. **Build** — The SDK constructs the transition (e.g., "create identity",
   "register name") from the parameters you provide.
2. **Sign** — You provide a private key (WIF format) and the SDK signs the
   transition.
3. **Broadcast** — The signed transition is sent to a DAPI node, which
   propagates it to the Platform chain.
4. **Wait** — The SDK waits for the transition to be included in a block and
   returns the result.

## Identity operations

### Create an identity

```typescript
// Generate keys for the new identity
const keyPair = await wallet.generateKeyPair('testnet');

const identity = await sdk.identities.create({
  privateKeyWif: fundingKeyWif,     // key with Dash balance for the asset lock
  identityPublicKeys: [{
    type: 0,                         // ECDSA_SECP256K1
    purpose: 0,                      // AUTHENTICATION
    securityLevel: 0,                // MASTER
    publicKeyHex: keyPair.publicKeyHex,
  }],
});
```

### Top up an identity

```typescript
await sdk.identities.topUp({
  identityId: 'BxPVr5...',
  amount: 100000,                   // credits (1 credit = 1000 duffs)
  privateKeyWif: fundingKeyWif,
});
```

### Transfer credits between identities

```typescript
await sdk.identities.creditTransfer({
  identityId: senderIdentityId,
  recipientId: recipientIdentityId,
  amount: 50000,
  privateKeyWif: senderAuthKeyWif,
  signingKeyIndex: 0,
  nonce: await sdk.identities.nonce(senderIdentityId),
});
```

## Document operations

### Create a document

```typescript
await sdk.documents.create({
  contractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
  documentType: 'domain',
  document: {
    label: 'my-username',
    normalizedLabel: 'my-username',
    normalizedParentDomainName: 'dash',
    records: { identity: identityId },
    subdomainRules: { allowSubdomains: false },
  },
  identityId,
  privateKeyWif: authKeyWif,
  signingKeyIndex: 0,
  nonce: await sdk.identities.contractNonce(identityId, dpnsContractId),
});
```

### Replace, delete, transfer

The `sdk.documents` facade also provides `replace()`, `delete()`,
`transfer()`, `purchase()`, and `setPrice()` methods. See the
[API reference](https://dashpay.github.io/evo-sdk-website/docs.html) for
parameters.

## Token operations

```typescript
// Mint tokens (requires minting authority)
await sdk.tokens.mint({
  tokenId: '...',
  amount: 1000,
  recipientId: '...',
  identityId: minterIdentityId,
  privateKeyWif: minterKeyWif,
  signingKeyIndex: 0,
  nonce: await sdk.identities.nonce(minterIdentityId),
});

// Transfer tokens
await sdk.tokens.transfer({
  tokenId: '...',
  amount: 100,
  recipientId: '...',
  identityId: senderIdentityId,
  privateKeyWif: senderKeyWif,
  signingKeyIndex: 0,
  nonce: await sdk.identities.nonce(senderIdentityId),
});
```

## DPNS name registration

```typescript
await sdk.dpns.register({
  name: 'alice',
  identityId,
  privateKeyWif: authKeyWif,
  signingKeyIndex: 0,
  nonce: await sdk.identities.contractNonce(identityId, dpnsContractId),
});
```

## Waiting for results

The `sdk.stateTransitions` facade provides low-level control:

```typescript
// Wait by hash for a transition you broadcast elsewhere.
// The returned status is an unverified node report, not proof of execution.
const result = await sdk.stateTransitions.waitForStateTransitionResult(stateTransitionHash);

// Strict: resolves only when the proof binds this transition's execution
const proved = await sdk.stateTransitions.waitForResponse(stateTransition);

// Snapshot: also accepts proofs that only authenticate the affected state
const snapshot = await sdk.stateTransitions.waitForAffectedState(stateTransition);
```

`waitForResponse` and `broadcastAndWait` are strict: they resolve only when
the proof shows this specific transition executed, and reject with an
execution-not-proved error for the families whose proofs cannot show that
(balance top-ups, credit transfers and withdrawals, address funds movements,
shields, no-history token operations). `waitForAffectedState` and
`broadcastAndWaitForAffectedState` accept those proofs too and return a
verified, height-pinned snapshot of the keys the transition affects, not
evidence that it executed. Both pairs return the same result type; the
method you call is what fixes the guarantee you hold.
`waitForStateTransitionResult` verifies nothing: it returns the node's
status string for the hash and counts any proof in the response as success
without checking it. For what each kind of result proves, and how
contract-call receipts fit in, see
[Results, Receipts and Proofs](../sdk/results-receipts-and-proofs.md).

## Nonces

Every state transition requires a **nonce** to prevent replay attacks. There
are two types:

- **Identity nonce** — incremented per identity for identity-level transitions
  (top-ups, credit transfers, token operations)
- **Contract nonce** — incremented per identity-contract pair for document and
  contract transitions

```typescript
const identityNonce = await sdk.identities.nonce(identityId);
const contractNonce = await sdk.identities.contractNonce(identityId, contractId);
```

Always fetch the nonce immediately before broadcasting. If another transition
lands between your fetch and broadcast, the nonce will be stale and the
transition will be rejected.
