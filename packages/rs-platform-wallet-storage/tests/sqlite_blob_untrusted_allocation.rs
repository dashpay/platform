//! Allocation-observation regression for the wallet BLOB codec.
//!
//! `blob::decode` runs bincode-serde in untrusted mode, which withholds the
//! collection length hints that Serde's `Vec` visitor would otherwise turn
//! into an up-front `Vec::with_capacity`. A blob whose length prefix claims
//! megabytes but whose payload ends after one element must therefore fail
//! without reserving the claimed capacity first. A regression to ordinary
//! Serde decoding would still return an error, but only after allocating
//! (up to Serde's own 1 MiB preallocation cap), which the observing allocator
//! below reports as a large request. Each test first runs the ordinary
//! decoder on the same bytes as a control, proving the ceiling can tell the
//! two apart.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use bincode::config::{Configuration, Limit, LittleEndian, Varint};
use platform_wallet_storage::sqlite::schema::blob::{self, BlobDecode, BLOB_SIZE_LIMIT_BYTES};
use platform_wallet_storage::WalletStorageError;

struct ObservedAllocator;

/// A byte sequence decoded through Serde's `Vec<u8>` visitor. Local so the
/// test can admit it to the codec without registering a production shape.
#[derive(Debug, serde::Deserialize)]
#[serde(transparent)]
struct Bytes(#[allow(dead_code)] Vec<u8>); // only ever rejected, never read

impl BlobDecode for Bytes {}

thread_local! {
    static OBSERVING: Cell<bool> = const { Cell::new(false) };
    static LARGEST_REQUEST: Cell<usize> = const { Cell::new(0) };
}

fn observe(size: usize) {
    let _ = OBSERVING.try_with(|enabled| {
        if enabled.get() {
            LARGEST_REQUEST.with(|largest| largest.set(largest.get().max(size)));
        }
    });
}

unsafe impl GlobalAlloc for ObservedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        observe(layout.size());
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        observe(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        observe(size);
        unsafe { System.realloc(ptr, layout, size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: ObservedAllocator = ObservedAllocator;

/// Largest single allocation tolerated while a truncated blob is rejected.
/// Serde's preallocation cap is 1 MiB, so an ordinary decode of the blobs
/// below requests at least that much and trips this ceiling.
const ALLOCATION_CEILING_BYTES: usize = 16 * 1024;

/// The codec's own configuration: little-endian varint with the row budget.
fn codec_config() -> Configuration<LittleEndian, Varint, Limit<BLOB_SIZE_LIMIT_BYTES>> {
    bincode::config::standard().with_limit::<BLOB_SIZE_LIMIT_BYTES>()
}

/// A collection blob whose length prefix claims `claimed_len` entries but
/// which carries only the already-encoded `present` entries.
fn truncated_collection(claimed_len: u64, present: &[u8]) -> Vec<u8> {
    let mut bytes =
        bincode::encode_to_vec(claimed_len, codec_config()).expect("encode the length prefix");
    bytes.extend_from_slice(present);
    bytes
}

/// Runs `f` with allocation observation on and returns its result with the
/// largest allocation request made while it ran.
fn observing<R>(f: impl FnOnce() -> R) -> (R, usize) {
    LARGEST_REQUEST.with(|largest| largest.set(0));
    OBSERVING.with(|enabled| enabled.set(true));
    let result = f();
    OBSERVING.with(|enabled| enabled.set(false));
    (result, LARGEST_REQUEST.with(Cell::get))
}

/// Control: the same bytes through ordinary bincode-serde decoding, which
/// honours the length prefix as a capacity hint. This is the behaviour a
/// regression would reintroduce, and it has to allocate enough for
/// [`ALLOCATION_CEILING_BYTES`] to tell the two decoders apart.
fn assert_ordinary_decoding_reserves_capacity<T: serde::de::DeserializeOwned>(
    what: &str,
    bytes: &[u8],
) {
    let (result, largest) = observing(|| {
        bincode::serde::decode_from_slice::<T, _>(bytes, codec_config()).map(|(value, _)| value)
    });
    assert!(
        result.is_err(),
        "{what}: the control decode must fail on the truncated payload"
    );
    assert!(
        largest >= ALLOCATION_CEILING_BYTES,
        "{what}: ordinary decoding reserved only {largest} bytes, so this test could not \
         detect a regression to it"
    );
}

/// The codec under test: `blob::decode` must fail without reserving the
/// claimed capacity, and must fail as truncated rather than as oversized.
fn assert_codec_rejects_within_budget<T: blob::BlobDecode + std::fmt::Debug>(
    what: &str,
    bytes: &[u8],
) {
    let (result, largest) = observing(|| blob::decode::<T>(bytes));
    let Err(error) = result else {
        panic!("{what}: a truncated collection must not decode");
    };
    assert!(
        !matches!(error, WalletStorageError::BlobTooLarge { .. }),
        "{what}: the payload is physically small, so it must fail as truncated, not as \
         oversized: {error:?}"
    );
    assert!(
        largest < ALLOCATION_CEILING_BYTES,
        "{what}: decoding reserved {largest} bytes before failing; the claimed length must \
         not be turned into capacity"
    );
}

#[test]
fn should_reject_truncated_u32_sequence_without_reserving_declared_capacity() {
    // One million u32s is a 4 MiB reservation, well under the 16 MiB row
    // budget, so only the untrusted decoder stands between the claimed length
    // and the allocator.
    let one_entry = bincode::encode_to_vec(7u32, codec_config()).expect("encode one entry");
    let blob = truncated_collection(1_000_000, &one_entry);

    assert_ordinary_decoding_reserves_capacity::<Vec<u32>>("Vec<u32>", &blob);
    assert_codec_rejects_within_budget::<Vec<u32>>("Vec<u32>", &blob);
}

#[test]
fn should_reject_truncated_byte_sequence_without_reserving_declared_capacity() {
    // Eight million bytes claimed, one present, again under the row budget.
    let blob = truncated_collection(8_000_000, &[0xAB]);

    assert_ordinary_decoding_reserves_capacity::<Vec<u8>>("Vec<u8>", &blob);
    assert_codec_rejects_within_budget::<Bytes>("Vec<u8>", &blob);
}

#[test]
fn should_still_decode_complete_collections() {
    // The observing allocator must not interfere with the happy path, and a
    // complete payload proves the truncated ones fail for the right reason.
    let values: Vec<u32> = (0..64).collect();
    let blob = blob::encode(&values).expect("encode a complete collection");

    let (result, _) = observing(|| blob::decode::<Vec<u32>>(&blob));

    assert_eq!(result.expect("a complete blob decodes"), values);
}
