#[cfg(feature = "platform-version")]
mod impl_alloc;
#[cfg(all(feature = "std", feature = "platform-version"))]
mod impl_std;

#[cfg(feature = "platform-version")]
pub use impl_alloc::platform_encode_to_vec;
