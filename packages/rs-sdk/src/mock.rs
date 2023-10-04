use drive_proof_verifier::{FromProof, QuorumInfoProvider};

use rs_dapi_client::{
    mock::Expectations,
    transport::{TransportClient, TransportRequest},
    Dapi, DapiClientError, RequestSettings,
};

use crate::platform::{Fetch, Query};

pub struct MockDashPlatformSdk {
    from_proof_expectations: Expectations,
    execute_expectations: Expectations,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ExpectationValue<T: Sized, O: Sized> {
    response: T,
    object: O,
}

impl MockDashPlatformSdk {
    pub(crate) fn new() -> Self {
        Self {
            from_proof_expectations: Default::default(),
            execute_expectations: Default::default(),
        }
    }

    pub fn expect_fetch<O: Fetch, Q: Query<<O as Fetch>::Request>>(
        &mut self,
        query: Q,
        object: O,
    ) -> &mut Self
    where
        Q: serde::Serialize,
        O: serde::Serialize + serde::de::DeserializeOwned,
        <<O as Fetch>::Request as TransportRequest>::Response: Default,
    {
        let grpc_request: <O as Fetch>::Request = query.query().expect("query must be correct");
        // This expectation will work for from_proof
        self.from_proof_expectations.add(&grpc_request, &object);

        // This expectation will work for execute
        let grpc_response = <<O as Fetch>::Request as TransportRequest>::Response::default();
        self.execute_expectations.add(&grpc_request, &grpc_response);

        self
    }

    pub(crate) fn parse_proof<I, O: FromProof<I>>(
        &self,
        request: O::Request,
        _response: O::Response,
    ) -> Result<Option<O>, drive_proof_verifier::Error>
    where
        O::Request: serde::Serialize,
        O: for<'de> serde::Deserialize<'de>,
        // O: FromProof<<O as FromProof<I>>::Request>,
    {
        let object = self.from_proof_expectations.get(request);
        if object.is_none() {
            panic!("from_proof_expectations not found")
        }

        Ok(object)
    }
}

#[async_trait::async_trait]
impl Dapi for MockDashPlatformSdk {
    /// Execute the request.
    ///
    /// We just return response defined earlier as an expectation.
    async fn execute<R>(
        &mut self,
        request: R,
        _settings: RequestSettings,
    ) -> Result<R::Response, DapiClientError<<R::Client as TransportClient>::Error>>
    where
        R: TransportRequest,
    {
        let item: Option<R::Response> = self.execute_expectations.get(&request);
        item.ok_or(DapiClientError::MockExpectationNotFound(format!(
            "unexpected mock request: {:?}",
            request
        )))
    }
}

impl QuorumInfoProvider for MockDashPlatformSdk {
    fn get_quorum_public_key(
        &self,
        _quorum_type: u32,
        _quorum_hash: [u8; 32],
        _core_chain_locked_height: u32,
    ) -> Result<[u8; 48], drive_proof_verifier::Error> {
        unimplemented!("MockDashPlatformSdk::get_quorum_public_key")
    }
}
