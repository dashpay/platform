use super::{PlatformVersionedBorrowDecode, PlatformVersionedDecode};
use bincode::de::{BorrowDecoder, Decoder};
use bincode::error::DecodeError;
use platform_version::version::PlatformVersion;

macro_rules! impl_tuple {
    () => {};
    ($first:ident $(, $extra:ident)*) => {
        impl<'de, $first $(, $extra)*> PlatformVersionedBorrowDecode<'de> for ($first, $($extra, )*)
        where
            $first: PlatformVersionedBorrowDecode<'de>,
        $(
            $extra : PlatformVersionedBorrowDecode<'de>,
        )*
         {
            fn platform_versioned_borrow_decode<BD: BorrowDecoder<'de>>(decoder: &mut BD, platform_version: &PlatformVersion) -> Result<Self, DecodeError> {
                Ok((
                    $first::platform_versioned_borrow_decode(decoder, platform_version)?,
                    $($extra :: platform_versioned_borrow_decode(decoder, platform_version)?, )*
                ))
            }
        }

        impl<$first $(, $extra)*> PlatformVersionedDecode for ($first, $($extra, )*)
        where
            $first: PlatformVersionedDecode,
        $(
            $extra : PlatformVersionedDecode,
        )*
        {
            fn platform_versioned_decode<DE: Decoder>(decoder: &mut DE, platform_version: &PlatformVersion) -> Result<Self, DecodeError> {
                Ok((
                    $first::platform_versioned_decode(decoder, platform_version)?,
                    $($extra :: platform_versioned_decode(decoder, platform_version)?, )*
                ))
            }
        }
    }
}

impl_tuple!(A);
impl_tuple!(A, B);
impl_tuple!(A, B, C);
impl_tuple!(A, B, C, D);
impl_tuple!(A, B, C, D, E);
impl_tuple!(A, B, C, D, E, F);
impl_tuple!(A, B, C, D, E, F, G);
impl_tuple!(A, B, C, D, E, F, G, H);
impl_tuple!(A, B, C, D, E, F, G, H, I);
impl_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
