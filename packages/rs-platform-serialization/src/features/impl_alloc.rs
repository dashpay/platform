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
    fn platform_versioned_decode<D: Decoder>(
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
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
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
    fn platform_versioned_decode<D: Decoder>(
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
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
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
    fn platform_versioned_decode<D: Decoder>(
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
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
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
    fn platform_versioned_decode<D: Decoder>(
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
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
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
    fn platform_versioned_decode<D: Decoder>(
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
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
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
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(String);

impl PlatformVersionedDecode for Box<str> {
    fn platform_versioned_decode<D: Decoder>(
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
    fn platform_versioned_decode<D: Decoder>(
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
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
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
    fn platform_versioned_decode<D: Decoder>(
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
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
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
    fn platform_versioned_decode<D: Decoder>(
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
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'cow>>(
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
    fn platform_versioned_decode<D: Decoder>(
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
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
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
    fn platform_versioned_decode<D: Decoder>(
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
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
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
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let t = T::platform_versioned_decode(decoder, platform_version)?;
        Ok(Arc::new(t))
    }
}

#[cfg(target_has_atomic = "ptr")]
impl PlatformVersionedDecode for Arc<str> {
    fn platform_versioned_decode<D: Decoder>(
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
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
        decoder: &mut D,
        platform_versioned: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let t = T::platform_versioned_borrow_decode(decoder, platform_versioned)?;
        Ok(Arc::new(t))
    }
}

#[cfg(target_has_atomic = "ptr")]
impl<'de> PlatformVersionedBorrowDecode<'de> for Arc<str> {
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
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
    fn platform_versioned_decode<D: Decoder>(
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
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let vec = Vec::platform_versioned_borrow_decode(decoder, platform_version)?;
        Ok(vec.into())
    }
}
