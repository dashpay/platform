# Contract-bound authentication keys

Protocol version 14 lets an identity register an AUTHENTICATION key whose
`contractBounds` name one contract, or one contract and one document type. Before
protocol 14 only ENCRYPTION and DECRYPTION keys could carry bounds. The bounds
variants and their encoding are unchanged; only which keys may carry them and what
they mean for signing changes.

## Registering a bound key

- The key has AUTHENTICATION purpose and a non-MASTER security level (HIGH is the
  usual choice for document operations).
- `contractBounds` is `singleContract { id }` or `documentType { id, documentTypeName }`.
- The contract, and the document type when named, must exist when the key is
  registered. Any contract may be bound; no contract opt-in is required, unlike
  encryption bounds.
- Several bound authentication keys may cover the same contract. Drive keeps one
  current-key pointer per contract (or contract and document type) that names the
  newest registered key.

Registration goes through the normal identity create or identity update flow, signed
by the identity's own keys. Below protocol 14 such a key is rejected as it is today.

## What a bound key may sign

A bound authentication key may sign only Batch transitions, and every member of the
batch must be inside the bounds:

- `singleContract`: document operations of any type on that contract, and token
  operations on tokens defined by that contract.
- `documentType`: document operations of that type on that contract. Token operations
  are contract-wide and are never covered by a document-type bound.

Anything else fails signature validation with `ContractBoundedKeyNonBatchError`
(identity updates, contract writes, credit transfers and withdrawals, votes). A batch
with a member outside the bounds fails as a paid validation failure with
`ContractBoundedKeyOutOfBoundsError`: fees are charged and the first member's
identity contract nonce advances, the same handling as other paid batch failures.

Document security-level requirements, ownership rules and token rules still apply. The
bounds add a restriction; they never grant anything the key's purpose and security
level do not already allow.

## What bounds do not do

There are no per-operation permissions, no spending limits and no expiry. A bound key
can perform every document and token operation on its contract, including document
purchases (credits move to the seller) and token transfers, until the identity disables
it through a master-key identity update. Treat a bound key as full authority over the
bound contract, limited in scope but not in amount.

## Compatibility

No wire format changes. Clients and wallets that already handle the two bounds
variants decode bound authentication keys today. The SDK signing helpers refuse to sign
a transition the bounds do not cover before it reaches the network.
