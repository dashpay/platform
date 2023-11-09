//! Query trait representing criteria for fetching data from the platform.
//!
//! [Query] trait is used to specify individual objects as well as search criteria for fetching multiple objects from the platform.
use std::fmt::Debug;

use dapi_grpc::{
    mock::Mockable,
    platform::v0::{
        self as proto, get_identity_keys_request,
        get_identity_keys_request::GetIdentityKeysRequestV0, AllKeys, GetIdentityKeysRequest,
        KeyRequestType,
    },
};
use dpp::prelude::Identifier;
use drive::query::DriveQuery;
use rs_dapi_client::transport::TransportRequest;

use crate::{error::Error, platform::document_query::DocumentQuery};

/// Trait implemented by objects that can be used as queries.
///
/// [Query] trait is used to specify criteria for fetching data from the platform.
/// It can be used to specify individual objects as well as search criteria for fetching multiple objects from the platform.
///
/// Some examples of queries include:
///
/// 1. [`Identifier`](crate::platform::Identifier) - fetches an object by its identifier; implemented for
/// [Identity](dpp::prelude::Identity), [DataContract](dpp::prelude::DataContract) and [Document](dpp::document::Document).
/// 2. [`DocumentQuery`] - fetches [Document](dpp::document::Document) based on search
/// conditions; see
/// [query syntax documentation](https://docs.dash.org/projects/platform/en/stable/docs/reference/query-syntax.html)
/// for more details.
///
/// ## Example
///
/// To fetch individual [Identity](dpp::prelude::Identity) object by its [Identifier](crate::platform::Identifier),
/// you just need to create it and use [Fetch](crate::platform::Fetch)
/// or [FetchMany](crate::platform::FetchMany) trait:
///
/// ```rust
/// use dash_platform_sdk::{Sdk, platform::{Query, Identifier, Fetch, Identity}};
///
/// # const SOME_IDENTIFIER : [u8; 32] = [0; 32];
/// let mut sdk = Sdk::new_mock();
/// let query = Identifier::new(SOME_IDENTIFIER);
/// let identity = Identity::fetch(&mut sdk, query);
/// ```
///
/// As [Identifier](crate::platform::Identifier) implements [Query], the `query` variable in the code
/// above can be used as a parameter for [Fetch::fetch()](crate::platform::Fetch::fetch())
/// and [FetchMany::fetch_many()](crate::platform::FetchMany::fetch_many()) methods.
pub trait Query<T: TransportRequest + Mockable>: Send + Debug + Clone {
    /// Converts the current instance into an instance of the `TransportRequest` type.
    ///
    /// This method takes ownership of the instance upon which it's called (hence `self`), and attempts to perform the conversion.
    ///
    /// # Arguments
    ///
    /// * `prove` - Whether to include proofs in the response. Only `true` is supported at the moment.
    ///
    /// # Returns
    /// On success, this method yields an instance of the `TransportRequest` type (`T`).
    /// On failure, it yields an [`Error`].
    ///
    /// # Error Handling
    /// This method propagates any errors encountered during the conversion process.
    /// These are returned as [`Error`] instances.
    fn query(self, prove: bool) -> Result<T, Error>;
}

impl<T> Query<T> for T
where
    T: TransportRequest + Sized + Send + Sync + Clone + Debug,
    T::Response: Send + Sync + Debug,
{
    fn query(self, prove: bool) -> Result<T, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        Ok(self)
    }
}

impl Query<proto::GetDataContractRequest> for Identifier {
    fn query(self, prove: bool) -> Result<proto::GetDataContractRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        let id = self.to_vec();
        Ok(proto::GetDataContractRequest {
            version: Some(proto::get_data_contract_request::Version::V0(
                proto::get_data_contract_request::GetDataContractRequestV0 { id, prove: true },
            )),
        })
    }
}

impl Query<proto::GetIdentityKeysRequest> for Identifier {
    /// Get all keys for an identity with provided identifier.
    fn query(self, prove: bool) -> Result<proto::GetIdentityKeysRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        let identity_id = self.to_vec();
        Ok(GetIdentityKeysRequest {
            version: Some(get_identity_keys_request::Version::V0(
                GetIdentityKeysRequestV0 {
                    identity_id,
                    prove,
                    limit: None,
                    offset: None,
                    request_type: Some(KeyRequestType {
                        request: Some(proto::key_request_type::Request::AllKeys(AllKeys {})),
                    }),
                },
            )),
        })
    }
}

impl<'a> Query<DocumentQuery> for DriveQuery<'a> {
    fn query(self, prove: bool) -> Result<DocumentQuery, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        let q: DocumentQuery = (&self).into();
        Ok(q)
    }
}
