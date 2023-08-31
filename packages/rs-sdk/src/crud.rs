//! Dash Platform create, read, update, delete (CRUD) operations.

use crate::{dapi::DAPI, error::Error};

#[async_trait::async_trait]
pub trait ReadOnly<A: DAPI, I, Q: ObjectQuery<I>>
where
    Self: Sized,
{
    async fn read(api: &A, query: &Q) -> Result<Self, Error>;
}

pub trait ReadWrite<I>
where
    Self: Sized,
{
    fn create() -> Result<Self, Error>;
    fn update() -> Result<Self, Error>;
    fn delete() -> Result<Self, Error>;
}

pub trait ObjectQuery<T>: Sized + Send + Sync
where
    T: Sized,
    Self: Sized,
{
    fn query(&self) -> Result<T, Error>;
}

impl ObjectQuery<[u8; 32]> for dpp::prelude::Identifier {
    fn query(&self) -> Result<[u8; 32], Error> {
        Ok(self.as_bytes().clone())
    }
}
