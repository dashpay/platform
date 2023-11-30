use std::{num::NonZeroUsize, str::FromStr, sync::Arc};

use clap::Parser;
use dash_platform_sdk::{
    mock::{
        provider::GrpcContextProvider,
        wallet::{core::CoreGrpcWallet, platform::PlatformSignerWallet, MockWallet},
    },
    platform::Fetch,
    Sdk, SdkBuilder,
};
use dpp::prelude::{DataContract, Identifier};
use rs_dapi_client::AddressList;

#[derive(clap::Parser, Debug)]
#[command(version)]
pub struct Config {
    /// Dash Platform server hostname or IPv4 address
    #[arg(short = 'i', long = "address")]
    pub server_address: String,

    /// Dash Core IP port
    #[arg(short = 'c', long)]
    pub core_port: u16,

    // Dash Core RPC user
    #[arg(short = 'u', long)]
    pub core_user: String,

    // Dash Core RPC password
    #[arg(short = 'p', long)]
    pub core_password: String,

    /// Dash Platform DAPI port
    #[arg(short = 'd', long)]
    pub platform_port: u16,
}

/// Read data contract.
///
/// This example demonstrates how to connect to running platform and try to read a data contract.
/// TODO: add wallet, context provider, etc.
#[tokio::main(flavor = "multi_thread", worker_threads = 1)]
async fn main() {
    // Replace const below with data contract identifier of data contract, 32 bytes
    const DATA_CONTRACT_ID_BYTES: [u8; 32] = [1; 32];

    // Read configuration
    let config = Config::parse();
    // Configure the Sdk
    let sdk = setup_sdk(&config);

    // read data contract

    // Convert bytes to identifier object that can be used as a Query
    let id = Identifier::from_bytes(&DATA_CONTRACT_ID_BYTES).expect("parse data contract id");

    // Fetch identity from the Platform
    let contract: Option<DataContract> =
        DataContract::fetch(&sdk, id).await.expect("fetch identity");

    // Check the result; note that in our case, we expect to not find the data contract, as the
    // identifier is not valid.
    assert!(contract.is_none(), "result: {:?}", contract);
}

/// Setup Rust SDK
fn setup_sdk(config: &Config) -> Arc<Sdk> {
    // First, we need to implement a Wallet. In our case, we'll just use GRPC wallet
    // implementation from the SDK mocking module, which in turn uses separate Platform and
    // Core wallet.
    //
    // Note that this logic is executed automatically when you [build](SdkBuilder::build()) SDK without providing a
    // wallet nor context provider. We do it manually here to show how it works, and how
    // you should use it with your own implementation of wallet and context provider.

    // For platform wallet, we use mock implementation that uses a single private key to sign
    let platform_wallet = PlatformSignerWallet::new_mock().expect("platform wallet");

    // For core wallet, we use mock implementation that uses core grpc api
    // It requires running Dash Core with grpc api enabled
    let core_wallet = CoreGrpcWallet::new(
        &config.server_address,
        config.core_port,
        &config.core_user,
        &config.core_password,
    )
    .expect("core wallet");

    // Now, let's create the wallet
    let wallet = MockWallet::new(core_wallet, platform_wallet);

    // We also need to implement a ContextProvider.
    // Here, we will just use a mock implementation.
    // Tricky thing here is that this implementation requires SDK, so we have a
    // circular dependency between SDK and ContextProvider.
    // We'll first provide `None` Sdk, and then update it later.
    //
    // To modify context provider, we need locks and Arc to overcome ownership rules.
    let context_provider = GrpcContextProvider::new(
        None,
        &config.server_address,
        config.core_port,
        &config.core_user,
        &config.core_password,
        NonZeroUsize::new(100).expect("data contracts cache size"),
        NonZeroUsize::new(100).expect("quorum public keys cache size"),
    )
    .expect("context provider");
    let context_provider = Arc::new(std::sync::Mutex::new(context_provider));
    // let context_provider = RefCell::new(context_provider);
    // Let's build the Sdk.
    // First, we need an URI of some Dash Platform DAPI host to connect to and use as seed.
    let uri = http::Uri::from_str(&format!(
        "http://{}:{}",
        config.server_address, config.platform_port
    ))
    .expect("parse uri");

    // Now, we create the Sdk with the wallet and context provider.
    let sdk = SdkBuilder::new(AddressList::from_iter([uri]))
        .with_wallet(wallet)
        .with_context_provider(Arc::clone(&context_provider))
        .build()
        .expect("cannot build sdk");

    // Reconfigure context provider with Sdk
    let mut guard = context_provider.lock().expect("lock context provider");
    guard.set_sdk(Some(Arc::clone(&sdk)));
    drop(guard);

    // Return the SDK we created
    sdk
}
