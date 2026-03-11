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
}
