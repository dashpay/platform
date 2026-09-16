# Token Shielded Pools

From protocol version 15 a token can own a shielded pool: an Orchard pool that holds that token
instead of credits. Holders move tokens between their identity balance and the pool with three
token transitions inside a batch, and transfer inside the pool without revealing amounts or
counterparties. This chapter describes the storage, the configuration flag, the transitions, the
validation rules, the block end bookkeeping, the queries and the client builders.

## Why one pool per token

The Orchard construction Platform uses for credits has no asset base: a note carries a value
but not an asset id, and the value balance the circuit proves is a single number. Mixing tokens
in one pool would let a spend of token A create a note of token B. A token therefore gets its
own pool, and each pool is a copy of the credit pool's layout rooted under the token.

## Storage layout

The credit shielded pool lives at `[ShieldedBalances(52)]/"M"`. Token pools live under the
tokens tree:

```text
[Tokens(16)]
  [TOKEN_SHIELDED_POOLS_KEY(224)]            BigSumTree
    [token_id]                               SumTree (the pool)
      NOTES[128]            CommitmentTree, chunk power 11 (the note commitments and ciphertexts)
      NULLIFIERS[64]        ProvableCountTree (spent nullifiers)
      ANCHORS_IN_POOL[192]  anchor -> block height
      TOTAL_BALANCE[32]     SumItem (tokens currently shielded)
      ANCHORS_BY_HEIGHT[96] block height -> anchor (for pruning)
```

The five children use the same keys as the credit pool, so the drive primitives that insert
notes and nullifiers, read balances and record anchors are shared: every credit pool method has
a pool-agnostic form taking the pool path, and a token twin that supplies the token's path. The
root of all token pools is a BigSumTree so the amount of every token that is shielded is one
sum, which the token conservation check reads.

The root tree is created by the version 15 upgrade transition (`transition_to_version_15`) on
an existing chain and by `create_initial_state_structure` version 4 on a new one. A pool's five
trees are created when a contract with the flag is inserted or updated.

## Configuration

`TokenConfiguration` gains a format version 1 whose only addition over version 0 is
`hasShieldedPool: bool`. A version 0 configuration behaves as `hasShieldedPool: false`.
The format version is admitted by
`dpp.contract_versions.token_versions.token_configuration_format`: protocol versions 13 and
below allow only version 0, protocol version 15 allows versions 0 and 1. Contract create and
update reject a token configuration outside the bounds with `UnsupportedVersionError`, so a
pre-14 network never stores the flag.

The flag is immutable. A contract update that changes it is rejected with
`DataContractTokenConfigurationUpdateError` for `hasShieldedPool`, because a pool that was
enabled can hold notes that would become unspendable, and a pool that is enabled late would
need a tree created under an existing token.

A pool also makes freezing and confiscation unenforceable: shielded notes belong to no identity
account, so a holder who expects a freeze shields first and nothing can freeze, destroy or even
see those notes. Rather than let an issuer advertise controls that cover only transparent
balances, a token with `hasShieldedPool` must disable them permanently: `freezeRules`,
`unfreezeRules` and `destroyFrozenFundsRules` must each authorize no one to take the action and
have no admin action takers, so no later configuration update can switch them on. Contract
create and update reject anything else with `TokenShieldedPoolIncompatibleRulesError` (10277).
Pausing still works: shield, unshield and shielded transfer are all rejected while the token is
paused. The frozen-account checks in the shield and unshield validators remain as defence in
depth for state written before this rule.

## Transitions

The three operations are `TokenTransition` variants inside a `Batch` transition, like every
other token operation. The identity signs the batch and pays the fee in credits. Tokens cannot
pay fees, so unlike the credit pool nothing is carved from the bundle's value balance.

| Transition | Flags | Value balance | Extra sighash data | Effect |
|---|---|---|---|---|
| `TokenShield` | outputs only | `-amount` | none | `amount` leaves the owner's balance and enters the pool as new notes. |
| `TokenUnshield` | spends and outputs | `+amount` | `token_id, owner_id, recipient_id, amount` | Notes are spent; `amount` is credited to `recipient_id`; change comes back as new notes. |
| `TokenShieldedTransfer` | spends and outputs | `0` | `token_id, owner_id` | Notes are spent and recreated; the pool balance is unchanged. |

Each transition carries the Orchard bundle (`actions`, `anchor`, `proof`, `binding_signature`)
next to the token base transition (`token_id`, contract id, contract position, identity
contract nonce). The extra sighash data is bound into the Orchard sighash by the client and
recomputed by consensus from the transition's own fields, so a bundle proven for one token,
owner, recipient or amount cannot be replayed with another. The layouts are in
`dpp::shielded::sighash` (`token_unshield_extra_sighash_data`,
`token_shielded_transfer_extra_sighash_data`).

The shield bundle has no spends, so its anchor is not checked against the pool; the client
builds it against the empty tree. Spending bundles must name an anchor the pool has recorded.

## Validation

Structure validation checks the amount bounds, the action count against
`SystemLimits::max_shielded_transition_actions`, the encrypted note sizes, a non-empty proof and
a non-zero anchor.

State validation runs in this order, and the first failure is returned:

1. The token base transition (contract exists, position valid, nonce).
2. `hasShieldedPool` on the token's configuration, else `TokenShieldedPoolNotEnabledError`.
3. Shield: the owner holds `amount`, the owner's account is not frozen, the token is not
   paused. Unshield: the token is not paused, the recipient identity exists, the recipient's
   account is not frozen unless the token allows transfers to frozen balances. Shielded
   transfer: the token is not paused.
4. Spending bundles: the anchor is recorded in the pool (`InvalidAnchorError`), no nullifier
   repeats within the bundle or is already spent (`NullifierAlreadySpentError`), and for an
   unshield the pool holds `amount`.
5. Proof verification. The fee for it, `compute_shielded_verification_fee(actions)`, is added
   as a precalculated operation before the Halo 2 proof and binding signature are checked, so a
   failed proof is a paid failure: the identity is charged, its nonce advances, and nothing
   else moves.

Batch state validation does not run in `CheckTx`. The mempool admission path
(`CheckTxProofVerifier`) verifies the proofs of batch token transitions keyed by the identity
contract nonce, so a proof is verified once per nonce before the block and not again for the
same submission.

The transitions are gated on the protocol version: below 14 `validate_is_allowed` rejects a
batch carrying any of them with `StateTransitionNotActiveError`.

## Execution and conservation

The drive operations are composites of the credit pool primitives re-rooted under the token:

- shield: remove `amount` from the owner's token balance, append the notes, add `amount` to
  the pool's `TOTAL_BALANCE`;
- unshield: insert the nullifiers, append the notes, subtract `amount` from the pool balance,
  add `amount` to the recipient's token balance;
- shielded transfer: insert the nullifiers, append the notes.

The total supply of a token never changes. `calculate_total_tokens_balance` version 1 reads
the token pools BigSumTree and the block end conservation check requires
`identity balances + pool balances == total supply`.

## Block end

Every successful or paid token pool transition records its pool in
`StateTransitionsProcessingResult::token_shielded_pools_touched`. At block end
`record_token_shielded_pool_anchors` (enabled by `DRIVE_ABCI_METHOD_VERSIONS_V10`) records
each touched pool's current anchor if the commitment tree changed and prunes that pool's
anchors older than `shielded_anchor_retention_blocks`, always keeping the newest one. Pruning
is driven by touches rather than by an interval because there can be many pools; an idle pool
keeps a valid anchor to spend against.

## Queries

The six shielded pool queries (`getShieldedPoolState`, `getShieldedNotesCount`,
`getShieldedAnchors`, `getMostRecentShieldedAnchor`, `getShieldedEncryptedNotes`,
`getShieldedNullifiers`) take an optional `token_id`. Without it they target the credit pool;
with a 32-byte token id they target that token's pool and answer with the same response shape.
A token id is rejected with `InvalidArgument` before protocol version 15 or when it is not 32
bytes. The proof verifier routes on the same field to the token twins of the verify functions
(`verify_token_shielded_pool_state` and the rest), and the Rust SDK exposes
`TokenShieldedPoolQuery`, `TokenShieldedEncryptedNotesQuery` and `TokenShieldedNullifiersQuery`.

A token pool transition proves its execution like the token transfer it resembles: a shield
proves the owner's balance, an unshield proves the recipient's balance, and a shielded transfer
proves the spent nullifiers in the token's pool.

## Client builders

`dpp::shielded::builder` provides `build_token_shield_transition`,
`build_token_unshield_transition` and `build_token_shielded_transfer_transition`. They prove
the bundle, bind the extra sighash data, and call the batch constructors
(`new_token_shield_transition` and siblings) which sign with the identity key. The wasm
bindings expose `TokenShieldTransition`, `TokenUnshieldTransition` and
`TokenShieldedTransferTransition`, and `TokenConfiguration` accepts `hasShieldedPool` and
reports `formatVersion`.

## Fees

See [Shielded Transaction Fees](../fees/shielded-fees.md#token-shielded-pool-fees). In short:
the identity pays the metered cost of the writes plus the proof verification fee, exactly like
`ShieldFromIdentity`, for all three transitions.
