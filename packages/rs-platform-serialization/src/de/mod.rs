//! Decoder-based structs and traits.

mod impl_core;
mod impl_tuples;
mod impls;

pub use bincode::de::{BorrowDecoder, Decoder};
pub use bincode::error::DecodeError;
pub use bincode::{BorrowDecode, Decode};
use platform_version::version::PlatformVersion;

/// Decode with the default `()` context to avoid repeated generic arguments.
pub trait DefaultDecode: bincode::Decode<crate::BincodeContext> {
    fn decode<D: Decoder<Context = crate::BincodeContext>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError>
    where
        Self: Sized,
    {
        <Self as bincode::Decode<crate::BincodeContext>>::decode(decoder)
    }
}
impl<T> DefaultDecode for T where T: bincode::Decode<crate::BincodeContext> {}

/// BorrowDecode with the default `()` context.
pub trait DefaultBorrowDecode<'de>: bincode::BorrowDecode<'de, crate::BincodeContext> {
    fn borrow_decode<D: BorrowDecoder<'de, Context = crate::BincodeContext>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError>
    where
        Self: Sized,
    {
        <Self as bincode::BorrowDecode<'de, crate::BincodeContext>>::borrow_decode(decoder)
    }
}
impl<'de, T> DefaultBorrowDecode<'de> for T where
    T: bincode::BorrowDecode<'de, crate::BincodeContext>
{
}

/// Trait that makes a type able to be decoded, akin to serde's `DeserializeOwned` trait.
///
/// This trait should be implemented for types which do not have references to data in the reader. For types that contain e.g. `&str` and `&[u8]`, implement [BorrowDecode] instead.
///
/// Whenever you implement `Decode` for your type, the base trait `BorrowDecode` is automatically implemented.
///
/// This trait will be automatically implemented if you enable the `derive` feature and add `#[derive(bincode::Decode)]` to your type. Note that if the type contains any lifetimes, `BorrowDecode` will be implemented instead.
///
/// # Implementing this trait manually
///
/// If you want to implement this trait for your type, the easiest way is to add a `#[derive(bincode::Decode)]`, build and check your `target/generated/bincode/` folder. This should generate a `<Struct name>_Decode.rs` file.
///
/// For this struct:
///
/// ```
/// struct Entity {
///     pub x: f32,
///     pub y: f32,
/// }
/// ```
///
/// It will look something like:
///
/// ```
/// # struct Entity {
/// #     pub x: f32,
/// #     pub y: f32,
/// # }
/// impl<Context> bincode::Decode<Context> for Entity {
///     fn decode<D: bincode::de::Decoder<Context = Context>>(
///         decoder: &mut D,
///     ) -> core::result::Result<Self, bincode::error::DecodeError> {
///         Ok(Self {
///             x: bincode::Decode::decode(decoder)?,
///             y: bincode::Decode::decode(decoder)?,
///         })
///     }
/// }
/// impl<'de, Context> bincode::BorrowDecode<'de, Context> for Entity {
///     fn borrow_decode<D: bincode::de::BorrowDecoder<'de, Context = Context>>(
///         decoder: &mut D,
///     ) -> core::result::Result<Self, bincode::error::DecodeError> {
///         Ok(Self {
///             x: bincode::BorrowDecode::borrow_decode(decoder)?,
///             y: bincode::BorrowDecode::borrow_decode(decoder)?,
///         })
///     }
/// }
/// ```
///
/// From here you can add/remove fields, or add custom logic.
///
/// To get specific integer types, you can use:
/// ```
/// # struct Foo;
/// # impl<Context> bincode::Decode<Context> for Foo {
/// #     fn decode<D: bincode::de::Decoder<Context = Context>>(
/// #         decoder: &mut D,
/// #     ) -> core::result::Result<Self, bincode::error::DecodeError> {
/// let x: u8 = bincode::Decode::decode(decoder)?;
/// let x = <u8 as bincode::Decode<Context>>::decode(decoder)?;
/// #         Ok(Foo)
/// #     }
/// # }
/// # bincode::impl_borrow_decode!(Foo);
/// ```
pub trait PlatformVersionedDecode: Sized {
    /// Attempt to decode this type with the given [Decode].
    fn platform_versioned_decode<D: Decoder<Context = crate::BincodeContext>>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError>;
}

/// Trait that makes a type able to be decoded, akin to serde's `Deserialize` trait.
///
/// This trait should be implemented for types that contain borrowed data, like `&str` and `&[u8]`. If your type does not have borrowed data, consider implementing [Decode] instead.
///
/// This trait will be automatically implemented if you enable the `derive` feature and add `#[derive(bincode::Decode)]` to a type with a lifetime.
pub trait PlatformVersionedBorrowDecode<'de>: Sized {
    /// Attempt to decode this type with the given [BorrowDecode].
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de, Context = crate::BincodeContext>>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError>;
}

/// Helper macro to implement `PlatformVersionedBorrowDecode` for any type that implements `PlatformVersionedDecode`.
#[macro_export]
macro_rules! impl_platform_versioned_borrow_decode {
    ($ty:ty) => {
        impl<'de> $crate::PlatformVersionedBorrowDecode<'de> for $ty {
            fn platform_versioned_borrow_decode<
                D: bincode::de::BorrowDecoder<'de, Context = $crate::BincodeContext>,
            >(
                decoder: &mut D,
                platform_version: &PlatformVersion,
            ) -> core::result::Result<Self, bincode::error::DecodeError> {
                // Here we directly call the platform_versioned_decode method from
                // PlatformVersionedDecode, assuming it correctly handles decoding based
                // on the platform version.
                <$ty as $crate::PlatformVersionedDecode>::platform_versioned_decode(
                    decoder,
                    platform_version,
                )
            }
        }
    };
}

/// Decodes only the option variant from the decoder. Will not read any more data than that.
#[inline]
pub(crate) fn decode_option_variant<D: Decoder<Context = crate::BincodeContext>>(
    decoder: &mut D,
    type_name: &'static str,
) -> Result<Option<()>, DecodeError> {
    let is_some = <u8 as DefaultDecode>::decode(decoder)?;
    match is_some {
        0 => Ok(None),
        1 => Ok(Some(())),
        x => Err(DecodeError::UnexpectedVariant {
            found: x as u32,
            allowed: &bincode::error::AllowedEnumVariants::Range { max: 1, min: 0 },
            type_name,
        }),
    }
}

/// Maximum allowed collection length during deserialization (1 Mi elements).
///
/// This acts as a defense-in-depth guard against crafted payloads that encode
/// enormous collection lengths in a varint prefix. Even when the bincode config
/// has no byte-budget limit, this cap prevents a single decoded length field
/// from triggering an unbounded allocation that could OOM-abort the process.
///
/// Set to 1 MiB (1,048,576) elements because:
/// - The largest legitimate structure (StateTransition) is capped at 100 KB,
///   meaning even a `Vec<u8>` can hold at most ~100 K elements in practice.
/// - For wider element types (`[u8; 32]`, etc.) the memory impact is
///   `element_count × element_size`, so a smaller cap keeps the worst case
///   well within OS limits (1 Mi × 64 bytes = 64 MiB).
/// - This is a last-resort guard; the primary protection is the per-type
///   byte-budget configured via `#[platform_serialize(limit = N)]`.
const MAX_COLLECTION_LEN: u64 = 1024 * 1024;

/// Decodes the length of any slice, container, etc from the decoder
#[inline]
pub(crate) fn decode_slice_len<D: Decoder<Context = crate::BincodeContext>>(
    decoder: &mut D,
) -> Result<usize, DecodeError> {
    let v = <u64 as DefaultDecode>::decode(decoder)?;

    if v > MAX_COLLECTION_LEN {
        return Err(DecodeError::LimitExceeded);
    }

    v.try_into().map_err(|_| DecodeError::OutsideUsizeRange(v))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bincode::config;
    use platform_version::version::PlatformVersion;

    fn cfg() -> impl bincode::config::Config {
        config::standard().with_big_endian().with_no_limit()
    }

    fn pv() -> &'static PlatformVersion {
        PlatformVersion::first()
    }

    /// Helper: encode a u64 varint using bincode big-endian standard config,
    /// then attempt to decode it as a collection length via `decode_slice_len`.
    fn try_decode_len(value: u64) -> Result<usize, DecodeError> {
        let config = config::standard().with_big_endian().with_no_limit();
        let encoded = bincode::encode_to_vec(value, config).unwrap();
        let reader = bincode::de::read::SliceReader::new(&encoded);
        let mut decoder = bincode::de::DecoderImpl::new(reader, config, ());
        decode_slice_len(&mut decoder)
    }

    #[test]
    fn decode_slice_len_accepts_small_values() {
        assert_eq!(try_decode_len(0).unwrap(), 0);
        assert_eq!(try_decode_len(1).unwrap(), 1);
        assert_eq!(try_decode_len(1024).unwrap(), 1024);
    }

    #[test]
    fn decode_slice_len_accepts_at_limit() {
        let limit = MAX_COLLECTION_LEN;
        assert_eq!(try_decode_len(limit).unwrap(), limit as usize);
    }

    #[test]
    fn decode_slice_len_rejects_above_limit() {
        let above = MAX_COLLECTION_LEN + 1;
        match try_decode_len(above) {
            Err(DecodeError::LimitExceeded) => {} // expected
            other => panic!("expected LimitExceeded, got {:?}", other),
        }
    }

    #[test]
    fn decode_slice_len_rejects_huge_value() {
        // Simulates the attack vector: a varint-encoded u64::MAX
        match try_decode_len(u64::MAX) {
            Err(DecodeError::LimitExceeded) => {} // expected
            other => panic!("expected LimitExceeded, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // decode_option_variant
    // -----------------------------------------------------------------------

    fn try_decode_option_variant(byte: u8) -> Result<Option<()>, DecodeError> {
        let data = bincode::encode_to_vec(byte, cfg()).unwrap();
        let reader = bincode::de::read::SliceReader::new(&data);
        let mut decoder = bincode::de::DecoderImpl::new(reader, cfg(), ());
        decode_option_variant(&mut decoder, "test::Option")
    }

    #[test]
    fn decode_option_variant_none() {
        assert_eq!(try_decode_option_variant(0).unwrap(), None);
    }

    #[test]
    fn decode_option_variant_some() {
        assert_eq!(try_decode_option_variant(1).unwrap(), Some(()));
    }

    #[test]
    fn decode_option_variant_invalid() {
        match try_decode_option_variant(2) {
            Err(DecodeError::UnexpectedVariant { found: 2, .. }) => {}
            other => panic!("expected UnexpectedVariant, got {:?}", other),
        }
        match try_decode_option_variant(255) {
            Err(DecodeError::UnexpectedVariant { found: 255, .. }) => {}
            other => panic!("expected UnexpectedVariant, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // PlatformVersionedDecode for Option<T>
    // -----------------------------------------------------------------------

    #[test]
    fn option_some_round_trip() {
        let value: Option<u32> = Some(42);
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: Option<u32> =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, Some(42));
    }

    #[test]
    fn option_none_round_trip() {
        let value: Option<u32> = None;
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: Option<u32> =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, None);
    }

    // -----------------------------------------------------------------------
    // PlatformVersionedDecode for Result<T, U>
    // -----------------------------------------------------------------------

    #[test]
    fn result_ok_round_trip() {
        let value: Result<u32, u8> = Ok(123);
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: Result<u32, u8> =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, Ok(123));
    }

    #[test]
    fn result_err_round_trip() {
        let value: Result<u32, u8> = Err(99);
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: Result<u32, u8> =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, Err(99));
    }

    #[test]
    fn result_invalid_variant() {
        // Manually encode variant tag 2 (invalid for Result which expects 0 or 1)
        let mut data = bincode::encode_to_vec(2u32, cfg()).unwrap();
        // Append some dummy payload bytes
        data.extend_from_slice(&[0u8; 8]);
        let result =
            crate::platform_versioned_decode_from_slice::<Result<u32, u8>, _>(&data, cfg(), pv());
        match result {
            Err(DecodeError::UnexpectedVariant { found: 2, .. }) => {}
            other => panic!("expected UnexpectedVariant, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // PlatformVersionedDecode for Bound<T>
    // -----------------------------------------------------------------------

    #[test]
    fn bound_unbounded_round_trip() {
        use core::ops::Bound;
        let value: Bound<u32> = Bound::Unbounded;
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: Bound<u32> =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, Bound::Unbounded);
    }

    #[test]
    fn bound_included_round_trip() {
        use core::ops::Bound;
        let value: Bound<u32> = Bound::Included(10);
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: Bound<u32> =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, Bound::Included(10));
    }

    #[test]
    fn bound_excluded_round_trip() {
        use core::ops::Bound;
        let value: Bound<u32> = Bound::Excluded(20);
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: Bound<u32> =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, Bound::Excluded(20));
    }

    #[test]
    fn bound_invalid_variant() {
        use core::ops::Bound;
        // variant 3 is invalid for Bound (valid: 0, 1, 2)
        let mut data = bincode::encode_to_vec(3u32, cfg()).unwrap();
        data.extend_from_slice(&[0u8; 8]);
        let result =
            crate::platform_versioned_decode_from_slice::<Bound<u32>, _>(&data, cfg(), pv());
        match result {
            Err(DecodeError::UnexpectedVariant { found: 3, .. }) => {}
            other => panic!("expected UnexpectedVariant, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Array decode: u8 optimization vs non-u8 path
    // -----------------------------------------------------------------------

    #[test]
    fn array_u8_decode_round_trip() {
        // This triggers the u8-optimized path in [T; N] decode
        let value: [u8; 4] = [0xDE, 0xAD, 0xBE, 0xEF];
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: [u8; 4] =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn array_non_u8_decode_round_trip() {
        // This triggers the non-u8 path through collect_into_array
        let value: [u32; 3] = [100, 200, 300];
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: [u32; 3] =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn array_borrow_decode_u8_path() {
        let value: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: [u8; 8] =
            crate::platform_versioned_borrow_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn array_borrow_decode_non_u8_path() {
        let value: [i16; 4] = [-1, 0, 1, 32767];
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: [i16; 4] =
            crate::platform_versioned_borrow_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    // -----------------------------------------------------------------------
    // Cell / RefCell
    // -----------------------------------------------------------------------

    #[test]
    fn cell_round_trip() {
        use core::cell::Cell;
        let value = Cell::new(42u32);
        let encoded = crate::platform_encode_to_vec(&value, cfg(), pv()).unwrap();
        let decoded: Cell<u32> =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded.get(), 42);
    }

    #[test]
    fn refcell_round_trip() {
        use core::cell::RefCell;
        let value = RefCell::new(99u16);
        let encoded = crate::platform_encode_to_vec(&value, cfg(), pv()).unwrap();
        let decoded: RefCell<u16> =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(*decoded.borrow(), 99);
    }

    // -----------------------------------------------------------------------
    // Range / RangeInclusive
    // -----------------------------------------------------------------------

    #[test]
    fn range_round_trip() {
        let value: core::ops::Range<u32> = 10..20;
        let encoded = crate::platform_encode_to_vec(value.clone(), cfg(), pv()).unwrap();
        let decoded: core::ops::Range<u32> =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn range_inclusive_round_trip() {
        let value: core::ops::RangeInclusive<i32> = -5..=5;
        let encoded = crate::platform_encode_to_vec(value.clone(), cfg(), pv()).unwrap();
        let decoded: core::ops::RangeInclusive<i32> =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    // -----------------------------------------------------------------------
    // PhantomData / unit
    // -----------------------------------------------------------------------

    #[test]
    fn unit_round_trip() {
        let value = ();
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: () =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, ());
    }

    #[test]
    fn phantom_data_round_trip() {
        use core::marker::PhantomData;
        let value: PhantomData<u32> = PhantomData;
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: PhantomData<u32> =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, PhantomData);
    }

    // -----------------------------------------------------------------------
    // Primitive type round-trips (representative subset)
    // -----------------------------------------------------------------------

    #[test]
    fn bool_round_trip() {
        for value in [true, false] {
            let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
            let decoded: bool =
                crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
            assert_eq!(decoded, value);
        }
    }

    #[test]
    fn char_round_trip() {
        for value in ['a', '\u{1F600}', '\0'] {
            let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
            let decoded: char =
                crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
            assert_eq!(decoded, value);
        }
    }

    #[test]
    fn f64_round_trip() {
        let value: f64 = core::f64::consts::PI;
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: f64 =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn duration_round_trip() {
        let value = core::time::Duration::new(123, 456_789);
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: core::time::Duration =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn borrow_decode_byte_slice() {
        // Use bincode's own encode for &[u8] to match the borrow decode path
        let data: &[u8] = b"hello bytes";
        let encoded = bincode::encode_to_vec(data, cfg()).unwrap();
        let decoded: &[u8] =
            crate::platform_versioned_borrow_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn borrow_decode_str() {
        let data = "borrow me";
        let encoded = crate::platform_encode_to_vec(data, cfg(), pv()).unwrap();
        let decoded: &str =
            crate::platform_versioned_borrow_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, data);
    }

    // -----------------------------------------------------------------------
    // NonZero type round-trips (cover the delegation impls in de/impls.rs)
    // -----------------------------------------------------------------------

    macro_rules! nonzero_round_trip_test {
        ($name:ident, $ty:ty, $val:expr) => {
            #[test]
            fn $name() {
                let value: $ty = <$ty>::new($val).unwrap();
                let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
                let decoded: $ty =
                    crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
                assert_eq!(decoded, value);
            }
        };
    }

    nonzero_round_trip_test!(nonzero_u8_round_trip, core::num::NonZeroU8, 1);
    nonzero_round_trip_test!(nonzero_u16_round_trip, core::num::NonZeroU16, 100);
    nonzero_round_trip_test!(nonzero_u32_round_trip, core::num::NonZeroU32, 1000);
    nonzero_round_trip_test!(nonzero_u64_round_trip, core::num::NonZeroU64, 10000);
    nonzero_round_trip_test!(nonzero_u128_round_trip, core::num::NonZeroU128, 100000);
    nonzero_round_trip_test!(nonzero_usize_round_trip, core::num::NonZeroUsize, 42);
    nonzero_round_trip_test!(nonzero_i8_round_trip, core::num::NonZeroI8, -1);
    nonzero_round_trip_test!(nonzero_i16_round_trip, core::num::NonZeroI16, -100);
    nonzero_round_trip_test!(nonzero_i32_round_trip, core::num::NonZeroI32, -1000);
    nonzero_round_trip_test!(nonzero_i64_round_trip, core::num::NonZeroI64, -10000);
    nonzero_round_trip_test!(nonzero_i128_round_trip, core::num::NonZeroI128, -100000);
    nonzero_round_trip_test!(nonzero_isize_round_trip, core::num::NonZeroIsize, -42);

    // -----------------------------------------------------------------------
    // Remaining primitive types (i8, i16, i32, i64, i128, isize, u128, f32)
    // -----------------------------------------------------------------------

    #[test]
    fn i8_round_trip() {
        let value: i8 = -128;
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: i8 =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn i16_round_trip() {
        let value: i16 = -32768;
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: i16 =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn i32_round_trip() {
        let value: i32 = -2_000_000;
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: i32 =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn i128_round_trip() {
        let value: i128 = i128::MIN;
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: i128 =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn isize_round_trip() {
        let value: isize = -999;
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: isize =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn u128_round_trip() {
        let value: u128 = u128::MAX;
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: u128 =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn usize_round_trip() {
        let value: usize = 12345;
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: usize =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn f32_round_trip() {
        let value: f32 = core::f32::consts::E;
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: f32 =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    // -----------------------------------------------------------------------
    // Borrow-decode round-trips for Cell, RefCell, Option, Range, etc.
    // -----------------------------------------------------------------------

    #[test]
    fn cell_borrow_decode_round_trip() {
        use core::cell::Cell;
        let value = Cell::new(77u32);
        let encoded = crate::platform_encode_to_vec(&value, cfg(), pv()).unwrap();
        let decoded: Cell<u32> =
            crate::platform_versioned_borrow_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded.get(), 77);
    }

    #[test]
    fn refcell_borrow_decode_round_trip() {
        use core::cell::RefCell;
        let value = RefCell::new(88u16);
        let encoded = crate::platform_encode_to_vec(&value, cfg(), pv()).unwrap();
        let decoded: RefCell<u16> =
            crate::platform_versioned_borrow_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(*decoded.borrow(), 88);
    }

    #[test]
    fn option_borrow_decode_some() {
        let value: Option<u32> = Some(42);
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: Option<u32> =
            crate::platform_versioned_borrow_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, Some(42));
    }

    #[test]
    fn option_borrow_decode_none() {
        let value: Option<u32> = None;
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: Option<u32> =
            crate::platform_versioned_borrow_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, None);
    }

    #[test]
    fn result_borrow_decode_ok() {
        let value: Result<u32, u8> = Ok(123);
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: Result<u32, u8> =
            crate::platform_versioned_borrow_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, Ok(123));
    }

    #[test]
    fn result_borrow_decode_err() {
        let value: Result<u32, u8> = Err(99);
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: Result<u32, u8> =
            crate::platform_versioned_borrow_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, Err(99));
    }

    #[test]
    fn result_borrow_decode_invalid_variant() {
        let mut data = bincode::encode_to_vec(2u32, cfg()).unwrap();
        data.extend_from_slice(&[0u8; 8]);
        let result = crate::platform_versioned_borrow_decode_from_slice::<Result<u32, u8>, _>(
            &data,
            cfg(),
            pv(),
        );
        match result {
            Err(DecodeError::UnexpectedVariant { found: 2, .. }) => {}
            other => panic!("expected UnexpectedVariant, got {:?}", other),
        }
    }

    #[test]
    fn range_borrow_decode_round_trip() {
        let value: core::ops::Range<u32> = 5..15;
        let encoded = crate::platform_encode_to_vec(value.clone(), cfg(), pv()).unwrap();
        let decoded: core::ops::Range<u32> =
            crate::platform_versioned_borrow_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn range_inclusive_borrow_decode_round_trip() {
        let value: core::ops::RangeInclusive<i32> = -10..=10;
        let encoded = crate::platform_encode_to_vec(value.clone(), cfg(), pv()).unwrap();
        let decoded: core::ops::RangeInclusive<i32> =
            crate::platform_versioned_borrow_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn bound_borrow_decode_all_variants() {
        use core::ops::Bound;
        for value in [
            Bound::Unbounded,
            Bound::Included(10u32),
            Bound::Excluded(20u32),
        ] {
            let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
            let decoded: Bound<u32> =
                crate::platform_versioned_borrow_decode_from_slice(&encoded, cfg(), pv()).unwrap();
            assert_eq!(decoded, value);
        }
    }

    #[test]
    fn bound_borrow_decode_invalid_variant() {
        use core::ops::Bound;
        let mut data = bincode::encode_to_vec(3u32, cfg()).unwrap();
        data.extend_from_slice(&[0u8; 8]);
        let result =
            crate::platform_versioned_borrow_decode_from_slice::<Bound<u32>, _>(&data, cfg(), pv());
        match result {
            Err(DecodeError::UnexpectedVariant { found: 3, .. }) => {}
            other => panic!("expected UnexpectedVariant, got {:?}", other),
        }
    }
}
