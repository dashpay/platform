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
            fn platform_versioned_borrow_decode<BD: BorrowDecoder<'de, Context = crate::BincodeContext>>(decoder: &mut BD, platform_version: &PlatformVersion) -> Result<Self, DecodeError> {
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
            fn platform_versioned_decode<DE: Decoder<Context = crate::BincodeContext>>(decoder: &mut DE, platform_version: &PlatformVersion) -> Result<Self, DecodeError> {
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

#[cfg(test)]
mod tests {
    use bincode::config;
    use platform_version::version::PlatformVersion;

    fn cfg() -> impl bincode::config::Config {
        config::standard().with_big_endian().with_no_limit()
    }

    fn pv() -> &'static PlatformVersion {
        PlatformVersion::first()
    }

    #[test]
    fn tuple_1_decode() {
        let value: (u32,) = (42,);
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: (u32,) =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn tuple_2_decode() {
        let value: (u8, i16) = (255, -100);
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: (u8, i16) =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn tuple_3_decode() {
        let value: (bool, u32, i64) = (true, 42, -999);
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: (bool, u32, i64) =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn tuple_borrow_decode() {
        let value: (u8, u16) = (1, 2);
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: (u8, u16) =
            crate::platform_versioned_borrow_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }
}
