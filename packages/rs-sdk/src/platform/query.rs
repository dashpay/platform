//! Query trait representing criteria for fetching data from the platform.
//!
//! [Query] trait is used to specify individual objects as well as search criteria for fetching multiple objects from the platform.
use std::fmt::Debug;

use dapi_grpc::platform::v0::{GetDataContractRequest, GetIdentityRequest};
use drive::query::DriveQuery;
use rs_dapi_client::transport::TransportRequest;

use crate::{error::Error, platform::document_query::DocumentQuery};

/// Specifies identifiers of single object or query criteria for fetching multiple objects from the platform.
pub trait Query<T: TransportRequest>: Send + Debug + Clone {
    /// Converts the current instance into an instance of the `TransportRequest` type.
    ///
    /// This method takes ownership of the instance upon which it's called (hence `self`), and attempts to perform the conversion.
    ///
    /// # Returns
    /// On success, this method yields an instance of the `TransportRequest` type (`T`).
    /// On failure, it yields an [`Error`](crate::error::Error).
    ///
    /// # Usage
    /// ```rust
    /// let transport_request = query.query()?;
    /// ```
    ///
    /// # Error Handling
    /// This method propagates any errors encountered during the conversion process. These are returned as [`Error`](crate::error::Error) instances.
    fn query(self) -> Result<T, Error>;
}

impl<T> Query<T> for T
where
    T: TransportRequest + Sized + Send + Sync + Clone + Debug,
    T::Response: Send + Sync + Debug,
{
    fn query(self) -> Result<T, Error> {
        Ok(self)
    }
}

impl Query<GetDataContractRequest> for dpp::prelude::Identifier {
    fn query(self) -> Result<GetDataContractRequest, Error> {
        let id = self.to_vec();
        Ok(GetDataContractRequest { id, prove: true })
    }
}

impl Query<GetIdentityRequest> for dpp::prelude::Identifier {
    fn query(self) -> Result<GetIdentityRequest, Error> {
        let id = self.to_vec();
        Ok(GetIdentityRequest { id, prove: true })
    }
}

impl<'a> Query<DocumentQuery> for DriveQuery<'a> {
    fn query(self) -> Result<DocumentQuery, Error> {
        let q: DocumentQuery = (&self).into();
        Ok(q)
    }
}
