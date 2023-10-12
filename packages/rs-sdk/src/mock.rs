//! Mocking support for Dash SDK.
//!
//! This module provides a way to mock SDK operations. It is used in tests and examples.
//!
//! In order to mock SDK operations, you need to create a mock SDK instance using [Sdk::new_mock()].
//! Next step is to create mock query expectations on [MockDashPlatformSdk] object returned by [Sdk::mock()], using
//! [MockDashPlatformSdk::expect_fetch()] and [MockDashPlatformSdk::expect_list()].
//!
//!
//! ## Example
//!
//! ```no_run
//! let mut sdk = rs_sdk::Sdk::new_mock();
//! let query = rs_sdk::platform::Identifier::random();
//! sdk.mock().expect_fetch(query, None as Option<rs_sdk::platform::Identity>);
//! ```
//!
//! See tests/mock_*.rs for more detailed examples.

use dpp::{
    document::serialization_traits::DocumentCborMethodsV0,
    document::Document,
    platform_serialization::{
        platform_encode_to_vec, platform_versioned_decode_from_slice, PlatformVersionEncode,
        PlatformVersionedDecode,
    },
    prelude::{DataContract, Identity},
    serialization::{
        PlatformDeserializableWithPotentialValidationFromVersionedStructure,
        PlatformSerializableWithPlatformVersion,
    },
    version::PlatformVersion,
};
use drive_proof_verifier::{FromProof, QuorumInfoProvider};
use rs_dapi_client::{mock::MockDapiClient, transport::TransportRequest};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::Mutex;

use crate::platform::{Fetch, List, Query};

/// Mechanisms to mock Dash Platform SDK.
///
/// This object is returned by [Sdk::mock()] and is used to define mock expectations.
///
/// Use [expect_list] to define expectations for [List] requests and [expect_fetch] for [Fetch] requests.
///
/// ## Panics
///
/// Can panic on errors.
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

    /// Expect a [Fetch] request and return provided object.
    ///
    /// This method is used to define mock expectations for [Fetch] requests.
    ///
    /// ## Generic Parameters
    ///
    /// - `O`: Type of the object that will be returned in response to the query. Must implement [Fetch] and [MockResponse].
    /// - `Q`: Type of the query that will be sent to the platform. Must implement [Query] and [MockRequest].
    ///
    /// ## Arguments
    ///
    /// - `query`: Query that will be sent to the platform.
    /// - `object`: Object that will be returned in response to `query`, or None if the object is expected to not exist.
    ///
    /// ## Returns
    ///
    /// * Some(O): If the object is expected to exist.
    /// * None: If the object is expected to not exist.
    ///
    /// ## Panics
    ///
    /// Can panic on errors.
    ///
    /// ## Example
    ///
    /// ```no_run
    /// # let r = tokio::runtime::Runtime::new().unwrap();
    /// #
    /// # r.block_on(async {
    ///     use rs_sdk::{Sdk, platform::{Identity, Fetch, dpp::identity::accessors::IdentityGettersV0}};
    ///
    ///     let mut api = Sdk::new_mock();
    ///     // Define expected response
    ///     let expected: Identity = Identity::random_identity(1, None, api.version())
    ///         .expect("create expected identity");
    ///     // Define query that will be sent
    ///     let query = expected.id();
    ///     // Expect that in response to `query`, `expected` will be returned
    ///     api.mock().expect_fetch(query, Some(expected.clone())).await;
    ///
    ///     // Fetch the identity
    ///     let retrieved = dpp::prelude::Identity::fetch(&mut api, query)
    ///         .await
    ///         .unwrap()
    ///         .expect("object should exist");
    ///
    ///     // Check that the identity is the same as expected
    ///     assert_eq!(retrieved, expected);
    /// # });
    /// ```
    pub async fn expect_fetch<
        O: Fetch + MockResponse,
        Q: Query<<O as Fetch>::Request> + MockRequest,
    >(
        &mut self,
        query: Q,
        object: Option<O>,
    ) -> &mut Self
    where
        <O as Fetch>::Request: MockRequest,
        <<O as Fetch>::Request as TransportRequest>::Response: Default,
    {
        let grpc_request = query.query().expect("query must be correct");
        self.expect(grpc_request, object).await;

        self
    }

    /// Expect a [List] request and return provided object.
    ///
    /// This method is used to define mock expectations for [List] requests.
    ///
    /// ## Generic Parameters
    ///
    /// - `O`: Type of the object that will be returned in response to the query.
    /// Must implement [List]. `Vec<O>` must implement [MockResponse].
    /// - `Q`: Type of the query that will be sent to the platform. Must implement [Query] and [MockRequest].
    ///
    /// ## Arguments
    ///
    /// - `query`: Query that will be sent to the platform.
    /// - `objects`: Vector of objects that will be returned in response to `query`, or None if no objects are expected.
    ///
    /// ## Returns
    ///
    /// * Some(Vec<O>): If the objects are expected to exist.
    /// * None: If the objects are expected to not exist.
    ///
    /// ## Panics
    ///
    /// Can panic on errors.
    ///
    /// ## Example
    ///
    /// Usage example is similar to [expect_fetch], but the expected object must be a vector of objects.
    pub async fn expect_list<O: List, Q: Query<<O as List>::Request> + MockRequest>(
        &mut self,
        query: Q,
        objects: Option<Vec<O>>,
    ) -> &mut Self
    where
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
        self.expect(grpc_request, objects).await;

        self
    }

    /// Save expectations for a request.
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

    /// Wrapper around [FromProof] that uses mock expectations instead of executing [FromProof] trait.
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
        unimplemented!("get_quorum_public_key is not implemented in mock SDK")
    }
}

/// Trait implemented by objects that can be used as requests in mock expectations.
pub trait MockRequest {
    /// Format the object as a key that will be used to match the request with the expectation.
    ///
    /// ## Panics
    ///
    /// Can panic on errors.
    fn mock_key(&self) -> Vec<u8>;
}

impl<T: serde::Serialize> MockRequest for T {
    fn mock_key(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("serialize mock request as a key")
    }
}

/// Trait implemented by objects that can be used in mock expectation responses.
///
/// ## Panics
///
/// Can panic on errors.
pub trait MockResponse {
    /// Serialize the object to save into expectations
    ///
    /// ## Panics
    ///
    /// Can panic on errors.
    fn mock_serialize(&self, mock_sdk: &MockDashPlatformSdk) -> Vec<u8>;

    /// Deserialize the object from expectations
    ///
    /// ## Panics
    ///
    /// Can panic on errors.
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
    Self: PlatformVersionedDecode + PlatformVersionEncode,
{
    fn mock_serialize(&self, sdk: &MockDashPlatformSdk) -> Vec<u8> {
        // self.clone().serialize_to_bytes().expect("serialize data")
        platform_encode_to_vec(self, bincode::config::standard(), sdk.version())
            .expect("serialize data")
    }

    fn mock_deserialize(sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized + PlatformVersionedDecode,
    {
        // Self::deserialize_from_bytes(buf).expect("deserialize data")
        platform_versioned_decode_from_slice(buf, bincode::config::standard(), sdk.version())
            .expect("deserialize data")
    }
}

// FIXME: Seems that DataContract doesn't implement PlatformVersionedDecode + PlatformVersionEncode,
// so we just use some methods implemented directly on these objects.
impl MockResponse for DataContract {
    fn mock_serialize(&self, sdk: &MockDashPlatformSdk) -> Vec<u8> {
        self.serialize_to_bytes_with_platform_version(sdk.version())
            .expect("encode data")
    }

    fn mock_deserialize(sdk: &MockDashPlatformSdk, buf: &[u8]) -> Self
    where
        Self: Sized,
    {
        DataContract::versioned_deserialize(buf, true, sdk.version()).expect("decode data")
    }
}

// FIXME: Seems that Document doesn't implement PlatformVersionedDecode + PlatformVersionEncode,
// so we use cbor.
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
