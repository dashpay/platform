use crate::version::PlatformVersion;

pub mod error;
pub mod version;

pub trait TryFromPlatformVersioned<T>: Sized {
    /// The type returned in the event of a conversion error.
    type Error;

    /// Performs the conversion.
    fn try_from_platform_versioned(
        value: T,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error>;
}

pub trait TryIntoPlatformVersioned<T>: Sized {
    /// The type returned in the event of a conversion error.
    type Error;

    /// Performs the conversion.
    fn try_into_platform_versioned(
        self,
        platform_version: &PlatformVersion,
    ) -> Result<T, Self::Error>;
}

// TryFrom implies TryInto
impl<T, U> TryIntoPlatformVersioned<U> for T
where
    U: TryFromPlatformVersioned<T>,
{
    type Error = U::Error;

    #[inline]
    fn try_into_platform_versioned(
        self,
        platform_version: &PlatformVersion,
    ) -> Result<U, U::Error> {
        U::try_from_platform_versioned(self, platform_version)
    }
}
