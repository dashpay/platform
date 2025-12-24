# Trusted Context Provider for Dash SDK

This crate provides a trusted HTTP-based context provider for the Dash SDK that fetches quorum information from trusted HTTP endpoints instead of requiring Core RPC access.

## Features

- Fetches quorum public keys from trusted HTTP endpoints
- Supports mainnet, testnet, and devnet networks
- Allows custom URLs for your own trusted endpoints
- LRU caching for quorum data
- Optional fallback provider for data contracts and token configurations
- Domain resolution verification during initialization

## Networks Supported

- **Mainnet**: Uses `https://quorums.mainnet.networks.dash.org/`
- **Testnet**: Uses `https://quorums.testnet.networks.dash.org/`
- **Devnet**: Uses `https://quorums.devnet.<devnet_name>.networks.dash.org/`

## Usage

### Basic Usage

```rust
use dash_sdk::{Sdk, SdkBuilder};
use dash_sdk::dapi_client::AddressList;
use dpp::dashcore::Network;
use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;
use std::num::NonZeroUsize;

// Create the trusted context provider
let context_provider = TrustedHttpContextProvider::new(
    Network::Testnet,
    None, // devnet_name - only needed for devnet
    NonZeroUsize::new(100).expect("cache size"),
)?;

// Build SDK with the trusted context provider
let sdk = SdkBuilder::new(AddressList::default())
    .with_context_provider(context_provider)
    .build()?;
```

### With Custom URL

If you want to use your own trusted HTTP endpoint instead of the default ones:

```rust
// Create provider with custom URL
let custom_provider = TrustedHttpContextProvider::new_with_url(
    Network::Testnet,
    "https://my-trusted-server.com".to_string(),
    NonZeroUsize::new(100).unwrap(),
)?;

// Build SDK with the custom provider
let sdk = SdkBuilder::new(AddressList::default())
    .with_context_provider(custom_provider)
    .build()?;
```

### With Fallback Provider

Since the trusted HTTP provider only provides quorum public keys, you may need to set a fallback provider for data contracts and token configurations:

```rust
use dash_sdk::mock::provider::GrpcContextProvider;

// Create a fallback provider that can fetch data contracts
let grpc_provider = GrpcContextProvider::new(
    None,
    "core.example.com",
    19998,
    "dashrpc",
    "password",
    NonZeroUsize::new(100).unwrap(),
    NonZeroUsize::new(100).unwrap(),
    NonZeroUsize::new(100).unwrap(),
)?;

// Create the trusted provider with fallback
let trusted_provider = TrustedHttpContextProvider::new(
    Network::Testnet,
    None,
    NonZeroUsize::new(100).unwrap(),
)?
.with_fallback_provider(grpc_provider);

// Build SDK with the trusted provider
let sdk = SdkBuilder::new(AddressList::default())
    .with_context_provider(trusted_provider)
    .build()?;
```

### With Pre-loaded Known Contracts

You can also pre-load known contracts that will be served immediately without requiring a fallback provider:

```rust
use dpp::data_contract::DataContract;

// Load your known contracts
let dpns_contract = DataContract::from_json(...)?;
let dashpay_contract = DataContract::from_json(...)?;

// Create the trusted provider with known contracts
let trusted_provider = TrustedHttpContextProvider::new(
    Network::Testnet,
    None,
    NonZeroUsize::new(100).unwrap(),
)?
.with_known_contracts(vec![dpns_contract, dashpay_contract]);

// Build SDK with the trusted provider
let sdk = SdkBuilder::new(AddressList::default())
    .with_context_provider(trusted_provider)
    .build()?;
```

## Implementation Details

The `TrustedHttpContextProvider` implements the `ContextProvider` trait and provides:

1. **Quorum Public Keys**: Fetched from trusted HTTP endpoints with LRU caching
2. **Data Contracts**: 
   - First checks pre-loaded known contracts (if any)
   - Then delegates to the fallback provider if set
   - Otherwise returns `None`
3. **Token Configurations**: Delegated to the fallback provider if set, otherwise returns `None`
4. **Platform Activation Height**: Returns hardcoded values for each network

## License

MIT