# Shielded Transaction Fees

> Introduced in protocol version 12. For the general fee system overview, see
> [Fee System Overview](overview.md). For address-based fees (protocol versions
> 10--11), see [Platform Address Fees](platform-address-fees.md).

Shielded transactions use the Orchard protocol's zero-knowledge proofs to hide
transaction amounts. Because amounts are hidden, the platform cannot inspect the
transaction to compute fees the way it does for transparent transitions. Instead,
the fee is embedded into the cryptographic structure of the bundle itself, and the
platform enforces a minimum.

This chapter explains the fee model, how it is validated, and how it differs from
the transparent and address-based fee systems.

## The Problem: Fees in a Privacy System

In a transparent state transition like `AddressFundsTransfer`, the platform can see
the transfer amount, compute the cost of storage and processing, and deduct the fee
from the sender's balance. The fee calculation happens *after* the transition is
applied.

Shielded transitions break this model. The amounts inside the ZK proof are hidden.
The platform cannot look inside the proof to determine how much was sent or received.
It can only see two things from the public fields:

1. **`value_balance`** — the net amount leaving the shielded pool (positive means credits
   flow out of the pool into the transparent world or to proposers as fees)
2. **`num_actions`** — the number of spend+output pairs in the bundle

The fee must therefore be encoded into `value_balance` by the client and validated
by the platform before execution.

## Fee Extraction by Transition Type

The fee is derived differently depending on the shielded transition type:

| Transition | Fee Formula | Explanation |
|---|---|---|
| **Shield** | Paid from transparent address inputs | Fee comes from the transparent side, not from `value_balance`. Skipped by shielded fee validation. |
| **ShieldedTransfer** | `fee = value_balance` | The entire `value_balance` is fee — nothing leaves the pool except the fee going to proposers. |
| **Unshield** | `fee = value_balance − amount` | `amount` goes to the output address; the remainder is fee. |
| **ShieldedWithdrawal** | `fee = value_balance − amount` | `amount` goes to the withdrawal document; the remainder is fee. |
| **ShieldFromAssetLock** | Paid from asset lock | Fee comes from the asset lock mechanism, not from `value_balance`. |

For `ShieldedTransfer`, the client constructs the bundle so that `total_spent −
total_output = desired_fee`. The Orchard circuit proves that value is conserved
(inputs = outputs + value_balance), and the binding signature cryptographically
commits to the `value_balance`. Mutating `value_balance` after signing will cause
the binding signature to fail verification.

## The Three-Component Fee Model

The minimum shielded fee has three components:

```
min_fee = proof_verification_fee + num_actions × (processing_fee + storage_fee)
```

### 1. Proof Verification Fee (per bundle)

A single Halo 2 ZK proof covers the entire bundle regardless of action count.
Verifying it is the most expensive operation — benchmarked at approximately
30× the cost of a per-action signature verification. This is a fixed cost per
bundle.

**Current value:** `100,000,000` credits (100M)

### 2. Per-Action Processing Fee

Each action in the bundle requires:
- RedPallas spend authorization signature verification
- Nullifier duplicate check (hash + tree lookup)
- Note commitment insertion into the Sinsemilla-based Merkle tree

The processing cost per action was calibrated at a 33:1 ratio against the proof
verification cost, based on benchmarks of signature verification and tree operations.

**Current value:** `3,000,000` credits (3M)

### 3. Per-Action Storage Fee

Each action permanently stores data in two places:

| Storage | Bytes | Contents |
|---|---|---|
| BulkAppendTree (commitment tree) | 280 | 32 cmx + 32 nullifier + 216 encrypted note |
| Nullifier tree | 32 | nullifier key (value is empty) |
| **Total** | **312** | |

The storage fee is derived from the platform's existing per-byte storage rates:

```
storage_fee_per_action = 312 × (storage_disk_usage_credit_per_byte
                              + storage_processing_credit_per_byte)
                       = 312 × (27,000 + 400)
                       = 312 × 27,400
                       = 8,548,800
```

This is not a separate constant — it is computed dynamically from the storage fee
version, ensuring shielded storage costs stay consistent with transparent storage
costs as fee parameters evolve.

## Fee Table

Combining all three components:

| Actions | Proof Fee | Processing | Storage | Total Minimum Fee |
|---|---|---|---|---|
| 2 | 100,000,000 | 6,000,000 | 17,097,600 | **123,097,600** |
| 3 | 100,000,000 | 9,000,000 | 25,646,400 | **134,646,400** |
| 4 | 100,000,000 | 12,000,000 | 34,195,200 | **146,195,200** |

Note: The Orchard protocol requires a minimum of 2 actions per bundle for privacy
(even a single-input single-output transfer produces 2 actions with a dummy padding
action). Bundles with 1 action are structurally invalid.

## Where Fee Validation Runs

Fee validation is integrated into the processor pipeline (see
[Validation Pipeline](../state-transitions/validation-pipeline.md)) between basic structure validation
and ZK proof verification:

```
... → Basic Structure → Minimum Fee Check → ZK Proof Verification → ...
```

The ordering is deliberate. The fee check is stateless and cheap — it only reads
`value_balance` and `actions.len()` from the transition, with no GroveDB lookups.
Placing it before proof verification means that bundles with insufficient fees are
rejected instantly, without spending ~100ms on Halo 2 verification.

The implementation lives in
`packages/rs-drive-abci/src/execution/validation/state_transition/processor/traits/shielded_proof.rs`:

```rust
pub(crate) trait StateTransitionShieldedMinimumFeeValidationV0 {
    fn validate_minimum_shielded_fee(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}
```

If the fee is below the minimum, the transition is rejected with
`InsufficientShieldedFeeError` — an unpaid consensus error. The sender is not
charged (there is no identity to charge), and the transition produces no execution
event.

## Fee Constants in the Version System

The fee parameters are stored in the platform version under
`drive_abci.validation_and_processing.event_constants`:

```rust
pub struct DriveAbciValidationConstants {
    pub maximum_vote_polls_to_process: u16,
    pub maximum_contenders_to_consider: u16,
    pub minimum_pool_notes_for_outgoing: u64,
    pub shielded_proof_verification_fee: u64,      // 100_000_000
    pub shielded_per_action_processing_fee: u64,    // 3_000_000
}
```

The storage component is not a separate constant — it is derived at runtime from
`fee_version.storage.storage_disk_usage_credit_per_byte` and
`fee_version.storage.storage_processing_credit_per_byte`, multiplied by the
constant `SHIELDED_STORAGE_BYTES_PER_ACTION = 312`.

This design means:
- **Proof and processing fees** can be tuned independently via version bumps
- **Storage fees** automatically track changes to the platform-wide storage rates
- No "magic number" for storage cost exists in the version constants

## How Fees Flow After Validation

Once the fee check passes and the transition is fully validated and executed, the
fee amount is deducted from the shielded pool's total balance and routed to block
proposers via the `PaidFromShieldedPool` execution event:

```
ShieldedTransfer:    pool_balance -= fee_amount
Unshield:           pool_balance -= (amount + fee_amount)
ShieldedWithdrawal: pool_balance -= (amount + fee_amount)
```

For `Unshield`, the `amount` goes to the output platform address. For
`ShieldedWithdrawal`, the `amount` goes to a Core withdrawal document. In both
cases, the `fee_amount` goes to proposers.

For `ShieldedTransfer`, the total pool value decreases by exactly the fee amount.
The rest of the value stays inside the pool (the sender's notes are spent and the
recipient's notes are created, but the pool's aggregate balance only drops by the
fee).

## Cryptographic Binding

The fee is not just a field that the platform trusts. It is cryptographically bound
to the ZK proof through two mechanisms:

1. **Value commitments (cv_net):** Each action contains a Pedersen commitment to the
   note value. The sum of all value commitments must equal `value_balance` (modulo
   the blinding factors). The binding signature proves this relationship holds.

2. **Platform sighash:** The bundle commitment (which includes `value_balance`) is
   hashed into the sighash that the spend authorization signatures sign over:
   ```
   sighash = SHA-256("DashPlatformSighash" || bundle_commitment || extra_data)
   ```
   Mutating `value_balance` after signing changes the sighash, invalidating all
   signatures. The `BatchValidator` checks both the Halo 2 proof and all signatures,
   so any tampering is caught.

This means a client cannot claim a lower fee than what the ZK proof actually commits
to — the proof and signatures would fail verification.

## Rules and Guidelines

**Do:**
- Always set `value_balance` to at least the minimum fee when building a shielded
  bundle on the client side. Use `min_fee = proof_verification_fee + num_actions ×
  (processing_fee + storage_fee)` with the current platform version constants.
- Include the fee in the note arithmetic: `total_spent = total_output + fee`. The
  Orchard builder handles this when you set the output amount to
  `spend_amount − desired_fee`.
- Remember that the minimum action count is 2 (Orchard privacy requirement).

**Do not:**
- Assume the fee is free for `Shield` transitions — the fee comes from transparent
  address inputs and is validated through the address balance system, not here.
- Mutate `value_balance` after building the bundle. The binding signature and sighash
  will be invalidated.
- Hardcode fee amounts. Always read from `PlatformVersion` — the constants are
  versioned and will change as the protocol evolves.
