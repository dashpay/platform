//! Query trait representing criteria for fetching data from the platform.
//!
//! [Query] trait is used to specify individual objects as well as search criteria for fetching multiple objects from the platform.
use std::fmt::Debug;

use dapi_grpc::platform::v0::{GetDataContractRequest, GetIdentityRequest};
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
/// 2. [`DocumentQuery`](crate::platform::DocumentQuery) - fetches [Document](dpp::document::Document) based on search
/// conditions; see
/// [query syntax documentation](https://docs.dash.org/projects/platform/en/stable/docs/reference/query-syntax.html)
/// for more details.
///
/// ## Example
///
/// To fetch individual [Identity] object by its [Identifier], you just need to create it and use [Fetch] or [List] trait:
///
/// ```rust
/// use rs_sdk::{Sdk, platform::{Query, Identifier, Fetch, Identity}};
///
/// # const SOME_IDENTIFIER : [u8; 32] = [0; 32];
/// let mut sdk = Sdk::new_mock();
/// let query = Identifier::new(SOME_IDENTIFIER);
/// let identity = Identity::fetch(&mut sdk, query);
/// ```
///
/// As [Identifier] implements [Query], the `query` variable in the code above can be used as a parameter for
/// [Sdk::fetch()](crate::Sdk::fetch) and [Sdk::list()](crate::Sdk::list) methods.
pub trait Query<T: TransportRequest>: Send + Debug + Clone {
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
    /// On failure, it yields an [`Error`](crate::error::Error).
    ///
    /// # Error Handling
    /// This method propagates any errors encountered during the conversion process. These are returned as [`Error`](crate::error::Error) instances.
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

impl Query<GetDataContractRequest> for dpp::prelude::Identifier {
    fn query(self, prove: bool) -> Result<GetDataContractRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        let id = self.to_vec();
        Ok(GetDataContractRequest { id, prove: true })
    }
}

impl Query<GetIdentityRequest> for dpp::prelude::Identifier {
    fn query(self, prove: bool) -> Result<GetIdentityRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        let id = self.to_vec();
        Ok(GetIdentityRequest { id, prove: true })
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
