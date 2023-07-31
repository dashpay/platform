mod v0;

use std::collections::BTreeMap;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use crate::identity::IdentityV0;
use crate::prelude::Identity;
use crate::ProtocolError;

impl Identity {
    pub fn create_basic_identity(id: [u8; 32], platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version.dpp.identity_versions.identity_structure_version {
            0 => Ok(Self::create_basic_identity_v0(id)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Identity::create_basic_identity".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}