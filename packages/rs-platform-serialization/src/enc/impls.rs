use crate::PlatformVersionEncode;

use bincode::enc::Encoder;
use bincode::error::EncodeError;
use bincode::Encode;
use core::{
    cell::{Cell, RefCell},
    marker::PhantomData,
    num::{
        NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize, NonZeroU128,
        NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize,
    },
    ops::{Bound, Range, RangeInclusive},
    time::Duration,
};
use platform_version::version::PlatformVersion;

impl PlatformVersionEncode for () {
    fn platform_encode<E: Encoder>(
        &self,
        _: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Ok(())
    }
}

impl<T> PlatformVersionEncode for PhantomData<T> {
    fn platform_encode<E: Encoder>(
        &self,
        _: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Ok(())
    }
}

impl PlatformVersionEncode for bool {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionEncode for u8 {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionEncode for NonZeroU8 {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionEncode for u16 {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionEncode for NonZeroU16 {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionEncode for u32 {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionEncode for NonZeroU32 {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionEncode for u64 {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionEncode for NonZeroU64 {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionEncode for u128 {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionEncode for NonZeroU128 {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionEncode for usize {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionEncode for NonZeroUsize {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionEncode for i8 {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionEncode for NonZeroI8 {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionEncode for i16 {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionEncode for NonZeroI16 {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionEncode for i32 {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionEncode for NonZeroI32 {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionEncode for i64 {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionEncode for NonZeroI64 {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionEncode for i128 {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionEncode for NonZeroI128 {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionEncode for isize {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionEncode for NonZeroIsize {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionEncode for f32 {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionEncode for f64 {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionEncode for char {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

// BlockedTODO: https://github.com/rust-lang/rust/issues/37653
//
// We'll want to implement encoding for both &[u8] and &[T: Encode],
// but those implementations overlap because u8 also implements Encode
// impl PlatformVersionEncode for &'_ [u8] {
//     fn platform_encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
//         encoder.writer().write(*self)
//     }
// }

impl<T> PlatformVersionEncode for [T]
where
    T: PlatformVersionEncode + Encode + 'static,
{
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        platform_version: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        super::encode_slice_len(encoder, self.len())?;

        if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u8>() {
            return Encode::encode(self, encoder);
        }

        for item in self {
            item.platform_encode(encoder, platform_version)?;
        }
        Ok(())
    }
}

impl PlatformVersionEncode for str {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl<T, const N: usize> PlatformVersionEncode for [T; N]
where
    T: PlatformVersionEncode,
{
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        platform_version: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        for item in self.iter() {
            item.platform_encode(encoder, platform_version)?;
        }
        Ok(())
    }
}

impl<T> PlatformVersionEncode for Option<T>
where
    T: PlatformVersionEncode,
{
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        platform_version: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        super::encode_option_variant(encoder, self)?;
        if let Some(val) = self {
            val.platform_encode(encoder, platform_version)?;
        }
        Ok(())
    }
}

impl<T, U> PlatformVersionEncode for Result<T, U>
where
    T: PlatformVersionEncode,
    U: PlatformVersionEncode,
{
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        platform_version: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        match self {
            Ok(val) => {
                0u32.platform_encode(encoder, platform_version)?;
                val.platform_encode(encoder, platform_version)
            }
            Err(err) => {
                1u32.platform_encode(encoder, platform_version)?;
                err.platform_encode(encoder, platform_version)
            }
        }
    }
}

impl<T> PlatformVersionEncode for Cell<T>
where
    T: PlatformVersionEncode + Copy,
{
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        platform_version: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        T::platform_encode(&self.get(), encoder, platform_version)
    }
}

impl<T> PlatformVersionEncode for RefCell<T>
where
    T: PlatformVersionEncode + ?Sized,
{
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        platform_version: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        let borrow_guard = self
            .try_borrow()
            .map_err(|e| EncodeError::RefCellAlreadyBorrowed {
                inner: e,
                type_name: core::any::type_name::<RefCell<T>>(),
            })?;
        T::platform_encode(&borrow_guard, encoder, platform_version)
    }
}

impl PlatformVersionEncode for Duration {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl<T> PlatformVersionEncode for Range<T>
where
    T: Encode,
{
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl<T> PlatformVersionEncode for RangeInclusive<T>
where
    T: Encode,
{
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl<T> PlatformVersionEncode for Bound<T>
where
    T: Encode,
{
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl<T> PlatformVersionEncode for &T
where
    T: PlatformVersionEncode + ?Sized,
{
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        platform_version: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        T::platform_encode(self, encoder, platform_version)
    }
}

impl<A> PlatformVersionEncode for (A,)
where
    A: PlatformVersionEncode,
{
    fn platform_encode<_E: Encoder>(
        &self,
        encoder: &mut _E,
        platform_version: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        self.0.platform_encode(encoder, platform_version)?;
        Ok(())
    }
}

impl<A, B> PlatformVersionEncode for (A, B)
where
    A: PlatformVersionEncode,
    B: PlatformVersionEncode,
{
    fn platform_encode<_E: Encoder>(
        &self,
        encoder: &mut _E,
        platform_version: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        self.0.platform_encode(encoder, platform_version)?;
        self.1.platform_encode(encoder, platform_version)?;
        Ok(())
    }
}

impl<A, B, C> PlatformVersionEncode for (A, B, C)
where
    A: PlatformVersionEncode,
    B: PlatformVersionEncode,
    C: PlatformVersionEncode,
{
    fn platform_encode<_E: Encoder>(
        &self,
        encoder: &mut _E,
        platform_version: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        self.0.platform_encode(encoder, platform_version)?;
        self.1.platform_encode(encoder, platform_version)?;
        self.2.platform_encode(encoder, platform_version)?;
        Ok(())
    }
}

impl<A, B, C, D> PlatformVersionEncode for (A, B, C, D)
where
    A: PlatformVersionEncode,
    B: PlatformVersionEncode,
    C: PlatformVersionEncode,
    D: PlatformVersionEncode,
{
    fn platform_encode<_E: Encoder>(
        &self,
        encoder: &mut _E,
        platform_version: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        self.0.platform_encode(encoder, platform_version)?;
        self.1.platform_encode(encoder, platform_version)?;
        self.2.platform_encode(encoder, platform_version)?;
        self.3.platform_encode(encoder, platform_version)?;
        Ok(())
    }
}

impl<A, B, C, D, E> PlatformVersionEncode for (A, B, C, D, E)
where
    A: PlatformVersionEncode,
    B: PlatformVersionEncode,
    C: PlatformVersionEncode,
    D: PlatformVersionEncode,
    E: PlatformVersionEncode,
{
    fn platform_encode<_E: Encoder>(
        &self,
        encoder: &mut _E,
        platform_version: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        self.0.platform_encode(encoder, platform_version)?;
        self.1.platform_encode(encoder, platform_version)?;
        self.2.platform_encode(encoder, platform_version)?;
        self.3.platform_encode(encoder, platform_version)?;
        self.4.platform_encode(encoder, platform_version)?;
        Ok(())
    }
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

    /// Helper to encode and then decode, verifying round-trip.
    fn round_trip<T>(value: T) -> T
    where
        T: PlatformVersionEncode + crate::PlatformVersionedDecode,
    {
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap()
    }

    // -----------------------------------------------------------------------
    // Slice encode: u8 optimization vs generic path
    // -----------------------------------------------------------------------

    #[test]
    fn encode_u8_slice_optimization() {
        // Exercise the TypeId::of::<u8>() fast path in [T] encode via Vec
        let data: Vec<u8> = vec![1, 2, 3, 4, 5];
        let encoded = crate::platform_encode_to_vec(&data, cfg(), pv()).unwrap();
        let decoded: Vec<u8> =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn encode_non_u8_slice() {
        // Exercise the generic (non-u8) path in [T] encode
        let data: Vec<u32> = vec![100, 200, 300];
        let encoded = crate::platform_encode_to_vec(&data, cfg(), pv()).unwrap();
        let decoded: Vec<u32> =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn encode_empty_slice() {
        let data: &[u8] = &[];
        let encoded = crate::platform_encode_to_vec(data, cfg(), pv()).unwrap();
        let decoded: Vec<u8> =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert!(decoded.is_empty());
    }

    // -----------------------------------------------------------------------
    // Array encode
    // -----------------------------------------------------------------------

    #[test]
    fn encode_array_u32() {
        let value: [u32; 3] = [10, 20, 30];
        assert_eq!(round_trip(value), value);
    }

    #[test]
    fn encode_array_single_element() {
        let value: [u64; 1] = [42];
        assert_eq!(round_trip(value), value);
    }

    // -----------------------------------------------------------------------
    // Option encode
    // -----------------------------------------------------------------------

    #[test]
    fn encode_option_some() {
        let value: Option<i32> = Some(-42);
        assert_eq!(round_trip(value), value);
    }

    #[test]
    fn encode_option_none() {
        let value: Option<i32> = None;
        assert_eq!(round_trip(value), value);
    }

    // -----------------------------------------------------------------------
    // Result encode
    // -----------------------------------------------------------------------

    #[test]
    fn encode_result_ok() {
        let value: Result<u32, u8> = Ok(42);
        assert_eq!(round_trip(value), value);
    }

    #[test]
    fn encode_result_err() {
        let value: Result<u32, u8> = Err(1);
        assert_eq!(round_trip(value), value);
    }

    // -----------------------------------------------------------------------
    // Cell / RefCell encode
    // -----------------------------------------------------------------------

    #[test]
    fn encode_cell() {
        let value = Cell::new(77u32);
        let encoded = crate::platform_encode_to_vec(&value, cfg(), pv()).unwrap();
        let decoded: u32 =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, 77);
    }

    #[test]
    fn encode_refcell() {
        let value = RefCell::new(88u16);
        let encoded = crate::platform_encode_to_vec(&value, cfg(), pv()).unwrap();
        let decoded: u16 =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, 88);
    }

    #[test]
    fn encode_refcell_already_borrowed_returns_error() {
        let value = RefCell::new(42u32);
        let _borrow = value.borrow_mut(); // hold a mutable borrow
        let result = crate::platform_encode_to_vec(&value, cfg(), pv());
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // str encode
    // -----------------------------------------------------------------------

    #[test]
    fn encode_str() {
        let value = "hello platform";
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        let decoded: String =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    // -----------------------------------------------------------------------
    // Reference encode
    // -----------------------------------------------------------------------

    #[test]
    fn encode_ref() {
        let value: u32 = 42;
        let encoded = crate::platform_encode_to_vec(&value, cfg(), pv()).unwrap();
        let decoded: u32 =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    // -----------------------------------------------------------------------
    // Tuple encode
    // -----------------------------------------------------------------------

    #[test]
    fn encode_tuple_1() {
        let value: (u32,) = (42,);
        assert_eq!(round_trip(value), value);
    }

    #[test]
    fn encode_tuple_2() {
        let value: (u32, i8) = (42, -1);
        assert_eq!(round_trip(value), value);
    }

    #[test]
    fn encode_tuple_3() {
        let value: (u8, u16, u32) = (1, 2, 3);
        assert_eq!(round_trip(value), value);
    }

    #[test]
    fn encode_tuple_4() {
        let value: (u8, u16, u32, u64) = (1, 2, 3, 4);
        assert_eq!(round_trip(value), value);
    }

    #[test]
    fn encode_tuple_5() {
        let value: (u8, u16, u32, u64, bool) = (1, 2, 3, 4, true);
        assert_eq!(round_trip(value), value);
    }

    // -----------------------------------------------------------------------
    // Duration / Range / RangeInclusive / Bound
    // -----------------------------------------------------------------------

    #[test]
    fn encode_duration() {
        let value = Duration::new(60, 500_000_000);
        assert_eq!(round_trip(value), value);
    }

    #[test]
    fn encode_range() {
        let value = 10u32..20u32;
        assert_eq!(round_trip(value.clone()), value);
    }

    #[test]
    fn encode_range_inclusive() {
        let value = 10u32..=20u32;
        assert_eq!(round_trip(value.clone()), value);
    }

    #[test]
    fn encode_bound_unbounded() {
        use core::ops::Bound;
        let value: Bound<u32> = Bound::Unbounded;
        assert_eq!(round_trip(value), value);
    }

    #[test]
    fn encode_bound_included() {
        use core::ops::Bound;
        let value: Bound<u32> = Bound::Included(5);
        assert_eq!(round_trip(value), value);
    }

    #[test]
    fn encode_bound_excluded() {
        use core::ops::Bound;
        let value: Bound<u32> = Bound::Excluded(5);
        assert_eq!(round_trip(value), value);
    }
}
