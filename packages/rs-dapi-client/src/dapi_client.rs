//! [DapiClient] definition.
#[cfg(feature = "dump")]
use std::path::PathBuf;

use backon::{ExponentialBuilder, Retryable};
#[cfg(feature = "dump")]
use hex::ToHex;
#[cfg(feature = "dump")]
use sha2::Digest;
use tonic::async_trait;
use tracing::Instrument;

use crate::{
    transport::{TransportClient, TransportRequest},
    AddressList, CanRetry, RequestSettings,
};

/// General DAPI request error type.
#[derive(Debug, thiserror::Error)]
pub enum DapiClientError<TE> {
    /// The error happened on transport layer
    #[error("transport error: {0}")]
    Transport(TE),
    /// There are no valid peer addresses to use.
    #[error("no available addresses to use")]
    NoAvailableAddresses,
    #[cfg(feature = "mocks")]
    /// Expectation not found
    #[error("mock expectation not found for request: {0}")]
    MockExpectationNotFound(String),
}

impl<TE: CanRetry> CanRetry for DapiClientError<TE> {
    fn can_retry(&self) -> bool {
        use DapiClientError::*;
        match self {
            NoAvailableAddresses => false,
            Transport(transport_error) => transport_error.can_retry(),
            #[cfg(feature = "mocks")]
            MockExpectationNotFound(_) => false,
        }
    }
}

#[async_trait]
/// DAPI client trait.
pub trait Dapi {
    /// Execute request using this DAPI client.
    async fn execute<R>(
        &mut self,
        request: R,
        settings: RequestSettings,
    ) -> Result<R::Response, DapiClientError<<R::Client as TransportClient>::Error>>
    where
        R: TransportRequest;
}

#[async_trait]
impl<D: Dapi + Send> Dapi for &mut D {
    async fn execute<R>(
        &mut self,
        request: R,
        settings: RequestSettings,
    ) -> Result<R::Response, DapiClientError<<R::Client as TransportClient>::Error>>
    where
        R: TransportRequest,
    {
        (**self).execute(request, settings).await
    }
}

/// Access point to DAPI.
#[derive(Debug)]
pub struct DapiClient {
    address_list: AddressList,
    settings: RequestSettings,
    #[cfg(feature = "dump")]
    dump_dir: Option<PathBuf>,
}

impl DapiClient {
    /// Initialize new [DapiClient] and optionally override default settings.
    pub fn new(address_list: AddressList, settings: RequestSettings) -> Self {
        Self {
            address_list,
            settings,
            #[cfg(feature = "dump")]
            dump_dir: None,
        }
    }

    /// Prefix of dump files.
    #[cfg(feature = "dump")]
    const DUMP_FILE_PREFIX: &str = "msg";

    /// Define director where dumps of all traffic will be saved.
    ///
    /// Each request and response pair will be saved to a JSON file in `dump_dir`.
    /// Data is saved as [DumpData] structure.
    /// Any errors are logged on `warn` level and ignored.
    ///
    /// Name of dump file follows convention: `<DUMP_FILE_PREFIX>-<timestamp>-<hash>.json`
    ///
    /// Useful for debugging and mocking.
    /// See also [MockDapiClient::load()](crate::mock::MockDapiClient::load()).
    #[cfg(feature = "dump")]
    pub fn dump_dir(mut self, dump_dir: Option<PathBuf>) -> Self {
        self.dump_dir = dump_dir;

        self
    }

    /// Save dump of request and response to disk.
    ///
    /// Any errors are logged on `warn` level and ignored.
    #[cfg(feature = "dump")]
    fn dump_request_response<R: TransportRequest>(
        request: R,
        response: R::Response,
        dump_dir: Option<PathBuf>,
    ) where
        R: serde::Serialize,
        R::Response: serde::Serialize,
    {
        let path = match dump_dir {
            Some(p) => p,
            None => return,
        };

        let data = DumpData { request, response };

        // Construct file name
        // Path consists of current timestamp + hash of message
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
        let id = match data.id() {
            Ok(h) => h,
            Err(e) => return tracing::warn!("unable to generate dump file name: {}", e),
        };
        let file = path.join(format!("{}-{}-{}.json", Self::DUMP_FILE_PREFIX, now, id));

        if let Err(e) = data.save(&file) {
            tracing::warn!("unable to write dump file {:?}: {}", path, e);
            return;
        }
    }
}

#[async_trait]
impl Dapi for DapiClient {
    /// Execute the [DapiRequest].
    async fn execute<R>(
        &mut self,
        request: R,
        settings: RequestSettings,
    ) -> Result<R::Response, DapiClientError<<R::Client as TransportClient>::Error>>
    where
        R: TransportRequest,
    {
        // Join settings of different sources to get final version of the settings for this execution:
        let applied_settings = self
            .settings
            .override_by(R::SETTINGS_OVERRIDES)
            .override_by(settings)
            .finalize();

        // Setup retry policy:
        let retry_settings = ExponentialBuilder::default().with_max_times(applied_settings.retries);

        // Save dump dir for later use, as self is moved into routine
        #[cfg(feature = "dump")]
        let dump_dir = self.dump_dir.clone();
        #[cfg(feature = "dump")]
        let dump_request = request.clone();

        // Setup DAPI request execution routine future. It's a closure that will be called
        // more once to build new future on each retry.
        let routine = move || {
            // Try to get an address to initialize transport on:
            let address = self.address_list.get_live_address().ok_or(
                DapiClientError::<<R::Client as TransportClient>::Error>::NoAvailableAddresses,
            );

            // Get a transport client requried by the DAPI request from this DAPI client.
            // It stays wrapped in `Result` since we want to return
            // `impl Future<Output = Result<...>`, not a `Result` itself.
            let transport_client = address.map(|addr| R::Client::with_uri(addr.uri().clone()));

            let transport_request = request.clone();

            // Create a future using `async` block that will be returned from the closure on
            // each retry. Could be just a request future, but need to unpack client first.
            async move {
                transport_request
                    .execute_transport(&mut transport_client?, &applied_settings)
                    .await
                    .map_err(|e| {
                        DapiClientError::<<R::Client as TransportClient>::Error>::Transport(e)
                    })
            }
        };

        // Start the routine with retry policy applied:
        let result = routine
            .retry(&retry_settings)
            .when(|e| e.can_retry())
            .instrument(tracing::info_span!("request routine"))
            .await;

        // Dump request and response to disk if dump_dir is set:
        #[cfg(feature = "dump")]
        if let Ok(ref result) = &result {
            Self::dump_request_response(dump_request, result.clone(), dump_dir);
        }

        result
    }
}

#[cfg(feature = "dump")]
#[derive(serde::Serialize, serde::Deserialize)]
/// Data format of dumps created with [DapiClient::dump_dir].
pub struct DumpData<T: TransportRequest> {
    /// Request that was sent to DAPI.
    pub request: T,
    /// Response that was received from DAPI.
    pub response: T::Response,
}
#[cfg(feature = "dump")]
impl<T: TransportRequest> DumpData<T> {
    /// Return unique identifier (hex-encoded sha256) of the request.
    /// Can be used to construct dump file name.
    pub fn id(&self) -> Result<String, std::io::Error> {
        let encoded = match serde_json::to_vec(&self.request) {
            Ok(b) => b,
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("unable to serialize json: {}", e),
                ))
            }
        };
        let mut e = sha2::Sha256::new();
        e.update(&encoded);
        let hash = e.finalize();

        return Ok(hash.encode_hex::<String>());
    }

    /// Load dump data from file.
    pub fn load<P: AsRef<std::path::Path>>(file: P) -> Result<Self, std::io::Error>
    where
        T: for<'de> serde::Deserialize<'de>,
        T::Response: for<'de> serde::Deserialize<'de>,
    {
        let f = std::fs::File::open(file)?;

        let data: Self = serde_json::from_reader(f).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unable to parse json: {}", e),
            )
        })?;

        Ok(data)
    }

    /// Save dump data to file.
    pub fn save(&self, file: &std::path::Path) -> Result<(), std::io::Error>
    where
        T: serde::Serialize,
        T::Response: serde::Serialize,
    {
        let encoded = serde_json::to_vec_pretty(self).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unable to serialize json: {}", e),
            )
        })?;

        std::fs::write(file, encoded)
    }
}
