pub mod enc;
mod features;
pub mod de;

pub use enc::platform_encode_to_vec;
pub use enc::PlatformVersionEncode;

pub use de::PlatformVersionedBorrowDecode;
pub use de::PlatformVersionedDecode;
