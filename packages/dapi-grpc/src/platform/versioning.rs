use std::fmt::Display;

use platform_version::version::PlatformVersion;

use super::v0::{Proof, ResponseMetadata};

pub trait VersionedGrpcResponse {
    type Error: Display;
    fn get_proof(&self, version: &PlatformVersion) -> Result<Proof, Self::Error>;
    fn get_metadata(&self, version: &PlatformVersion) -> Result<ResponseMetadata, Self::Error>;
}
