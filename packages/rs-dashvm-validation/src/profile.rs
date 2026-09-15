//! The preparation profile: the projection of the protocol's DashVM table that this crate
//! reads.
//!
//! The profile owns no number. It copies the limits out of `PlatformVersion::dashvm` and records
//! the preparation generation; every check in the crate reads the profile and nothing else, so
//! the answer to "what bounds a module at protocol version N" is the version table.

use crate::errors::ProfileError;
use platform_version::version::dashvm_versions::{DashVmLimits, DashVmVersion};
use platform_version::version::FeatureVersion;
use platform_version::version::PlatformVersion;

/// The preparation generation this crate implements.
pub const PREPARATION_GENERATION_0: FeatureVersion = 0;

/// Everything preparation needs to know about the protocol version it runs under.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparationProfile {
    /// The preparation generation the table selects. Recorded in every prepared module so the
    /// runtime can bind it into the compiled artifact key.
    pub generation: FeatureVersion,
    /// The metering generation the table selects. Preparation does not meter, but a prepared
    /// bundle carries the value so the runtime can key artifacts by it.
    pub metering: FeatureVersion,
    /// The numeric limits.
    pub limits: DashVmLimits,
}

impl PreparationProfile {
    /// Builds the profile from a DashVM table entry.
    ///
    /// Fails when the table selects a preparation generation this crate does not implement,
    /// which is a versioning error (the binary is older than the protocol), never a paid
    /// rejection.
    pub fn from_dashvm_version(dashvm: &DashVmVersion) -> Result<Self, ProfileError> {
        match dashvm.preparation {
            PREPARATION_GENERATION_0 => Ok(Self {
                generation: dashvm.preparation,
                metering: dashvm.metering,
                limits: dashvm.limits.clone(),
            }),
            generation => Err(ProfileError::UnknownGeneration { generation }),
        }
    }
}

impl TryFrom<&PlatformVersion> for PreparationProfile {
    type Error = ProfileError;

    /// Builds the profile for a protocol version.
    ///
    /// Fails with [`ProfileError::NotActive`] on every version that predates smart contracts:
    /// no code path may prepare contract code there.
    fn try_from(platform_version: &PlatformVersion) -> Result<Self, Self::Error> {
        let dashvm = platform_version
            .dashvm
            .as_ref()
            .ok_or(ProfileError::NotActive {
                protocol_version: platform_version.protocol_version,
            })?;
        Self::from_dashvm_version(dashvm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_version::version::dashvm_versions::v1::DASHVM_VERSION_V1;
    use platform_version::version::v16::PROTOCOL_VERSION_16;
    use platform_version::version::{PlatformVersion, INITIAL_PROTOCOL_VERSION};

    #[test]
    fn should_fail_before_the_5_0_protocol_version_and_succeed_on_it() {
        for protocol_version in INITIAL_PROTOCOL_VERSION..=PROTOCOL_VERSION_16 {
            let platform_version =
                PlatformVersion::get(protocol_version).expect("registered protocol version");
            assert_eq!(
                PreparationProfile::try_from(platform_version),
                Err(ProfileError::NotActive { protocol_version }),
                "protocol version {protocol_version} must not prepare contract code"
            );
        }
        let latest = PlatformVersion::latest();
        let profile = PreparationProfile::try_from(latest).expect("the latest version prepares");
        assert_eq!(profile.generation, PREPARATION_GENERATION_0);
        assert_eq!(
            profile.limits,
            latest
                .dashvm
                .as_ref()
                .expect("the latest version carries the table")
                .limits
        );
    }

    #[test]
    fn should_reject_a_preparation_generation_this_crate_does_not_implement() {
        let future = DashVmVersion {
            preparation: PREPARATION_GENERATION_0 + 1,
            ..DASHVM_VERSION_V1
        };
        assert_eq!(
            PreparationProfile::from_dashvm_version(&future),
            Err(ProfileError::UnknownGeneration {
                generation: PREPARATION_GENERATION_0 + 1
            })
        );
    }
}
