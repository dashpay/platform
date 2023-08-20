use crate::ProtocolError;
use lazy_static::lazy_static;
pub use platform_version::error::PlatformVersionError;
pub use platform_version::version::*;
pub use platform_version::*;
use std::sync::RwLock;

lazy_static! {
    static ref CURRENT_PLATFORM_VERSION: RwLock<Option<&'static PlatformVersion>> =
        RwLock::new(None);
}

pub trait PlatformVersionCurrentVersion {
    fn set_current(platform_version: &'static PlatformVersion);
    fn get_current<'a>() -> Result<&'a Self, ProtocolError>;
    fn get_maybe_current<'a>() -> Option<&'a Self>;
    fn get_version_or_current_or_latest<'a>(
        version: Option<u32>,
    ) -> Result<&'a Self, ProtocolError>;
}

impl PlatformVersionCurrentVersion for PlatformVersion {
    fn set_current(platform_version: &'static PlatformVersion) {
        let mut context = CURRENT_PLATFORM_VERSION.write().unwrap();
        *context = Some(platform_version);
    }

    fn get_current<'a>() -> Result<&'a Self, ProtocolError> {
        CURRENT_PLATFORM_VERSION
            .read()
            .unwrap()
            .ok_or(ProtocolError::CurrentProtocolVersionNotInitialized)
    }

    fn get_maybe_current<'a>() -> Option<&'a Self> {
        let lock_guard = CURRENT_PLATFORM_VERSION.read().unwrap();

        if lock_guard.is_some() {
            Some(lock_guard.unwrap())
        } else {
            None
        }
    }

    fn get_version_or_current_or_latest<'a>(
        version: Option<u32>,
    ) -> Result<&'a Self, ProtocolError> {
        if let Some(version) = version {
            Ok(PlatformVersion::get(version)?)
        } else if let Some(current_version) = Self::get_maybe_current() {
            Ok(current_version)
        } else {
            Ok(Self::latest())
        }
    }
}
