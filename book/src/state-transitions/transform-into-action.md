# Transform Into Action

After a state transition passes the early validation stages -- signature verification,
nonce checks, basic structure -- the platform needs to translate it from a raw
"what the client sent" representation into a "what the platform should do" representation.
This translation step is called **transform into action**, and it is where the
platform reads state, resolves references, and prepares a validated, self-contained
instruction for the Drive storage layer.

## Why Actions Exist

Consider a `DataContractCreateTransition`. The client sends the contract in its
serialized form. But before the platform can store it, it needs to:

1. Deserialize the contract from its wire format
2. Validate the contract schema (JSON Schema validation, index rules, etc.)
3. Compute the contract ID from the owner ID and nonce
4. Check version compatibility

The **transition** is what the client sends. The **action** is what the platform
will execute. The action is a validated, resolved, ready-to-apply object. It has
already been through deserialization, its references have been resolved against
current state, and it carries exactly the information needed to generate Drive
operations.

This separation is a deliberate design choice. The transition type lives in `rs-dpp`
(the protocol library, shared between client and platform). The action type lives in
`rs-drive` (the storage layer, platform-only). By keeping them separate:

- Clients never need to depend on storage internals
- The platform can evolve its internal representation without changing the wire format
- Validation logic lives close to state access, not scattered across the protocol layer

## The StateTransitionActionTransformer Trait

The transformation is defined by a trait in
`packages/rs-drive-abci/src/execution/validation/state_transition/transformer/mod.rs`:

```rust
pub trait StateTransitionActionTransformer {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        remaining_address_input_balances: &Option<
            BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        >,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}
```

The key parameters:

- **`platform`** -- Access to the full platform state, including Drive (the storage layer),
  the current platform state, and configuration.
- **`block_info`** -- The current block height, time, epoch. Some validations are
  time-dependent.
- **`remaining_address_input_balances`** -- For address-based transitions, the
  balances remaining after earlier deductions.
- **`validation_mode`** -- Controls how thorough the validation is (lighter for `CheckTx`,
  full for `Validator`).
- **`execution_context`** -- Accumulates information about what operations were performed
  during validation (used for fee estimation).
- **`tx`** -- The GroveDB transaction handle for consistent state reads.

The return type is `ConsensusValidationResult<StateTransitionAction>` -- it can carry
both the resulting action *and* any validation errors encountered during transformation.

## The Top-Level Dispatch

The `StateTransition` enum implements `StateTransitionActionTransformer` by dispatching
to each variant's own implementation:

```rust
impl StateTransitionActionTransformer for StateTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        remaining_address_input_balances: &Option<BTreeMap<PlatformAddress, (AddressNonce, Credits)>>,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        match self {
            StateTransition::DataContractCreate(st) => st.transform_into_action(
                platform, block_info, remaining_address_input_balances,
                validation_mode, execution_context, tx,
            ),
            StateTransition::Batch(st) => st.transform_into_action(
                platform, block_info, remaining_address_input_balances,
                validation_mode, execution_context, tx,
            ),
            StateTransition::IdentityCreate(st) => {
                let signable_bytes = self.signable_bytes()?;
                st.transform_into_action_for_identity_create_transition(
                    platform, signable_bytes, validation_mode,
                    execution_context, tx,
                )
            },
            StateTransition::IdentityTopUp(st) => {
                let signable_bytes = self.signable_bytes()?;
                st.transform_into_action_for_identity_top_up_transition(
                    platform, signable_bytes, validation_mode,
                    execution_context, tx,
                )
            },
            // ... remaining variants
        }
    }
}
```

Notice that `IdentityCreate` and `IdentityTopUp` use specialized method names
(`transform_into_action_for_identity_create_transition`) and pass `signable_bytes`
explicitly. This is because these transitions need the signable bytes to verify
the asset lock proof signature, and computing them at the `StateTransition` level
(before the inner struct is extracted) ensures the correct bytes are used.

## The StateTransitionAction Enum

The output of transformation is a `StateTransitionAction`, defined in
`packages/rs-drive/src/state_transition_action/mod.rs`:

```rust
pub enum StateTransitionAction {
    DataContractCreateAction(DataContractCreateTransitionAction),
    DataContractUpdateAction(DataContractUpdateTransitionAction),
    BatchAction(BatchTransitionAction),
    IdentityCreateAction(IdentityCreateTransitionAction),
    IdentityTopUpAction(IdentityTopUpTransitionAction),
    IdentityCreditWithdrawalAction(IdentityCreditWithdrawalTransitionAction),
    IdentityUpdateAction(IdentityUpdateTransitionAction),
    IdentityCreditTransferAction(IdentityCreditTransferTransitionAction),
    MasternodeVoteAction(MasternodeVoteTransitionAction),
    // ... address-based actions ...
    BumpIdentityNonceAction(BumpIdentityNonceAction),
    BumpIdentityDataContractNonceAction(BumpIdentityDataContractNonceAction),
    PartiallyUseAssetLockAction(PartiallyUseAssetLockAction),
    BumpAddressInputNoncesAction(BumpAddressInputNoncesAction),
}
```

Notice the **system actions** at the bottom: `BumpIdentityNonceAction`,
`BumpIdentityDataContractNonceAction`, `PartiallyUseAssetLockAction`, and
`BumpAddressInputNoncesAction`. These are not user-requested actions. They are
generated by the platform when a transition *fails validation after the nonce check*.
The platform still needs to bump the nonce (so the same invalid transition cannot
be replayed) and charge a fee. These system actions handle that housekeeping.

## Versioned Dispatch Within transform_into_action

Each transition type's `transform_into_action` implementation uses the standard
versioned dispatch pattern. Here is the data contract create example from
`packages/rs-drive-abci/src/execution/validation/state_transition/state_transitions/data_contract_create/mod.rs`:

```rust
impl StateTransitionActionTransformer for DataContractCreateTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        _remaining_address_input_balances: &Option<
            BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        >,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        _tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .contract_create_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0::<C>(
                block_info, validation_mode,
                execution_context, platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "data contract create transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
```

The version number (`0` here) is looked up from the platform version struct, allowing
the logic to change in future protocol upgrades without modifying the dispatch layer.

## What Happens Inside: Reading State

The core work inside `transform_into_action` varies by transition type, but the
common thread is **resolving references against current state**. Here are the key
patterns for different transition types:

**DataContractCreate:** Deserializes the contract from its wire format, validates the
schema, and wraps it in a `DataContractCreateTransitionAction`. The validation depth
depends on `ValidationMode` -- `CheckTx` mode skips full schema validation for speed.

**DataContractUpdate:** Fetches the existing contract from Drive to compare against
the proposed update. Validates that schema changes are backward-compatible (e.g., you
can add fields but not remove required ones).

**Batch (Documents):** This is the most complex transformation. For each document
sub-transition in the batch, the platform must:
1. Fetch the data contract that the document belongs to
2. Look up the document type within that contract
3. For creates: validate the document against the type's schema
4. For replaces/deletes: fetch the existing document to verify ownership and revision
5. For token operations: validate token configuration and authorization

**IdentityCreate:** Validates the asset lock proof against the core chain (via RPC),
verifies the signature on the transition using keys embedded in the transition itself,
and constructs the identity that will be created.

**MasternodeVote:** Fetches the contested resource being voted on, verifies that the
masternode is eligible to vote, and constructs the vote action.

## The Batch Transition: A Deeper Look

The `BatchTransition` is worth examining more closely because it demonstrates how
transform_into_action handles multiple sub-operations. A single batch can contain
dozens of document and token transitions targeting different contracts and document
types.

During transformation, the platform:

1. **Groups transitions by contract.** This allows fetching each contract only once.
2. **Fetches all needed contracts.** Each contract is loaded from Drive (with caching).
3. **Resolves each sub-transition.** Document creates get validated against their
   type's schema. Replaces fetch the existing document. Token operations check
   authorization rules.
4. **Produces a `BatchTransitionAction`** containing individual
   `BatchedTransitionAction` items -- each of which is either a `DocumentAction`
   or a `TokenAction`.

If any sub-transition fails validation, the entire batch fails, and the platform
produces a `BumpIdentityDataContractNonceAction` instead (bumping the nonce to
prevent replay, while charging the user).

## When Transformation Fails

Transformation can fail in two ways:

**System error** (`Result::Err`): Something unexpected happened -- a database read
failed, a version mismatch, corrupted state. These propagate up as `Error` and
typically halt block processing.

**Consensus error** (`ValidationResult` with errors): The transition itself is invalid.
The document does not match its schema, the contract does not exist, the asset lock is
already spent. In this case, the result still carries data -- typically a nonce-bump
action -- so the pipeline can charge the user and move on.

This distinction is reflected in the return type:
`Result<ConsensusValidationResult<StateTransitionAction>, Error>`. The outer `Result`
is for system errors. The inner `ConsensusValidationResult` is for user errors.

## Rules and Guidelines

**Do:**
- Always respect `ValidationMode`. Skip expensive work during `CheckTx` but never
  skip it during `Validator` mode.
- Accumulate validation costs in `execution_context` -- every state read, every
  schema validation -- so that fee calculation is accurate.
- Return a nonce-bump action when transformation fails for user-caused reasons.
  Never silently swallow the transition.
- Fetch contracts through Drive's caching layer. A batch transition might reference
  the same contract dozens of times; fetching it from disk each time would be
  prohibitively expensive.

**Do not:**
- Perform state writes during transform_into_action. This is a read-only phase.
  State is only modified when the resulting action is applied later.
- Mix up the transition and action types. The transition is `DataContractCreateTransition`
  (from `rs-dpp`). The action is `DataContractCreateTransitionAction` (from `rs-drive`).
  They live in different crates for good reason.
- Forget to handle the `remaining_address_input_balances` parameter for address-based
  transitions. It is `None` for identity-based transitions but must be `Some` for
  address-based ones -- failing to provide it is a corrupted code execution error.
- Add new transition types without implementing both the transformer trait and
  adding the variant to the `StateTransitionAction` enum. These must stay in sync.
