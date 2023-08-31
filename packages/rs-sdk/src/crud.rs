//! Dash Platform create, read, update, delete (CRUD) operations.

use crate::{dapi::DAPI, error::Error};

#[async_trait::async_trait]
pub trait Readable<A: DAPI, I, Q: ObjectQuery<I>>
where
    Self: Sized,
{
    async fn read(api: &A, query: &Q) -> Result<Self, Error>;
}

// TODO this will change, not tested at all
pub trait Writable<A: DAPI, W>
where
    Self: Sized,
{
    fn create(self, api: &A, wallet: &W) -> Result<Self, Error>;
    fn update(self, api: &A, wallet: &W) -> Result<Self, Error>;
    fn delete(self, api: &A, wallet: &W) -> Result<(), Error>;
}

// TODO this will change, not tested at all
pub trait Listable<A: DAPI, I, Q: ObjectQuery<I>>
where
    Self: Sized,
{
    fn list(api: &A, query: &Q) -> Result<Vec<Self>, Error>;
}

pub trait ObjectQuery<O>: Sized + Send + Sync
where
    O: Sized,
    Self: Sized,
{
    fn query(&self) -> Result<O, Error>;
}

impl ObjectQuery<[u8; 32]> for dpp::prelude::Identifier {
    fn query(&self) -> Result<[u8; 32], Error> {
        Ok(self.as_bytes().clone())
    }
}
