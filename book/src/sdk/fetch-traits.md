# Fetch Traits

Reading data from Dash Platform is the most common SDK operation. You need to look up
an identity by its identifier, retrieve a data contract, query documents, check a
balance. The SDK provides a unified abstraction for all of these: the `Fetch` and
`FetchMany` traits.

This chapter covers how these traits work, how queries are formed, how proofs are
verified, and how different Platform types plug into the system.

## The Problem

Platform stores many different types of data: identities, data contracts, documents,
balances, epoch info, votes, token configurations, and more. Each requires a different
gRPC request, returns a different response, and needs different proof verification
logic.

Without an abstraction, every type would need its own fetch function with duplicated
retry logic, proof parsing, metadata validation, and error handling. The `Fetch` trait
eliminates that duplication.

## The Fetch Trait

`Fetch` is defined in `packages/rs-sdk/src/platform/fetch.rs`:

```rust
#[async_trait::async_trait]
pub trait Fetch
where
    Self: Sized
        + Debug
        + MockResponse
        + FromProof<
            <Self as Fetch>::Request,
            Request = <Self as Fetch>::Request,
            Response = <<Self as Fetch>::Request as DapiRequest>::Response,
        >,
{
    /// The gRPC request type used to fetch this object.
    type Request: TransportRequest
        + Into<<Self as FromProof<<Self as Fetch>::Request>>::Request>;

    /// Fetch a single object from Platform.
    async fn fetch<Q: Query<<Self as Fetch>::Request>>(
        sdk: &Sdk,
        query: Q,
    ) -> Result<Option<Self>, Error> {
        Self::fetch_with_settings(sdk, query, RequestSettings::default()).await
    }

    /// Fetch with metadata (block height, time, etc.)
    async fn fetch_with_metadata<Q: Query<<Self as Fetch>::Request>>(
        sdk: &Sdk,
        query: Q,
        settings: Option<RequestSettings>,
    ) -> Result<(Option<Self>, ResponseMetadata), Error> { /* ... */ }

    /// Fetch with metadata and the raw proof.
    async fn fetch_with_metadata_and_proof<Q: Query<<Self as Fetch>::Request>>(
        sdk: &Sdk,
        query: Q,
        settings: Option<RequestSettings>,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error> { /* ... */ }

    /// Fetch with custom request settings.
    async fn fetch_with_settings<Q: Query<<Self as Fetch>::Request>>(
        sdk: &Sdk,
        query: Q,
        settings: RequestSettings,
    ) -> Result<Option<Self>, Error> { /* ... */ }

    /// Convenience: fetch by identifier.
    async fn fetch_by_identifier(
        sdk: &Sdk,
        id: Identifier,
    ) -> Result<Option<Self>, Error>
    where
        Identifier: Query<<Self as Fetch>::Request>,
    {
        Self::fetch(sdk, id).await
    }
}
```

### The Key Insight: `Option` Semantics

Notice the return type: `Result<Option<Self>, Error>`.

- `Ok(Some(item))` -- the object was found and verified.
- `Ok(None)` -- the object was *proven to not exist*. This is not an error; it is a
  cryptographic proof of absence.
- `Err(error)` -- something went wrong (network failure, proof verification failed, etc.)

This design means "not found" is a normal, expected outcome. Code that uses `Fetch`
does not need to handle "not found" as an error case.

### Usage

```rust
use dash_sdk::platform::{Fetch, Identifier, Identity};

// Fetch an identity
let identity = Identity::fetch(&sdk, some_identifier).await?;

match identity {
    Some(id) => println!("Found identity with balance: {}", id.balance()),
    None => println!("Identity does not exist"),
}
```

## Implementing Fetch for a Type

For most types, implementing `Fetch` is a one-liner:

```rust
impl Fetch for Identity {
    type Request = IdentityRequest;
}

impl Fetch for dpp::prelude::DataContract {
    type Request = platform_proto::GetDataContractRequest;
}

impl Fetch for drive_proof_verifier::types::IdentityBalance {
    type Request = platform_proto::GetIdentityBalanceRequest;
}

impl Fetch for drive_proof_verifier::types::IdentityNonceFetcher {
    type Request = platform_proto::GetIdentityNonceRequest;
}

impl Fetch for ExtendedEpochInfo {
    type Request = platform_proto::GetEpochsInfoRequest;
}

impl Fetch for Vote {
    type Request = platform_proto::GetContestedResourceIdentityVotesRequest;
}

impl Fetch for drive_proof_verifier::types::TotalCreditsInPlatform {
    type Request = platform_proto::GetTotalCreditsInPlatformRequest;
}
```

The `type Request` associates each fetchable type with its gRPC request message.
The default method implementations handle everything else -- sending the request,
parsing the proof, verifying metadata. All you need to provide is the request type.

### Document: A Custom Override

Documents are special because they depend on a data contract schema for deserialization.
If the cached contract is outdated, deserialization fails. The `Document` implementation
overrides the default to add retry logic:

```rust
#[async_trait::async_trait]
impl Fetch for Document {
    type Request = DocumentQuery;

    async fn fetch_with_metadata_and_proof<Q: Query<<Self as Fetch>::Request>>(
        sdk: &Sdk,
        query: Q,
        settings: Option<RequestSettings>,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error> {
        let document_query: DocumentQuery = query.query(sdk.prove())?;

        // First attempt with current (possibly cached) contract
        match fetch_request(sdk, &document_query, settings).await {
            Ok(result) => Ok(result),
            Err(e) if is_document_deserialization_error(&e) => {
                // Contract schema might have changed -- refetch it
                let fresh_query =
                    refetch_contract_for_query(sdk, &document_query).await?;
                fetch_request(sdk, &fresh_query, settings).await
            }
            Err(e) => Err(e),
        }
    }
}
```

If deserialization fails with a `CorruptedSerialization` error, the SDK refetches the
data contract from the network, updates the cache, and retries. This handles the case
where a contract was updated but the local cache still has the old version.

## The Query Trait

`Query` converts user-friendly search criteria into gRPC request messages:

```rust
pub trait Query<T: TransportRequest + Mockable>: Send + Debug + Clone {
    fn query(self, prove: bool) -> Result<T, Error>;
}
```

The simplest implementation: any `TransportRequest` is a query for itself:

```rust
impl<T> Query<T> for T
where
    T: TransportRequest + Sized + Send + Sync + Clone + Debug,
{
    fn query(self, prove: bool) -> Result<T, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported");
        }
        Ok(self)
    }
}
```

But you can also implement `Query` for more ergonomic types. For example, `Identifier`
implements `Query<GetIdentityRequest>`, so you can write:

```rust
let identity = Identity::fetch(&sdk, my_identifier).await?;
```

Instead of manually constructing a `GetIdentityRequest` proto message.

## The FromProof Trait

Every `Fetch` implementation requires that the fetched type implements `FromProof`.
This trait, defined in `packages/rs-drive-proof-verifier/src/proof.rs`, verifies the
cryptographic proof returned by the Platform node:

```rust
pub trait FromProof<Req> {
    type Request;
    type Response;

    /// Parse and verify the proof, returning the requested object.
    ///
    /// Returns:
    /// - Ok(Some(object, metadata)) when found
    /// - Ok(None) when proven to not exist
    /// - Err when verification fails
    fn maybe_from_proof_with_metadata(
        request: Self::Request,
        response: Self::Response,
        network: Network,
        platform_version: &PlatformVersion,
        provider: &impl ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: Sized;
}
```

The chain is: `Query` produces a request -> DAPI returns a response with a proof ->
`FromProof` verifies the proof and extracts the object. Every step is type-safe and
generic over the specific Platform type being fetched.

## FetchMany: Retrieving Collections

`FetchMany` extends the pattern to collections:

```rust
pub trait FetchMany<K: Ord, O: FromIterator<(K, Option<Self>)>>
where
    Self: Sized,
    O: MockResponse
        + FromProof<Self::Request, ...>
        + Send
        + Default,
{
    type Request: TransportRequest;

    async fn fetch_many<Q: Query<Self::Request>>(
        sdk: &Sdk,
        query: Q,
    ) -> Result<O, Error> { /* ... */ }

    // ... with_settings, with_metadata, with_limit variants
}
```

The `O` type parameter is the output collection type. It must implement
`FromIterator<(K, Option<Self>)>` -- a collection of key-value pairs where the value
might be `None` (proven absent). This handles queries like "fetch documents matching
these criteria" where some requested items might not exist.

## The Internal Fetch Pipeline

When you call `Identity::fetch(&sdk, id)`, here is what happens:

1. **Query conversion**: `id.query(true)` produces a `GetIdentityRequest` with proofs
   enabled.

2. **Request execution with retry**: The SDK sends the request to a DAPI node,
   with automatic retry logic:

   ```rust
   let fut = |settings: RequestSettings| async move {
       let response = request.clone().execute(sdk, settings).await?;
       let (object, metadata, proof) = sdk
           .parse_proof_with_metadata_and_proof(request, response)
           .await?;
       Ok((object, metadata, proof))
   };

   retry(sdk.address_list(), settings, fut).await
   ```

3. **Proof verification**: `parse_proof_with_metadata_and_proof` calls
   `FromProof::maybe_from_proof_with_metadata`, which verifies the GroveDB proof
   against quorum signatures.

4. **Metadata validation**: The SDK checks that the response metadata (height, time)
   is fresh enough based on the configured tolerances.

5. **Result return**: The verified object (or `None`) is returned to the caller.

## Rules

**Do:**

- Use `Fetch::fetch()` for single-object lookups by identifier.
- Use `FetchMany::fetch_many()` for queries that return collections.
- Handle `Ok(None)` as a normal case -- it means the object does not exist, proven
  cryptographically.
- Implement `Fetch` for new types by specifying just `type Request`.
- Override `fetch_with_metadata_and_proof` only when you need custom logic (like
  the Document retry pattern).

**Don't:**

- Treat `Ok(None)` as an error -- "not found" is a valid, proven result.
- Bypass the `Fetch` trait to make raw gRPC calls -- you would skip proof verification.
- Forget to implement `FromProof` for new fetchable types -- without it, proofs
  cannot be verified.
- Disable proofs in production -- `query(prove: false)` is not supported and will panic.
- Implement `Query` conversions that lose information -- the query must fully
  specify what to fetch.
