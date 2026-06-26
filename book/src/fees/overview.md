# Fee System Overview

Every state transition on Dash Platform costs credits. Credits are the internal
unit of account (1 Dash = 100,000,000,000 credits). The fee system ensures that
validators are compensated for computation and storage, that spam is economically
infeasible, and that the platform's state does not grow unboundedly without
payment.

This section covers the three fee eras that the platform has gone through:

1. **Identity Credit Fees (protocol versions 1--9)** — Fees paid from an
   identity's credit balance, funded by asset lock transactions on Core.
2. **Platform Address Fees (protocol versions 10--11)** — Fees paid from
   platform address balances using a UTXO-like input/output model.
3. **Shielded Transaction Fees (protocol version 12)** — Fees embedded in
   zero-knowledge proofs and cryptographically bound to the Orchard bundle.

Each era introduced new `ExecutionEvent` variants and fee validation logic, but
the underlying cost accounting (storage fees, processing fees, epoch
distribution) is shared across all three.

## Credits and Denomination

Platform credits are the smallest unit of value:

| Unit | Credits |
|---|---|
| 1 credit | 1 |
| 1 mDash | 100,000,000 |
| 1 Dash | 100,000,000,000 |

All fee constants in the codebase are denominated in credits.

## Cost Components

The platform distinguishes two fundamental kinds of cost:

### Storage Fees

Storage fees pay for bytes that persist in GroveDB indefinitely. The rate is set
in `FeeStorageVersion`:

| Parameter | Value | Description |
|---|---|---|
| `storage_disk_usage_credit_per_byte` | 27,000 | Permanent disk storage cost |
| `storage_processing_credit_per_byte` | 400 | I/O cost to write the bytes |
| `storage_load_credit_per_byte` | 20 | I/O cost to read stored bytes |
| `non_storage_load_credit_per_byte` | 10 | I/O cost for ephemeral reads |
| `storage_seek_cost` | 2,000 | Cost of a single disk seek |

Storage fees are **refundable**: when data is deleted, a portion of the original
storage fee is returned to the identity that paid it (see [Refunds](#refunds)
below).

### Processing Fees

Processing fees pay for computation that does not leave a permanent trace in
storage: signature verification, hashing, tree traversal, and so on. These
are **non-refundable** — the computation has already been performed.

Processing costs are built up from individual operations:

```
processing_fee =
    seek_count × storage_seek_cost
  + added_bytes × storage_processing_credit_per_byte
  + replaced_bytes × storage_processing_credit_per_byte
  + loaded_bytes × storage_load_credit_per_byte
  + hash_node_calls × (blake3_base + blake3_per_block)
```

Signature verification adds a fixed cost per algorithm:

| Algorithm | Cost (credits) |
|---|---|
| ECDSA secp256k1 | 15,000 |
| BLS12-381 | 300,000 |
| ECDSA hash160 | 15,500 |
| BIP13 script hash | 300,000 |
| EdDSA ed25519 hash160 | 3,500 |

Hashing costs scale with the number of blocks processed:

| Hash Function | Base | Per Block |
|---|---|---|
| SHA-256 | 100 | 5,000 |
| Blake3 | 100 | 300 |
| SHA-256 + RIPEMD-160 | 6,000 | 5,000 |

## Minimum Fees

Every state transition type has a minimum fee that must be met regardless of the
actual computation cost. This prevents zero-cost spam. The minimums are defined
in `StateTransitionMinFees`:

### Identity-Based Transitions (protocol versions 1--9)

| Transition | Minimum Fee (credits) |
|---|---|
| Credit Transfer | 100,000 |
| Credit Transfer to Addresses | 500,000 |
| Credit Withdrawal | 400,000,000 |
| Identity Update | 100,000 |
| Document Batch (per sub-transition) | 100,000 |
| Contract Create | 100,000 |
| Contract Update | 100,000 |
| Masternode Vote | 100,000 |

### Address-Based Transitions (protocol versions 10--11)

| Transition | Minimum Fee (credits) |
|---|---|
| Address Funds Transfer (per input) | 500,000 |
| Address Funds Transfer (per output) | 6,000,000 |
| Address Credit Withdrawal | 400,000,000 |
| Identity Create (base) | 2,000,000 |
| Identity Create (per key) | 6,500,000 |
| Identity Top-Up (base) | 500,000 |

### Data Contract Registration Fees (protocol version 9+)

Protocol version 9 introduced significant registration fees for data contracts
to prevent namespace squatting:

| Component | Fee | Dash Equivalent |
|---|---|---|
| Base contract registration | 10,000,000,000 | 0.1 Dash |
| Document type registration | 2,000,000,000 | 0.02 Dash |
| Non-unique index | 1,000,000,000 | 0.01 Dash |
| Unique index | 1,000,000,000 | 0.01 Dash |
| Contested index | 100,000,000,000 | 1.0 Dash |
| Token registration | 10,000,000,000 | 0.1 Dash |
| Search keyword | 10,000,000,000 | 0.1 Dash |

Before protocol version 9, all registration fees were zero.

## User Fee Increase

Every state transition carries a `user_fee_increase` field (a `UserFeeIncrease`
value). This allows the sender to voluntarily pay more than the base fee to
prioritize their transition. The multiplier works as follows:

- `0` = 100% of base fee (no increase)
- `1` = 101% of base fee
- `10` = 110% of base fee
- `100` = 200% of base fee

The increase applies only to the **processing fee** component, not to storage
fees. This is because storage fees are a direct function of bytes stored and
should not be inflated.

```rust
fn apply_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
    let increase = self.processing_fee * user_fee_increase as u64 / 100;
    self.processing_fee = self.processing_fee.saturating_add(increase);
}
```

## ExecutionEvent Variants

The `ExecutionEvent` enum (in `rs-drive-abci`) determines how fees are collected
for each state transition. There are eight variants:

| Variant | Fee Source | Used By |
|---|---|---|
| `Paid` | Identity credit balance | Most identity-based transitions |
| `PaidFromAssetLock` | Asset lock transaction value | IdentityCreate, IdentityTopUp |
| `PaidFromAssetLockWithoutIdentity` | Asset lock (fixed amount) | PartiallyUseAssetLock |
| `PaidFromAssetLockToPool` | Asset lock value; fee routed to the fee pools | ShieldFromAssetLock |
| `PaidFromAddressInputs` | Platform address balances | All address-based transitions; `Shield` (metered + a ZK compute fee via `additional_fixed_fee_cost`) |
| `PaidFixedCost` | Fixed fee to pool | MasternodeVote |
| `PaidFromShieldedPool` | Shielded pool value_balance | ShieldedTransfer, Unshield, ShieldedWithdrawal |
| `PaidFromShieldedPoolToNewIdentity` | Shielded pool (the fixed `denomination`); the metered write + ZK compute fee is moved from the new identity's balance into the fee pools | IdentityCreateFromShieldedPool |

Each variant carries the operations to execute and enough context for the fee
validation and execution pipeline to deduct the correct amount from the correct
source.

## FeeResult

All fee calculations produce a `FeeResult`:

```rust
pub struct FeeResult {
    pub storage_fee: Credits,
    pub processing_fee: Credits,
    pub fee_refunds: FeeRefunds,
    pub removed_bytes_from_system: u32,
}
```

- **`storage_fee`** — credits for new bytes written to persistent storage
- **`processing_fee`** — credits for computation and I/O
- **`fee_refunds`** — credits returned because previously stored data was deleted
- **`removed_bytes_from_system`** — bytes removed that were stored by the system
  (not by any identity), so no refund is issued

The total base fee is `storage_fee + processing_fee`. The `FeeResult` is
produced by `Drive::apply_drive_operations()`, which executes the GroveDB
operations and measures the actual cost of each insert, delete, and query.

## Refunds

When data is removed from GroveDB (a document is deleted, a key is removed),
the system calculates a refund of the original storage fee. Refunds are tracked
per identity per epoch:

```rust
pub struct FeeRefunds(pub CreditsPerEpochByIdentifier);
// BTreeMap<IdentifierBytes32, BTreeMap<EpochIndex, Credits>>
```

Refunds are not 1:1 with the original fee because storage fees are distributed
across future epochs (see below). The refund amount depends on how many epochs
have elapsed since the data was stored — the longer the data has been stored, the
smaller the refund, because more of the distributed fees have already been paid
out to proposers.

There is a **dust limit**: refunds below 32 bytes worth of storage credits are
discarded to prevent micro-refund spam.

## Epoch-Based Fee Distribution

Fees do not go directly to the block proposer. Instead, they accumulate in
epoch-specific pools and are distributed to proposers at epoch boundaries.

### How Epochs Work

- An **epoch** is a fixed window of blocks
- An **era** consists of 40 epochs
- Storage fees are distributed across **50 eras** (2,000 epochs, roughly 50
  years) using a declining schedule

The distribution table allocates percentages per era:

| Era | Percentage | Cumulative |
|---|---|---|
| 0 | 5.000% | 5.0% |
| 1 | 4.800% | 9.8% |
| 2 | 4.600% | 14.4% |
| ... | ... | ... |
| 49 | 0.125% | 100.0% |

Within each era, the percentage is divided equally among the era's epochs. For
example, a 1,000,000-credit storage fee distributes 50,000 credits (5%) to era 0,
split evenly across 40 epochs = 1,250 credits per epoch.

### Fee Flow

```
Block execution
  └→ FeeResult (storage + processing)
       └→ End of block: add_distribute_block_fees_into_pools()
            ├→ Processing fees → current epoch pool
            └→ Storage fees → global distribution pool → spread across future epochs

Epoch change
  └→ add_distribute_fees_from_oldest_unpaid_epoch_pool_to_proposers()
       ├→ Calculate Core block rewards for the epoch
       ├→ Add Core rewards to system credits
       └→ Distribute (Platform fees + Core rewards) to proposers
```

Processing fees are paid to proposers at the end of the epoch in which they were
collected. Storage fees are spread across 50 eras of future epochs, providing a
long-term revenue stream for validators.

## Fee Versioning

All fee parameters are versioned through `FeeVersion`, stored in
`PlatformVersion`. This allows the protocol to adjust fee rates without a hard
fork — a new protocol version simply references different fee constants.

The current fee version structure:

```rust
pub struct FeeVersion {
    pub fee_version_number: FeeVersionNumber,
    pub uses_version_fee_multiplier_permille: Option<u64>,
    pub storage: FeeStorageVersion,
    pub signature: FeeSignatureVersion,
    pub hashing: FeeHashingVersion,
    pub processing: FeeProcessingVersion,
    pub data_contract_validation: FeeDataContractValidationVersion,
    pub data_contract_registration: FeeDataContractRegistrationVersion,
    pub state_transition_min_fees: StateTransitionMinFees,
    pub vote_resolution_fund_fees: VoteResolutionFundFees,
}
```

Fee versions are stored in the `FEE_VERSIONS` array and looked up by number. The
`uses_version_fee_multiplier_permille` field allows a global scaling factor
(permille = divide by 1000; a value of 1000 means no change).

## Key Source Files

| File | Contents |
|---|---|
| `rs-platform-version/src/version/fee/` | All fee version definitions |
| `rs-platform-version/src/version/fee/storage/v1.rs` | Storage fee rates |
| `rs-platform-version/src/version/fee/signature/v1.rs` | Signature verification costs |
| `rs-platform-version/src/version/fee/state_transition_min_fees/v1.rs` | Minimum fees per transition |
| `rs-platform-version/src/version/fee/data_contract_registration/v2.rs` | Contract registration fees |
| `rs-drive/src/fees/op.rs` | LowLevelDriveOperation and cost calculation |
| `rs-dpp/src/fee/fee_result/mod.rs` | FeeResult, BalanceChangeForIdentity |
| `rs-dpp/src/fee/epoch/distribution.rs` | Epoch distribution table and refund logic |
| `rs-drive-abci/src/execution/types/execution_event/mod.rs` | ExecutionEvent enum |
| `rs-drive-abci/src/execution/platform_events/fee_pool_inwards_distribution/` | Block fee collection |
| `rs-drive-abci/src/execution/platform_events/fee_pool_outwards_distribution/` | Proposer payout |
