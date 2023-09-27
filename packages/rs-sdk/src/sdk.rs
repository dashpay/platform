//! Dash API implementation

use crate::error::Error;
use drive_proof_verifier::QuorumInfoProvider;
use rs_dapi_client::{DapiClient, RequestSettings};
use tokio::sync::{Mutex, MutexGuard};

pub use http::Uri;
pub use rs_dapi_client::AddressList;

#[async_trait::async_trait]
pub trait Sdk: Send + Sync {
    async fn platform_client<'a>(&self) -> MutexGuard<'a, crate::platform::PlatformClient>
    where
        'life0: 'a;
    fn quorum_info_provider<'a>(&'a self) -> Result<&'a dyn QuorumInfoProvider, Error>;
}

mockall::mock! {
    pub DashPlatformSdk {}
    #[async_trait::async_trait]
    impl Sdk for DashPlatformSdk {
        async fn platform_client<'a>(&self) -> MutexGuard<'a, crate::platform::PlatformClient>;
        fn quorum_info_provider<'a>(&'a self) -> Result<&'a dyn QuorumInfoProvider, Error>;
    }
}

pub struct DashPlatformSdk {
    dapi: tokio::sync::Mutex<crate::platform::PlatformClient>,
    quorum_provider: Box<dyn QuorumInfoProvider>,
}

impl DashPlatformSdk {
    pub fn new(
        addresses: AddressList,
        quorum_info_provider: Box<dyn QuorumInfoProvider>,
    ) -> Result<Self, Error> {
        let dapi = DapiClient::new(addresses, RequestSettings::default());
        Ok(Self {
            dapi: Mutex::new(dapi),
            quorum_provider: quorum_info_provider,
        })
    }
}

#[async_trait::async_trait]
impl Sdk for DashPlatformSdk {
    async fn platform_client<'a>(&self) -> MutexGuard<'a, crate::platform::PlatformClient>
    where
        'life0: 'a,
    {
        self.dapi.lock().await
    }
    fn quorum_info_provider<'a>(&'a self) -> Result<&'a dyn QuorumInfoProvider, Error> {
        let provider = self.quorum_provider.as_ref();
        Ok(provider)
    }
}
