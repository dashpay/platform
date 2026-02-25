# Platform Address Fees

Protocol versions 10 and 11 introduced a new class of state transitions that
operate on **platform addresses** rather than identities. Platform addresses are
derived from public keys (similar to Bitcoin addresses) and hold a credit
balance directly, without requiring an identity to be registered.

This chapter explains how fees work for address-based transitions, how they
differ from the identity credit model, and how the fee strategy mechanism gives
clients control over fee deduction.

## Background: Why Platform Addresses?

Before protocol version 10, every action on the platform required an identity —
a registered entity funded by an asset lock transaction. Creating an identity
required a Core transaction, waiting for confirmations, and then submitting a
state transition. This was a multi-step process that created friction for simple
operations like "send credits to an address."

Platform addresses solve this by allowing credits to exist at addresses without
a full identity. Users can fund an address (via an asset lock or a transfer from
another address) and then spend from it directly using a signature from the
address's key pair.

## Address-Based State Transitions

Protocol versions 10 and 11 added the following transition types:

| Transition | Protocol Version | Description |
|---|---|---|
| `IdentityCreateFromAddresses` | 10 | Create an identity funded from platform address balances |
| `IdentityTopUpFromAddresses` | 10 | Add credits to an existing identity from address balances |
| `AddressFundsTransfer` | 11 | Transfer credits between platform addresses |
| `AddressFundingFromAssetLock` | 11 | Fund an address directly from an asset lock |
| `AddressCreditWithdrawal` | 11 | Withdraw credits from an address back to Core |

All of these use the `PaidFromAddressInputs` execution event variant.

## The Input/Output Model

Address-based transitions follow a UTXO-inspired model with **inputs** and
**outputs**:

### Inputs

Each input specifies a platform address, the expected nonce, and the amount to
consume from that address's balance:

```
inputs: [
    { address: A, nonce: 5, amount: 100_000_000 },
    { address: B, nonce: 3, amount:  50_000_000 },
]
```

The platform validates each input by:

1. Checking the address exists in state
2. Verifying the nonce is exactly `current_nonce + 1` (replay protection)
3. Confirming the address has sufficient balance for the requested amount

After validation, the **remaining balance** of each input is tracked:

```
remaining = actual_balance - requested_amount
```

This remaining balance is what is available to pay fees from (after the
requested amount is committed to outputs).

### Outputs

Outputs specify destination addresses and the credits to send:

```
outputs: [
    { address: C, amount: 80_000_000 },
    { address: D, amount: 60_000_000 },
]
```

Outputs are added to recipient balances. They can also be reduced to pay fees
(see [Fee Strategy](#fee-strategy) below).

### Balance Equation

The fundamental constraint is:

```
sum(input_amounts) >= sum(output_amounts) + fees
```

If the inputs cannot cover both the outputs and the fees, the transition is
rejected with `AddressesNotEnoughFundsError`.

## Fee Strategy

Unlike identity-based transitions where fees are always deducted from the
identity's balance, address-based transitions use an explicit **fee strategy**
that the client includes in the transition. The fee strategy is an ordered
sequence of steps that tells the platform where to find the fee credits:

```rust
pub enum AddressFundsFeeStrategyStep {
    /// Deduct fee from a specific input address's remaining balance.
    DeductFromInput(u16),
    /// Reduce a specific output's amount to cover the fee.
    ReduceOutput(u16),
}

pub type AddressFundsFeeStrategy = Vec<AddressFundsFeeStrategyStep>;
```

### How It Works

The platform processes the steps in order. At each step, it deducts as much of
the remaining fee as possible from the specified source:

```
fee_strategy: [DeductFromInput(0), ReduceOutput(0)]

Step 1: Try to deduct full fee from input 0's remaining balance
  - remaining_fee = 10,000,000
  - input_0_remaining = 25,000,000
  - deducted = 10,000,000
  - input_0_remaining = 15,000,000
  - remaining_fee = 0 → done
```

If the first source is insufficient, the algorithm moves to the next step:

```
fee_strategy: [DeductFromInput(0), ReduceOutput(1)]

Step 1: Try to deduct from input 0
  - remaining_fee = 10,000,000
  - input_0_remaining = 3,000,000
  - deducted = 3,000,000
  - input_0_remaining = 0 (removed)
  - remaining_fee = 7,000,000

Step 2: Try to reduce output 1
  - remaining_fee = 7,000,000
  - output_1_amount = 60,000,000
  - deducted = 7,000,000
  - output_1_amount = 53,000,000
  - remaining_fee = 0 → done
```

### Index Stability

The indices in the fee strategy refer to the **original** BTreeMap iteration
order. The implementation snapshots the address lists before processing any
steps, so removing a drained entry at step 1 does not shift the indices for
step 2. This is critical for correctness:

```rust
let input_addresses: Vec<PlatformAddress> = inputs.keys().copied().collect();
let output_addresses: Vec<PlatformAddress> = outputs.keys().copied().collect();

for step in fee_strategy {
    match step {
        DeductFromInput(index) => {
            let address = input_addresses[*index as usize];
            // look up by address, not by index into the live BTreeMap
        }
        // ...
    }
}
```

### FeeDeductionResult

The deduction algorithm produces:

```rust
pub struct FeeDeductionResult {
    pub remaining_input_balances: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    pub adjusted_outputs: BTreeMap<PlatformAddress, Credits>,
    pub fee_fully_covered: bool,
}
```

If `fee_fully_covered` is false, the transition is rejected.

## Nonce System (Replay Protection)

Every platform address has a nonce that starts at 0 and increments by 1 with
each transition that uses the address as an input. The transition must specify
the expected nonce, which must be exactly `current_nonce + 1`:

```rust
let expected_next_nonce = state_nonce.saturating_add(1);
if provided_nonce != expected_next_nonce {
    return Err(AddressInvalidNonceError {
        expected: expected_next_nonce,
        provided: provided_nonce,
    });
}
```

This prevents:
- **Replay attacks** — resubmitting an old transition (wrong nonce)
- **Double-spending** — using the same balance twice (nonce already consumed)
- **Ordering attacks** — submitting transitions out of order (nonce gap)

If a nonce reaches `u32::MAX`, the address is exhausted and cannot be used as
an input anymore.

Multiple addresses can be used as inputs in a single transition, each with its
own nonce. The platform enforces a maximum number of inputs per transition
(configured in `platform_version.dpp.state_transitions.max_address_inputs`).

## The PaidFromAddressInputs Event

When an address-based transition passes validation, the processor creates a
`PaidFromAddressInputs` execution event:

```rust
ExecutionEvent::PaidFromAddressInputs {
    input_current_balances: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    added_to_balance_outputs: BTreeMap<PlatformAddress, Credits>,
    fee_strategy: AddressFundsFeeStrategy,
    operations: Vec<DriveOperation>,
    execution_operations: Vec<ValidationOperation>,
    additional_fixed_fee_cost: Option<Credits>,
    user_fee_increase: UserFeeIncrease,
}
```

- **`input_current_balances`** — the remaining balance of each input after
  consuming the requested amounts, plus the validated nonce
- **`added_to_balance_outputs`** — the output amounts before any fee deductions
- **`fee_strategy`** — the client's ordered fee deduction instructions
- **`operations`** — the GroveDB operations (balance updates, nonce bumps)
- **`execution_operations`** — validation operations that also incur fees
- **`additional_fixed_fee_cost`** — optional fixed costs (e.g., registration fees)
- **`user_fee_increase`** — voluntary processing fee multiplier

## Fee Validation Pipeline

Address-based fee validation runs in two phases:

### Phase 1: Pre-Check (Minimum Balance)

Before expensive state reads, a quick estimate verifies that the inputs
have enough credits to cover the minimum possible fee:

```rust
fn validate_addresses_minimum_balance_pre_check(
    &self,
    remaining_address_balances: &BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error>
```

This catches obviously insufficient balances early. Only `AddressFundsTransfer`,
`AddressCreditWithdrawal`, `IdentityCreateFromAddresses`, and
`IdentityTopUpFromAddresses` run this pre-check. `AddressFundingFromAssetLock`
skips it because its funds come from the asset lock, not existing address
balances.

### Phase 2: Full Fee Validation

After the operations are determined, the platform:

1. Applies all drive operations (in estimation mode) to calculate the actual
   `FeeResult`
2. Adds validation operation costs
3. Applies the `user_fee_increase` multiplier
4. Adds any `additional_fixed_fee_cost`
5. Runs the fee strategy to deduct the total from inputs/outputs
6. Checks `fee_fully_covered`

```rust
let fee_deduction_result = deduct_fee_from_outputs_or_remaining_balance_of_inputs(
    input_current_balances.clone(),
    added_to_balance_outputs.clone(),
    fee_strategy,
    required_balance,
    platform_version,
)?;

if !fee_deduction_result.fee_fully_covered {
    return Err(AddressesNotEnoughFundsError);
}
```

## Fee Execution

When the transition is executed, the fee deduction runs a second time with the
real (not estimated) fee amount, and the adjusted balances are written to state:

1. **Apply all drive operations** — the core state changes (transfers, nonce
   bumps, etc.)
2. **Calculate actual fee** — from the FeeResult of the applied operations
3. **Deduct fee** — run the fee strategy against the actual fee amount
4. **Adjust outputs** — if any output was reduced, call
   `remove_balance_from_address` for the difference
5. **Adjust inputs** — if any input's remaining balance was reduced, call
   `set_balance_to_address` with the adjusted amount
6. **Apply adjustment operations** — batch the balance corrections into GroveDB

The fee goes into the epoch pool just like identity-based fees, and is
distributed to proposers at epoch end.

## Minimum Fee Calculation

For `AddressFundsTransfer`, the minimum fee scales with the number of inputs
and outputs:

```
min_fee = num_inputs × address_funds_transfer_input_cost
        + num_outputs × address_funds_transfer_output_cost
```

With current constants:

| Inputs | Outputs | Minimum Fee |
|---|---|---|
| 1 | 1 | 6,500,000 |
| 1 | 2 | 12,500,000 |
| 2 | 1 | 7,000,000 |
| 2 | 2 | 13,000,000 |

For `IdentityCreateFromAddresses`:

```
min_fee = identity_create_base_cost + num_keys × identity_key_in_creation_cost
```

| Keys | Minimum Fee |
|---|---|
| 1 | 8,500,000 |
| 2 | 15,000,000 |
| 3 | 21,500,000 |

## Key Differences from Identity Fees

| Aspect | Identity Credit Fees | Platform Address Fees |
|---|---|---|
| **Fee source** | Identity balance (single pool) | Input addresses + output reduction |
| **Fee deduction** | Automatic from identity | Explicit fee strategy |
| **Debt allowed** | Yes (negative balance tracked) | No — must have funds |
| **Refunds** | Yes — deleted storage refunded to identity | No refund mechanism |
| **Nonce** | Per identity, per contract | Per address, monotonic u32 |
| **User fee increase** | Applied to processing fees | Applied to processing fees |
| **Minimum fee** | Per transition type | Per input + per output |
| **ExecutionEvent** | `Paid` / `PaidFromAssetLock` | `PaidFromAddressInputs` |
| **Error on insufficient** | `BalanceIsNotEnoughError` | `AddressesNotEnoughFundsError` |

### No Debt, No Refunds

The most significant difference is that address-based transitions have **no debt
mechanism** and **no refund mechanism**:

- **Identity fees** can create a negative balance when the processing fee cannot
  be fully covered. This debt is tracked and must be repaid before the identity
  can submit new transitions.
- **Address fees** must be fully covered by available inputs and outputs. If the
  fee cannot be paid, the transition is rejected outright.

- **Identity refunds** return credits to the identity when stored data is
  deleted. The refund is calculated per epoch based on when the data was
  originally stored.
- **Address-based transitions** do not produce refundable storage. If an
  address-based transition stores data and that data is later deleted, no refund
  is issued to any address.

## Example: AddressFundsTransfer

A concrete example of how fees flow for a two-input, two-output transfer:

```
Transition:
  inputs:  [{A, nonce: 5, amount: 1_000_000_000}, {B, nonce: 3, amount: 500_000_000}]
  outputs: [{C, amount: 800_000_000}, {D, amount: 600_000_000}]
  fee_strategy: [DeductFromInput(0), DeductFromInput(1)]
  user_fee_increase: 0

1. Validate inputs:
   A: state_nonce=4, balance=2_000_000_000 → nonce 5 ✓, balance ≥ 1B ✓
   B: state_nonce=2, balance=700_000_000  → nonce 3 ✓, balance ≥ 500M ✓

   Remaining: A=(5, 1_000_000_000), B=(3, 200_000_000)

2. Pre-check minimum fee:
   min = 2 × 500,000 + 2 × 6,000,000 = 13,000,000
   sum(remaining) = 1,200,000,000 >> 13M ✓

3. Create PaidFromAddressInputs event:
   input_current_balances: {A: (5, 1B), B: (3, 200M)}
   added_to_balance_outputs: {C: 800M, D: 600M}

4. Calculate actual fee:
   storage_fee = 14,000,000  (hypothetical)
   processing_fee = 2,500,000
   total = 16,500,000

5. Apply fee strategy:
   Step 1: DeductFromInput(0) → A: 1B - 16.5M = 983,500,000
   remaining_fee = 0 ✓

6. Execute:
   A.balance = 983,500,000, A.nonce = 5
   B.balance = 200,000,000, B.nonce = 3
   C.balance += 800,000,000
   D.balance += 600,000,000
   16,500,000 → epoch fee pool
```

## Key Source Files

| File | Contents |
|---|---|
| `rs-dpp/src/address_funds/fee_strategy/mod.rs` | AddressFundsFeeStrategy definition |
| `rs-dpp/src/address_funds/fee_strategy/deduct_fee_from_inputs_and_outputs/` | Fee deduction algorithm |
| `rs-drive-abci/src/execution/types/execution_event/mod.rs` | PaidFromAddressInputs variant |
| `rs-drive-abci/src/execution/validation/.../processor/traits/address_balances_and_nonces.rs` | Nonce and balance validation |
| `rs-drive-abci/src/execution/validation/.../processor/traits/addresses_minimum_balance.rs` | Minimum balance pre-check |
| `rs-drive-abci/src/execution/platform_events/.../validate_fees_of_event/` | Full fee validation |
| `rs-drive-abci/src/execution/platform_events/.../execute_event/` | Fee execution with adjustments |
| `rs-platform-version/src/version/fee/state_transition_min_fees/v1.rs` | Address fee constants |
