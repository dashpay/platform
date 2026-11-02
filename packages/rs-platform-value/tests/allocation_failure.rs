//! Allocation failure injection for the bounded value codec.
//!
//! Run with `cargo test -p platform-value --no-default-features --test allocation_failure`
//! for the allocation-only profile, or without the flag for the native one.
//! A global allocator counts requests on the current thread and, once armed,
//! refuses every request from a chosen index onwards. Each test retries with
//! the failure point moved one request later until the operation succeeds, so
//! every reservation the bounded codec makes is refused once. A reservation
//! that is not fallible aborts the test binary instead of returning an error.

use bincode::error::DecodeError;
use platform_serialization::bounded::{BoundedDecodeError, CodecBounds};
use platform_value::guest_bounds::BoundedEncodeError;
use platform_value::Value;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct FailingAllocator;

thread_local! {
    /// The index of the first refused request while armed.
    static FAIL_FROM: Cell<Option<usize>> = const { Cell::new(None) };
    static REQUESTS: Cell<usize> = const { Cell::new(0) };
}

/// Counts one request and says whether to refuse it.
fn refuse() -> bool {
    FAIL_FROM
        .try_with(|fail_from| {
            let Some(first_refused) = fail_from.get() else {
                return false;
            };
            REQUESTS.with(|requests| {
                let index = requests.get();
                requests.set(index + 1);
                index >= first_refused
            })
        })
        .unwrap_or(false)
}

unsafe impl GlobalAlloc for FailingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if refuse() {
            return core::ptr::null_mut();
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if refuse() {
            return core::ptr::null_mut();
        }
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        if refuse() {
            return core::ptr::null_mut();
        }
        unsafe { System.realloc(ptr, layout, size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: FailingAllocator = FailingAllocator;

/// Runs `operation` with every allocation request from `first_refused` on
/// refused, and disarms before returning so assertions allocate normally.
fn with_requests_refused_from<T>(first_refused: usize, operation: impl FnOnce() -> T) -> T {
    REQUESTS.with(|requests| requests.set(0));
    FAIL_FROM.with(|fail_from| fail_from.set(Some(first_refused)));
    let result = operation();
    FAIL_FROM.with(|fail_from| fail_from.set(None));
    result
}

const BOUNDS: CodecBounds = CodecBounds {
    max_bytes: 4096,
    max_depth: 8,
    max_elements: 64,
};

/// Reaches every reservation site: nested array and map frames, container
/// element growth, byte and text leaves and a string list.
fn every_reservation_site() -> Value {
    Value::Map(vec![
        (
            Value::Text("leaves".to_owned()),
            Value::Array(vec![
                Value::Bytes(vec![7; 40]),
                Value::Text("a text leaf".to_owned()),
                Value::EnumString(vec!["first".to_owned(), "second".to_owned()]),
            ]),
        ),
        (
            Value::Text("nested".to_owned()),
            Value::Map(vec![(Value::U8(1), Value::Array(vec![Value::Null]))]),
        ),
    ])
}

#[test]
fn should_report_every_refused_decode_reservation_as_a_limit_error() {
    let value = every_reservation_site();
    let bytes = value
        .encode_bounded(&BOUNDS)
        .expect("encodes within the bounds");

    let mut refused = 0;
    let decoded = loop {
        let result = with_requests_refused_from(refused, || Value::decode_bounded(&bytes, &BOUNDS));
        match result {
            Ok(decoded) => break decoded,
            Err(BoundedDecodeError::Decode(DecodeError::LimitExceeded)) => refused += 1,
            Err(other) => panic!("request {refused} refused: unexpected error {other:?}"),
        }
    };

    assert_eq!(decoded, value);
    // Two frames per map, one per array, plus element storage and five
    // leaves: far more than a handful of reservations were each refused once.
    assert!(refused >= 10, "only {refused} reservations were exercised");
}

#[test]
fn should_report_every_refused_encode_reservation_as_allocation_failed() {
    let value = every_reservation_site();
    let expected = value
        .encode_bounded(&BOUNDS)
        .expect("encodes within the bounds");

    let mut refused = 0;
    let encoded = loop {
        let result = with_requests_refused_from(refused, || value.encode_bounded(&BOUNDS));
        match result {
            Ok(encoded) => break encoded,
            Err(BoundedEncodeError::AllocationFailed) => refused += 1,
            Err(other) => panic!("request {refused} refused: unexpected error {other:?}"),
        }
    };

    assert_eq!(encoded, expected);
    // The size pass reserves the frame stack, then the output is reserved.
    assert!(refused >= 2, "only {refused} reservations were exercised");
}

#[test]
fn should_report_a_refused_output_buffer_for_a_leaf_as_allocation_failed() {
    // A leaf needs no traversal frame, so the only reservation is the output.
    let value = Value::Bytes(vec![1; 64]);

    let result = with_requests_refused_from(0, || value.encode_bounded(&BOUNDS));

    assert!(
        matches!(result, Err(BoundedEncodeError::AllocationFailed)),
        "{result:?}"
    );
}
