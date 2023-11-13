use std::fmt::Display;

use super::v0::{Proof, ResponseMetadata};

pub trait VersionedGrpcResponse {
    type Error: Display;
    fn proof(&self) -> Result<&Proof, Self::Error>;

    fn proof_owned(self) -> Result<Proof, Self::Error>;
    fn metadata(&self) -> Result<&ResponseMetadata, Self::Error>;
}
