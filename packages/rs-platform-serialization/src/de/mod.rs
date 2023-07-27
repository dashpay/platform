//! Decoder-based structs and traits.

mod impl_core;
mod impl_tuples;
mod impls;

use bincode::de::{BorrowDecoder, Decoder};
use bincode::error::DecodeError;
use bincode::Decode;
use platform_version::version::PlatformVersion;

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
/// impl bincode::Decode for Entity {
///     fn decode<D: bincode::de::Decoder>(
///         decoder: &mut D,
///     ) -> core::result::Result<Self, bincode::error::DecodeError> {
///         Ok(Self {
///             x: bincode::Decode::decode(decoder)?,
///             y: bincode::Decode::decode(decoder)?,
///         })
///     }
/// }
/// impl<'de> bincode::BorrowDecode<'de> for Entity {
///     fn borrow_decode<D: bincode::de::BorrowDecoder<'de>>(
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
/// # impl bincode::Decode for Foo {
/// #     fn decode<D: bincode::de::Decoder>(
/// #         decoder: &mut D,
/// #     ) -> core::result::Result<Self, bincode::error::DecodeError> {
/// let x: u8 = bincode::Decode::decode(decoder)?;
/// let x = <u8 as bincode::Decode>::decode(decoder)?;
/// #         Ok(Foo)
/// #     }
/// # }
/// # bincode::impl_borrow_decode!(Foo);
/// ```
pub trait PlatformVersionedDecode: Sized {
    /// Attempt to decode this type with the given [Decode].
    fn platform_versioned_decode<D: Decoder>(
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
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError>;
}

/// Helper macro to implement `PlatformVersionedBorrowDecode` for any type that implements `PlatformVersionedDecode`.
#[macro_export]
macro_rules! impl_platform_versioned_borrow_decode {
    ($ty:ty) => {
        impl<'de> $crate::PlatformVersionedBorrowDecode<'de> for $ty {
            fn platform_versioned_borrow_decode<D: bincode::de::BorrowDecoder<'de>>(
                decoder: &mut D,
                platform_version: &PlatformVersion,
            ) -> core::result::Result<Self, bincode::error::DecodeError> {
                $crate::PlatformVersionedBorrowDecode::platform_versioned_borrow_decode(
                    decoder,
                    platform_version,
                )
            }
        }
    };
}

/// Decodes only the option variant from the decoder. Will not read any more data than that.
#[inline]
pub(crate) fn decode_option_variant<D: Decoder>(
    decoder: &mut D,
    type_name: &'static str,
) -> Result<Option<()>, DecodeError> {
    let is_some = u8::decode(decoder)?;
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

/// Decodes the length of any slice, container, etc from the decoder
#[inline]
pub(crate) fn decode_slice_len<D: Decoder>(decoder: &mut D) -> Result<usize, DecodeError> {
    let v = u64::decode(decoder)?;

    v.try_into().map_err(|_| DecodeError::OutsideUsizeRange(v))
}
