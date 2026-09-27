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
| Token uses a perpetual distribution | 10,000,000,000 | 0.1 Dash |
| Token uses a pre-programmed distribution | 10,000,000,000 | 0.1 Dash |
| Token uses a once-per-identity distribution (protocol version 14+) | 10,000,000,000 | 0.1 Dash |
| Search keyword | 10,000,000,000 | 0.1 Dash |

Before protocol version 9, all registration fees were zero.

### Contest Funds

A document create that opens or joins a contest (a contested unique index)
prefunds the masternode votes: the amount leaves the contender's balance for the
contest's prefunded balance, each vote takes a fixed cost from it, and what is
left when the contest ends is released as processing fees. The amounts are
`VoteResolutionFundFees` in the fee version:

| Component | Protocol versions 1 to 13 | Protocol version 14 |
|---|---|---|
| Contested document fund (DPNS and every other contest) | 0.2 Dash | 0.1 Dash |
| Moderation election fund (an `electedCharter` application) | none exist | 0.5 Dash |
| One vote | 0.0001 Dash | 0.00002 Dash |

`required_vote_resolution_fund` in `rs-dpp` picks between the two funds; the
schedules before 14 carry the contested document amount in the moderation
field, so the choice changes nothing there.

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
| `Paid` | Identity credit balance, or the contract owner's for a sponsored document batch (below) | Most identity-based transitions |
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

### Gas paid by the contract owner

From protocol version 14 a document action that is paid for with a token can
have its gas (the storage and processing fee) paid by the contract owner. The
document type's token cost offers it (`gasFeesPaidBy`: `DocumentOwner`,
`ContractOwner` or `PreferContractOwner`) and the transition's
`$tokenPaymentInfo` asks for it with the same enum; `GasFeesPaidBy::resolve`
in `rs-dpp` names the payer. A document owner can always opt out, can always
state a preference, and can insist (`ContractOwner`) only on a document type
that commits to paying. Only a token-paid action can be sponsored, so every
sponsored transition is backed by a token the contract owner chose to hand out.

The batch transformer (v2) resolves one payer for the whole batch, reads the
contract owner's balance into the action (`ResolvedGasSponsor`, billed to the
batch) and the execution event carries it in `Paid.gas_sponsor`. Fee
validation v1 judges the fee against the sponsor's balance and the signer only
has to fund `removed_balance`; when the sponsor's balance falls short a batch
that insists is refused unpaid (`GasSponsorInsufficientBalanceError`, 40222)
and a batch that prefers falls back to the signer's balance. Execution v1 then
charges whoever fee validation admitted. A batch that fails validation is never
sponsored: its signer pays for the work that ran, and a request the document
type does not offer is a paid rejection (`GasFeesPaidByNotAllowedError`,
40129). Storage refunds still go to the document's owner, whoever paid the
storage: a sponsored document refunds its owner when it is deleted or replaced
by a smaller one, even when the sponsor pays for that transition too. Each
token the sponsor hands out is therefore worth up to the storage fee of the
largest document the type allows, so a document type that offers sponsorship
should bound its documents' size (`maxLength`, `maxItems`) and price the
action accordingly.

The signer's minimum balance pre-check runs before the contracts are loaded;
its v1 asks a batch that requests sponsorship for its principal only
(purchases, contest collateral) and leaves the gas to fee validation, so an
identity without credits can act on tokens it was given. Such a signer
could not pay for a failed batch, and a failed batch is never sponsored, so
check tx validates the batch of a signer under the fee minimum against the
state in full, on the first check and on every recheck, as it does a
masternode vote: what nobody could be charged for is refused there, or leaves
the mempool once the tokens it counted on are spent, instead of being executed
for free by a proposer.

### Optional token costs

A token cost may declare `optional: true` (v3 meta-schema, protocol version
14). A transition on such an action may leave `$tokenPaymentInfo` out: it then
pays no token, its signer pays the gas in credits as on an action without a
token cost, and no sponsorship applies. With `$tokenPaymentInfo` present the
token is charged exactly as for a required cost, sponsorship included, and an
insufficient token balance is a rejection rather than a fallback to credits:
the client chooses between token and credits before signing. Together with
contract-owner gas this is the "free usage" pattern: an app hands out tokens,
a user posts for free while they last, and keeps posting on credits after.

### Document action fees

From protocol version 14 a document type may charge a fixed fee in credits for
an action on one of its documents, on top of the gas. The `actionFees` keyword
(v3 document meta-schema) sits beside `tokenCost` and prices the same six
actions:

```json
"post": {
  "type": "object",
  "actionFees": {
    "pricing": "feeMultiplier",
    "create": { "moderators": 100000000, "owner": 10000000 }
  }
}
```

Creating a post here costs an extra 0.001 Dash for the contract's moderation
team and 0.0001 Dash for its owner. Each fee has those two parts, either of
which may be left out. The `owner` parts collect in the contract's **owner
pot** and the `moderators` parts in its **moderators pot**, and a
`ContractFeeClaim` state transition pays a pot out (see
[Contract Moderation](../data-model/contract-moderation.md#fee-pots-and-the-claim)).
A `moderators` part needs a contract that declares moderation
(`DocumentActionFeesWithoutModerationError`, 10902): the moderation team is
who that pot is for.

**Pricing.** `fixed` charges the declared amounts as written. `feeMultiplier`,
the default, scales them by the fee multiplier of the epoch the action
executes in (`declared * multiplier_permille / 1000`, rounded down), so a fee
follows the network's fees. The multiplier is the item every epoch tree
records, read once per batch and billed to it
(`fetch_action_fee_multiplier_with_fee`). In the first block of an epoch that
item is not there yet, because state transitions execute before the end of the
block, where the epoch is initialized; the multiplier the epoch is about to be
initialized with, the fee schedule's, is used then. Nothing else reads the
epoch multiplier today: the metered fees do not scale with it. A scaled
amount is held at the maximum number of credits rather than overflowing: a fee
nobody can pay refuses the action for an insufficient balance, a consensus
error, where an overflow would have failed every transition on the action with
an internal one.

**The transition agrees to the fee.** The contract is read when the action
executes, not when the transition was signed, so a transition that said
nothing would pay whatever the contract declares by then. Every transition on
an action that charges a fee therefore carries an *action fee agreement*
(`$actionFeeAgreement`, on version 2 of the document base transition, the
default from protocol version 14):

```json
"$actionFeeAgreement": {
  "$formatVersion": "0",
  "owner": 10000000,
  "moderators": 100000000,
  "feeMultiplier": { "knownPermille": 1000, "increaseTolerancePercent": 20 }
}
```

- `owner` and `moderators` are the amounts the document type declares for the
  action, before any multiplier. They must **match exactly**, each pot on its
  own: a fee that was raised, lowered, or moved between the pots since the
  signer read the contract refuses the action
  (`DocumentActionFeeAgreementMismatchError`, 40133), and the signer reads the
  contract again.
- `feeMultiplier` says how the fee is priced. It is named for a
  `feeMultiplier` fee and left out for a `fixed` one, and an agreement to the
  other pricing is the same mismatch. `knownPermille` is the fee multiplier the
  signer priced the fee with, and `increaseTolerancePercent` how far above it
  the multiplier of the executing epoch may be, in percent of the known one:
  20 accepts up to 1.2 times. A transition signed just before an epoch
  boundary is then not refused for a small move; one the multiplier outran is
  (`DocumentActionFeeMultiplierNotToleratedError`, 40134). A multiplier that
  fell is always accepted. What is charged follows the epoch's multiplier,
  never the known one.
- A transition without an agreement on an action that charges a fee is
  refused (`DocumentActionFeeAgreementNotSetError`, 40132), whoever pays: a
  sponsored action says what it agrees to as well, since a preferred sponsor
  can hand the fee back to the signer. An agreement on an action that charges
  nothing is ignored.

All three are paid refusals that bump the nonce and charge no fee. They are
judged in the batch's advanced structure validation
(`BatchTransitionAction::validate_action_fee_agreements`), off the action
alone: the base action carries the declaration beside the agreement, and the
batch action the multiplier its transformer read. The mempool runs the same
check when a transition arrives and again on every recheck, so a transition
whose agreement no longer holds leaves the mempool with the same error instead
of failing in the block that would have refused it. A client builds the
agreement from the contract it showed its user with
`DocumentActionFeeAgreement::for_document_type_action`, never from a contract
fetched behind their back at signing time.

**A seated team's discount.** On a document type an elected contract
moderates, the `moderators` part of an agreement may name less than the
declared amount: the share the contract's seated moderation charter takes
(its proposal's `moderatorsShare`, a percentage; none declared is the full
amount), applied to the declared amount and rounded down to the credit
(`moderation_charter::moderators_share_of`). Everything else must still match:
the `owner` part and the pricing. With a share of 60, the post above admits
exactly 60000000 for the moderators:

```json
"$actionFeeAgreement": {
  "$formatVersion": "0",
  "owner": 10000000,
  "moderators": 60000000,
  "feeMultiplier": { "knownPermille": 1000, "increaseTolerancePercent": 20 }
}
```

The action is then charged the agreed amount, which is what reaches the
moderators pot (scaled by the multiplier for a `feeMultiplier` fee, like the
declared amount). An agreement to the declared amount stays valid whatever the
team charges and reads no charter; only one that names less has the batch
transformer read the seated charter (the `byTargetContract` index of the
moderation charters contract) and the proposal it runs on, billed to the
batch. Any other amount below the declared one, including a discount on a
contract with no seated charter yet, is refused like a mismatch, paid and
without a fee (`DocumentActionFeeModeratorsShareMismatchError`, 40139). A
lower amount anywhere else (a type the contract does not moderate, a contract
that is not elected) is the plain mismatch (40133). The mempool judges it
the same way on arrival and on every recheck, since the recheck transforms the
batch anew.

**The amounts do not change yet.** A contract update may not add, change or
remove the `actionFees` of an existing document type, nor switch their pricing
(`DocumentTypeUpdateError`). A document type *added* by an update may declare
its own, so a live contract gets fees through new document types only. The
agreement is what makes lifting this safe later: an owner who changes a fee
cannot make a transition signed against the old one pay the new one.

**Who pays.** Whoever pays the gas pays the action fee: the signer, or the
contract owner when they sponsor the gas. A sponsor's balance has to cover the
gas *and* the fees they would owe; one that insisted and falls short is the
same unpaid refusal as before (40222), and one that only preferred hands the
gas and the fees back to the signer. The gas is estimated for the payer:
first with the sponsor paying, then, when the sponsor does not pay, with the
signer paying. Fee validation settles the payer through one function
(`gas_sponsor_pays`) and hands it to execution, which charges that payer, so
they always name the same one. The contract owner never pays the `owner` part:
it would travel through the owner pot back to them and only cost writes. A
sponsor is always the contract owner, so a sponsored action pays into the
moderators pot only, which a contract that sponsors gas should price in. The
fee counts against the budget of a budgeted signing key when its identity pays
it.

**Only an action that executes is charged.** A transition that fails, in the
transformer or later in state validation, becomes a nonce bump, and a bump owes
nothing. The fees are therefore read off the transitions when the execution
event is built, after state validation had its say, not when the transformer
ran.

**The fee is not part of the `FeeResult`.** It moves as balance operations in
the batch's own operation list: one removal from the payer, one addition per
pot. The transition may already write the payer's balance: a purchase moves
its price, a contested document its voting fund, and a sale pays a contract
owner who may be sponsoring the gas. A balance operation computes the new
balance from the one committed before its batch and GroveDB keeps only the
last write of a key, so `apply_drive_operations` (generation 1, protocol
version 14) merges every write of one identity balance, one fee pot or one
prefunded specialized balance in a batch into a single net operation. Token
writes cannot be merged that way (a transfer writes two balances, a mint or a
burn a balance and the supply), so a batch that writes one token balance or
supply twice is refused; no state transition makes one. The fee pools and the proposers see
exactly what they saw before. The
pots sit under the `PreFundedSpecializedBalances` root sum tree, which the
per-block total credits check already sums, so the credits stay accounted for
while they wait to be claimed.

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
| `rs-platform-version/src/version/fee/data_contract_registration/v3.rs` | Protocol version 14 addition: once-per-identity distribution surcharge |
| `rs-drive/src/fees/op.rs` | LowLevelDriveOperation and cost calculation |
| `rs-dpp/src/fee/fee_result/mod.rs` | FeeResult, BalanceChangeForIdentity |
| `rs-dpp/src/fee/epoch/distribution.rs` | Epoch distribution table and refund logic |
| `rs-drive-abci/src/execution/types/execution_event/mod.rs` | ExecutionEvent enum |
| `rs-drive-abci/src/execution/platform_events/fee_pool_inwards_distribution/` | Block fee collection |
| `rs-drive-abci/src/execution/platform_events/fee_pool_outwards_distribution/` | Proposer payout |
