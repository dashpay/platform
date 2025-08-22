use crate::version::PlatformVersion;

pub mod error;
pub mod version;

pub trait DefaultForPlatformVersion: Sized {
    /// The type returned in the event of a conversion error.
    type Error;

    /// Performs the conversion.
    fn default_for_platform_version(
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error>;
}

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

/// A trait for converting a value of type `T` into `Self` using a provided `PlatformVersion`.
///
/// This trait is infallible and guarantees a valid result for supported versions.
pub trait FromPlatformVersioned<T>: Sized {
    /// Performs the conversion.
    fn from_platform_versioned(value: T, platform_version: &PlatformVersion) -> Self;
}

/// A trait for converting `Self` into another type `T` using a `PlatformVersion`.
///
/// This is the infallible counterpart of `TryIntoPlatformVersioned`.
pub trait IntoPlatformVersioned<T>: Sized {
    /// Performs the conversion.
    fn into_platform_versioned(self, platform_version: &PlatformVersion) -> T;
}

// FromPlatformVersioned implies IntoPlatformVersioned
impl<T, U> IntoPlatformVersioned<U> for T
where
    U: FromPlatformVersioned<T>,
{
    #[inline]
    fn into_platform_versioned(self, platform_version: &PlatformVersion) -> U {
        U::from_platform_versioned(self, platform_version)
    }
}
