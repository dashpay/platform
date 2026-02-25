# The Validation Pipeline

When a state transition arrives at a Dash Platform node, it does not get applied
immediately. It must survive a gauntlet of validation stages, each designed to catch
a different category of error. Understanding this pipeline is essential to understanding
how the platform maintains security, prevents abuse, and ensures deterministic state
across all validators.

## Two Entry Points: check_tx and process_proposal

State transitions enter the pipeline through two different doors, depending on when
they are being validated.

**check_tx** runs when a transition first arrives at a node (before entering the mempool)
and again when rechecking mempool contents. It performs a lighter validation:
signature verification, basic structure, and balance checks. The goal is to filter out
obvious garbage cheaply, without doing expensive state lookups.

This is implemented in
`packages/rs-drive-abci/src/execution/validation/state_transition/check_tx_verification/mod.rs`:

```rust
pub(in crate::execution) fn state_transition_to_execution_event_for_check_tx<'a, C: CoreRPCLike>(
    platform: &'a PlatformRef<C>,
    state_transition: StateTransition,
    check_tx_level: CheckTxLevel,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<Option<ExecutionEvent<'a>>>, Error> { ... }
```

**process_proposal** (also called the "Validator" path) runs during block execution.
This is the full pipeline. It is implemented in
`packages/rs-drive-abci/src/execution/validation/state_transition/processor/v0/mod.rs`:

```rust
pub(super) fn process_state_transition_v0<'a, C: CoreRPCLike>(
    platform: &'a PlatformRef<C>,
    block_info: &BlockInfo,
    state_transition: StateTransition,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<ExecutionEvent<'a>>, Error> { ... }
```

Let us walk through the full pipeline, stage by stage.

## Stage 1: Is Allowed

Some state transition types are only available starting from a certain protocol version.
For example, address-based transitions like `IdentityCreateFromAddresses` require
protocol version 11 or higher. The first check asks: is this transition type even
permitted on the current network?

```rust
if state_transition.has_is_allowed_validation()? {
    let result = state_transition.validate_is_allowed(platform, platform_version)?;
    if !result.is_valid() {
        return Ok(ConsensusValidationResult::new_with_errors(result.errors));
    }
}
```

The trait is defined in
`packages/rs-drive-abci/src/execution/validation/state_transition/processor/traits/is_allowed.rs`:

```rust
pub(crate) trait StateTransitionIsAllowedValidationV0 {
    fn has_is_allowed_validation(&self) -> Result<bool, Error>;
    fn validate_is_allowed<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<()>, Error>;
}
```

Transitions like `DataContractCreate` and `IdentityCreate` skip this check entirely
(they have always been allowed). The `Batch` transition has its own is_allowed logic
because certain token operations may be gated.

## Stage 2: Identity Signature Verification

For identity-signed transitions (everything except `IdentityCreate`, `IdentityTopUp`,
and the address-based types), the platform fetches the signer's identity from state
and verifies the signature against one of its registered public keys.

```rust
let mut maybe_identity = if state_transition.uses_identity_in_state() {
    let result = if state_transition.validates_signature_based_on_identity_info() {
        state_transition.validate_identity_signed_state_transition(
            platform.drive,
            transaction,
            &mut state_transition_execution_context,
            platform_version,
        )
    } else {
        state_transition.retrieve_identity_info(...)
    }?;
    if !result.is_valid() {
        return Ok(ConsensusValidationResult::new_with_errors(result.errors));
    }
    Some(result.into_data()?)
} else {
    None
};
```

If the signature does not match, the transition is rejected **without charging a fee**.
This is critical: if someone forges a transition with your identity ID but a wrong
signature, you should not pay for their garbage.

## Stage 3: Address Witness Validation

For address-based transitions, instead of identity signatures, the platform validates
witnesses (proofs of ownership) for the input addresses:

```rust
if state_transition.has_address_witness_validation(platform_version)? {
    let result = state_transition.validate_address_witnesses(
        &mut state_transition_execution_context,
        platform_version,
    )?;
    if !result.is_valid() {
        return Ok(ConsensusValidationResult::new_with_errors(result.errors));
    }
}
```

## Stage 4: Address Balances and Nonces

For transitions that use platform addresses as inputs, the platform verifies that
each input address has sufficient balance and that the provided nonces are correct
(preventing replay):

```rust
let remaining_address_balances =
    if state_transition.has_addresses_balances_and_nonces_validation() {
        let result = state_transition.validate_address_balances_and_nonces(
            platform.drive,
            &mut state_transition_execution_context,
            transaction,
            platform_version,
        )?;
        if !result.is_valid() {
            return Ok(ConsensusValidationResult::new_with_errors(result.errors));
        }
        Some(result.into_data()?)
    } else {
        None
    };
```

## Stage 5: Identity Nonce Validation

Nonces prevent replay attacks. Each identity-signed transition carries a nonce that must
be strictly greater than the last used nonce for that identity (or identity-contract pair).
The platform checks this against the stored nonce in state.

```rust
if state_transition.has_identity_nonce_validation(platform_version)? {
    let result = state_transition.validate_identity_nonces(
        &platform.into(),
        platform.state.last_block_info(),
        transaction,
        &mut state_transition_execution_context,
        platform_version,
    )?;
    if !result.is_valid() {
        return Ok(ConsensusValidationResult::new_with_errors(result.errors));
    }
}
```

Identity create and identity top up skip this check -- they do not have nonces because
the identity may not exist yet.

## Stage 6: Basic Structure Validation

This stage checks that the transition's data is well-formed without looking at platform
state. For example: are all required fields present? Are field values within allowed
ranges? Is the data contract schema valid JSON Schema?

```rust
if state_transition.has_basic_structure_validation(platform_version) {
    let consensus_result = state_transition.validate_basic_structure(
        platform.config.network,
        platform_version,
    )?;
    if !consensus_result.is_valid() {
        return Ok(ConsensusValidationResult::new_with_errors(
            consensus_result.errors,
        ));
    }
}
```

The trait lives in `processor/traits/basic_structure.rs`:

```rust
pub(crate) trait StateTransitionBasicStructureValidationV0 {
    fn validate_basic_structure(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;

    fn has_basic_structure_validation(&self, _platform_version: &PlatformVersion) -> bool {
        true
    }
}
```

`MasternodeVote` skips basic structure validation entirely. `DataContractCreate` and
`DataContractUpdate` conditionally enable it based on the platform version.

## Stage 7: Balance Pre-Check

Before doing expensive state validation, the platform checks that the identity has
enough credits to plausibly pay for this transition. For credit transfers and
withdrawals, this includes checking that the transfer amount plus estimated fees
does not exceed the balance.

```rust
if state_transition.has_identity_minimum_balance_pre_check_validation() {
    let result = state_transition.validate_identity_minimum_balance_pre_check(
        identity, platform_version,
    )?;
    if !result.is_valid() {
        return Ok(ConsensusValidationResult::new_with_errors(result.errors));
    }
}
```

## Stage 8: Advanced Structure Validation (without State)

Some transitions need structural validation that goes beyond basic checks but does not
require reading from platform state. For example, `IdentityUpdate` verifies that
added public keys have valid signatures. `DataContractCreate` validates the contract
schema more deeply.

```rust
if state_transition.has_advanced_structure_validation_without_state() {
    let consensus_result = state_transition.validate_advanced_structure(
        identity, &mut state_transition_execution_context, platform_version,
    )?;
    if !consensus_result.is_valid() {
        // Note: this returns an action so the nonce gets bumped even on failure
        return consensus_result.map_result(|action| {
            ExecutionEvent::create_from_state_transition_action(...)
        });
    }
}
```

An important detail: if advanced structure validation fails, the identity's nonce is
still bumped. This prevents an attacker from replaying a structurally invalid transition
over and over without cost.

## Stage 9: Transform Into Action + Advanced Structure (with State)

For certain transition types (`Batch`, `IdentityCreate`, `MasternodeVote`,
`AddressFundingFromAssetLock`, `IdentityCreateFromAddresses`), the platform needs
to read state *before* it can finish structure validation. For example, a document
create transition needs to fetch the data contract to validate the document against
its schema.

This stage first **transforms** the raw state transition into an action (reading
state in the process), then validates the action's structure:

```rust
let action = if state_transition.has_advanced_structure_validation_with_state() {
    let state_transition_action_result = state_transition.transform_into_action(
        platform, block_info, &remaining_address_balances,
        ValidationMode::Validator, &mut state_transition_execution_context, transaction,
    )?;
    if !state_transition_action_result.is_valid_with_data() {
        return state_transition_action_result.map_result(|action| { ... });
    }
    let action = state_transition_action_result.into_data()?;

    let result = state_transition.validate_advanced_structure_from_state(
        block_info, platform.config.network, &action,
        maybe_identity.as_ref(), &mut state_transition_execution_context,
        platform_version,
    )?;
    if !result.is_valid() {
        return result.map_result(|action| { ... });
    }
    Some(action)
} else {
    None
};
```

We will cover `transform_into_action` in detail in the next chapter.

## Stage 10: State Validation

The final validation stage checks for state-level conflicts. Does a data contract
with this ID already exist? Is there already a document with this unique index value?
Has this asset lock already been spent?

```rust
let result = if state_transition.has_state_validation() {
    state_transition.validate_state(
        action, platform, ValidationMode::Validator, block_info,
        &mut state_transition_execution_context, transaction,
    )?
} else if let Some(action) = action {
    ConsensusValidationResult::new_with_data(action)
} else {
    state_transition.transform_into_action(...)? // For transitions that skipped earlier
};
```

Not all transitions need state validation. `IdentityTopUp`, `IdentityCreditWithdrawal`,
`AddressFundsTransfer`, and several others skip it -- their validation is fully covered
by the earlier stages.

## ConsensusValidationResult: How Errors Accumulate

Throughout the pipeline, errors are communicated through `ConsensusValidationResult`,
defined in `packages/rs-dpp/src/validation/validation_result.rs`:

```rust
pub type ConsensusValidationResult<TData> = ValidationResult<TData, ConsensusError>;
pub type SimpleConsensusValidationResult = ConsensusValidationResult<()>;

pub struct ValidationResult<TData: Clone, E: Debug> {
    pub errors: Vec<E>,
    pub data: Option<TData>,
}
```

Key properties of this type:

- **It can carry data alongside errors.** When `transform_into_action` partially
  succeeds but has warnings, the action is in `data` and the warnings are in `errors`.
- **`is_valid()` checks if `errors` is empty.** Any error means failure.
- **`is_valid_with_data()` checks both.** Valid and has associated data.
- **Errors can be merged** from multiple validation steps using `add_errors()` or `merge()`.
- **Chainable** via `and_then_validation()` for pipeline-style composition.

The `SimpleConsensusValidationResult` alias (where `TData = ()`) is used by validation
stages that just check pass/fail without producing a transformed object.

## ValidationMode

The pipeline behavior varies based on context:

```rust
pub enum ValidationMode {
    CheckTx,       // Mempool admission -- lighter checks
    RecheckTx,     // Periodic mempool revalidation
    Validator,     // Full block execution
    NoValidation,  // Testing/tooling only
}
```

For example, `should_fully_validate_contract_on_transform_into_action()` returns
`true` only in `Validator` mode. During `CheckTx`, the platform skips expensive
contract validation to keep mempool admission fast.

## The Early-Return Pattern

You will notice a consistent pattern throughout the pipeline: each stage checks
`is_valid()` and returns early if validation failed. This is intentional.

When the pipeline returns early due to a signature failure or invalid nonce, the
transition produces **no execution event** -- the user is not charged. This protects
users from being billed for transitions they did not actually submit (forged signatures)
or that are replays (invalid nonces).

But when the pipeline returns early after the nonce has been validated -- for example,
during advanced structure validation or state validation -- it returns a
**nonce-bump action** so the identity still pays a small fee. This prevents a different
attack: submitting thousands of structurally invalid transitions to waste validator
resources for free.

## Rules and Guidelines

**Do:**
- Run the full pipeline in `Validator` mode during block execution. Skipping stages
  can lead to consensus divergence.
- Return early on signature/nonce failures without charging the user.
- Bump the nonce (and charge) when structure or state validation fails after the nonce
  check has passed.
- Use `ConsensusValidationResult` everywhere -- do not use `Result<(), Error>` for
  validation outcomes, because a validation failure is not a system error.

**Do not:**
- Add expensive state reads to basic structure validation. It runs during `check_tx`
  and must be fast.
- Skip `is_allowed` validation -- it is the mechanism that enables protocol upgrades
  to gate new transition types.
- Assume the pipeline order is arbitrary. Each stage depends on information established
  by previous stages (e.g., the identity fetched during signature validation is used
  in balance checks).
- Confuse `ConsensusError` (a validation failure that the user caused) with
  `Error` (a system error like a database failure). The former accumulates in
  `ValidationResult::errors`; the latter propagates via `Result::Err`.
