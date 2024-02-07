//! # Fetch Module
//!
//! This module provides an abstract way to fetch data from a platform using the `Fetch` trait.
//! It allows fetching of various types of data such as `Identity`, `DataContract`, and `Document`.
//!
//! ## Traits
//! - [Fetch]: An asynchronous trait that defines how to fetch data from the platform.
//!   It requires the implementing type to also implement [Debug] and [FromProof]
//!   traits. The associated [Fetch::Request]` type needs to implement [TransportRequest].

use crate::mock::MockResponse;
use crate::{error::Error, platform::query::Query, Sdk};
use dapi_grpc::platform::v0::{self as platform_proto};
use dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dpp::platform_value::Identifier;
use dpp::{document::Document, prelude::Identity};
use drive_proof_verifier::FromProof;
use rs_dapi_client::{transport::TransportRequest, DapiRequest, RequestSettings};
use std::fmt::Debug;

use super::types::identity::IdentityRequest;
use super::DocumentQuery;

/// Trait implemented by objects that can be fetched from the platform.
///
/// To fetch an object from the platform, you need to define some query (criteria that fetched object must match) and
/// use [Fetch::fetch()] for your object type.
///
/// ## Example
///
/// A common use case is to fetch an [Identity] object by its [Identifier]. As [Identifier] implements [Query] for
/// identity requests, you need to:
/// * create a [Query], which will be an [Identifier] instance that will be used to identify requested [Identity],
/// * call [Identity::fetch()] with the query and an instance of [Sdk].
///
/// ```rust
/// use rs_sdk::{Sdk, platform::{Query, Identifier, Fetch, Identity}};
///
/// # const SOME_IDENTIFIER : [u8; 32] = [0; 32];
/// let sdk = Sdk::new_mock();
/// let query = Identifier::new(SOME_IDENTIFIER);
///
/// let identity = Identity::fetch(&sdk, query);
/// ```
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
    /// Type of request used to fetch data from the platform.
    ///
    /// Most likely, one of the types defined in [`dapi_grpc::platform::v0`].
    ///
    /// This type must implement [`TransportRequest`] and [`MockRequest`].
    type Request: TransportRequest + Into<<Self as FromProof<<Self as Fetch>::Request>>::Request>;

    /// Fetch single object from the Platfom.
    ///
    /// Fetch object from the platform that satisfies provided [Query].
    /// Most often, the Query is an [Identifier] of the object to be fetched.
    ///
    /// ## Parameters
    ///
    /// - `sdk`: An instance of [Sdk].
    /// - `query`: A query parameter implementing [`crate::platform::query::Query`] to specify the data to be fetched.
    ///
    /// ## Returns
    ///
    /// Returns:
    /// * `Ok(Some(Self))` when object is found
    /// * `Ok(None)` when object is not found
    /// * [`Err(Error)`](Error) when an error occurs
    ///
    /// ## Error Handling
    ///
    /// Any errors encountered during the execution are returned as [Error] instances.
    async fn fetch<Q: Query<<Self as Fetch>::Request>>(
        sdk: &Sdk,
        query: Q,
    ) -> Result<Option<Self>, Error> {
        let request = query.query(sdk.prove())?;

        let response = request
            .clone()
            .execute(sdk, RequestSettings::default())
            .await?;

        let object_type = std::any::type_name::<Self>().to_string();
        tracing::trace!(request = ?request, response = ?response, object_type, "fetched object from platform");

        let object: Option<Self> = sdk.parse_proof(request, response)?;

        match object {
            Some(item) => Ok(item.into()),
            None => Ok(None),
        }
    }

    /// Fetch single object from the Platfom by identifier.
    ///
    /// Conveniance method that allows fetching objects by identifier for types that implement [Query] for [Identifier].
    ///
    /// See [`Fetch::fetch()`] for more details.
    ///
    /// ## Parameters
    ///
    /// - `sdk`: An instance of [Sdk].
    /// - `id`: An [Identifier] of the object to be fetched.
    async fn fetch_by_identifier(sdk: &Sdk, id: Identifier) -> Result<Option<Self>, Error>
    where
        Identifier: Query<<Self as Fetch>::Request>,
    {
        Self::fetch(sdk, id).await
    }
}

impl Fetch for Identity {
    type Request = IdentityRequest;
}

impl Fetch for dpp::prelude::DataContract {
    type Request = platform_proto::GetDataContractRequest;
}

impl Fetch for Document {
    type Request = DocumentQuery;
}

impl Fetch for drive_proof_verifier::types::IdentityBalance {
    type Request = platform_proto::GetIdentityBalanceRequest;
}

impl Fetch for drive_proof_verifier::types::IdentityBalanceAndRevision {
    type Request = platform_proto::GetIdentityBalanceAndRevisionRequest;
}

impl Fetch for ExtendedEpochInfo {
    type Request = platform_proto::GetEpochsInfoRequest;
}
