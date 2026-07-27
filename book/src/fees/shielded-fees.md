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
| **Shield** | `fee = metered(storage + processing) + shielded_verification_fee`, paid from transparent address inputs | Charged on the transparent side (not from `value_balance`), on top of the shielded amount. The storage and processing of the note/nullifier writes are **metered** by GroveDB; only the ZK compute fee (`proof + num_actions × per_action_processing`) is added on top. Skipped by the `value_balance`-based shielded fee validation; enforced through the address-input fee path. See [Entry-Transition Fees](#entry-transition-fees-shield-and-shieldfromassetlock). |
| **ShieldedTransfer** | `fee = value_balance` (pinned to the minimum) | The entire `value_balance` is the fee and must equal `compute_minimum_shielded_fee(num_actions)` exactly (overpayment is rejected). Nothing leaves the pool except the fee. |
| **Unshield** | `fee = compute_minimum_shielded_fee(num_actions) + unshield_address_storage_fee` | `value_balance` (the transition's `unshielding_amount`) is the **gross** amount leaving the pool. The output address receives `unshielding_amount − fee`; validation requires `unshielding_amount ≥ fee`. Unshield also writes the net to the output platform address (`AddBalanceToAddress`), a real storage write priced on top of the base shielded minimum (`unshield_address_storage_fee = 222 × per_byte_rate`, ≈6.08M credits, flat regardless of action count — 222 bytes is the *storage* portion of the ≈6.24M metered address write) so the address write is covered and the proof fee isn't diverted to pay for it. See [Per-Action Storage Fee](#3-per-action-storage-fee). |
| **ShieldedWithdrawal** | `fee = compute_minimum_shielded_fee(num_actions) + withdrawal_document_storage_fee` | `value_balance` (`unshielding_amount`) is the **gross** amount leaving the pool. The Core withdrawal document receives `unshielding_amount − fee` (which must also clear `MIN_WITHDRAWAL_AMOUNT`). Unlike the other pool-paid transitions, ShieldedWithdrawal also **writes a Core withdrawal document** — a real document insert into the withdrawals contract plus its index entries (`AddWithdrawalDocument`), with a real metered cost of ≈110M credits that is **flat regardless of action count**. That cost is priced on top of the base shielded minimum as a flat ~4,100-byte storage component (`withdrawal_document_storage_fee = 4100 × per_byte_rate`), so the document write is covered and the proof-verification fee isn't diverted from the proposer to pay for it. See [Per-Action Storage Fee](#3-per-action-storage-fee). |
| **ShieldFromAssetLock** | `pool_fee = compute_minimum_shielded_fee(num_actions) + asset_lock_base_cost`, paid from the asset lock | The flat shielded minimum plus the asset-lock processing base cost is routed to the fee pools. Any remaining asset-lock value (the *surplus*) goes to an optional signed `surplus_output` platform address, or — if none is set — folds into the fee pools up to `shielded_implicit_fee_cap`. See [Entry-Transition Fees](#entry-transition-fees-shield-and-shieldfromassetlock). |
| **IdentityCreateFromShieldedPool** | `total_fee = metered(insert_nullifiers + AddNewIdentity(identity + N keys)) + shielded_verification_fee`, **moved from the new identity's balance** | `value_balance` is a **fixed `denomination`** (a member of the versioned set `{0.1, 0.3, 0.5, 1.0}` DASH) and must equal it EXACTLY. The new identity is created holding the full `denomination`, funded by decrementing the shielded pool by exactly that amount — a move *between* two balance trees (like `Unshield`'s pool→address), so the global system-credit supply is unchanged (**no** `AddToSystemCredits`); the fee is then **moved** from that balance into the fee pools, so the identity ends with `denomination − total_fee`. Unlike the flat pool-paid transitions, the `AddNewIdentity` write grows with the key count, so the cost is **metered** (not a flat carve) — only the ZK compute fee (`compute_shielded_verification_fee`) is added on top, exactly like the transparent `Shield`. The client predicts it offline with `compute_shielded_identity_create_fee(num_actions, num_keys)`; consensus rejects `denomination < total_fee` with `IdentityInsufficientBalanceError`. |

For `ShieldedTransfer`, the client constructs the bundle so that `total_spent −
total_output = desired_fee`. The Orchard circuit proves that value is conserved
(inputs = outputs + value_balance), and the binding signature cryptographically
commits to the `value_balance`. Mutating `value_balance` after signing will cause
the binding signature to fail verification.

## Entry-Transition Fees (Shield and ShieldFromAssetLock)

The two *entry* transitions — `Shield` (transparent → shielded) and
`ShieldFromAssetLock` (Core asset lock → shielded) — move value **into** the pool, so
there is no spent note from which `value_balance` could carry a fee. Their fees are
therefore charged from the funding side, and both cover the same Halo 2 proof
verification and per-action work the other shielded transitions pay for — but they
account for it differently. **`Shield`** debits a state-queryable transparent address
balance, so GroveDB *meters* its real storage/processing and only the compute portion
(`compute_shielded_verification_fee`, no storage term) is added on top. **`ShieldFromAssetLock`**
is funded by a consumed asset lock with no metering anchor, so it pays the flat
`compute_minimum_shielded_fee(num_actions)` (plus the asset-lock base cost). `num_actions`
is the on-wire action count of the bundle (a single-output, spends-disabled Orchard bundle
pads to 2 actions, so the minimum is the 2-action fee).

### Shield

`Shield` is charged like any other address-funded transition: GroveDB **meters** the
real storage and processing cost of applying it (the note-commitment and nullifier
writes plus the address-balance updates), and the **shielded compute fee** is added on
top:

```
fee = metered_storage + metered_processing + shielded_verification_fee
shielded_verification_fee = proof_verification_fee + num_actions × per_action_processing_fee
```

`shielded_verification_fee` is the ZK-verification cost (Halo 2 proof + per-action spend-auth
verification) that GroveDB metering cannot see. It is added as the transition's
`additional_fixed_fee_cost` — exactly the mechanism `IdentityCreateFromAddresses` uses
for its registration cost. It carries **no storage term**: storage comes entirely from
metering, so it is never double-counted. The address inputs must cover `shield_amount +
fee`, and the booked storage/processing equals the deducted amount, so credits are
conserved by the standard machinery (no special-case override).

`Shield` is skipped by the `value_balance`-based minimum-fee validation (its
`value_balance` is the amount entering the pool, not a fee). The stateless structure
floor requires only `shield_amount + shielded_verification_fee` (a conservative lower bound,
since metered storage is unknowable without state); the authoritative `metered +
compute` funding gate is `validate_fees_of_event`.

### ShieldFromAssetLock

The asset lock funds the pool, so the fee is taken from the consumed asset-lock value.
The pool fee is:

```
pool_fee = compute_minimum_shielded_fee(num_actions) + asset_lock_base_cost
```

`asset_lock_base_cost` is the same asset-lock-proof processing base cost charged to
every asset-lock-funded transition (e.g. `IdentityCreate`):
`required_asset_lock_duff_balance_for_processing_start_for_address_funding`
(50,000 duffs) × `CREDITS_PER_DUFF` (1,000) = **50,000,000 credits**. Adding it makes
the `ShieldFromAssetLock` pool fee strictly greater than the bare `F` paid by the
transparent `Shield`, pricing the extra cost of verifying the Core asset-lock proof.

The asset lock must cover `shield_amount + pool_fee`; the remainder is the **surplus**:

```
surplus = consumed_asset_lock_value − shield_amount − pool_fee   (always ≥ 0)
```

The surplus is disposed of in one of two ways:

- **`surplus_output` set** — the transition carries an optional `Option<PlatformAddress>`
  `surplus_output`. When present, `surplus` is credited to that platform address (via an
  `AddBalanceToAddress` drive operation). This field is **part of the signed payload**
  (it sits before the `signature` field, which alone is excluded from the sighash), so a
  surplus recipient cannot be substituted or truncated after signing.
- **`surplus_output` unset** — the surplus folds into the fee pools, but only up to
  `shielded_implicit_fee_cap` (**20,000,000,000 credits = 0.2 DASH**, a versioned
  constant). If the unclaimed surplus would exceed the cap, the transition is rejected
  with `ShieldedImplicitFeeCapExceededError` so a client cannot accidentally donate a
  large remainder to proposers. To intentionally over-fund, the client must set
  `surplus_output` (which has no cap).

Value conservation across the whole transition is exact:

```
consumed_asset_lock_value = shield_amount + surplus_amount + fee_amount
```

where `surplus_amount` is `surplus` when `surplus_output` is set and `0` otherwise (in
which case the surplus is part of `fee_amount`).

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

The per-action processing fee prices the marginal Halo 2 verification work that
each additional action adds to the bundle (≈1.1 ms/action measured against a
≈5 ms bundle base): a bundle with more actions is a larger circuit and a longer
batch verification. For a spend-bearing action that marginal work includes:
- RedPallas spend authorization signature verification
- Nullifier duplicate check (hash + tree lookup)
- Note commitment insertion into the Sinsemilla-based Merkle tree

Output-only entry transitions (Shield / ShieldFromAssetLock) do no spends or
nullifier checks, but each output action still enlarges the proof and so carries
the same per-action processing charge — this fee tracks the marginal verification
work, not a fixed per-action checklist.

The fee is calibrated at roughly a 4.5:1 ratio against the fixed
proof-verification fee (100M : 22M) rather than the looser ratio used before the
recalibration. (Note the two ratios on this page use different baselines: the
“30×” in §1 is the proof fee relative to a single RedPallas signature
verification, whereas this 4.5:1 is the proof fee relative to the per-action
processing fee.)

**Current value:** `22,000,000` credits (22M)

### 3. Per-Action Storage Fee

Each action permanently stores data in two places:

| Storage | Bytes | Contents |
|---|---|---|
| BulkAppendTree (commitment tree) | 280 | 32 cmx + 32 rho + 216 encrypted note |
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
| 2 | 100,000,000 | 44,000,000 | 17,097,600 | **161,097,600** |
| 3 | 100,000,000 | 66,000,000 | 25,646,400 | **191,646,400** |
| 4 | 100,000,000 | 88,000,000 | 34,195,200 | **222,195,200** |

Note: The Orchard protocol requires a minimum of 2 actions per bundle for privacy
(even a single-input single-output transfer produces 2 actions with a dummy padding
action). Bundles with 1 action are structurally invalid.

The totals above are the **base** `compute_minimum_shielded_fee` and apply directly to
`ShieldedTransfer`. The two pool-paid transitions that write one extra per-transition output add a
flat storage component on top of this base:

- **`Unshield` adds the output-address write cost**: a flat
  `unshield_address_storage_fee = 222 × per_byte_rate = 222 × 27,400 = 6,082,800` credits,
  independent of action count. So the 2-action Unshield fee is
  `161,097,600 + 6,082,800 = 167,180,400` credits (and likewise `+6,082,800` at every action
  count). See the [Fee Extraction](#fee-extraction-by-transition-type) Unshield row for why this
  component exists.
- **`ShieldedWithdrawal` adds the Core withdrawal-document storage cost**: a flat
  `withdrawal_document_storage_fee = 4100 × per_byte_rate = 4100 × 27,400 = 112,340,000` credits,
  independent of action count. So the 2-action ShieldedWithdrawal fee is
  `161,097,600 + 112,340,000 = 273,437,600` credits (and likewise `+112,340,000` at every action
  count). See the [Fee Extraction](#fee-extraction-by-transition-type) ShieldedWithdrawal row for
  why this component exists.

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
    pub shielded_anchor_retention_blocks: u64,
    pub shielded_anchor_pruning_interval: u64,
    pub shielded_proof_verification_fee: u64,      // 100_000_000
    pub shielded_per_action_processing_fee: u64,    // 22_000_000
    pub shielded_implicit_fee_cap: u64,             // 20_000_000_000 (0.2 DASH)
}
```

The `shielded_implicit_fee_cap` bounds the surplus that a `ShieldFromAssetLock` may
implicitly donate to the fee pools when no `surplus_output` is set (see
[Entry-Transition Fees](#entry-transition-fees-shield-and-shieldfromassetlock)).

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
shielded pool's total balance is decremented and the fee is booked via the
`PaidFromShieldedPool` execution event:

```
ShieldedTransfer:    pool_balance -= fee_amount          // fee == value_balance
Unshield:            pool_balance -= unshielding_amount   // gross
ShieldedWithdrawal:  pool_balance -= unshielding_amount   // gross
```

For `Unshield` and `ShieldedWithdrawal`, `unshielding_amount` is the **gross** amount
leaving the pool. Of that, `unshielding_amount − fee_amount` is credited to the output
platform address (`Unshield`) or written into the Core withdrawal document
(`ShieldedWithdrawal`), and `fee_amount` is booked as the transition fee. For `Unshield`
that fee is `compute_shielded_unshield_fee` — the base fee **plus** the flat
`AddBalanceToAddress` output-write storage cost (`+6,082,800` credits), since `Unshield`
also writes the net to a transparent platform address. For `ShieldedWithdrawal` it is
`compute_shielded_withdrawal_fee` — the same base fee **plus** the flat Core
withdrawal-document storage cost (`+112,340,000` credits), since ShieldedWithdrawal also
writes a real document into the withdrawals contract. Validation guarantees
`unshielding_amount ≥ fee_amount` (and, for `ShieldedWithdrawal`, that the net also clears
`MIN_WITHDRAWAL_AMOUNT`), so the subtraction never underflows. Because each fee prices its
extra write, the booking split (storage routed to the storage pool, the remainder paid to
the proposer) covers that write instead of zeroing the proposer's processing reward to
cover it.

For `ShieldedTransfer`, the pool decreases by exactly the fee (the sender's notes are
spent and the recipient's notes are created, but the pool's aggregate balance only drops
by the fee).

In all cases the booked `fee_amount` is split the same way as every other transition's
fee: the storage cost of the permanent shielded writes is routed to the storage pool
(amortized across epochs and subject to the per-epoch fee multiplier at payout), and the
remainder — proof verification plus per-action processing — is the processing fee paid to
the current block proposer.

The two entry transitions do not decrement the pool (they add to it), so their fees are
booked from the funding side instead:

```
Shield:              fee_amount = metered + shielded_verification_fee     // from transparent address inputs
ShieldFromAssetLock: fee_amount = pool_fee (+ unclaimed surplus)     // from the consumed asset lock
```

For `Shield`, the fee is deducted from the transparent address inputs and booked through
the standard `PaidFromAddressInputs` event (deducted == booked, no override): metered
storage and processing, plus the `shielded_verification_fee` folded into processing. For
`ShieldFromAssetLock`, the consumed asset-lock value is partitioned into `shield_amount`
(into the pool), `surplus_amount` (to `surplus_output`, or `0`), and `fee_amount` (to the
fee pools); see [Entry-Transition Fees](#entry-transition-fees-shield-and-shieldfromassetlock).

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
