# Token Contract Lifecycles

From protocol version 17 every contract that issues tokens carries a lifecycle record, and Drive keeps a ledger of destroyed issuers next to it. The record is what lets Platform destroy an issuer in one bounded write set, without visiting a holder or a token, and still keep the token conservation check exact. This chapter describes the ledger, who writes it, and how the block end check reads it.

## Storage

```text
[Tokens 16]
  └ 224  TOKEN_CONTRACT_LIFECYCLES_KEY               NormalTree
       ├ [0]  TOKEN_DESTROYED_SUPPLY_KEY              Item, u128 big endian (16 bytes)
       ├ [1]  TOKEN_LIFECYCLE_CLEANUP_QUEUE_KEY       NormalTree, empty until physical cleanup lands
       └ contract_id (32 bytes)                       Item, bincode ContractTokenLifecycle
```

`ContractTokenLifecycle::V0 { issued_supply: u128, wiped: Option<ContractWipe> }` lives in `rs-dpp` under `tokens::contract_lifecycle`. `issued_supply` is the sum of the supplies of every token the contract issues. `wiped` is set once, when the issuer is destroyed, and carries the block height and time; a wiped record never goes back to live and is never deleted, which is what makes the issuer's token ids unusable for ever.

The one-byte and thirty-two-byte keys cannot collide. The key values are provisional: the allocation register leaves new keys under `[Tokens]` unallocated, and they are revised, if at all, before any network is asked to propose protocol version 17.

## Why one rollup

Every native path that changes a token's supply changes the summed holder balances of that token by the same amount in the same batch: mint, mint to many, claim, direct purchase, burn, destroy frozen funds, and the base supply at contract creation. The per-token equality of supply and balance sum is therefore an invariant of the write paths, and the record carries one number that is both the issuer's supply rollup and its balance rollup.

The upgrade to protocol version 17 is where that invariant is checked once: `transition_to_version_17_token_lifecycles` walks the supply tree, reads the stored sum of each token's balance tree (one read of the parent element, no holder walk), and fails the upgrade block with `CorruptedDriveState` on any mismatch. Failing closed is a provisional choice recorded in the issue register; the liveness-preserving alternative is to record the mismatch and refuse destruction for that issuer.

## Why a scalar and not a sum tree

An issuer's rollup can exceed `i64`: two tokens at the largest legal base supply are legal today, so a `SumItem` cannot hold it. The destroyed supply is a 16 byte item read once by conservation, written once per destruction and once per cleanup step, following the total system credits precedent under `[Misc]`.

## Who writes the ledger

| Path | Generation | What it does |
|---|---|---|
| Genesis | `create_initial_state_structure` v4 | Calls `insert_token_contract_lifecycles_structure`, which inserts the ledger tree, the zero scalar and the empty queue sequentially. |
| Upgrade | `transition_to_version_17_token_lifecycles` | Calls the same helper, then backfills one record per issuer with insert-if-not-exists. |
| Contract creation | `insert_contract` v2 | Inserts the record seeded with the checked sum of the base supplies. |
| Token creation | `create_token_trees` v1 | Inserts a zero record if the contract has none and refuses a destroyed issuer, scanning the batch it is handed so several tokens created together insert the record once. |
| Contract update | `update_contract` v2 | Hands its accumulated batch to every added token's tree creation, so the record is written once per update however many tokens are added. |
| Supply increase | `add_to_token_total_supply` v1 | Resolves the issuer through the contract info leaf and raises the rollup by the amount actually added (the saturated amount when saturation was allowed). |
| Supply decrease | `remove_from_token_total_supply` v1 | Lowers the rollup by the same amount. |
| Destruction | `destroy_token_issuer` | Marks the record wiped and raises the scalar by its rollup: two reads, two writes, whatever the issuer holds. A contract without tokens gets a wiped zero record. The operations builder folds onto a write of the record or of the scalar already pending in the batch it is handed, so several destructions composed into one batch accumulate the scalar and a record the batch already wiped is refused. |

Transfers change no rollup and read no record. The supply writers refuse a wiped issuer as corrupted state: every path that changes supply is closed by validation before it reaches Drive, so reaching a wiped record there means a validator missed the check.

A Drive batch is lowered in full before any of it is applied, so two supply writes for tokens of one issuer in the same batch would both read the stored record and emit two replacements of the same key. `mint`, `burn` and `mint_many` v1 hand the batch accumulated so far down to the supply writers, and the rollup mover takes a replacement of the issuer's record already pending in that batch as its base and rewrites it in place. One replacement per issuer leaves the batch whatever the number of its tokens written; the `apply_drive_operations` tests cover distinct tokens of one issuer and two issuers in one batch.

Both the genesis and the upgrade path build the ledger through the same sequential helper, for the same reason the shielded pool does: the ledger has two one-byte keys whose Merk placement depends on the insertion order, and a fresh genesis node and an upgraded node must hold a byte-identical subtree.

## Token conservation

`calculate_total_tokens_balance` v1 reads the two aggregates of v0 and the destroyed supply scalar. `TotalTokensBalance::ok` requires the raw supply to equal the raw balances, as before, and the destroyed supply to be non-negative and no larger than the raw supply. The destroyed supply is a subset of the raw totals: the leaves it excludes are still stored and counted by both aggregates until physical cleanup removes them, and cleanup lowers the raw totals and the scalar together so the active supply (raw minus destroyed) never moves.

## Reads

`fetch_contract_token_lifecycle` reads one record. `fetch_token_lifecycles` resolves token ids to `TokenLifecycle::Live` or `TokenLifecycle::Wiped { block_height }` through the contract info leaves in two merged reads; a token without a contract info leaf is left out of the result, and a known token whose issuer has no record is corruption. Both are `OptionalFeatureVersion` slots that report `VersionNotActive` on the tables of protocol versions without the ledger.

## Estimation

`add_estimation_costs_for_token_contract_lifecycles` declares the root, the tokens root and the ledger layer. Record writes are priced as inserts of the largest record under a contract id sized key, because the estimator charges a replace no storage while the real record grows when its rollup or wipe marker does, and the estimate has to cover the applied cost.
