use dpp::{
    document::serialization_traits::DocumentCborMethodsV0,
    document::Document,
    platform_serialization::{PlatformVersionEncode, PlatformVersionedDecode},
    prelude::{DataContract, Identity},
    serialization::{
        PlatformDeserializable,
        PlatformDeserializableWithPotentialValidationFromVersionedStructure, PlatformSerializable,
        PlatformSerializableWithPlatformVersion,
    },
    version::PlatformVersion,
};
use drive_proof_verifier::{FromProof, QuorumInfoProvider};
use rs_dapi_client::{mock::MockDapiClient, transport::TransportRequest};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::Mutex;

use crate::platform::{Fetch, List, Query};

pub struct MockDashPlatformSdk {
    from_proof_expectations: BTreeMap<Vec<u8>, Vec<u8>>,
    platform_version: &'static PlatformVersion,
    dapi: Arc<Mutex<MockDapiClient>>,
}

impl MockDashPlatformSdk {
    pub(crate) fn new(version: &'static PlatformVersion, dapi: Arc<Mutex<MockDapiClient>>) -> Self {
        Self {
            from_proof_expectations: Default::default(),
            platform_version: version,
            dapi,
        }
    }

    pub(crate) fn version<'v>(&self) -> &'v PlatformVersion {
        self.platform_version
    }

    pub async fn expect_fetch<O: Fetch, Q: Query<<O as Fetch>::Request>>(
        &mut self,
        query: Q,
        object: Option<O>,
    ) -> &mut Self
    where
        Q: MockRequest,
        O: MockResponse,
        <O as Fetch>::Request: MockRequest,
        <<O as Fetch>::Request as TransportRequest>::Response: Default,
    {
        let grpc_request = query.query().expect("query must be correct");
        self.expect(grpc_request, object).await;

        self
    }

    pub async fn expect_list<O: List, Q: Query<<O as List>::Request>>(
        &mut self,
        query: Q,
        object: Option<Vec<O>>,
    ) -> &mut Self
    where
        Q: MockRequest,
        Vec<O>: MockResponse,
        <O as List>::Request: MockRequest,
        <<O as List>::Request as TransportRequest>::Response: Default,
        Vec<O>: FromProof<
                <O as List>::Request,
                Request = <O as List>::Request,
                Response = <<O as List>::Request as TransportRequest>::Response,
            > + Sync,
    {
        let grpc_request = query.query().expect("query must be correct");
        self.expect(grpc_request, object).await;

        self
    }

    async fn expect<I: TransportRequest + MockRequest, O: MockResponse>(
        &mut self,
        grpc_request: I,
        returned_object: Option<O>,
    ) where
        I::Response: Default,
    {
        let key = grpc_request.mock_key();

        // This expectation will work for from_proof
        self.from_proof_expectations
            .insert(key, returned_object.mock_serialize(self));

        // This expectation will work for execute
        let mut dapi_guard = self.dapi.lock().await;
        dapi_guard.expect(&grpc_request, &Default::default());
    }

    pub(crate) fn parse_proof<I, O: FromProof<I>>(
        &self,
        request: O::Request,
        _response: O::Response,
    ) -> Result<Option<O>, drive_proof_verifier::Error>
    where
        O::Request: MockRequest,
        Option<O>: MockResponse,
        // O: FromProof<<O as FromProof<I>>::Request>,
    {
        let data = match self.from_proof_expectations.get(&request.mock_key()) {
            Some(d) => d,
            None => panic!("from_proof_expectations not found"),
        };

        Ok(Option::<O>::mock_deserialize(self, data))
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

pub trait MockRequest {
    fn mock_key(&self) -> Vec<u8>;
}

impl<T: serde::Serialize> MockRequest for T {
    fn mock_key(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("serialize mock request as a key")
    }
}

/// Trait implemented by objects that can be used in mock expectation responses.
pub trait MockResponse {
    /// Format the object to save into expectations
    fn mock_serialize(&self, mock_sdk: &MockDashPlatformSdk) -> Vec<u8>;

    /// Deserializes the object with one loaded from expectations
    fn mock_deserialize(mock_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized;
}

impl<T: MockResponse> MockResponse for Option<T> {
    fn mock_deserialize(mock_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        if buf.is_empty() {
            return None;
        }

        Some(T::mock_deserialize(mock_sdk, buf))
    }
    fn mock_serialize(&self, mock_sdk: &MockDashPlatformSdk) -> Vec<u8> {
        match self {
            Some(item) => item.mock_serialize(mock_sdk),
            None => vec![],
        }
    }
}

impl<T: MockResponse> MockResponse for Vec<T> {
    fn mock_deserialize(mock_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        let items: Vec<Vec<u8>> = bincode::decode_from_slice(buf, bincode::config::standard())
            .expect("decode vec of data")
            .0;
        items
            .into_iter()
            .map(|item| T::mock_deserialize(mock_sdk, &item))
            .collect()
    }

    fn mock_serialize(&self, mock_sdk: &MockDashPlatformSdk) -> Vec<u8> {
        let data: Vec<Vec<u8>> = self
            .iter()
            .map(|item| item.mock_serialize(mock_sdk))
            .collect();

        bincode::encode_to_vec(data, bincode::config::standard()).expect("encode vec of data")
    }
}

impl MockResponse for Identity
where
    Identity: PlatformVersionedDecode + PlatformVersionEncode,
{
    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        // self.platform_encode(encoder, platform_version)
        self.clone().serialize_to_bytes().expect("serialize data")
    }
    fn mock_deserialize(_sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        Self::deserialize_from_bytes(buf).expect("deserialize data")
    }
}

impl MockResponse for DataContract {
    fn mock_deserialize(sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        DataContract::versioned_deserialize(buf, true, sdk.version()).expect("decode data")
    }

    fn mock_serialize(&self, sdk: &MockDashPlatformSdk) -> Vec<u8> {
        self.serialize_to_bytes_with_platform_version(sdk.version())
            .expect("encode data")
    }
}

impl MockResponse for Document {
    fn mock_serialize(&self, _sdk: &MockDashPlatformSdk) -> Vec<u8> {
        self.to_cbor().expect("encode data")
    }

    fn mock_deserialize(sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        Self::from_cbor(buf, None, None, sdk.version()).expect("decode data")
    }
}
