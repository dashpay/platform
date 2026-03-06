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
5. Wait for the transition to be included in a block.
6. Verify the proof that the write was applied.

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
- **`put_to_platform_and_wait_for_response`** broadcasts and then waits for the
  platform to include the transition in a block, returning the confirmed `Document`.

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
    let (document, entropy) = document_state_transition_entropy
        .map(|e| (self.clone(), e))
        .unwrap_or_else(|| {
            let mut rng = StdRng::from_entropy();
            let mut document = self.clone();
            let entropy = rng.gen::<[u8; 32]>();
            document.set_id(Document::generate_document_id_v0(
                &document_type.data_contract_id(),
                &document.owner_id(),
                document_type.name(),
                entropy.as_slice(),
            ));
            (document, entropy)
        });

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
and uses it to deterministically generate the document ID. This ensures the same
inputs always produce the same document ID.

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

Broadcasting is implemented as a trait on `StateTransition`:

```rust
#[async_trait::async_trait]
pub trait BroadcastStateTransition {
    async fn broadcast(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<(), Error>;

    async fn wait_for_response<T: TryFrom<StateTransitionProofResult> + Send>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<T, Error>;

    async fn broadcast_and_wait<T: TryFrom<StateTransitionProofResult> + Send>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<T, Error>;
}
```

Three methods, three use cases:

- **`broadcast`**: Fire-and-forget. Returns `Ok(())` when the node accepts the
  transition. The response is always empty -- confirmation comes later.
- **`wait_for_response`**: Poll until the transition is included in a block.
  Returns the proven result.
- **`broadcast_and_wait`**: Combines both -- broadcast, then wait.

### The Wait Mechanism

`wait_for_response` uses the `WaitForStateTransitionResult` gRPC endpoint. It sends
the transition's hash and blocks until the platform includes it in a block:

```rust
async fn wait_for_response<T>(&self, sdk: &Sdk, settings: Option<PutSettings>)
    -> Result<T, Error>
{
    let factory = |request_settings: RequestSettings| async move {
        let request = self.wait_for_state_transition_result_request()?;
        let response = request.execute(sdk, request_settings).await?;

        // Check for broadcast errors
        if let Some(e) = state_transition_broadcast_error {
            return Err(Error::from(e));
        }

        // Extract and verify the proof
        let proof = grpc_response.proof()?;
        let (_, result) = Drive::verify_state_transition_was_executed_with_proof(
            self,
            &block_info,
            proof.grovedb_proof.as_slice(),
            &context_provider.as_contract_lookup_fn(sdk.version()),
            sdk.version(),
        )?;

        // Convert to the expected output type
        T::try_from(result)
    };

    retry(sdk.address_list(), retry_settings, factory).await
}
```

The wait includes full proof verification: the SDK verifies a GroveDB proof that
the state transition was actually applied. This is not just checking a status flag --
it is cryptographic proof of inclusion.

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
