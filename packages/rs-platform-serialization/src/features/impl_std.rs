use crate::{
    impl_platform_versioned_borrow_decode, PlatformVersionEncode, PlatformVersionedBorrowDecode,
    PlatformVersionedDecode,
};
use bincode::{
    config::Config,
    de::{read::Reader, BorrowDecoder, Decode, Decoder, DecoderImpl},
    enc::{write::Writer, Encode, Encoder, EncoderImpl},
    error::{DecodeError, EncodeError},
};

use platform_version::version::PlatformVersion;
use std::{
    collections::{HashMap, HashSet},
    ffi::{CStr, CString},
    hash::Hash,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    path::{Path, PathBuf},
    sync::{Mutex, RwLock},
    time::SystemTime,
};

/// Decode type `D` from the given reader with the given `Config`. The reader can be any type that implements `std::io::Read`, e.g. `std::fs::File`.
///
/// See the [config] module for more information about config options.
///
/// [config]: config/index.html
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[allow(dead_code)]
#[deprecated(note = "This function is marked as unused.")]
#[allow(deprecated)]
pub fn platform_versioned_decode_from_std_read<D: Decode, C: Config, R: std::io::Read>(
    src: &mut R,
    config: C,
) -> Result<D, DecodeError> {
    let reader = IoReader::new(src);
    let mut decoder = DecoderImpl::<_, C>::new(reader, config);
    D::decode(&mut decoder)
}

pub(crate) struct IoReader<R> {
    reader: R,
}

impl<R> IoReader<R> {
    #[allow(dead_code)]
    #[deprecated(note = "This function is marked as unused.")]
    #[allow(deprecated)]
    pub fn new(reader: R) -> Self {
        Self { reader }
    }
}

impl<R> Reader for IoReader<R>
where
    R: std::io::Read,
{
    #[inline(always)]
    fn read(&mut self, bytes: &mut [u8]) -> Result<(), DecodeError> {
        self.reader
            .read_exact(bytes)
            .map_err(|inner| DecodeError::Io {
                inner,
                additional: bytes.len(),
            })
    }
}

/// Encode the given value into any type that implements `std::io::Write`, e.g. `std::fs::File`, with the given `Config`.
/// See the [config] module for more information.
/// Returns the amount of bytes written.
///
/// [config]: config/index.html
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[allow(dead_code)]
#[deprecated(note = "This function is marked as unused.")]
#[allow(deprecated)]
pub fn encode_into_std_write<E: Encode, C: Config, W: std::io::Write>(
    val: E,
    dst: &mut W,
    config: C,
) -> Result<usize, EncodeError> {
    let writer = IoWriter::new(dst);
    let mut encoder = EncoderImpl::<_, C>::new(writer, config);
    val.encode(&mut encoder)?;
    Ok(encoder.into_writer().bytes_written())
}

pub(crate) struct IoWriter<'a, W: std::io::Write> {
    writer: &'a mut W,
    bytes_written: usize,
}

impl<'a, W: std::io::Write> IoWriter<'a, W> {
    #[allow(dead_code)]
    #[deprecated(note = "This function is marked as unused.")]
    #[allow(deprecated)]
    pub fn new(writer: &'a mut W) -> Self {
        Self {
            writer,
            bytes_written: 0,
        }
    }
    #[allow(dead_code)]
    #[deprecated(note = "This function is marked as unused.")]
    #[allow(deprecated)]
    pub fn bytes_written(&self) -> usize {
        self.bytes_written
    }
}

impl<W: std::io::Write> Writer for IoWriter<'_, W> {
    #[inline(always)]
    fn write(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
        self.writer
            .write_all(bytes)
            .map_err(|inner| EncodeError::Io {
                inner,
                index: self.bytes_written,
            })?;
        self.bytes_written += bytes.len();
        Ok(())
    }
}

impl PlatformVersionEncode for &CStr {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        self.to_bytes().encode(encoder)
    }
}

impl PlatformVersionEncode for CString {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        self.as_bytes().encode(encoder)
    }
}

impl PlatformVersionedDecode for CString {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(CString);

impl<T> PlatformVersionEncode for Mutex<T>
where
    T: PlatformVersionEncode,
{
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        platform_version: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        let t = self.lock().map_err(|_| EncodeError::LockFailed {
            type_name: core::any::type_name::<Mutex<T>>(),
        })?;
        t.platform_encode(encoder, platform_version)
    }
}

impl<T> PlatformVersionedDecode for Mutex<T>
where
    T: PlatformVersionedDecode,
{
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let t = T::platform_versioned_decode(decoder, platform_version)?;
        Ok(Mutex::new(t))
    }
}
impl<'de, T> PlatformVersionedBorrowDecode<'de> for Mutex<T>
where
    T: PlatformVersionedBorrowDecode<'de>,
{
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let t = T::platform_versioned_borrow_decode(decoder, platform_version)?;
        Ok(Mutex::new(t))
    }
}

impl<T> PlatformVersionEncode for RwLock<T>
where
    T: PlatformVersionEncode,
{
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        platform_version: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        let t = self.read().map_err(|_| EncodeError::LockFailed {
            type_name: core::any::type_name::<RwLock<T>>(),
        })?;
        t.platform_encode(encoder, platform_version)
    }
}

impl<T> PlatformVersionedDecode for RwLock<T>
where
    T: PlatformVersionedDecode,
{
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let t = T::platform_versioned_decode(decoder, platform_version)?;
        Ok(RwLock::new(t))
    }
}
impl<'de, T> PlatformVersionedBorrowDecode<'de> for RwLock<T>
where
    T: PlatformVersionedBorrowDecode<'de>,
{
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let t = T::platform_versioned_borrow_decode(decoder, platform_version)?;
        Ok(RwLock::new(t))
    }
}

impl PlatformVersionEncode for SystemTime {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        bincode::Encode::encode(self, encoder)
    }
}

impl PlatformVersionedDecode for SystemTime {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(SystemTime);

impl PlatformVersionEncode for &'_ Path {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        bincode::Encode::encode(self, encoder)
    }
}

impl<'de> PlatformVersionedBorrowDecode<'de> for &'de Path {
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::BorrowDecode::borrow_decode(decoder)
    }
}

impl PlatformVersionEncode for PathBuf {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        bincode::Encode::encode(self, encoder)
    }
}

impl PlatformVersionedDecode for PathBuf {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let _string = std::string::String::decode(decoder)?;
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(PathBuf);

impl PlatformVersionEncode for IpAddr {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _platform_version: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        bincode::Encode::encode(self, encoder)
    }
}

impl PlatformVersionedDecode for IpAddr {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(IpAddr);

impl PlatformVersionEncode for Ipv4Addr {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _platform_version: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        bincode::Encode::encode(self, encoder)
    }
}

impl PlatformVersionedDecode for Ipv4Addr {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(Ipv4Addr);

impl PlatformVersionEncode for Ipv6Addr {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionedDecode for Ipv6Addr {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(Ipv6Addr);

impl PlatformVersionEncode for SocketAddr {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionedDecode for SocketAddr {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(SocketAddr);

impl PlatformVersionEncode for SocketAddrV4 {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionedDecode for SocketAddrV4 {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(SocketAddrV4);

impl PlatformVersionEncode for SocketAddrV6 {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}

impl PlatformVersionedDecode for SocketAddrV6 {
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
impl_platform_versioned_borrow_decode!(SocketAddrV6);

impl<K, V, S> PlatformVersionEncode for HashMap<K, V, S>
where
    K: PlatformVersionEncode,
    V: PlatformVersionEncode,
{
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        platform_version: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        crate::enc::encode_slice_len(encoder, self.len())?;
        for (k, v) in self.iter() {
            PlatformVersionEncode::platform_encode(k, encoder, platform_version)?;
            PlatformVersionEncode::platform_encode(v, encoder, platform_version)?;
        }
        Ok(())
    }
}

impl<K, V, S> PlatformVersionedDecode for HashMap<K, V, S>
where
    K: PlatformVersionedDecode + Eq + std::hash::Hash,
    V: PlatformVersionedDecode,
    S: std::hash::BuildHasher + Default,
{
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let len = crate::de::decode_slice_len(decoder)?;
        decoder.claim_container_read::<(K, V)>(len)?;

        let hash_builder: S = Default::default();
        let mut map = HashMap::with_capacity_and_hasher(len, hash_builder);
        for _ in 0..len {
            // See the documentation on `unclaim_bytes_read` as to why we're doing this here
            decoder.unclaim_bytes_read(core::mem::size_of::<(K, V)>());

            let k = K::platform_versioned_decode(decoder, platform_version)?;
            let v = V::platform_versioned_decode(decoder, platform_version)?;
            map.insert(k, v);
        }
        Ok(map)
    }
}
impl<'de, K, V, S> PlatformVersionedBorrowDecode<'de> for HashMap<K, V, S>
where
    K: PlatformVersionedBorrowDecode<'de> + Eq + std::hash::Hash,
    V: PlatformVersionedBorrowDecode<'de>,
    S: std::hash::BuildHasher + Default,
{
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let len = crate::de::decode_slice_len(decoder)?;
        decoder.claim_container_read::<(K, V)>(len)?;

        let hash_builder: S = Default::default();
        let mut map = HashMap::with_capacity_and_hasher(len, hash_builder);
        for _ in 0..len {
            // See the documentation on `unclaim_bytes_read` as to why we're doing this here
            decoder.unclaim_bytes_read(core::mem::size_of::<(K, V)>());

            let k = K::platform_versioned_borrow_decode(decoder, platform_version)?;
            let v = V::platform_versioned_borrow_decode(decoder, platform_version)?;
            map.insert(k, v);
        }
        Ok(map)
    }
}

impl<T, S> PlatformVersionedDecode for HashSet<T, S>
where
    T: PlatformVersionedDecode + Eq + Hash,
    S: std::hash::BuildHasher + Default,
{
    fn platform_versioned_decode<D: Decoder>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let len = crate::de::decode_slice_len(decoder)?;
        decoder.claim_container_read::<T>(len)?;

        let hash_builder: S = Default::default();
        let mut map: HashSet<T, S> = HashSet::with_capacity_and_hasher(len, hash_builder);
        for _ in 0..len {
            // See the documentation on `unclaim_bytes_read` as to why we're doing this here
            decoder.unclaim_bytes_read(core::mem::size_of::<T>());

            let key = T::platform_versioned_decode(decoder, platform_version)?;
            map.insert(key);
        }
        Ok(map)
    }
}

impl<'de, T, S> PlatformVersionedBorrowDecode<'de> for HashSet<T, S>
where
    T: PlatformVersionedBorrowDecode<'de> + Eq + Hash,
    S: std::hash::BuildHasher + Default,
{
    fn platform_versioned_borrow_decode<D: BorrowDecoder<'de>>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        let len = crate::de::decode_slice_len(decoder)?;
        decoder.claim_container_read::<T>(len)?;

        let mut map = HashSet::with_capacity_and_hasher(len, S::default());
        for _ in 0..len {
            // See the documentation on `unclaim_bytes_read` as to why we're doing this here
            decoder.unclaim_bytes_read(core::mem::size_of::<T>());

            let key = T::platform_versioned_borrow_decode(decoder, platform_version)?;
            map.insert(key);
        }
        Ok(map)
    }
}

impl<T, S> PlatformVersionEncode for HashSet<T, S>
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
