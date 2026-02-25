# Builder Pattern

The Dash Platform Rust SDK (`packages/rs-sdk`) is the primary way applications interact
with Dash Platform. Before you can fetch identities, create documents, or broadcast
state transitions, you need an `Sdk` instance. And to create an `Sdk`, you use the
builder pattern.

This chapter covers `SdkBuilder`, the `Sdk` struct it produces, and the two modes
of operation: normal (real network) and mock (testing).

## The Problem

Creating an `Sdk` requires many pieces of configuration:

- Network addresses (where are the DAPI nodes?)
- Network type (mainnet, testnet, devnet, regtest?)
- Request settings (timeouts, retries, ban policies)
- Context provider (where do cached data contracts and quorum keys come from?)
- Staleness checks (how old can metadata be before we reject it?)
- Platform version (which protocol version should we use?)
- Cancellation token (how do we abort pending requests?)
- TLS certificates (for secure connections)

Most of these have sensible defaults. A constructor with 10 parameters would be
unusable. The builder pattern lets you set only what you need.

## SdkBuilder

`SdkBuilder` lives in `packages/rs-sdk/src/sdk.rs`:

```rust
pub struct SdkBuilder {
    addresses: Option<AddressList>,
    settings: Option<RequestSettings>,
    network: Network,
    core_ip: String,
    core_port: u16,
    core_user: String,
    core_password: Zeroizing<String>,
    proofs: bool,
    version: &'static PlatformVersion,
    context_provider: Option<Box<dyn ContextProvider>>,
    metadata_height_tolerance: Option<u64>,
    metadata_time_tolerance_ms: Option<u64>,
    cancel_token: CancellationToken,

    #[cfg(feature = "mocks")]
    data_contract_cache_size: NonZeroUsize,
    #[cfg(feature = "mocks")]
    token_config_cache_size: NonZeroUsize,
    #[cfg(feature = "mocks")]
    quorum_public_keys_cache_size: NonZeroUsize,
    #[cfg(feature = "mocks")]
    dump_dir: Option<PathBuf>,

    #[cfg(not(target_arch = "wasm32"))]
    ca_certificate: Option<Certificate>,
}
```

### Constructor Methods

The builder offers several constructors for different scenarios:

```rust
// Normal mode: connect to specified DAPI nodes
let sdk = SdkBuilder::new(address_list)
    .with_network(Network::Testnet)
    .build()?;

// Mock mode: no network, useful for tests
let sdk = SdkBuilder::new_mock()
    .build()?;

// Convenience (not yet implemented):
let sdk = SdkBuilder::new_testnet().build()?;
let sdk = SdkBuilder::new_mainnet().build()?;
```

The key distinction: if `addresses` is `Some`, you get a real `DapiClient` that
connects to DAPI nodes over gRPC. If `addresses` is `None` (the mock path), you get
a `MockDapiClient` that responds with pre-programmed data.

### Configuration Methods

Every builder method follows the same signature pattern: take `mut self`, modify a
field, return `self`:

```rust
impl SdkBuilder {
    pub fn with_network(mut self, network: Network) -> Self {
        self.network = network;
        self
    }

    pub fn with_settings(mut self, settings: RequestSettings) -> Self {
        self.settings = Some(settings);
        self
    }

    pub fn with_version(mut self, version: &'static PlatformVersion) -> Self {
        self.version = version;
        self
    }

    pub fn with_context_provider<C: ContextProvider + 'static>(
        mut self,
        context_provider: C,
    ) -> Self {
        self.context_provider = Some(Box::new(context_provider));
        self
    }

    pub fn with_cancellation_token(
        mut self,
        cancel_token: CancellationToken,
    ) -> Self {
        self.cancel_token = cancel_token;
        self
    }
}
```

This lets you chain configuration fluently:

```rust
let sdk = SdkBuilder::new(addresses)
    .with_network(Network::Testnet)
    .with_version(PlatformVersion::latest())
    .with_settings(RequestSettings { retries: Some(5), ..Default::default() })
    .with_context_provider(my_provider)
    .with_cancellation_token(token)
    .build()?;
```

### Staleness Configuration

The SDK protects against stale responses from out-of-date nodes:

```rust
// Reject responses whose height is behind by more than 1 block
let sdk = SdkBuilder::new(addresses)
    .with_height_tolerance(Some(1))     // default
    .with_time_tolerance(Some(360_000)) // 6 minutes
    .build()?;
```

Height tolerance defaults to `Some(1)` -- if a node returns metadata with a height
more than 1 block behind the last seen height, the SDK considers it stale. Time
tolerance defaults to `None` (disabled) because it requires synchronized clocks.

### Dash Core Integration

For development, the SDK can use Dash Core as a wallet and context provider:

```rust
let sdk = SdkBuilder::new(addresses)
    .with_core("127.0.0.1", 19998, "user", "password")
    .build()?;
```

This is a convenience method that internally creates a `GrpcContextProvider` backed
by Core's RPC interface. For production, you should implement `ContextProvider` yourself.

### Dump Directory

For debugging, the SDK can record all gRPC requests and responses to disk:

```rust
let sdk = SdkBuilder::new(addresses)
    .with_dump_dir(Path::new("./sdk-dumps"))
    .build()?;
```

This creates files like `msg-*.json`, `quorum_pubkey-*.json`, and
`data_contract-*.json` that can be replayed in mock mode.

## The Sdk Struct

The `build()` method produces an `Sdk`:

```rust
pub struct Sdk {
    pub network: Network,
    inner: SdkInstance,
    proofs: bool,
    internal_cache: Arc<InternalSdkCache>,
    context_provider: ArcSwapOption<Box<dyn ContextProvider>>,
    metadata_last_seen_height: Arc<atomic::AtomicU64>,
    metadata_height_tolerance: Option<u64>,
    metadata_time_tolerance_ms: Option<u64>,
    pub(crate) cancel_token: CancellationToken,
    pub(crate) dapi_client_settings: RequestSettings,
}
```

### SdkInstance: Normal vs Mock

The `inner` field is an enum that holds either a real or mock client:

```rust
enum SdkInstance {
    Dapi {
        dapi: DapiClient,
        version: &'static PlatformVersion,
    },
    #[cfg(feature = "mocks")]
    Mock {
        dapi: Arc<Mutex<MockDapiClient>>,
        mock: Arc<Mutex<MockDashPlatformSdk>>,
        address_list: AddressList,
        version: &'static PlatformVersion,
    },
}
```

All public `Sdk` methods work identically in both modes. Code that uses the SDK
does not know (or care) whether it is talking to a real network or a mock.

### Thread Safety

`Sdk` is `Clone` and thread-safe. It uses `Arc` for shared state and `ArcSwapOption`
for the context provider (which allows lock-free reads). The mock mode uses `tokio::Mutex`
for the mock client since mock state is modified in async contexts.

### Nonce Management

The SDK maintains an internal cache of identity nonces to avoid querying the network
on every state transition:

```rust
pub async fn get_identity_nonce(
    &self,
    identity_id: Identifier,
    bump_first: bool,
    settings: Option<PutSettings>,
) -> Result<IdentityNonce, Error> {
    // 1. Check cache
    // 2. If stale or absent, query Platform
    // 3. Optionally bump (increment) before returning
    // 4. Apply IDENTITY_NONCE_VALUE_FILTER mask
}
```

The cache has a staleness timeout (default: 20 minutes). When `bump_first` is true,
the nonce is incremented before being returned -- this is used when creating new
state transitions that need the *next* nonce value.

## The Quick Mock Path

For tests that need a mock SDK immediately:

```rust
let sdk = Sdk::new_mock();
```

This is a shorthand for `SdkBuilder::default().build().unwrap()`. It creates an
SDK in mock mode with all default settings. You can then configure expectations:

```rust
let mut sdk = Sdk::new_mock();
sdk.mock().expect_fetch(identity, None);
```

## Request Settings

The SDK applies a chain of settings to every request:

```rust
const DEFAULT_REQUEST_SETTINGS: RequestSettings = RequestSettings {
    retries: Some(3),
    timeout: None,
    ban_failed_address: None,
    connect_timeout: None,
    max_decoding_message_size: None,
};
```

When building, user-provided settings override defaults:

```rust
let dapi_client_settings = match self.settings {
    Some(settings) => DEFAULT_REQUEST_SETTINGS.override_by(settings),
    None => DEFAULT_REQUEST_SETTINGS,
};
```

And when making individual requests, per-request settings override global settings:

```rust
let settings = sdk
    .dapi_client_settings
    .override_by(request_specific_settings);
```

This three-level cascade (defaults -> builder -> per-request) gives you control
without verbosity.

## Rules

**Do:**

- Use `SdkBuilder::new(addresses)` for production code with real DAPI connections.
- Use `Sdk::new_mock()` for quick unit tests.
- Set `with_context_provider()` in production -- the fallback to Core RPC is for development only.
- Use `with_height_tolerance()` to detect stale nodes.
- Clone the SDK freely -- it is designed for shared ownership via `Arc`.

**Don't:**

- Use `new_testnet()` or `new_mainnet()` -- they are not implemented yet.
- Disable proofs (`with_proofs(false)`) in production -- proofs are the security model.
- Set `metadata_time_tolerance_ms` too low -- network delays and time skew can cause
  false positives.
- Forget the cancellation token in long-running applications -- without it, you cannot
  gracefully shut down pending requests.
- Construct `Sdk` directly -- always use the builder.
