# Platform Serialization

Dash Platform needs to serialize data structures -- state transitions, consensus errors, documents, identities -- into bytes and back. You might ask: why not just use bincode directly? Or serde? Or protobuf? The answer is that Platform has requirements that no off-the-shelf serialization library handles on its own:

1. **Version awareness.** The serialization of a type may differ between protocol versions. A `DataContract` serialized under protocol version 3 might have different fields than under version 4. The serialization system needs to accept a `PlatformVersion` parameter and dispatch accordingly.

2. **Size limits.** A malicious actor should not be able to send a 100 MB state transition that costs the node minutes to decode. Serialization must enforce configurable byte limits.

3. **Determinism.** Every node must produce identical bytes for identical data. This rules out serialization formats that allow field reordering (like JSON) or that depend on hash map iteration order.

4. **Compatibility with bincode.** Platform already uses bincode extensively for its compact binary format and deterministic encoding. The custom layer should wrap bincode, not replace it.

The `rs-platform-serialization` crate provides this layer. It sits between bincode and the rest of the platform, adding version-aware encoding/decoding while delegating the actual byte-level work to bincode.

## The core traits

The crate defines two pairs of traits in `packages/rs-platform-serialization/src/enc/mod.rs` and `packages/rs-platform-serialization/src/de/mod.rs`.

### Encoding: `PlatformVersionEncode`

```rust
pub trait PlatformVersionEncode {
    /// Encode a given type.
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        platform_version: &PlatformVersion,
    ) -> Result<(), EncodeError>;
}
```

Compare this to bincode's standard `Encode` trait:

```rust
// bincode's Encode (for reference)
pub trait Encode {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError>;
}
```

The only difference is the `platform_version` parameter. For types whose serialization does not change between versions, the implementation simply ignores it and delegates to bincode's `Encode`:

```rust
impl PlatformVersionEncode for String {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &PlatformVersion,  // ignored -- String encoding never changes
    ) -> Result<(), EncodeError> {
        Encode::encode(self, encoder)
    }
}
```

For types that do change between versions, the implementation can dispatch to different encoding logic based on the version.

### Decoding: `PlatformVersionedDecode`

```rust
pub trait PlatformVersionedDecode: Sized {
    fn platform_versioned_decode<D: Decoder<Context = crate::BincodeContext>>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError>;
}
```

Again, this mirrors bincode's `Decode` but with the version parameter. There is also a borrowed variant for zero-copy decoding:

```rust
pub trait PlatformVersionedBorrowDecode<'de>: Sized {
    fn platform_versioned_borrow_decode<
        D: BorrowDecoder<'de, Context = crate::BincodeContext>,
    >(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DecodeError>;
}
```

And a convenience macro to implement `PlatformVersionedBorrowDecode` for any type that implements `PlatformVersionedDecode`:

```rust
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
                <$ty as $crate::PlatformVersionedDecode>::platform_versioned_decode(
                    decoder,
                    platform_version,
                )
            }
        }
    };
}
```

## The convenience functions

The crate provides top-level functions that mirror bincode's API but add version support. The most commonly used one is `platform_encode_to_vec` in `packages/rs-platform-serialization/src/features/impl_alloc.rs`:

```rust
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
```

This function does something clever: it encodes twice. The first pass uses a `SizeWriter` that counts bytes without allocating. The second pass uses a `VecWriter` pre-allocated to exactly the right size. This avoids reallocations during encoding.

For decoding:

```rust
pub fn platform_versioned_decode_from_slice<D: PlatformVersionedDecode, C: Config>(
    src: &[u8],
    config: C,
    platform_version: &PlatformVersion,
) -> Result<D, error::DecodeError> {
    let reader = read::SliceReader::new(src);
    let mut decoder = DecoderImpl::<_, C, crate::BincodeContext>::new(reader, config, ());
    D::platform_versioned_decode(&mut decoder, platform_version)
}
```

There are also `platform_encode_into_slice` for encoding into a pre-allocated buffer, `encode_into_writer` for encoding into arbitrary writers, and `platform_versioned_decode_from_reader` for decoding from arbitrary readers.

## How it wraps bincode

The wrapping is lightweight. Platform serialization does **not** add a version prefix to the bytes by default. It does not change the wire format. When you call `platform_encode_to_vec`, the resulting bytes are identical to what `bincode::encode_to_vec` would produce -- the difference is that the encoding logic can vary based on the `PlatformVersion` parameter.

The bincode configuration used throughout the platform is:

```rust
let config = bincode::config::standard()
    .with_big_endian()  // network byte order for determinism
    .with_no_limit();   // or .with_limit::<{ N }>() for size-limited encoding
```

Big endian is used for deterministic cross-platform encoding. The limit is applied at the `PlatformSerialize` / `PlatformDeserialize` level (see the derive macros chapter) rather than at the raw bincode level.

## Standard type implementations

The crate provides `PlatformVersionEncode` and `PlatformVersionedDecode` implementations for all standard Rust types. These live in `packages/rs-platform-serialization/src/features/impl_alloc.rs` and the other impl files. Here are the key patterns:

### Types that ignore the version

Primitive types, strings, and simple wrappers just delegate to bincode:

```rust
impl PlatformVersionedDecode for String {
    fn platform_versioned_decode<D: Decoder<Context = crate::BincodeContext>>(
        decoder: &mut D,
        _: &PlatformVersion,
    ) -> Result<Self, DecodeError> {
        bincode::Decode::decode(decoder)
    }
}
```

### Collections that propagate the version

Collections like `Vec`, `BTreeMap`, and `BTreeSet` encode their length, then encode each element with the platform version:

```rust
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
        // Optimization: byte slices are written directly
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
```

Notice the `Vec<u8>` optimization -- byte vectors are written in bulk rather than element-by-element, which is significantly faster for large binary data.

### Smart pointers

`Box<T>`, `Rc<T>`, `Arc<T>`, and `Cow<T>` all delegate to their inner type:

```rust
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
```

## Size limits and why they matter

Without size limits, a malicious state transition could contain a `Vec` claiming to have 2^64 elements, causing the node to allocate unbounded memory during decoding. Or a deeply nested document structure could produce a multi-megabyte serialized form that consumes excessive bandwidth.

Size limits are enforced at two levels:

1. **At the `PlatformSerialize` trait level** -- the `#[platform_serialize(limit = N)]` attribute on a type causes the derive macro to use `bincode::config::standard().with_big_endian().with_limit::<{ N }>()`. If encoding exceeds N bytes, it returns a `MaxEncodedBytesReachedError`.

2. **At the bincode decoder level** -- bincode's `claim_container_read` mechanism prevents allocating excessive memory for containers:

```rust
fn platform_versioned_decode<D: Decoder<Context = crate::BincodeContext>>(
    decoder: &mut D,
    platform_versioned: &PlatformVersion,
) -> Result<Self, DecodeError> {
    let len = crate::de::decode_slice_len(decoder)?;
    decoder.claim_container_read::<T>(len)?;  // checks memory budget

    let mut vec = Vec::with_capacity(len);
    for _ in 0..len {
        decoder.unclaim_bytes_read(core::mem::size_of::<T>());
        vec.push(T::platform_versioned_decode(decoder, platform_version)?);
    }
    Ok(vec)
}
```

The `claim_container_read` call tells the decoder "I am about to read `len` elements of type `T`" and the decoder checks whether this fits within the remaining byte budget. If not, it returns an error before any allocation happens.

## Allocation-only guest profile

DashVM guest code runs inside a sandbox with no operating system, no threads
and no access to the native `PlatformVersion` registry. It still has to read
and write the same value bytes as the node. `platform-serialization` and
`platform-value` therefore build in two profiles selected by Cargo features:

| Crate | Feature | What it adds |
|---|---|---|
| `platform-serialization` | `std` | bincode's std readers and writers |
| `platform-serialization` | `platform-version` | `PlatformVersionEncode`, `PlatformVersionedDecode`, the free functions and every standard-type impl above |
| `platform-value` | `std` | the thread-local decode depth scope, patch diffing (`treediff`), the `indexmap` and `HashSet` helpers |
| `platform-value` | `random` | `Identifier::random`, `Identifier::random_with_rng`, `Bytes32::random_with_rng` (implies `std`) |
| `platform-value` | `platform-version` | the `PlatformVersion` aware impls for `Identifier` |
| `platform-value` | `json`, `cbor` | the JSON and CBOR converters (both imply `std`) |

Default features enable everything, so no native consumer changes. With
`--no-default-features` both crates are `#![no_std]` plus `alloc`, and CI
builds them for `wasm32v1-none`, a target with no std library at all, so a
std leak in the alloc profile is a compile error rather than a surprise when a
guest is first built.

The alloc profile keeps the `Value` type, its derived `Encode` and the
iterative `Decode` impl, the serde `to_value` and `from_value` conversions and
the `platform_value!` macro. What it does not have is ambient state: there is
no thread-local depth limit to read. The plain `Decode` impl therefore always
applies `DEFAULT_MAX_VALUE_DECODE_DEPTH`, and anything that needs other limits
uses the bounded pair instead:

```rust
use platform_serialization::bounded::CodecBounds;
use platform_value::Value;

let bounds = CodecBounds { max_bytes: 64 * 1024, max_depth: 256, max_elements: 65_536 };
let bytes = value.encode_bounded(&bounds)?;
let decoded = Value::decode_bounded(&bytes, &bounds)?;
assert_eq!(decoded, value);
```

Bounded decoding is not canonical validation. bincode accepts overlong
variable-length integers, so two different byte strings can decode to the same
value; only encoder-produced bytes are guaranteed to round-trip byte for byte.
The ABI layer establishes canonical bytes by re-encoding the decoded value and
comparing.

`CodecBounds` lives in `packages/rs-platform-serialization/src/bounded.rs`
together with `CodecBudget` (the running counters), `BoundsError` (fixed-width
fields only, so it can become wire-visible later without depending on the word
size), `BoundedSliceReader` (a slice reader that reports its unread remainder),
`canonical_config()` (the same big-endian varint configuration as the native
path) and `bounded_decode_from_slice`, which rejects trailing bytes.

The bounded decoder runs the same state machine as the native `Decode` impl,
so accepted inputs produce identical values, but it treats every declared
length as untrusted. Container counts, byte strings, text and string lists are
checked against the unread input and the budget before anything is allocated;
byte leaves then allocate exactly the declared length, and containers start
empty and grow by push. Depth and element counts are charged at the container
header. Heap usage is bounded by the three limits together: byte leaves by
`max_bytes`, container storage by `max_elements` (plus vector growth slack),
and the traversal stack by `max_depth`. It is not bounded by `max_bytes`
alone, so callers size the depth and element limits deliberately rather than
relying on a small byte budget. The native path is untouched: it keeps the
thread-local limit, bincode's own leaf decoders and pre-sized containers,
because shipped protocol versions decode through it.

## The `BincodeContext` type alias

You will see `crate::BincodeContext` throughout the code:

```rust
pub type BincodeContext = ();
```

This is the bincode context type. Bincode supports contextual decoding where a context value is threaded through the decoder -- useful for things like string interning or reference resolution. Platform does not use this feature, so the context is the unit type `()`. The alias exists to make it easy to change later if needed.

## Rules

**Do:**
- Use `PlatformVersionEncode` / `PlatformVersionedDecode` for types whose serialization may change between protocol versions
- Use standard bincode `Encode` / `Decode` for types whose format is permanently fixed
- Use big-endian configuration everywhere for deterministic encoding
- Apply size limits on types that are received from untrusted sources (state transitions, consensus errors)
- Use the `platform_encode_to_vec` convenience function rather than constructing encoders manually

**Do not:**
- Use little-endian or native-endian configuration -- this breaks determinism across architectures
- Forget to propagate the `platform_version` parameter through collection types
- Skip `claim_container_read` in custom collection decoders -- this is a denial-of-service defense
- Add a version prefix to the bytes manually -- the platform version is a parameter, not part of the wire format
- Use serde for consensus-critical serialization -- serde's flexibility (field names, human-readable formats) is a liability when you need deterministic bytes
