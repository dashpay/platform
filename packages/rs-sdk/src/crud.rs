//! Dash Platform create, read, update, delete (CRUD) operations.

use drive::query::DriveQuery;

use crate::{dapi::DashAPI, error::Error, platform::document_query::DocumentQuery};

#[async_trait::async_trait]
pub trait Readable<API: DashAPI>
where
    Self: Sized,
{
    type Identifier;

    async fn read<Q: ObjectQuery<Self::Identifier>>(api: &API, query: &Q) -> Result<Self, Error>;
}

// TODO this will change, not tested at all
#[async_trait::async_trait]
pub trait Writable<A: DashAPI, W>
where
    Self: Sized,
{
    async fn create(self, api: &A, wallet: &W) -> Result<Self, Error>;
    async fn update(self, api: &A, wallet: &W) -> Result<Self, Error>;
    async fn delete(self, api: &A, wallet: &W) -> Result<(), Error>;
}

// TODO this will change, not tested at all
#[async_trait::async_trait]
pub trait Listable<API: DashAPI>
where
    Self: Sized,
{
    type Request;

    async fn list<Q: ObjectQuery<Self::Request>>(api: &API, query: &Q) -> Result<Vec<Self>, Error>;
}

// pub trait ObjectQuery<O>: Sized + Send + Sync
// where
//     O: Sized,
//     Self: Sized,
// {
// }

pub trait ObjectQuery<I>: Sized + Send + Sync + Clone {
    fn query(&self) -> Result<I, Error>;
}

impl<T> ObjectQuery<T> for T
where
    T: Sized + Send + Sync + Clone,
{
    fn query(&self) -> Result<T, Error> {
        Ok(self.clone()) // or whatever logic you want here
    }
}

impl ObjectQuery<[u8; 32]> for dpp::prelude::Identifier {
    fn query(&self) -> Result<[u8; 32], Error> {
        Ok(self.as_bytes().clone())
    }
}

impl<'a> ObjectQuery<DocumentQuery> for DriveQuery<'a> {
    fn query(&self) -> Result<DocumentQuery, Error> {
        let q: DocumentQuery = self.into();
        Ok(q)
    }
}
