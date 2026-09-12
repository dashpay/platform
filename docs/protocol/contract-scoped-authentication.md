# Contract-scoped authentication keys

Protocol version 14 adds application authentication scopes. A wallet can register
a separate key for an application while retaining the identity's master key.
Validators enforce the registered scope on every batch member. Existing identity
ownership, key purpose, security level and document rules still apply.

## Registering an application key

The key must have AUTHENTICATION purpose and a non-MASTER security level. A HIGH
key is suitable for normal document operations. Its `contractBounds` is a new
`scoped` variant containing a versioned authentication scope:

- `contracts`: explicit contract IDs with optional document-type restrictions.
- `permissions`: an action bitmask shared by every listed contract.
- `expiresAt`: an optional expiry in milliseconds, checked against block time.

A missing/null document-type restriction authorizes all types in that contract,
including types added by later contract updates. An empty array is invalid.
Contract IDs and document-type names must be sorted and unique on the wire. There
are at most 16 contracts and 16 types per contract, and the encoded scope must not
exceed 2048 bytes. The WASM constructor sorts entries and rejects duplicates.

For an application that creates, updates and deletes documents and pays their
configured token fees, construct the bounds with the WASM SDK:

```javascript
const P = wasm.AuthenticationPermission;
const bounds = wasm.ContractBounds.Scoped(
  [
    { id: socialContractId, documentTypes: ['like', 'post'] },
    { id: profileContractId, documentTypes: ['profile'] },
  ],
  P.DocumentCreate | P.DocumentReplace | P.DocumentDelete | P.DocumentTokenPayment,
  BigInt(Date.now() + 24 * 60 * 60 * 1000),
);

const keyToAdd = new wasm.IdentityPublicKeyInCreation({
  keyId: nextKeyId,
  purpose: 'authentication',
  securityLevel: 'high',
  keyType: 'ecdsa_hash160',
  isReadOnly: false,
  data: applicationPublicKeyHash160,
  signature: new Uint8Array(),
  contractBounds: bounds,
});
```

Use the normal wallet-authorized identity-update procedure to register the key.
The registration signature binds the scope as well as the public-key material.
The browser only needs the application key's private material. Registering a
scope requires its referenced contracts/types to exist and its expiry, if any,
to be in the future. No contract encryption-key opt-in or unique-key setting is
required for scoped authentication.

## Permissions and token fees

Document create, replace, delete, ownership transfer, price updates and purchases
have separate bits. Index-only deletion uses the delete bit. Standalone token
transition kinds also have separate bits. New/unknown bits are rejected.

`DocumentTokenPayment` permits the actual contract-defined token cost of an
otherwise-authorized document action. It also covers fees using a token issued
by another contract. That does not authorize document writes or standalone token
operations on the issuing contract. Without the bit, a document action with a
positive token cost is rejected, even if its create/replace/delete bit is set.

A document-token payment permission does not implicitly authorize token transfer,
burn, mint, purchase or administration transitions. Explicitly granting one of
those bits still cannot override its normal purpose/security/ownership rules.
Contract updates can change document token fees; v0 scopes do not pin the fee
amount or currency.

## Expiry, revocation and failures

A key is expired when executing block time is greater than or equal to its expiry.
Mempool checks use the last committed block information; a transaction may expire
between admission and execution. Disable the key through the normal identity
update to revoke it. Extending expiry or expanding permissions requires a
wallet-authorized replacement. Expired keys are not automatically deleted.

Scoped keys cannot execute non-batch transitions, including identity-key updates,
contract creation/updates, credit transfers/withdrawals or masternode votes.
Expired keys and non-batch use fail in identity-signature authorization.

Batch scope violations follow normal paid validation-failure handling. Requested
document/token operations do not execute, but Platform credit validation fees
can be charged and the first batch member's identity-contract nonce can advance,
even if that member is outside the scope. Replays follow the usual nonce rules.

There are no per-key budgets in scope version 0. A stolen key can exhaust credit
balances through fees and permitted token balances through allowed operations.
Expiry limits the time window, not total financial loss.

## Compatibility

The scoped variant is appended to the existing bounds enum; old key encodings
remain unchanged. Older protocols reject scoped registration, and older clients
cannot be assumed to decode scoped keys. SDK signing performs local structural
checks, but validator checks against current state remain authoritative.

The native key ABI carries an encoded scope pointer/length. Native libraries,
generated headers and Swift/Kotlin consumers must be updated together. Key query,
persistence, restore and refresh paths must preserve scope metadata. It must
never be dropped or reconstructed as an unrestricted key.
