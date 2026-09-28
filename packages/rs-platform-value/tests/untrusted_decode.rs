use bincode::config;
use platform_value::Value;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct ObservedAllocator;

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

#[test]
fn should_reject_missing_value_payload_without_reserving_declared_lengths() {
    let config = config::standard().with_big_endian().with_no_limit();
    // Bytes, enum bytes/strings, text, arrays and maps all allocate. Include
    // arrays/maps with one complete entry before the missing remainder.
    for variant in [10u32, 14, 15, 18, 21, 22] {
        let mut bytes = bincode::encode_to_vec(variant, config).unwrap();
        bytes.extend(bincode::encode_to_vec(262_144u64, config).unwrap());
        if variant == 21 {
            bytes.extend(bincode::encode_to_vec(Value::Null, config).unwrap());
        } else if variant == 22 {
            bytes.extend(bincode::encode_to_vec((Value::Null, Value::Null), config).unwrap());
        }

        LARGEST_REQUEST.with(|largest| largest.set(0));
        OBSERVING.with(|enabled| enabled.set(true));
        let owned = bincode::decode_from_slice_untrusted::<Value, _>(&bytes, config);
        let borrowed = bincode::borrow_decode_from_slice_untrusted::<Value, _>(&bytes, config);
        OBSERVING.with(|enabled| enabled.set(false));
        let largest = LARGEST_REQUEST.with(Cell::get);

        assert!(owned.is_err() && borrowed.is_err(), "variant {variant}");
        assert!(
            largest < 16_384,
            "variant {variant} reserved {largest} bytes"
        );
    }
}

#[test]
fn should_preserve_value_wire_format_and_consumed_bytes() {
    let value = Value::Map(vec![
        (Value::Text("key".into()), Value::Bytes(vec![0, 1, 255])),
        (
            Value::U64(u64::MAX),
            Value::Array(vec![Value::Null, Value::EnumString(vec!["choice".into()])]),
        ),
    ]);
    macro_rules! check {
        ($config:expr) => {{
            let mut bytes = bincode::encode_to_vec(&value, $config).unwrap();
            let consumed = bytes.len();
            bytes.push(42);
            let ordinary = bincode::decode_from_slice::<Value, _>(&bytes, $config).unwrap();
            let guarded =
                bincode::decode_from_slice_untrusted::<Value, _>(&bytes, $config).unwrap();
            let borrowed =
                bincode::borrow_decode_from_slice_untrusted::<Value, _>(&bytes, $config).unwrap();
            assert_eq!(ordinary, (value.clone(), consumed));
            assert_eq!(guarded, ordinary);
            assert_eq!(borrowed, ordinary);
        }};
    }
    check!(config::standard());
    check!(config::standard().with_big_endian());
    check!(config::standard().with_fixed_int_encoding());
    check!(config::standard()
        .with_big_endian()
        .with_fixed_int_encoding());
}

#[test]
fn should_preserve_value_depth_and_budget_checks() {
    let config = config::standard().with_big_endian();
    let mut bytes = [21, 1].repeat(1_000);
    bytes.push(20); // Null at the bottom of the nested arrays.
    assert!(bincode::decode_from_slice_untrusted::<Value, _>(&bytes, config).is_err());
    let bytes = bincode::encode_to_vec(Value::Bytes(vec![1; 32]), config).unwrap();
    assert!(
        bincode::decode_from_slice_untrusted::<Value, _>(&bytes, config.with_limit::<8>()).is_err()
    );
}
