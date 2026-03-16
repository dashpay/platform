use crate::{
    impl_platform_versioned_borrow_decode, PlatformVersionEncode, PlatformVersionedBorrowDecode,
    PlatformVersionedDecode,
};
#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Arc;
use alloc::{
    borrow::{Cow, ToOwned},
    boxed::Box,
    collections::*,
    rc::Rc,
    string::String,
    vec::Vec,
};
use bincode::config::Config;
use bincode::de::read::Reader;
use bincode::de::{BorrowDecoder, Decoder};
use bincode::enc::write::{SizeWriter, Writer};
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{enc, Encode};
use platform_version::version::PlatformVersion;

#[derive(Default)]
pub(crate) struct VecWriter {
    inner: Vec<u8>,
}

impl VecWriter {
    /// Create a new vec writer with the given capacity
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            inner: Vec::with_capacity(cap),
        }
    }
    // May not be used in all feature combinations
    #[allow(dead_code)]
    pub(crate) fn collect(self) -> Vec<u8> {
        self.inner
    }
}

impl bincode::enc::write::Writer for VecWriter {
    #[inline(always)]
    fn write(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
        self.inner.extend_from_slice(bytes);
        Ok(())
    }
}

/// PlatformVersionEncode the given value into a `Vec<u8>` with the given `Config`. See the [config] module for more information.
///
/// [config]: config/index.html
#[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
pub fn platform_encode_to_vec<E: PlatformVersionEncode, C: Config>(
    val: E,
    config: C,
    platform_version: &PlatformVersion,
) -> Result<Vec<u8>, EncodeError> {
    let size = {
        let mut size_writer = enc::EncoderImpl::<_, C>::new(SizeWriter::default(), config);
        val.platform_encode(&mut size_writer, platform_version)?;
        size_writer.into_writer().bytes_written
    };
    let writer = VecWriter::with_capacity(size);
    let mut encoder = enc::EncoderImpl::<_, C>::new(writer, config);
    val.platform_encode(&mut encoder, platform_version)?;
    Ok(encoder.into_writer().inner)
}

impl<T> PlatformVersionedDecode for BinaryHeap<T>
where
    T: PlatformVersionedDecode + Ord,
{
    fn platform_versioned_decode<D: Decoder<Context = crate::BincodeContext>>(
        decoder: &mut D,
        platform_versioned: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let len = crate::de::decode_slice_len(decoder)?;
        decoder.claim_container_read::<T>(len)?;

        let mut map = BinaryHeap::with_capacity(len);
        for _ in 0..len {
            // See the documentation on `unclaim_bytes_read` as to why we're doing this here
            decoder.unclaim_bytes_read(core::mem::size_of::<T>());

            let key = T::platform_versioned_decode(decoder, platform_versioned)?;
            map.push(key);
        }
        Ok(map)
    }
}
impl<'de, T> PlatformVersionedBorrowDecode<'de> for BinaryHeap<T>
where
    T: PlatformVersionedBorrowDecode<'de> + Ord,
{
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de, Context = crate::BincodeContext>>(
        decoder: &mut D,
        platform_versioned: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let len = crate::de::decode_slice_len(decoder)?;
        decoder.claim_container_read::<T>(len)?;

        let mut map = BinaryHeap::with_capacity(len);
        for _ in 0..len {
            // See the documentation on `unclaim_bytes_read` as to why we're doing this here
            decoder.unclaim_bytes_read(core::mem::size_of::<T>());

            let key = T::platform_versioned_borrow_decode(decoder, platform_versioned)?;
            map.push(key);
        }
        Ok(map)
    }
}

impl<T> PlatformVersionEncode for BinaryHeap<T>
where
    T: PlatformVersionEncode + Ord,
{
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        platform_version: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        crate::enc::encode_slice_len(encoder, self.len())?;
        for val in self.iter() {
            val.platform_encode(encoder, platform_version)?;
        }
        Ok(())
    }
}

impl<K, V> PlatformVersionedDecode for BTreeMap<K, V>
where
    K: PlatformVersionedDecode + Ord,
    V: PlatformVersionedDecode,
{
    fn platform_versioned_decode<D: Decoder<Context = crate::BincodeContext>>(
        decoder: &mut D,
        platform_versioned: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let len = crate::de::decode_slice_len(decoder)?;
        decoder.claim_container_read::<(K, V)>(len)?;

        let mut map = BTreeMap::new();
        for _ in 0..len {
            // See the documentation on `unclaim_bytes_read` as to why we're doing this here
            decoder.unclaim_bytes_read(core::mem::size_of::<(K, V)>());

            let key = K::platform_versioned_decode(decoder, platform_versioned)?;
            let value = V::platform_versioned_decode(decoder, platform_versioned)?;
            map.insert(key, value);
        }
        Ok(map)
    }
}
impl<'de, K, V> PlatformVersionedBorrowDecode<'de> for BTreeMap<K, V>
where
    K: PlatformVersionedBorrowDecode<'de> + Ord,
    V: PlatformVersionedBorrowDecode<'de>,
{
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de, Context = crate::BincodeContext>>(
        decoder: &mut D,
        platform_versioned: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let len = crate::de::decode_slice_len(decoder)?;
        decoder.claim_container_read::<(K, V)>(len)?;

        let mut map = BTreeMap::new();
        for _ in 0..len {
            // See the documentation on `unclaim_bytes_read` as to why we're doing this here
            decoder.unclaim_bytes_read(core::mem::size_of::<(K, V)>());

            let key = K::platform_versioned_borrow_decode(decoder, platform_versioned)?;
            let value = V::platform_versioned_borrow_decode(decoder, platform_versioned)?;
            map.insert(key, value);
        }
        Ok(map)
    }
}

impl<K, V> PlatformVersionEncode for BTreeMap<K, V>
where
    K: PlatformVersionEncode + Ord,
    V: PlatformVersionEncode,
{
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        platform_version: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        crate::enc::encode_slice_len(encoder, self.len())?;
        for (key, val) in self.iter() {
            key.platform_encode(encoder, platform_version)?;
            val.platform_encode(encoder, platform_version)?;
        }
        Ok(())
    }
}

impl<T> PlatformVersionedDecode for BTreeSet<T>
where
    T: PlatformVersionedDecode + Ord,
{
    fn platform_versioned_decode<D: Decoder<Context = crate::BincodeContext>>(
        decoder: &mut D,
        platform_versioned: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let len = crate::de::decode_slice_len(decoder)?;
        decoder.claim_container_read::<T>(len)?;

        let mut map = BTreeSet::new();
        for _ in 0..len {
            // See the documentation on `unclaim_bytes_read` as to why we're doing this here
            decoder.unclaim_bytes_read(core::mem::size_of::<T>());

            let key = T::platform_versioned_decode(decoder, platform_versioned)?;
            map.insert(key);
        }
        Ok(map)
    }
}
impl<'de, T> PlatformVersionedBorrowDecode<'de> for BTreeSet<T>
where
    T: PlatformVersionedBorrowDecode<'de> + Ord,
{
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de, Context = crate::BincodeContext>>(
        decoder: &mut D,
        platform_versioned: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let len = crate::de::decode_slice_len(decoder)?;
        decoder.claim_container_read::<T>(len)?;

        let mut map = BTreeSet::new();
        for _ in 0..len {
            // See the documentation on `unclaim_bytes_read` as to why we're doing this here
            decoder.unclaim_bytes_read(core::mem::size_of::<T>());

            let key = T::platform_versioned_borrow_decode(decoder, platform_versioned)?;
            map.insert(key);
        }
        Ok(map)
    }
}

impl<T> PlatformVersionEncode for BTreeSet<T>
where
    T: PlatformVersionEncode + Ord,
{
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        platform_version: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        crate::enc::encode_slice_len(encoder, self.len())?;
        for item in self.iter() {
            item.platform_encode(encoder, platform_version)?;
        }
        Ok(())
    }
}

impl<T> PlatformVersionedDecode for VecDeque<T>
where
    T: PlatformVersionedDecode,
{
    fn platform_versioned_decode<D: Decoder<Context = crate::BincodeContext>>(
        decoder: &mut D,
        platform_versioned: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let len = crate::de::decode_slice_len(decoder)?;
        decoder.claim_container_read::<T>(len)?;

        let mut map = VecDeque::with_capacity(len);
        for _ in 0..len {
            // See the documentation on `unclaim_bytes_read` as to why we're doing this here
            decoder.unclaim_bytes_read(core::mem::size_of::<T>());

            let key = T::platform_versioned_decode(decoder, platform_versioned)?;
            map.push_back(key);
        }
        Ok(map)
    }
}
impl<'de, T> PlatformVersionedBorrowDecode<'de> for VecDeque<T>
where
    T: PlatformVersionedBorrowDecode<'de>,
{
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de, Context = crate::BincodeContext>>(
        decoder: &mut D,
        platform_versioned: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let len = crate::de::decode_slice_len(decoder)?;
        decoder.claim_container_read::<T>(len)?;

        let mut map = VecDeque::with_capacity(len);
        for _ in 0..len {
            // See the documentation on `unclaim_bytes_read` as to why we're doing this here
            decoder.unclaim_bytes_read(core::mem::size_of::<T>());

            let key = T::platform_versioned_borrow_decode(decoder, platform_versioned)?;
            map.push_back(key);
        }
        Ok(map)
    }
}

impl<T> PlatformVersionEncode for VecDeque<T>
where
    T: PlatformVersionEncode,
{
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        platform_version: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        crate::enc::encode_slice_len(encoder, self.len())?;
        for item in self.iter() {
            item.platform_encode(encoder, platform_version)?;
        }
        Ok(())
    }
}

impl<T> PlatformVersionedDecode for Vec<T>
where
    T: PlatformVersionedDecode + 'static,
{
    fn platform_versioned_decode<D: Decoder<Context = crate::BincodeContext>>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let len = crate::de::decode_slice_len(decoder)?;

        if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u8>() {
            decoder.claim_container_read::<T>(len)?;
            // optimize for reading u8 vecs
            let mut vec = vec![0u8; len];
            decoder.reader().read(&mut vec)?;
            // Safety: Vec<T> is Vec<u8>
            return Ok(unsafe { core::mem::transmute::<Vec<u8>, Vec<T>>(vec) });
        }
        decoder.claim_container_read::<T>(len)?;

        let mut vec = Vec::with_capacity(len);
        for _ in 0..len {
            // See the documentation on `unclaim_bytes_read` as to why we're doing this here
            decoder.unclaim_bytes_read(core::mem::size_of::<T>());

            vec.push(T::platform_versioned_decode(decoder, platform_version)?);
        }
        Ok(vec)
    }
}

impl<'de, T> PlatformVersionedBorrowDecode<'de> for Vec<T>
where
    T: PlatformVersionedBorrowDecode<'de>,
{
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de, Context = crate::BincodeContext>>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let len = crate::de::decode_slice_len(decoder)?;
        decoder.claim_container_read::<T>(len)?;

        let mut vec = Vec::with_capacity(len);
        for _ in 0..len {
            // See the documentation on `unclaim_bytes_read` as to why we're doing this here
            decoder.unclaim_bytes_read(core::mem::size_of::<T>());

            vec.push(T::platform_versioned_borrow_decode(
                decoder,
                platform_version,
            )?);
        }
        Ok(vec)
    }
}

impl<T> PlatformVersionEncode for Vec<T>
where
    T: PlatformVersionEncode + 'static,
{
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        platform_version: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        crate::enc::encode_slice_len(encoder, self.len())?;
        if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u8>() {
            let slice: &[u8] = unsafe { core::mem::transmute(self.as_slice()) };
            encoder.writer().write(slice)?;
            return Ok(());
        }
        for item in self.iter() {
            item.platform_encode(encoder, platform_version)?;
        }
        Ok(())
    }
}

impl PlatformVersionedDecode for String {
    fn platform_versioned_decode<D: Decoder<Context = crate::BincodeContext>>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(String);

impl PlatformVersionedDecode for Box<str> {
    fn platform_versioned_decode<D: Decoder<Context = crate::BincodeContext>>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(Box<str>);

impl PlatformVersionEncode for String {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl<T> PlatformVersionedDecode for Box<T>
where
    T: PlatformVersionedDecode,
{
    fn platform_versioned_decode<D: Decoder<Context = crate::BincodeContext>>(
        decoder: &mut D,
        platform_versioned: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let t = T::platform_versioned_decode(decoder, platform_versioned)?;
        Ok(Box::new(t))
    }
}
impl<'de, T> PlatformVersionedBorrowDecode<'de> for Box<T>
where
    T: PlatformVersionedBorrowDecode<'de>,
{
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de, Context = crate::BincodeContext>>(
        decoder: &mut D,
        platform_versioned: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let t = T::platform_versioned_borrow_decode(decoder, platform_versioned)?;
        Ok(Box::new(t))
    }
}

impl<T> PlatformVersionEncode for Box<T>
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

impl<T> PlatformVersionedDecode for Box<[T]>
where
    T: PlatformVersionedDecode + 'static,
{
    fn platform_versioned_decode<D: Decoder<Context = crate::BincodeContext>>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let vec = Vec::platform_versioned_decode(decoder, platform_version)?;
        Ok(vec.into_boxed_slice())
    }
}

impl<'de, T> PlatformVersionedBorrowDecode<'de> for Box<[T]>
where
    T: PlatformVersionedBorrowDecode<'de> + 'de,
{
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de, Context = crate::BincodeContext>>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let vec = Vec::platform_versioned_borrow_decode(decoder, platform_version)?;
        Ok(vec.into_boxed_slice())
    }
}

impl<T> PlatformVersionedDecode for Cow<'_, T>
where
    T: ToOwned + ?Sized,
    <T as ToOwned>::Owned: PlatformVersionedDecode,
{
    fn platform_versioned_decode<D: Decoder<Context = crate::BincodeContext>>(
        decoder: &mut D,
        platform_versioned: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let t = <T as ToOwned>::Owned::platform_versioned_decode(decoder, platform_versioned)?;
        Ok(Cow::Owned(t))
    }
}
impl<'cow, T> PlatformVersionedBorrowDecode<'cow> for Cow<'cow, T>
where
    T: ToOwned + ?Sized,
    &'cow T: PlatformVersionedBorrowDecode<'cow>,
{
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'cow, Context = crate::BincodeContext>>(
        decoder: &mut D,
        platform_versioned: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let t = <&T>::platform_versioned_borrow_decode(decoder, platform_versioned)?;
        Ok(Cow::Borrowed(t))
    }
}

impl<T> PlatformVersionEncode for Cow<'_, T>
where
    T: ToOwned + ?Sized,
    for<'a> &'a T: PlatformVersionEncode,
{
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        platform_version: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        self.as_ref().platform_encode(encoder, platform_version)
    }
}

#[test]
fn test_cow_round_trip() {
    let start = Cow::Borrowed("Foo");
    let encoded = crate::platform_encode_to_vec(
        &start,
        bincode::config::standard(),
        PlatformVersion::first(),
    )
    .unwrap();
    let end = crate::platform_versioned_borrow_decode_from_slice::<Cow<str>, _>(
        &encoded,
        bincode::config::standard(),
        PlatformVersion::first(),
    )
    .unwrap();
    assert_eq!(start, end);
    let end = crate::platform_versioned_decode_from_slice::<Cow<str>, _>(
        &encoded,
        bincode::config::standard(),
        PlatformVersion::first(),
    )
    .unwrap();
    assert_eq!(start, end);
}

impl<T> PlatformVersionedDecode for Rc<T>
where
    T: PlatformVersionedDecode,
{
    fn platform_versioned_decode<D: Decoder<Context = crate::BincodeContext>>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let t = T::platform_versioned_decode(decoder, platform_version)?;
        Ok(Rc::new(t))
    }
}

impl<'de, T> PlatformVersionedBorrowDecode<'de> for Rc<T>
where
    T: PlatformVersionedBorrowDecode<'de>,
{
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de, Context = crate::BincodeContext>>(
        decoder: &mut D,
        platform_versioned: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let t = T::platform_versioned_borrow_decode(decoder, platform_versioned)?;
        Ok(Rc::new(t))
    }
}

impl<T> PlatformVersionEncode for Rc<T>
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

impl<T> PlatformVersionedDecode for Rc<[T]>
where
    T: PlatformVersionedDecode + 'static,
{
    fn platform_versioned_decode<D: Decoder<Context = crate::BincodeContext>>(
        decoder: &mut D,
        platform_versioned: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let vec = Vec::platform_versioned_decode(decoder, platform_versioned)?;
        Ok(vec.into())
    }
}

impl<'de, T> PlatformVersionedBorrowDecode<'de> for Rc<[T]>
where
    T: PlatformVersionedBorrowDecode<'de> + 'de,
{
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de, Context = crate::BincodeContext>>(
        decoder: &mut D,
        platform_versioned: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let vec = Vec::platform_versioned_borrow_decode(decoder, platform_versioned)?;
        Ok(vec.into())
    }
}

#[cfg(target_has_atomic = "ptr")]
impl<T> PlatformVersionedDecode for Arc<T>
where
    T: PlatformVersionedDecode,
{
    fn platform_versioned_decode<D: Decoder<Context = crate::BincodeContext>>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let t = T::platform_versioned_decode(decoder, platform_version)?;
        Ok(Arc::new(t))
    }
}

#[cfg(target_has_atomic = "ptr")]
impl PlatformVersionedDecode for Arc<str> {
    fn platform_versioned_decode<D: Decoder<Context = crate::BincodeContext>>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}

#[cfg(target_has_atomic = "ptr")]
impl<'de, T> PlatformVersionedBorrowDecode<'de> for Arc<T>
where
    T: PlatformVersionedBorrowDecode<'de>,
{
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de, Context = crate::BincodeContext>>(
        decoder: &mut D,
        platform_versioned: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let t = T::platform_versioned_borrow_decode(decoder, platform_versioned)?;
        Ok(Arc::new(t))
    }
}

#[cfg(target_has_atomic = "ptr")]
impl<'de> PlatformVersionedBorrowDecode<'de> for Arc<str> {
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de, Context = crate::BincodeContext>>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::BorrowDecode::borrow_decode(decoder)
    }
}

#[cfg(target_has_atomic = "ptr")]
impl<T> PlatformVersionEncode for Arc<T>
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

#[cfg(target_has_atomic = "ptr")]
impl<T> PlatformVersionedDecode for Arc<[T]>
where
    T: PlatformVersionedDecode + 'static,
{
    fn platform_versioned_decode<D: Decoder<Context = crate::BincodeContext>>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let vec = Vec::platform_versioned_decode(decoder, platform_version)?;
        Ok(vec.into())
    }
}

#[cfg(target_has_atomic = "ptr")]
impl<'de, T> PlatformVersionedBorrowDecode<'de> for Arc<[T]>
where
    T: PlatformVersionedBorrowDecode<'de> + 'de,
{
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de, Context = crate::BincodeContext>>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let vec = Vec::platform_versioned_borrow_decode(decoder, platform_version)?;
        Ok(vec.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque};
    use bincode::config;

    fn cfg() -> impl bincode::config::Config {
        config::standard().with_big_endian().with_no_limit()
    }

    fn pv() -> &'static PlatformVersion {
        PlatformVersion::first()
    }

    fn round_trip<T>(value: T) -> T
    where
        T: PlatformVersionEncode + crate::PlatformVersionedDecode,
    {
        let encoded = crate::platform_encode_to_vec(value, cfg(), pv()).unwrap();
        crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap()
    }

    // -----------------------------------------------------------------------
    // platform_encode_to_vec
    // -----------------------------------------------------------------------

    #[test]
    fn encode_to_vec_basic() {
        let encoded = crate::platform_encode_to_vec(42u32, cfg(), pv()).unwrap();
        assert!(!encoded.is_empty());
        let decoded: u32 =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, 42);
    }

    // -----------------------------------------------------------------------
    // Vec<T> encode/decode: u8 optimization vs generic
    // -----------------------------------------------------------------------

    #[test]
    fn vec_u8_round_trip() {
        let value: Vec<u8> = vec![0, 1, 2, 255, 128];
        assert_eq!(round_trip(value.clone()), value);
    }

    #[test]
    fn vec_u32_round_trip() {
        let value: Vec<u32> = vec![100, 200, 300];
        assert_eq!(round_trip(value.clone()), value);
    }

    #[test]
    fn vec_empty_round_trip() {
        let value: Vec<u32> = vec![];
        assert_eq!(round_trip(value.clone()), value);
    }

    // -----------------------------------------------------------------------
    // BTreeMap
    // -----------------------------------------------------------------------

    #[test]
    fn btree_map_round_trip() {
        let mut map = BTreeMap::new();
        map.insert(1u32, "one".to_string());
        map.insert(2, "two".to_string());
        map.insert(3, "three".to_string());
        assert_eq!(round_trip(map.clone()), map);
    }

    #[test]
    fn btree_map_empty_round_trip() {
        let map: BTreeMap<u32, String> = BTreeMap::new();
        assert_eq!(round_trip(map.clone()), map);
    }

    // -----------------------------------------------------------------------
    // BTreeSet
    // -----------------------------------------------------------------------

    #[test]
    fn btree_set_round_trip() {
        let mut set = BTreeSet::new();
        set.insert(10u32);
        set.insert(20);
        set.insert(30);
        assert_eq!(round_trip(set.clone()), set);
    }

    // -----------------------------------------------------------------------
    // BinaryHeap
    // -----------------------------------------------------------------------

    #[test]
    fn binary_heap_round_trip() {
        let mut heap = BinaryHeap::new();
        heap.push(3u32);
        heap.push(1);
        heap.push(2);
        let result = round_trip(heap);
        // BinaryHeap doesn't implement Eq, so compare sorted vecs
        let sorted: Vec<u32> = result.into_sorted_vec();
        assert_eq!(sorted, vec![1, 2, 3]);
    }

    // -----------------------------------------------------------------------
    // VecDeque
    // -----------------------------------------------------------------------

    #[test]
    fn vec_deque_round_trip() {
        let mut deque = VecDeque::new();
        deque.push_back(1u32);
        deque.push_back(2);
        deque.push_front(0);
        assert_eq!(round_trip(deque.clone()), deque);
    }

    // -----------------------------------------------------------------------
    // String
    // -----------------------------------------------------------------------

    #[test]
    fn string_round_trip() {
        let value = "hello world".to_string();
        assert_eq!(round_trip(value.clone()), value);
    }

    #[test]
    fn string_empty_round_trip() {
        let value = String::new();
        assert_eq!(round_trip(value.clone()), value);
    }

    // -----------------------------------------------------------------------
    // Box<T>, Box<str>, Box<[T]>
    // -----------------------------------------------------------------------

    #[test]
    fn box_round_trip() {
        let value = Box::new(42u32);
        assert_eq!(round_trip(value.clone()), value);
    }

    #[test]
    fn box_str_round_trip() {
        let value: Box<str> = "boxed string".into();
        assert_eq!(round_trip(value.clone()), value);
    }

    #[test]
    fn box_slice_round_trip() {
        let value: Box<[u32]> = vec![1, 2, 3].into_boxed_slice();
        assert_eq!(round_trip(value.clone()), value);
    }

    // -----------------------------------------------------------------------
    // Rc<T>, Rc<[T]>
    // -----------------------------------------------------------------------

    #[test]
    fn rc_round_trip() {
        let value = Rc::new(42u32);
        let encoded = crate::platform_encode_to_vec(&value, cfg(), pv()).unwrap();
        let decoded: Rc<u32> =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(*decoded, 42);
    }

    #[test]
    fn rc_slice_round_trip() {
        let value: Rc<[u32]> = vec![1, 2, 3].into();
        let encoded = crate::platform_encode_to_vec(&value, cfg(), pv()).unwrap();
        let decoded: Rc<[u32]> =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(&*decoded, &[1, 2, 3]);
    }

    // -----------------------------------------------------------------------
    // Arc<T>, Arc<str>, Arc<[T]>
    // -----------------------------------------------------------------------

    #[cfg(target_has_atomic = "ptr")]
    #[test]
    fn arc_round_trip() {
        let value = Arc::new(42u32);
        let encoded = crate::platform_encode_to_vec(&value, cfg(), pv()).unwrap();
        let decoded: Arc<u32> =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(*decoded, 42);
    }

    #[cfg(target_has_atomic = "ptr")]
    #[test]
    fn arc_str_round_trip() {
        let value: Arc<str> = Arc::from("arc string");
        let encoded = crate::platform_encode_to_vec(&value, cfg(), pv()).unwrap();
        let decoded: Arc<str> =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(&*decoded, "arc string");
    }

    #[cfg(target_has_atomic = "ptr")]
    #[test]
    fn arc_slice_round_trip() {
        let value: Arc<[u32]> = vec![10, 20, 30].into();
        let encoded = crate::platform_encode_to_vec(&value, cfg(), pv()).unwrap();
        let decoded: Arc<[u32]> =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(&*decoded, &[10, 20, 30]);
    }

    // -----------------------------------------------------------------------
    // Cow
    // -----------------------------------------------------------------------

    #[test]
    fn cow_owned_round_trip() {
        let value: Cow<str> = Cow::Owned("owned".to_string());
        let encoded = crate::platform_encode_to_vec(&value, cfg(), pv()).unwrap();
        let decoded: Cow<str> =
            crate::platform_versioned_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, "owned");
    }

    // -----------------------------------------------------------------------
    // VecWriter (alloc version)
    // -----------------------------------------------------------------------

    #[test]
    fn vec_writer_with_capacity() {
        let writer = VecWriter::with_capacity(100);
        let collected = writer.collect();
        assert!(collected.is_empty());
        assert!(collected.capacity() >= 100);
    }

    // -----------------------------------------------------------------------
    // Borrow-decode paths for collection types
    // -----------------------------------------------------------------------

    #[test]
    fn btree_map_borrow_decode() {
        let mut map = BTreeMap::new();
        map.insert(1u32, 10u32);
        map.insert(2, 20);
        let encoded = crate::platform_encode_to_vec(&map, cfg(), pv()).unwrap();
        let decoded: BTreeMap<u32, u32> =
            crate::platform_versioned_borrow_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, map);
    }

    #[test]
    fn btree_set_borrow_decode() {
        let mut set = BTreeSet::new();
        set.insert(1u32);
        set.insert(2);
        let encoded = crate::platform_encode_to_vec(&set, cfg(), pv()).unwrap();
        let decoded: BTreeSet<u32> =
            crate::platform_versioned_borrow_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, set);
    }

    #[test]
    fn binary_heap_borrow_decode() {
        let mut heap = BinaryHeap::new();
        heap.push(5u32);
        heap.push(3);
        heap.push(7);
        let encoded = crate::platform_encode_to_vec(&heap, cfg(), pv()).unwrap();
        let decoded: BinaryHeap<u32> =
            crate::platform_versioned_borrow_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        let sorted: Vec<u32> = decoded.into_sorted_vec();
        assert_eq!(sorted, vec![3, 5, 7]);
    }

    #[test]
    fn vec_deque_borrow_decode() {
        let mut deque = VecDeque::new();
        deque.push_back(1u32);
        deque.push_back(2);
        let encoded = crate::platform_encode_to_vec(&deque, cfg(), pv()).unwrap();
        let decoded: VecDeque<u32> =
            crate::platform_versioned_borrow_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, deque);
    }

    #[test]
    fn vec_borrow_decode() {
        let value: Vec<u32> = vec![1, 2, 3];
        let encoded = crate::platform_encode_to_vec(&value, cfg(), pv()).unwrap();
        let decoded: Vec<u32> =
            crate::platform_versioned_borrow_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn box_borrow_decode() {
        let value = Box::new(42u32);
        let encoded = crate::platform_encode_to_vec(&value, cfg(), pv()).unwrap();
        let decoded: Box<u32> =
            crate::platform_versioned_borrow_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn box_slice_borrow_decode() {
        let value: Box<[u32]> = vec![10, 20, 30].into_boxed_slice();
        let encoded = crate::platform_encode_to_vec(&value, cfg(), pv()).unwrap();
        let decoded: Box<[u32]> =
            crate::platform_versioned_borrow_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn rc_borrow_decode() {
        let value = Rc::new(42u32);
        let encoded = crate::platform_encode_to_vec(&value, cfg(), pv()).unwrap();
        let decoded: Rc<u32> =
            crate::platform_versioned_borrow_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(*decoded, 42);
    }

    #[test]
    fn rc_slice_borrow_decode() {
        let value: Rc<[u32]> = vec![1, 2, 3].into();
        let encoded = crate::platform_encode_to_vec(&value, cfg(), pv()).unwrap();
        let decoded: Rc<[u32]> =
            crate::platform_versioned_borrow_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(&*decoded, &[1, 2, 3]);
    }

    #[cfg(target_has_atomic = "ptr")]
    #[test]
    fn arc_borrow_decode() {
        let value = Arc::new(42u32);
        let encoded = crate::platform_encode_to_vec(&value, cfg(), pv()).unwrap();
        let decoded: Arc<u32> =
            crate::platform_versioned_borrow_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(*decoded, 42);
    }

    #[cfg(target_has_atomic = "ptr")]
    #[test]
    fn arc_str_borrow_decode() {
        let value: Arc<str> = Arc::from("test arc str");
        let encoded = crate::platform_encode_to_vec(&value, cfg(), pv()).unwrap();
        let decoded: Arc<str> =
            crate::platform_versioned_borrow_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(&*decoded, "test arc str");
    }

    #[cfg(target_has_atomic = "ptr")]
    #[test]
    fn arc_slice_borrow_decode() {
        let value: Arc<[u32]> = vec![10, 20].into();
        let encoded = crate::platform_encode_to_vec(&value, cfg(), pv()).unwrap();
        let decoded: Arc<[u32]> =
            crate::platform_versioned_borrow_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(&*decoded, &[10, 20]);
    }

    #[test]
    fn cow_borrow_decode() {
        use alloc::borrow::Cow;
        let value = "borrowed cow";
        let encoded = crate::platform_encode_to_vec(&value, cfg(), pv()).unwrap();
        let decoded: Cow<str> =
            crate::platform_versioned_borrow_decode_from_slice(&encoded, cfg(), pv()).unwrap();
        assert_eq!(&*decoded, value);
    }
}
