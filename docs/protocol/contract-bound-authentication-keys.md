# Contract-bound authentication keys

Protocol version 14 lets an identity register an AUTHENTICATION key whose
`contractBounds` name one contract, one contract and one document type, or a contract
group. Before protocol 14 only ENCRYPTION and DECRYPTION keys could carry bounds. The
two contract variants and their encoding are unchanged; the `contractGroup` variant is
new and is the only wire change.

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

## Bounds to a contract group

`contractBounds` may be `contractGroup { id }`, naming a contract group (see
`contract-groups.md`). The rules above apply with the group in place of the contract:

- AUTHENTICATION purpose and a non-MASTER security level. ENCRYPTION and DECRYPTION
  keys cannot be bound to a group: their bounds are opt-in per contract through
  `requiresIdentityEncryptionBoundedKey` and `requiresIdentityDecryptionBoundedKey`, and
  a group has no configuration to ask with. Such a key fails with
  `InvalidKeyPurposeForContractBoundsError` naming AUTHENTICATION as the only purpose.
- The group must exist when the key is registered (`ContractGroupNotFoundError`
  otherwise). Any identity may bind a key to any group; the group's owner and admins are
  not consulted, because a bound restricts the holder's own key and grants nothing on
  the group.
- A group-bound key may sign only Batch transitions. A member on contract `C` is inside
  the bounds when `C` is a whole-contract member of the group, when the member's document
  type is a member of the group, or when the member's token is a member of the group.
  Consensus reads `C`'s group memberships once per batch member and bills the read; a
  member outside the group fails as a paid `ContractBoundedKeyOutOfBoundsError`, as for
  a contract bound.
- Memberships are append-only, so what a group-bound key may sign grows whenever the
  group's owner or an admin adds a contract, document type or token. Binding a key to a
  group trusts the group's owner and admins with that growth.
- Drive stores group-bound keys next to contract-bound ones, under the group id, with
  the same current-key pointer per group.
- `IdentityCreateFromShieldedPool` cannot register a group-bound key: its Orchard
  sighash preimage layout predates group bounds, and the transition fails before proof
  verification with `ContractGroupBoundKeyNotAllowedInShieldedIdentityCreationError`.
  Register the key with an identity create, an identity create from addresses or an
  identity update instead.

## What bounds do not do

There are no per-operation permissions. A bound key can perform every document and token
operation on its contract, including document purchases (credits move to the seller) and
token transfers, until the identity disables it through a master-key identity update.
Treat a bound key as full authority over the bound contract, limited in scope but not in
amount.

Bounds themselves carry no spending limit and no expiry. Those are separate, optional
properties of the key and combine with bounds: see
[authentication keys with a budget or an expiry](authentication-key-limits.md). A budget
caps the credits a key can take from the identity; it does not cap token amounts.

## Compatibility

The two contract variants are unchanged, so clients and wallets that handle them decode
contract-bound authentication keys today. `contractGroup` is a third variant with
bincode tag 2 and JSON `$type` `contractGroup`; a client built before protocol 14 cannot
decode a key carrying it, and below protocol 14 a transition carrying it is not active, so
every node rejects it without charging anything. The SDK signing helpers refuse to sign a transition a contract bound does not
cover before it reaches the network, and leave a group bound to consensus, which holds
the memberships.
