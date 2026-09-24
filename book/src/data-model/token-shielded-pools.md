# Token Shielded Pools

From protocol version 14 a token can own a shielded pool: an Orchard pool that holds that token
instead of credits. Holders move tokens between their identity balance and the pool with token
transitions inside a batch, issuers mint, burn, release and sell straight into or out of it,
document costs can be paid out of it, and three identity-less transitions move tokens with the
fee paid from the credit shielded pool, so no identity appears at all. This chapter describes the
storage, the configuration flag, the transitions, the validation rules, the block end
bookkeeping, the queries and the client builders.

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

The root tree is created by the version 14 upgrade transition (`transition_to_version_14`) on
an existing chain and by `create_initial_state_structure` version 4 on a new one. A pool's five
trees are created when a contract with the flag is inserted or updated.

## Configuration

`TokenConfiguration` gains a format version 1 whose only addition over version 0 is
`hasShieldedPool: bool`. A version 0 configuration behaves as `hasShieldedPool: false`.
The format version is admitted by
`dpp.contract_versions.token_versions.token_configuration_format`: protocol versions 13 and
below allow only version 0, protocol version 14 allows versions 0 and 1. Contract create and
update reject a token configuration outside the bounds with `UnsupportedVersionError`, so a
pre-14 network never stores the flag.

The flag is immutable. A contract update that changes it is rejected with
`DataContractTokenConfigurationUpdateError` for `hasShieldedPool`, because a pool that was
enabled can hold notes that would become unspendable, and a pool that is enabled late would
need a tree created under an existing token.

A pool also makes freezing and confiscation unenforceable: shielded notes belong to no identity
account, so a holder who expects a freeze shields first and nothing can freeze or destroy those
notes, because no account holds them for an issuer to act on. The notes themselves are not
hidden — their commitments and ciphertexts are stored and queryable — but who owns one and how
much it carries are. Rather than let an issuer advertise controls that cover only transparent
balances, a token with `hasShieldedPool` must disable them permanently: `freezeRules`,
`unfreezeRules` and `destroyFrozenFundsRules` must each authorize no one to take the action and
have no admin action takers, so no later configuration update can switch them on. Contract
create and update reject anything else with `TokenShieldedPoolIncompatibleRulesError` (10278).
Pausing still works: every pool operation, inflows included (shield, mint, claim and purchase
into the pool) and outflows (unshield, shielded transfer, burn from the pool), is rejected while
the token is paused. A pooled token can never freeze an account, so the pool validators read and bill no
frozen-account check.

## Batch transitions

Seven operations are `TokenTransition` variants inside a `Batch` transition, like every other
token operation. The identity signs the batch and pays the fee in credits. Tokens cannot pay
fees, so unlike the credit pool nothing is carved from the bundle's value balance.

| Transition | Flags | Value balance | Extra sighash data | Effect |
|---|---|---|---|---|
| `TokenShield` | outputs only | `-amount` | `tag(0x80), token_id` | `amount` leaves the owner's balance and enters the pool as new notes. |
| `TokenUnshield` | spends and outputs | `+amount` | `token_id, owner_id, recipient_id, amount` | Notes are spent; `amount` is credited to `recipient_id`; change comes back as new notes. |
| `TokenShieldedTransfer` | spends and outputs | `0` | `token_id, owner_id` | Notes are spent and recreated; the pool balance is unchanged. |
| `TokenMintToPool` | outputs only | `-amount` | `tag(0x81), token_id` | An authorized minter (manual minting rules, group actions supported) mints `amount` into new notes; the supply and the pool balance grow. Allowed only where `mintingAllowChoosingDestination` is set, since the notes' recipients are the minter's choice. |
| `TokenBurnFromPool` | spends and outputs | `+amount` | `token_id, burner_id, amount` | An authorized burner (manual burning rules, group actions supported) spends notes and destroys `amount`; the supply and the pool balance shrink. `burner_id` is the batch owner, or the proposer of a group action. |
| `TokenClaimToPool` | outputs only | `-amount` | `tag(0x82), token_id` | A distribution claim released into new notes instead of the claimant's balance; a perpetual claim names the cycle-aligned moment it claims up to so the amount is predictable. |
| `TokenDirectPurchaseToPool` | outputs only | `-token_count` | `tag(0x83), token_id` | The buyer pays credits at the direct purchase price and the tokens are minted into new notes. |

Each transition carries the Orchard bundle (`actions`, `anchor`, `proof`, `binding_signature`)
next to the token base transition (`token_id`, contract id, contract position, identity
contract nonce). The extra sighash data is bound into the Orchard sighash by the client and
recomputed by consensus from the transition's own fields, so a bundle cannot be replayed
against whatever its layout names. The layouts differ: the ones that spend bind the token, the
owner and, where tokens leave the pool, the recipient and amount; the four that only create
notes bind their kind and the token, and nothing about who submits them. The layouts are in
`dpp::shielded::sighash` (`token_unshield_extra_sighash_data`,
`token_shielded_transfer_extra_sighash_data`, `token_burn_from_pool_extra_sighash_data` and
`token_pool_output_only_extra_sighash_data`).

Outputs-only bundles (shield, mint, claim, purchase) have no spends, so their anchor is not
checked against the pool; the client builds them against the empty tree. Spending bundles must
name an anchor the pool has recorded.

Because the preimage of an outputs-only bundle binds no owner, anybody can lift one out of the
mempool and submit it again as the same kind into the same pool. The copy would land a second
note with the same commitment and the same `rho`, hence the same nullifier, and only one of the
two could ever be spent. The pool refuses it on the state side: each action's dummy nullifier,
from which its note takes its `rho`, is recorded in the pool's nullifier tree when the bundle
enters, and a bundle whose dummy nullifier is already there is rejected with
`NullifierAlreadySpentError`. `TokenPurchaseFromShieldedPool` carries an outputs-only token
bundle too and is checked the same way.

A mint or burn into the pool that goes through a group action stores
`TokenEvent::MintToPool` / `TokenEvent::BurnFromPool` with a digest of the serialized actions
(`serialized_actions_digest`), so every signer commits to exactly the same notes. A burn's
bundle is therefore proven once, by the proposer: the digest covers the spend authorization
signatures, so the sighash cannot depend on which signer's batch carries the bundle. It binds
the group action's proposer as `burner_id` (the batch owner for a direct burn) and every other
signer submits the proposer's bundle unchanged. No token history document is written for pool
operations, so the ledger records no actor for them. The operations still leave public traces:
spent nullifiers, the pool's note count and its anchors are all in state.

## Documents paid from the pool

A document type whose action has a token cost can be paid out of the token's pool.
`TokenPaymentInfo` gains a format version 1 that carries a `TokenShieldedPayment`: a spend
bundle in the payment token's pool (`amount`, `actions`, `anchor`, `proof`,
`binding_signature`) whose value balance is the document action's token cost. The identity still
signs the batch and pays the credit fee; its token balance is never touched. The bundle's sighash
binds the token id, the batch owner, the document's contract and id and the amount
(`document_token_payment_extra_sighash_data`), so it cannot pay for another document.

The transformer rejects a payment whose amount is not the document type's cost
(`TokenShieldedPaymentAmountMismatchError`, 40724) and a shielded payment on an action with no
token cost (`TokenShieldedPaymentNotRequiredError`, 40725). Document base state validation
version 1 skips the owner's balance and frozen-account checks for a shielded payment; the pool
side (pool exists, token not paused, anchor, unspent nullifiers, pool balance, proof) is
validated once the document action itself is valid. The lowering pays the cost from the pool: a
`TransferTokenToContractOwner` effect is a token unshield into the contract owner's balance, a
`BurnToken` effect a burn from the pool. The verification fee is charged like the batch pool
transitions, and CheckTx admits the bundle under the identity contract nonce.

## Identity-less transitions

Every batch transition above is signed by an identity that pays credits, so the chain sees
which identity moved token `T` at height `H`. Three top-level state transitions remove the
identity: each carries a bundle in the token's pool and a second spend bundle in the credit
shielded pool that pays the fee, both authorized only by Orchard spend keys.

| Type | Transition | Token bundle | Fee bundle | Effect |
|---|---|---|---|---|
| 26 | `TokenShieldedTransferWithShieldedFee` | spends, value `0` | spends, value = fee | Notes spent and recreated in the token pool; the fee leaves the credit pool. |
| 27 | `TokenUnshieldWithShieldedFee` | spends, value `+amount` | spends, value = fee | `amount` leaves the token pool into `recipient_id`'s token balance. |
| 28 | `TokenPurchaseFromShieldedPool` | outputs only, value `-token_count` | spends, value = price + fee | `token_count` is minted into the token pool at the direct purchase price; the price is credited to the contract owner. |

Every transition names the contract, the token position and the token id (which must derive
from the two), the two bundles and `credit_amount`, the fee bundle's value balance. The token
bundle's sighash binds the state transition type byte, the token id and the transparent fields
(recipient and amount, or count and price); the fee bundle's sighash binds the type byte, the
token id and a digest of the token bundle's actions (`token_pool_fee_bundle_extra_sighash_data`),
so a fee bundle can only ever pay for that exact token bundle.

The processor treats them like the credit pool's own pool-paid transitions: structure
validation checks both bundles, the minimum fee validation pins `credit_amount` to exactly the
two-bundle fee (`compute_token_pool_paid_shielded_fee`: the base fee of each bundle plus the
flat storage of what the transition writes outside the pools; a purchase adds the agreed price
on top), both proofs are verified statelessly, and the transform validates the pools: the
token owns a pool and is not paused, the token bundle's anchor is recorded and its nullifiers
unspent in the token pool, the credit pool holds what leaves it and the fee bundle's anchor and
nullifiers check out there, plus the transition's own rules (recipient exists for an
unshield, the pricing schedule and the max supply for a purchase). Execution is a
`PaidFromShieldedPool` event: the fee goes to the fee pools, the token side is the matching
token operation, and a purchase credits the contract owner. Uniqueness is by the spent
nullifiers of both bundles; a replay is an unpaid rejection. CheckTx admits their proofs under
the generic pool-paid limiter. All three are gated on `TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION`.

A transfer proves its execution with the token pool nullifiers it spent, an unshield with the
recipient's token balance, and a purchase with the token pool's total balance.

## Validation

Structure validation checks the amount bounds, the action count against
`SystemLimits::max_shielded_transition_actions`, the encrypted note sizes, a non-empty proof and
a non-zero anchor.

Unlike the credit pool, which refuses an outgoing spend until it holds
`minimum_pool_notes_for_outgoing` (250) notes, a token pool has no such floor:
`minimum_token_pool_notes_for_outgoing` is 0, so a pool with a single note lets that note be
spent at once. The floor was left out so that a small token's pool is usable from its first
note; the price is that a spend from a nearly empty pool is linkable to the shield that filled
it. The constant lives in the event constants and can be raised by a later protocol version.

State validation runs in this order, and the first failure is returned:

1. The token base transition (contract exists, position valid, nonce).
2. `hasShieldedPool` on the token's configuration, else `TokenShieldedPoolNotEnabledError`.
3. The token is not paused, for every operation. Shield: the owner holds `amount`. Unshield:
   the recipient identity exists.
   Mint to pool: the configuration lets the minter choose the destination
   (`mintingAllowChoosingDestination`), the minting rules authorize the identity (or group)
   and the max supply is not exceeded. Burn from pool: the burning rules authorize
   the identity and the token is not paused. Claim to pool: the claim resolves exactly as a
   claim into a balance does (the shared `resolve_token_claim`). Purchase to pool: the pricing
   schedule and the max supply.
4. Spending bundles: the anchor is recorded in the pool (`InvalidAnchorError`), no nullifier
   repeats within the bundle or is already spent (`NullifierAlreadySpentError`), and for an
   unshield the pool holds `amount`. Outputs-only bundles: no dummy nullifier repeats within
   the bundle or is already recorded in the pool (`NullifierAlreadySpentError`).
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

- shield: remove `amount` from the owner's token balance, insert the bundle's dummy
  nullifiers, append the notes, add `amount` to the pool's `TOTAL_BALANCE`;
- unshield: insert the nullifiers, append the notes, subtract `amount` from the pool balance,
  add `amount` to the recipient's token balance;
- shielded transfer: insert the nullifiers, append the notes;
- mint, claim and purchase to pool: insert the bundle's dummy nullifiers, append the notes,
  add `amount` to the pool balance and to the total supply (`TokenMintToPool`);
- burn from pool: insert the nullifiers, append the change notes, subtract `amount` from the
  pool balance and from the total supply (`TokenBurnFromPool`).

Only a mint, claim, purchase or burn changes a token's total supply. `calculate_total_tokens_balance` version 1 reads
the token pools BigSumTree and the block end conservation check requires
`identity balances + pool balances == total supply`.

## Block end

Every successful or paid token pool transition, document paid from a pool and identity-less
token pool transition records its pool in
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
A token id is rejected with `InvalidArgument` before protocol version 14 or when it is not 32
bytes. The proof verifier routes on the same field to the token twins of the verify functions
(`verify_token_shielded_pool_state` and the rest), and the Rust SDK exposes
`TokenShieldedPoolQuery`, `TokenShieldedEncryptedNotesQuery` and `TokenShieldedNullifiersQuery`.

A token pool transition proves its execution like the token transfer it resembles: a shield
proves the owner's balance, an unshield proves the recipient's balance, and a shielded transfer
proves the spent nullifiers in the token's pool.

## Client builders

`dpp::shielded::builder` provides `build_token_shield_transition`,
`build_token_unshield_transition`, `build_token_shielded_transfer_transition`,
`build_token_mint_to_pool_transition`, `build_token_burn_from_pool_transition`,
`build_token_claim_to_pool_transition` and `build_token_direct_purchase_to_pool_transition`.
They prove the bundle, bind the extra sighash data, and call the batch constructors
(`new_token_shield_transition` and siblings) which sign with the identity key.
`build_document_shielded_token_payment` builds the bundle of a `TokenPaymentInfo::V1`.
For document creation, call `Document::set_id_for_creation` with the creation nonce before
building the payment: the proof must bind the final document ID, not the factory placeholder.
`build_token_shielded_transfer_with_shielded_fee_transition`,
`build_token_unshield_with_shielded_fee_transition` and
`build_token_purchase_from_shielded_pool_transition` build the identity-less transitions from a
`TokenPoolSpender` (the token pool notes and keys) and a `ShieldedFeePayer` (the credit pool
notes and keys). The wasm bindings expose a wrapper per transition, `TokenPaymentInfo` accepts
a `shieldedPayment`, and `TokenConfiguration` accepts `hasShieldedPool` and reports
`formatVersion`.

## Fees

See [Shielded Transaction Fees](../fees/shielded-fees.md#token-shielded-pool-fees). In short:
the identity pays the metered cost of the writes plus the proof verification fee, exactly like
`ShieldFromIdentity`, for every batch transition and for a document paid from the pool; the
identity-less transitions carve the two-bundle fee from the credit pool bundle.
