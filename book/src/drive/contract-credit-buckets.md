# Contract Credit Buckets

Ordinary data contracts are not identities. They have no keys, no nonce and no identity balance, and the smart contract work keeps it that way. When a contract needs to hold credits (to pay for scheduled work, to escrow a purchase, to fund its own storage) those credits live in a dedicated structure that is separate from every identity tree: the contract credits root tree. This chapter describes that tree, the lifecycle wrapper that marks a wiped contract, and how the tree takes part in credit conservation. Later chapters add the bucket identifiers and rules, the storage operations, the proofs and the matching token holdings as those pieces land.

## Why a separate tree

Identity balances sit in the `Balances` root sum tree keyed by identity id. Reusing that tree for contracts would give a contract an identity-shaped balance element, and every query, proof and validation rule that reads `Balances` would have to learn that some keys are not identities. A separate root tree avoids all of that: nothing under `Identities` or `Balances` changes, identity balance proofs stay exactly as they were, and a contract's credits are found by walking a path that names the contract, not an identity.

A sum tree was chosen over the specialized provable-sum collections because the two questions that matter for contract credits are answered by a single authenticated element:

- How much does one bucket hold? That is a `SumItem` under the contract's subtree, proven like any other item.
- How much does the whole contract hold? That is the stored sum of the contract's `SumTree` element, which the parent Merk commits to and a proof of the parent path authenticates.

Range sums across buckets are not a query anyone makes, so the cost of a provable sum tree buys nothing here. Range-sum proof capabilities stay with the specialized collections used by document sum indexes.

## Layout

The root tree is `RootTree::ContractCredits`, key `100`, an ordinary `SumTree`. The key value is provisional: the allocation register leaves new root keys unallocated and `100` was chosen as a free value next to the other balance trees.

```text
Root Merk
  100  ContractCredits                        SumTree      sum = total of live contract credits
    contract_id (32 bytes)                    SumTree      a live contract; sum = its total
    contract_id (32 bytes)                    NotSummed(SumTree)   a wiped contract; inner sum retained, contributes 0
      bucket_key (2 bytes, big-endian u16)    SumItem      one bucket, 0 <= value <= MAX_CREDITS
```

The path helpers live in `packages/rs-drive/src/drive/contract/balances/mod.rs`: `contract_credits_root_path()` for `[100]` and `contract_credits_path(contract_id)` for `[100, contract_id]`, each with a `_vec` form.

## Genesis and upgrade

The tree exists from the 5.0 protocol version (provisionally 17) on both kinds of node:

- A fresh chain creates it in `Drive::create_initial_state_structure_v4`, as a standalone root insert placed right after `ShieldedBalances`, before the lower-layer batch.
- A node that upgrades in place creates it in `Platform::transition_to_version_17`, which runs from `perform_events_on_first_block_of_protocol_change` on the first block at the new version, with an insert-if-not-exists.

Both paths insert the same `Element::empty_sum_tree()` with no flags, and a test in the protocol change hook builds one platform each way and compares the subtree byte for byte. The insert-if-not-exists also makes the upgrade idempotent: a validator that ran the hook inside a rejected proposal and runs it again in the next round produces the same state as one that ran it once.

## Lifecycle: live and wiped

A contract's subtree is one of two things:

- **Live**: an `Element::SumTree` whose stored sum is the contract's spendable total. Deposits and spends are allowed.
- **Wiped**: the same subtree wrapped in `Element::NotSummed`. GroveDB reports a not-summed element's contribution to its parent as zero, so the root aggregate no longer counts it, while the buckets underneath keep their values for cleanup. Nothing may deposit into or spend from a wiped contract, and no reader may report a retained bucket as available credit.

The wrapper is a property of the parent element, so a proof of the contract's path authenticates the lifecycle along with the total. The operation that wraps a populated live tree is not part of this change; the layout above is what it must produce.

## Credit conservation

Since protocol version 12 the end-of-block check compares the total credits in Platform with the sum of five trees. From the 5.0 protocol version the equation has a sixth term, read by `calculate_total_credits_balance` v3 from the root aggregate of the contract credits tree:

```text
total_credits_in_platform == total_in_pools
                           + total_identity_balances
                           + total_specialized_balances
                           + total_in_addresses
                           + total_in_shielded_balances
                           + total_in_contract_credits
```

Because the root aggregate is a `SumTree` sum and wiped subtrees are `NotSummed`, retained credits of wiped contracts are excluded by construction. A wipe that moves a contract's live total somewhere else (the processing pool of the current epoch, by the owner-confirmed policy) must therefore move it exactly once, or the equation fails at the end of that block.

The `TotalCreditsBalance` type in `rs-dpp` carries the new field `total_in_contract_credits`; the earlier calculator generations set it to zero because the tree does not exist on the chains they run against.

## What is not here yet

- Bucket identifiers (a two-byte position per contract), bucket spending and deposit rules, and the Drive operations that create a contract's subtree and move credits in and out of a bucket.
- Proofs of a single bucket and of a contract's total, including the live or wiped state.
- Contract token holdings, which follow the same shape under the per-token ledger.
- The wipe itself, the transfer state transitions that call the bucket operations, and the query surface.
