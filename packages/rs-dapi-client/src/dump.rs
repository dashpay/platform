//! Dumping of requests and responses to disk

use dapi_grpc::mock::Mockable;

use crate::{
    mock::{Key, MockResult},
    transport::TransportRequest,
    DapiClient,
};
use std::{any::type_name, path::PathBuf};

/// Data format of dumps created with [DapiClient::dump_dir].
#[derive(Clone)]
pub struct DumpData<T: TransportRequest> {
    /// Request that was sent to DAPI.
    pub serialized_request: Vec<u8>,
    /// Response that was received from DAPI.
    pub serialized_response: Vec<u8>,

    phantom: std::marker::PhantomData<T>,
}
impl<T: TransportRequest> DumpData<T> {
    /// Return deserialized request
    pub fn deserialize(&self) -> (T, MockResult<T>) {
        let req = T::mock_deserialize(&self.serialized_request).unwrap_or_else(|| {
            panic!(
                "unable to deserialize mock data of type {}",
                type_name::<T>()
            )
        });
        let resp =
            <MockResult<T>>::mock_deserialize(&self.serialized_response).unwrap_or_else(|| {
                panic!(
                    "unable to deserialize mock data of type {}",
                    type_name::<T::Response>()
                )
            });

        (req, resp)
    }
}

impl<T: TransportRequest> dapi_grpc::mock::Mockable for DumpData<T>
where
    T: Mockable,
    T::Response: Mockable,
{
    // We use null-delimited JSON as a format for dump data to make it readable.
    fn mock_serialize(&self) -> Option<Vec<u8>> {
        // nulls are not allowed in serialized data as we use it as a delimiter
        if self.serialized_request.contains(&0) {
            panic!("null byte in serialized request");
        }
        if self.serialized_response.contains(&0) {
            panic!("null byte in serialized response");
        }

        let data = [
            &self.serialized_request,
            "\n\0\n".as_bytes(),
            &self.serialized_response,
        ]
        .concat();

        Some(data)
    }

    fn mock_deserialize(buf: &[u8]) -> Option<Self> {
        // we panic as we expect this to be called only with data serialized by mock_serialize()

        // Split data into request and response
        let buf = buf.split(|&b| b == 0).collect::<Vec<_>>();
        if buf.len() != 2 {
            panic!("invalid mock data format, expected exactly two items separated by null byte");
        }

        let request = buf.first().expect("missing request in mock data");
        let response = buf.last().expect("missing response in mock data");

        Some(Self {
            serialized_request: request.to_vec(),
            serialized_response: response.to_vec(),
            phantom: std::marker::PhantomData,
        })
    }
}

impl<T: TransportRequest> DumpData<T> {
    /// Create new dump data.
    pub fn new(request: &T, response: &MockResult<T>) -> Self {
        let request = request
            .mock_serialize()
            .expect("unable to serialize request");
        let response = response
            .mock_serialize()
            .expect("unable to serialize response");

        Self {
            serialized_request: request,
            serialized_response: response,
            phantom: std::marker::PhantomData,
        }
    }

    // Return request type (T) name without module prefix
    fn request_type() -> String {
        let req_type = std::any::type_name::<T>();
        req_type.split(':').last().unwrap_or(req_type).to_string()
    }
    /// Generate unique filename for this dump.
    ///
    /// Filename consists of:
    ///
    /// * [DapiClient::DUMP_FILE_PREFIX]
    /// * basename of the type of request, like `GetIdentityRequest`
    /// * unique identifier (hash) of the request
    pub fn filename(&self) -> Result<String, std::io::Error> {
        let key = Key::try_new(&self.serialized_request)?;
        // get request type without underscores (which we use as a file name separator)
        let request_type = Self::request_type().replace('_', "-");

        let file = format!(
            "{}_{}_{}.json",
            DapiClient::DUMP_FILE_PREFIX,
            request_type,
            key
        );

        Ok(file)
    }

    /// Load dump data from file.
    pub fn load<P: AsRef<std::path::Path>>(file: P) -> Result<Self, std::io::Error>
    where
        T: Mockable,
        T::Response: Mockable,
    {
        let data = std::fs::read(file)?;

        Self::mock_deserialize(&data).ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "unable to deserialize mock data of type {}",
                type_name::<T>()
            ),
        ))
    }

    /// Save dump data to file.
    pub fn save(&self, file: &std::path::Path) -> Result<(), std::io::Error>
    where
        T: Mockable,
        T::Response: Mockable,
    {
        let encoded = self.mock_serialize().ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("unable to serialize mock data of type {}", type_name::<T>()),
        ))?;

        std::fs::write(file, encoded)
    }
}

impl DapiClient {
    /// Prefix of dump files.
    pub const DUMP_FILE_PREFIX: &'static str = "msg";

    /// Define directory where dumps of all traffic will be saved.
    ///
    /// Each request and response pair will be saved to a JSON file in `dump_dir`.
    /// Data is saved as [DumpData] structure.
    /// Any errors are logged on `warn` level and ignored.
    ///
    /// Dump file name is generated by [DumpData::filename()].
    ///
    /// Useful for debugging and mocking.
    /// See also [MockDapiClient::load()](crate::mock::MockDapiClient::load()).
    pub fn dump_dir(mut self, dump_dir: Option<PathBuf>) -> Self {
        self.dump_dir = dump_dir;

        self
    }

    /// Save dump of request and response to disk.
    ///
    /// Any errors are logged on `warn` level and ignored.
    pub(crate) fn dump_request_response<R: TransportRequest>(
        request: &R,
        response: &MockResult<R>,
        dump_dir: Option<PathBuf>,
    ) where
        R: Mockable,
        <R as TransportRequest>::Response: Mockable,
    {
        let path = match dump_dir {
            Some(p) => p,
            None => return,
        };

        let data = DumpData::new(request, response);

        // Construct file name
        let filename = match data.filename() {
            Ok(f) => f,
            Err(e) => return tracing::warn!("unable to create dump file name: {}", e),
        };

        let file = path.join(filename);

        if let Err(e) = data.save(&file) {
            tracing::warn!("unable to write dump file {:?}: {}", path, e);
        }
    }
}
