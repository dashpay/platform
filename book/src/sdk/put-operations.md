# Put Operations

Reading data from Platform is handled by `Fetch`. *Writing* data -- creating documents,
deploying contracts, registering identities -- is handled by the put operation traits.
This chapter covers the write path through the SDK: how state transitions are built,
signed, broadcast, and confirmed.

## The Problem

Writing to Platform is fundamentally different from reading. A read is a simple
request/response: send a query, get back a proof. A write involves multiple steps:

1. Determine the correct nonce for the identity.
2. Build a state transition from the data to be written.
3. Sign the transition with the identity's private key.
4. Broadcast the signed transition to the network.
5. Wait for the transition's result.
6. Authenticate the proof and require the guarantee the application needs.

Each step can fail independently, and the SDK needs to handle all of them coherently.

## The PutDocument Trait

The primary write trait for documents is `PutDocument`, defined in
`packages/rs-sdk/src/platform/transition/put_document.rs`:

```rust
#[async_trait::async_trait]
pub trait PutDocument<S: Signer<IdentityPublicKey>>: Waitable {
    async fn put_to_platform(
        &self,
        sdk: &Sdk,
        document_type: DocumentType,
        document_state_transition_entropy: Option<[u8; 32]>,
        identity_public_key: IdentityPublicKey,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error>;

    async fn put_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        document_type: DocumentType,
        document_state_transition_entropy: Option<[u8; 32]>,
        identity_public_key: IdentityPublicKey,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Document, Error>;
}
```

There are two methods:

- **`put_to_platform`** broadcasts the transition and returns immediately. You get back
  the `StateTransition` that was broadcast but no confirmation that it was applied.
- **`put_to_platform_and_wait_for_response`** broadcasts and then waits for a
  proof-backed result. Ordinary document types require `ExecutionProved` and return
  the executed `Document`. For `indexOnly` types, it accepts `AffectedState` and
  returns the document entry at the proof's block; this does not prove that the put
  executed.

## The Nonce-Build-Broadcast-Wait Pipeline

Let's walk through `put_to_platform` step by step:

### Step 1: Get the Nonce

```rust
let new_identity_contract_nonce = sdk
    .get_identity_contract_nonce(
        self.owner_id(),
        document_type.data_contract_id(),
        true,  // bump_first: increment the nonce
        settings,
    )
    .await?;
```

Every identity has a nonce that increments with each state transition targeting a
specific contract. The SDK caches nonces internally and bumps them optimistically.
`bump_first: true` means "give me the next unused nonce."

The SDK's nonce management is sophisticated:

- Nonces are cached per `(identity_id, contract_id)` pair.
- If the cache is older than the staleness timeout (default 20 minutes), the SDK
  re-fetches from Platform.
- If Platform reports a higher nonce than the cache, the cache is updated.
- A filter mask (`IDENTITY_NONCE_VALUE_FILTER`) is applied to keep the nonce in
  the valid range.

### Step 2: Build the Transition

The SDK decides whether to create a new document or replace an existing one based
on the document's revision:

```rust
let transition = if self.revision().is_some()
    && self.revision().unwrap() != INITIAL_REVISION
{
    // This is an update -- create a replacement transition
    BatchTransition::new_document_replacement_transition_from_document(
        self.clone(),
        document_type.as_ref(),
        &identity_public_key,
        new_identity_contract_nonce,
        settings.user_fee_increase.unwrap_or_default(),
        token_payment_info,
        signer,
        sdk.version(),
        settings.state_transition_creation_options,
    )
} else {
    // This is a new document -- generate entropy and create
    let (document, entropy) = match document_state_transition_entropy {
        Some(entropy) => (self.clone(), entropy),
        None => {
            let mut rng = StdRng::from_entropy();
            let mut document = self.clone();
            let entropy = rng.gen::<[u8; 32]>();
            document.set_id(Document::generate_document_id(
                &document_type.data_contract_id(),
                &document.owner_id(),
                document_type.name(),
                entropy.as_slice(),
                new_identity_contract_nonce,
                sdk.version(),
            )?);
            (document, entropy)
        }
    };

    BatchTransition::new_document_creation_transition_from_document(
        document,
        document_type.as_ref(),
        entropy,
        &identity_public_key,
        new_identity_contract_nonce,
        settings.user_fee_increase.unwrap_or_default(),
        token_payment_info,
        signer,
        sdk.version(),
        settings.state_transition_creation_options,
    )
}?;
```

For new documents, the SDK generates 32 bytes of entropy (unless you provide your own)
and derives the document ID from it and, from protocol version 14, from the identity
contract nonce it just fetched. `sdk.version()` tracks the network's protocol version,
so the SDK switches derivation when the network does. Because the ID depends on the
nonce, the ID on the document you pass in is a placeholder: use the ID of the confirmed
document that `put_to_platform_and_wait_for_response` returns.

The JavaScript SDK follows the same pipeline. `sdk.documents.create` goes through
`put_to_platform_and_wait_for_response` and hands the confirmed document back. An app
that builds the transition itself (to sign it separately or cache the signed bytes)
gets the derivation from `wasm-dpp2`: `new DocumentCreateTransition({ document,
identityContractNonce })` derives the ID from the document's entropy and the nonce for
the network's protocol version (`platformVersion` option, latest by default), writes it
onto the transition and back onto `document`, and the transition is then batched, signed
and broadcast as before. The IDs such a transition carries are final; nothing has to be
hashed on the app side.

### Step 3: Validate Structure

Before broadcasting, the SDK validates the transition's basic structure:

```rust
ensure_valid_state_transition_structure(&transition, sdk.version())?;
```

This catches obvious errors (wrong field types, missing required fields) before
the transition hits the network, saving a round-trip.

### Step 4: Broadcast

```rust
transition.broadcast(sdk, Some(settings)).await?;
```

This sends the serialized transition to a DAPI node.

## The BroadcastStateTransition Trait

`BroadcastStateTransition` is implemented on `StateTransition`. Choose the wait
method according to the guarantee the application needs:

| Method | Behavior |
|---|---|
| `broadcast` | Returns `Ok(())` when the node accepts the submission; this does not prove execution. |
| `wait_for_response` | Verifies a result for an already submitted transition and requires `ExecutionProved`. |
| `wait_for_affected_state` | Verifies a result for an already submitted transition, accepting `AffectedState` or `ExecutionProved`. |
| `broadcast_and_wait` | Broadcasts, then performs the strict execution wait. |
| `broadcast_and_wait_for_affected_state` | Broadcasts, then performs the affected-state wait. |

Each wait method also has a `_with_metadata` variant returning
`(T, ResponseMetadata)`. The signed `metadata.height` identifies the block whose
state the proof authenticates; use it when persisting proof-attested balances.

An `AffectedState` proof authenticates the keys derived from the transition and
their values at the proof's block. It cannot establish that this particular
transition executed. This applies to contract moderation and fee claims,
contract updates, creates and deletes of indexOnly documents, and other
snapshot-only families documented by the trait. Typed moderation, fee-claim and
indexOnly document helpers already choose the affected-state wait.

Strict waits deliberately return `Error::ExecutionNotProved` for a valid
affected-state proof. After a combined call reaches that error, the broadcast
has been accepted; the error establishes neither execution nor failure of the
transition. If the application can use a snapshot, call `wait_for_affected_state`
on the same submitted transition without broadcasting it again. A snapshot is
not an execution receipt or an exactly-once guarantee.

### The Wait Mechanism

Both wait policies use the `WaitForStateTransitionResult` gRPC endpoint with the
transition's hash and request a proof. The shared
`WaitForOutcome::wait_for_outcome_with_metadata` path verifies the GroveDB proof
and trusted quorum signature before returning a `StateTransitionProofOutcome`
and response metadata. The outcome's `guarantee()` is either `ExecutionProved`
or `AffectedState`; the public strict wait applies its guarantee check after
the transport retry loop. Accepting a snapshot does not disable proof verification.

The signature authenticates the state root, protocol version, height, time and
core-chain-locked height through the state ID, and the vote's chain ID, height,
round and block ID. The trusted quorum is selected by its type, hash and
core-chain-locked height. The response's epoch is not signed and is not used to
construct the proof-verification block information. A Rust metadata-returning
wait exposes the signed height alongside that unsigned epoch; the generic
JavaScript waits return the converted result without metadata or a height field.

From protocol version 14 the proof of an owned, fee-paying transition (document
and token batches, contract creates and updates, identity updates and key limit
updates, contract moderation) also carries the credit balance of the owner
after it, read from the same state as the result, so a wallet learns what the
write left it with without a second query. The verified
`StateTransitionProofOutcome` hands it out through `owner_balance()` (`None`
for a proof made at an earlier version or a transition without an owner);
`wait_for_document_and_owner_balance` and
`put_to_platform_and_wait_for_response_with_owner_balance` return it next to
the document. The balance is a snapshot at the proof's block: it may already
include later transitions of the same identity.
`put_to_platform_and_wait_for_response_with_owner_balance_and_metadata` also
returns the verified response metadata, whose `height` is that block, so a
wallet keeping the balance can order it against balances it read elsewhere; it
checks the proof as strictly as `put_to_platform_and_wait_for_response` does
(this write must be proved executed unless the document type is indexOnly),
where `put_to_platform_and_wait_for_response_with_owner_balance` accepts an
affected-state snapshot for any document type. For a DPNS registration,
`Sdk::register_dpns_name_with_owner_balance_and_metadata` returns the
registration result with the balance and metadata of the domain's proof, the
registration's last write. A wait that asks for no proof
but sets `request_user_balance` gets the owner's balance back unverified, as
the response's `success_with_owner_balance` result, read from a Drive state at
or past the block that executed the write; that works for any transition with
an owner.

### Timeout Handling

`wait_for_response` supports an optional timeout:

```rust
match wait_timeout {
    Some(timeout) => {
        tokio::time::timeout(timeout, future)
            .await
            .map_err(|_| Error::TimeoutReached(timeout, details))?
    }
    None => future.await,
}
```

Without a timeout, the wait is unbounded. For production use, always set a timeout
via `PutSettings`.

## The Waitable Trait

`Waitable` provides type-specific post-processing after a broadcast:

```rust
#[async_trait::async_trait]
pub trait Waitable: Sized {
    async fn wait_for_response(
        sdk: &Sdk,
        state_transition: StateTransition,
        settings: Option<PutSettings>,
    ) -> Result<Self, Error>;
}
```

Each type implements this differently:

**DataContract and Vote**: straightforward delegation:

```rust
impl Waitable for DataContract {
    async fn wait_for_response(
        sdk: &Sdk,
        state_transition: StateTransition,
        settings: Option<PutSettings>,
    ) -> Result<DataContract, Error> {
        state_transition.wait_for_response(sdk, settings).await
    }
}
```

**Document**: extracts the single document from the batch transition result:

```rust
impl Waitable for Document {
    async fn wait_for_response(
        sdk: &Sdk,
        state_transition: StateTransition,
        settings: Option<PutSettings>,
    ) -> Result<Self, Error> {
        // Verify this is a batch transition with exactly one document
        let doc_id = /* extract from transition */;

        let mut documents: BTreeMap<Identifier, Option<Document>> =
            state_transition.wait_for_response(sdk, settings).await?;

        documents.remove(&doc_id)
            .ok_or(Error::InvalidProvedResponse(...))?
            .ok_or(Error::InvalidProvedResponse(...))
    }
}
```

**Identity**: handles the "already exists" case specially by falling back to a fetch:

```rust
impl Waitable for Identity {
    async fn wait_for_response(
        sdk: &Sdk,
        state_transition: StateTransition,
        settings: Option<PutSettings>,
    ) -> Result<Self, Error> {
        match state_transition.wait_for_response(sdk, settings).await {
            Ok(identity) => Ok(identity),
            Err(Error::AlreadyExists(_)) => {
                // Identity already exists -- fetch it instead
                let identity_id = /* extract from transition */;
                Identity::fetch(sdk, identity_id).await?
                    .ok_or(Error::Generic("proved to not exist but said to exist"))
            }
            Err(e) => Err(e),
        }
    }
}
```

## Error Handling at the SDK Level

Errors during put operations fall into several categories:

- **Nonce errors**: The cached nonce was stale. The SDK refreshes nonces on broadcast
  failure: `sdk.refresh_identity_nonce(&owner_id).await`
- **Broadcast errors**: The network rejected the transition. Returned as
  `StateTransitionBroadcastError`.
- **Proof errors**: The proof verification failed. Returned as `DriveProofError` with
  the raw proof bytes and block info for debugging.
- **Timeout errors**: The transition was not included in time. Returned as
  `TimeoutReached` with the timeout duration and a description.
- **Conversion errors**: The proof result could not be converted to the expected
  type. Returned as `InvalidProvedResponse`.

## PutSettings

All put operations accept optional `PutSettings`:

```rust
pub struct PutSettings {
    pub request_settings: RequestSettings,
    pub identity_nonce_stale_time_s: Option<u64>,
    pub user_fee_increase: Option<u16>,
    pub wait_timeout: Option<Duration>,
    pub state_transition_creation_options: Option<...>,
}
```

The most important field is `wait_timeout`. In production, always set it to avoid
hanging indefinitely.

## Rules

**Do:**

- Use `put_to_platform_and_wait_for_response` when you need confirmation.
- Use `put_to_platform` when you want fire-and-forget semantics.
- Always set `wait_timeout` in production.
- Let the SDK manage nonces -- do not manually set them.
- Handle `Error::AlreadyExists` gracefully, especially for identity creation.

**Don't:**

- Call `broadcast` without eventually calling `wait_for_response` -- you will not know
  if the transition succeeded.
- Retry a failed transition without refreshing the nonce -- the old nonce may be consumed.
- Set `user_fee_increase` to zero in congested networks -- your transition may be deprioritized.
- Provide custom entropy unless you need deterministic document IDs for testing.
- Ignore `TimeoutReached` errors -- they may indicate network issues that affect
  subsequent operations.
