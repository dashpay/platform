# Authentication keys with a budget or an expiry

Protocol version 14 lets an identity register an AUTHENTICATION key that can only spend so much,
only sign until a given time, or both. The typical use is a key handed to an application: the
identity decides up front how far the application can go, and nothing has to be revoked for the
key to stop working. Both limits combine freely with `contractBounds`
(see [contract-bound authentication keys](contract-bound-authentication-keys.md)): bounds say
where a key may act, limits say how much and for how long.

## The version 1 public key

Limits live on a new version of the public key, `IdentityPublicKey::V1`
(`$formatVersion: "1"`), which is the version 0 key followed by two optional fields:

| Field | Type | Meaning |
|---|---|---|
| `totalBudget` | credits, optional | The total credits that state transitions signed with this key may take from the identity over its lifetime. It never changes; what is left of it is tracked by Platform, not in the key. |
| `expiresAt` | milliseconds, optional | The block time from which the key can no longer sign. Same unit and clock as `disabledAt`. |

`IdentityPublicKeyInCreation::V1` mirrors it, and both fields are part of the signable bytes of
the identity create or identity update that registers the key: the identity signs the limits it
grants. They cannot be changed afterwards. To give an application more, register a new key.

Version 0 keys are untouched. Every key already in state keeps decoding as it did, and a key
without limits keeps being written as version 0, so clients only meet the new version on
identities that use it. A client that does not know version 1 cannot decode such a key, and
therefore not the identity that holds it.

A state transition that carries a version 1 key in creation is active from protocol version 14.
Earlier protocol versions refuse it while decoding, the way a binary that predates the format
does.

## Registering a limited key

- The key has AUTHENTICATION purpose and a security level below MASTER. Anything else fails with
  `IdentityPublicKeyLimitsNotAllowedError` (10536). The master key is what replaces a spent or
  expired key, so it must not run out itself.
- A `totalBudget`, when present, is not zero (`InvalidIdentityPublicKeyBudgetError`, 10537).
- An `expiresAt`, when present, is after the time of the block that registers the key
  (`IdentityPublicKeyAlreadyExpiredError`, 40219, a paid failure). A key that is dead on arrival
  would still use up its id and, for a unique key type, its public key hash. Passing seconds
  instead of milliseconds is the usual way to get here.

Registration goes through the normal identity create or identity update flow. Whoever adds a
budgeted key pays for the entry that tracks what is left of its budget.

The one exception is an identity created from the shielded pool. That transition has no identity
signature: its keys are bound into the Orchard sighash field by field, in a layout that predates
the version 1 key, so a budget or an expiry would not be covered and could be altered in transit
when no key of the transition carries a proof of possession (every key is a hash based type). A
key that carries a budget or an expiry is refused there
(`IdentityPublicKeyLimitsNotAllowedInShieldedIdentityCreationError`, 10538), by consensus and by
the transition builder; add it with an identity update once the identity exists. A version 1 key
without limits is accepted: everything it holds is covered.

## What counts against a budget

Everything a state transition signed with the key takes from the identity:

- the fee, net of the storage refunds the same transition returns to the identity;
- credits the transition moves out of the identity, such as a document purchase price or the
  prefunded voting balance of a contested document.

A failed state transition that is still paid for (a penalty and a nonce bump) spends from the
budget like a successful one. Storage refunds never add to a budget: it only goes down.

What is left is kept by Platform next to the key, not in the key, and starts at the full budget.

## When a budgeted key is refused

Before a state transition runs, everything it requires from the budget must fit in what is left:

```
required = credits moved out of the identity
         + storage fee
         + fees priced up front (for example a data contract registration)
         + the part of the processing fee added by the user fee increase
```

If `required` is more than what is left, the transition is refused with
`IdentityPublicKeyBudgetExceededError` (40218).

The one thing that may take a key over its budget is the metered processing fee, the cost of
the work validators actually did. It is only known once the transition has run, and it is
bounded by what one transition can do. When it does not fit, the identity still pays it in
full, and what is left of the budget becomes zero.

A key with nothing left can no longer sign: `PublicKeyBudgetExhaustedError` (20015), raised
during signature validation before any other work is done.

This is the rule identity balances already follow, where the storage fee must be covered and the
processing fee may leave a debt, applied to the key instead of the identity.

## When an expired key is refused

A key is expired from `expiresAt` on: it signs at `expiresAt - 1` and not at `expiresAt`. The
time is the time of the block the transition is executed in; mempool admission uses the last
committed block time. An expired key fails with `PublicKeyExpiredError` (20016).

## Refusals are not charged

All three refusals at signing time (20015, 20016, 40218) leave the state transition unpaid, like
an identity that cannot afford its fee: the transition is rejected from the mempool, a proposer
drops it from the block, and neither the identity balance, the key budget nor the nonce changes.
A key that may not spend is not used to charge a penalty either, so an invalid transition signed
by such a key is dropped rather than recorded as a paid failure.

Disabling works as before and is independent of the limits: a master-key identity update can
disable a limited key at any time.

## Reading what is left

`getIdentityKeysRemainingBudgets` takes an identity id and a list of key ids (at least one, none
repeated, at most `max_returned_elements`) and answers every one of them: the remaining credits
for a key with a budget (zero means spent), nothing for a key without a budget or a key that does
not exist. With `prove` the answer is a proof over the identity's key budgets subtree, which also
proves the absence of an entry and the absence of the subtree itself. The number is the state of
the last committed block, so it is an upper bound on what the next transition can spend.

In the SDKs: `IdentityKeysRemainingBudgets::fetch` (Rust), `getIdentityKeysRemainingBudgets`
(wasm-sdk), `sdk.identities.keysRemainingBudgets` (js-evo-sdk).

## Raising limits

`IdentityKeyLimitsUpdate` (state transition type 23, protocol version 14) raises the limits of one
key of the identity. It carries the key id, the new `totalBudget` and the new `expiresAt` (each
optional, at least one given, each the new absolute value), the identity's next `revision` and
an identity nonce. An update only ever loosens a key: the new total must be greater than the
current one, the new expiry later than the current one, and a limit the key does not have cannot
be added. The remaining budget grows by the amount the total grew. After the update the key must
not be expired, so an expired key can be revived by an extension but not topped up while it stays
expired.

It is signed by a MASTER key, or by a CRITICAL authentication key that carries no limits itself;
a key with limits can never raise limits, its own included (`PublicKeyWithLimitsCannotUpdateKeyLimitsError`,
20017, unpaid). The structural refusals are unpaid as well: nothing to change
(`IdentityKeyLimitsUpdateEmptyError`, 10539) and a zero budget (10537). Every other refusal is
paid for by bumping the identity nonce: a stale revision (40203), a missing (40209) or disabled
(40208) key, a limit the key does not have (`IdentityPublicKeyLimitNotSetError`, 40220), a value
that does not raise it (`IdentityPublicKeyLimitNotRaisedError`, 40221), and a key that would stay
expired (40219).

The proof is the identity's keys with its revision; the key must hold exactly the values the
transition asked for. It authenticates the resulting state rather than this exact transition (the
nonce is not stored), so SDKs wait for it as affected state. In the SDKs: `Identity::update_key_limits`, `top_up_key_budget`,
`extend_key_expiry` (Rust), `identityUpdateKeyLimits` (wasm-sdk), `sdk.identities.updateKeyLimits`
(js-evo-sdk).

## Not included

- Lowering or removing the limits of an existing key: disable it instead.
- SDK helpers for creating limited keys and for choosing a usable key when signing. These follow
  separately.
