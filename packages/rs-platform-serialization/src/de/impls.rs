use crate::de::PlatformVersionedDecode;
use crate::{impl_platform_versioned_borrow_decode, PlatformVersionedBorrowDecode};
use bincode::de::read::Reader;
use bincode::de::{BorrowDecoder, Decoder};
use bincode::error::DecodeError;
use bincode::Decode;
use core::{
    any::TypeId,
    cell::{Cell, RefCell},
    num::{
        NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize, NonZeroU128,
        NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize,
    },
    ops::{Bound, Range, RangeInclusive},
    time::Duration,
};
use platform_version::version::PlatformVersion;

impl PlatformVersionedDecode for bool {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(bool);

impl PlatformVersionedDecode for u8 {
    #[inline]
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(u8);

impl PlatformVersionedDecode for NonZeroU8 {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(NonZeroU8);

impl PlatformVersionedDecode for u16 {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(u16);

impl PlatformVersionedDecode for NonZeroU16 {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(NonZeroU16);

impl PlatformVersionedDecode for u32 {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(u32);

impl PlatformVersionedDecode for NonZeroU32 {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(NonZeroU32);

impl PlatformVersionedDecode for u64 {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(u64);

impl PlatformVersionedDecode for NonZeroU64 {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(NonZeroU64);

impl PlatformVersionedDecode for u128 {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(u128);

impl PlatformVersionedDecode for NonZeroU128 {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(NonZeroU128);

impl PlatformVersionedDecode for usize {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(usize);

impl PlatformVersionedDecode for NonZeroUsize {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(NonZeroUsize);

impl PlatformVersionedDecode for i8 {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(i8);

impl PlatformVersionedDecode for NonZeroI8 {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(NonZeroI8);

impl PlatformVersionedDecode for i16 {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(i16);

impl PlatformVersionedDecode for NonZeroI16 {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(NonZeroI16);

impl PlatformVersionedDecode for i32 {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(i32);

impl PlatformVersionedDecode for NonZeroI32 {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(NonZeroI32);

impl PlatformVersionedDecode for i64 {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(i64);

impl PlatformVersionedDecode for NonZeroI64 {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(NonZeroI64);

impl PlatformVersionedDecode for i128 {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(i128);

impl PlatformVersionedDecode for NonZeroI128 {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(NonZeroI128);

impl PlatformVersionedDecode for isize {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(isize);

impl PlatformVersionedDecode for NonZeroIsize {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(NonZeroIsize);

impl PlatformVersionedDecode for f32 {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(f32);

impl PlatformVersionedDecode for f64 {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(f64);

impl PlatformVersionedDecode for char {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(char);

impl<'a, 'de: 'a> PlatformVersionedBorrowDecode<'de> for &'a [u8] {
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::BorrowDecode::borrow_decode(decoder)
    }
}

impl<'a, 'de: 'a> PlatformVersionedBorrowDecode<'de> for &'a str {
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::BorrowDecode::borrow_decode(decoder)
    }
}

impl<T, const N: usize> PlatformVersionedDecode for [T; N]
where
    T: PlatformVersionedDecode + Sized + 'static,
{
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        decoder.claim_bytes_read(core::mem::size_of::<[T; N]>())?;

        // Optimize for `[u8; N]`
        if TypeId::of::<u8>() == TypeId::of::<T>() {
            let mut buf = [0u8; N];
            decoder.reader().read(&mut buf)?;
            let ptr = &mut buf as *mut _ as *mut [T; N];

            // Safety: we know that T is a u8, so it is perfectly safe to
            // translate an array of u8 into an array of T
            let res = unsafe { ptr.read() };
            Ok(res)
        } else {
            let result = super::impl_core::collect_into_array(&mut (0..N).map(|_| {
                // See the documentation on `unclaim_bytes_read` as to why we're doing this here
                decoder.unclaim_bytes_read(core::mem::size_of::<T>());
                T::platform_versioned_decode(decoder, platform_version)
            }));

            // result is only None if N does not match the values of `(0..N)`, which it always should
            // So this unwrap should never occur
            result.unwrap()
        }
    }
}

impl<'de, T, const N: usize> PlatformVersionedBorrowDecode<'de> for [T; N]
where
    T: PlatformVersionedBorrowDecode<'de> + Sized + 'static,
{
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        decoder.claim_bytes_read(core::mem::size_of::<[T; N]>())?;

        // Optimize for `[u8; N]`
        if TypeId::of::<u8>() == TypeId::of::<T>() {
            let mut buf = [0u8; N];
            decoder.reader().read(&mut buf)?;
            let ptr = &mut buf as *mut _ as *mut [T; N];

            // Safety: we know that T is a u8, so it is perfectly safe to
            // translate an array of u8 into an array of T
            let res = unsafe { ptr.read() };
            Ok(res)
        } else {
            let result = super::impl_core::collect_into_array(&mut (0..N).map(|_| {
                // See the documentation on `unclaim_bytes_read` as to why we're doing this here
                decoder.unclaim_bytes_read(core::mem::size_of::<T>());
                T::platform_versioned_borrow_decode(decoder, platform_version)
            }));

            // result is only None if N does not match the values of `(0..N)`, which it always should
            // So this unwrap should never occur
            result.unwrap()
        }
    }
}

impl PlatformVersionedDecode for () {
    fn platform_versioned_decode<D: Decoder>(
        _: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        Ok(())
    }
}
impl_platform_versioned_borrow_decode!(());

impl<T> PlatformVersionedDecode for core::marker::PhantomData<T> {
    fn platform_versioned_decode<D: Decoder>(
        _: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        Ok(core::marker::PhantomData)
    }
}
impl<'de, T> PlatformVersionedBorrowDecode<'de> for core::marker::PhantomData<T> {
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
        _: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        Ok(core::marker::PhantomData)
    }
}

impl<T> PlatformVersionedDecode for Option<T>
where
    T: PlatformVersionedDecode,
{
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        match super::decode_option_variant(decoder, core::any::type_name::<Option<T>>())? {
            Some(_) => {
                let val = T::platform_versioned_decode(decoder, platform_version)?;
                Ok(Some(val))
            }
            None => Ok(None),
        }
    }
}

impl<'de, T> PlatformVersionedBorrowDecode<'de> for Option<T>
where
    T: PlatformVersionedBorrowDecode<'de>,
{
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        match super::decode_option_variant(decoder, core::any::type_name::<Option<T>>())? {
            Some(_) => {
                let val = T::platform_versioned_borrow_decode(decoder, platform_version)?;
                Ok(Some(val))
            }
            None => Ok(None),
        }
    }
}

// BlockedTODO: https://github.com/rust-lang/rust/issues/37653
//
// We'll want to implement BorrowDecode for both Option<&[u8]> and Option<&[T: Encode]>,
// but those implementations overlap because &'a [u8] also implements BorrowDecode
// impl<'a, 'de: 'a> PlatformVersionedBorrowDecode<'de> for Option<&'a [u8]> {
//     fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(decoder: &mut D, platform_version: &PlatformVersion) -> Result<Self, DecodeError> {
//         match super::decode_option_variant(decoder, core::any::type_name::<Option<&[u8]>>())? {
//             Some(_) => {
//                 let val = BorrowDecode::platform_versioned_borrow_decode(decoder, platform_version)?;
//                 Ok(Some(val))
//             }
//             None => Ok(None),
//         }
//     }
// }

impl<T, U> PlatformVersionedDecode for Result<T, U>
where
    T: PlatformVersionedDecode,
    U: PlatformVersionedDecode,
{
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let is_ok = u32::decode(decoder)?;
        match is_ok {
            0 => {
                let t = T::platform_versioned_decode(decoder, platform_version)?;
                Ok(Ok(t))
            }
            1 => {
                let u = U::platform_versioned_decode(decoder, platform_version)?;
                Ok(Err(u))
            }
            x => Err(DecodeError::UnexpectedVariant {
                found: x,
                allowed: &bincode::error::AllowedEnumVariants::Range { max: 1, min: 0 },
                type_name: core::any::type_name::<Result<T, U>>(),
            }),
        }
    }
}

impl<'de, T, U> PlatformVersionedBorrowDecode<'de> for Result<T, U>
where
    T: PlatformVersionedBorrowDecode<'de>,
    U: PlatformVersionedBorrowDecode<'de>,
{
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let is_ok = u32::decode(decoder)?;
        match is_ok {
            0 => {
                let t = T::platform_versioned_borrow_decode(decoder, platform_version)?;
                Ok(Ok(t))
            }
            1 => {
                let u = U::platform_versioned_borrow_decode(decoder, platform_version)?;
                Ok(Err(u))
            }
            x => Err(DecodeError::UnexpectedVariant {
                found: x,
                allowed: &bincode::error::AllowedEnumVariants::Range { max: 1, min: 0 },
                type_name: core::any::type_name::<Result<T, U>>(),
            }),
        }
    }
}

impl<T> PlatformVersionedDecode for Cell<T>
where
    T: PlatformVersionedDecode,
{
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let t = T::platform_versioned_decode(decoder, platform_version)?;
        Ok(Cell::new(t))
    }
}

impl<'de, T> PlatformVersionedBorrowDecode<'de> for Cell<T>
where
    T: PlatformVersionedBorrowDecode<'de>,
{
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let t = T::platform_versioned_borrow_decode(decoder, platform_version)?;
        Ok(Cell::new(t))
    }
}

impl<T> PlatformVersionedDecode for RefCell<T>
where
    T: PlatformVersionedDecode,
{
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let t = T::platform_versioned_decode(decoder, platform_version)?;
        Ok(RefCell::new(t))
    }
}

impl<'de, T> PlatformVersionedBorrowDecode<'de> for RefCell<T>
where
    T: PlatformVersionedBorrowDecode<'de>,
{
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let t = T::platform_versioned_borrow_decode(decoder, platform_version)?;
        Ok(RefCell::new(t))
    }
}

impl PlatformVersionedDecode for Duration {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(Duration);

impl<T> PlatformVersionedDecode for Range<T>
where
    T: PlatformVersionedDecode,
{
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let min = T::platform_versioned_decode(decoder, platform_version)?;
        let max = T::platform_versioned_decode(decoder, platform_version)?;
        Ok(min..max)
    }
}
impl<'de, T> PlatformVersionedBorrowDecode<'de> for Range<T>
where
    T: PlatformVersionedBorrowDecode<'de>,
{
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let min = T::platform_versioned_borrow_decode(decoder, platform_version)?;
        let max = T::platform_versioned_borrow_decode(decoder, platform_version)?;
        Ok(min..max)
    }
}

impl<T> PlatformVersionedDecode for RangeInclusive<T>
where
    T: PlatformVersionedDecode,
{
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let min = T::platform_versioned_decode(decoder, platform_version)?;
        let max = T::platform_versioned_decode(decoder, platform_version)?;
        Ok(RangeInclusive::new(min, max))
    }
}

impl<'de, T> PlatformVersionedBorrowDecode<'de> for RangeInclusive<T>
where
    T: PlatformVersionedBorrowDecode<'de>,
{
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let min = T::platform_versioned_borrow_decode(decoder, platform_version)?;
        let max = T::platform_versioned_borrow_decode(decoder, platform_version)?;
        Ok(RangeInclusive::new(min, max))
    }
}

impl<T> PlatformVersionedDecode for Bound<T>
where
    T: PlatformVersionedDecode,
{
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        match u32::decode(decoder)? {
            0 => Ok(Bound::Unbounded),
            1 => Ok(Bound::Included(T::platform_versioned_decode(
                decoder,
                platform_version,
            )?)),
            2 => Ok(Bound::Excluded(T::platform_versioned_decode(
                decoder,
                platform_version,
            )?)),
            x => Err(DecodeError::UnexpectedVariant {
                allowed: &bincode::error::AllowedEnumVariants::Range { max: 2, min: 0 },
                found: x,
                type_name: core::any::type_name::<Bound<T>>(),
            }),
        }
    }
}

impl<'de, T> PlatformVersionedBorrowDecode<'de> for Bound<T>
where
    T: PlatformVersionedBorrowDecode<'de>,
{
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        match u32::decode(decoder)? {
            0 => Ok(Bound::Unbounded),
            1 => Ok(Bound::Included(T::platform_versioned_borrow_decode(
                decoder,
                platform_version,
            )?)),
            2 => Ok(Bound::Excluded(T::platform_versioned_borrow_decode(
                decoder,
                platform_version,
            )?)),
            x => Err(DecodeError::UnexpectedVariant {
                allowed: &bincode::error::AllowedEnumVariants::Range { max: 2, min: 0 },
                found: x,
                type_name: core::any::type_name::<Bound<T>>(),
            }),
        }
    }
}

const UTF8_CHAR_WIDTH: [u8; 256] = [
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, // 0x1F
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, // 0x3F
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, // 0x5F
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, // 0x7F
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, // 0x9F
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, // 0xBF
    0, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
    2, // 0xDF
    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, // 0xEF
    4, 4, 4, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0xFF
];

// This function is a copy of core::str::utf8_char_width
const fn utf8_char_width(b: u8) -> usize {
    UTF8_CHAR_WIDTH[b as usize] as usize
}
