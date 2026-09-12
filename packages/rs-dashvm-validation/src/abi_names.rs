//! The names and signatures of the host envelope, the guest exports and the instrumentation
//! imports.
//!
//! Every identifier the validator checks or generates lives here so that the ABI allocation
//! (register entry A07) changes exactly one file. The names are provisional placeholders until
//! that allocation is confirmed; the runtime crate reads the same constants, so validator and
//! runtime cannot disagree on a name.

use crate::bundle::{FuncSignature, ValueType};

/// Import module of the host envelope: the functions a guest calls to reach the node.
pub const HOST_MODULE: &str = "dash_host";

/// Import module reserved for the preparation generation's own instrumentation. A submitted
/// module may not import from it; the instrumenter adds exactly the entries listed below.
pub const VM_MODULE: &str = "dash_vm";

/// Prefix of import modules that bind to another module of the same bundle: `dash:<name>`.
pub const INTERNAL_MODULE_PREFIX: &str = "dash:";

/// `dash_host.host_call(op: i32, in_ptr: i32, in_len: i32) -> i64`: performs a host operation
/// on a guest-memory range and returns a response handle, or a negative fault code.
pub const HOST_CALL: &str = "host_call";
/// `dash_host.response_len(handle: i32) -> i32`: the byte length of a response.
pub const RESPONSE_LEN: &str = "response_len";
/// `dash_host.response_read(handle: i32, out_ptr: i32, out_cap: i32) -> i32`: copies a response
/// into guest memory and returns the bytes written.
pub const RESPONSE_READ: &str = "response_read";
/// `dash_host.response_release(handle: i32)`: releases a response handle.
pub const RESPONSE_RELEASE: &str = "response_release";

/// The guest's linear memory export; the host reads arguments and results through it.
pub const MEMORY_EXPORT: &str = "memory";
/// `dash_alloc(len: i32) -> i32`: the guest allocator the host uses to place arguments.
pub const ALLOC_EXPORT: &str = "dash_alloc";

/// `dash_vm.stack_bytes`: mutable `i32` holding the remaining logical-stack bytes.
pub const STACK_BYTES_GLOBAL: &str = "stack_bytes";
/// `dash_vm.stack_depth`: mutable `i32` holding the remaining guest activation frames.
pub const STACK_DEPTH_GLOBAL: &str = "stack_depth";
/// `dash_vm.trap(code: i32)`: raised by instrumentation when a logical limit is exhausted.
pub const TRAP_IMPORT: &str = "trap";

/// The host envelope functions a submitted module may import, with their exact signatures.
pub fn host_envelope() -> [(&'static str, FuncSignature); 4] {
    use ValueType::{I32, I64};
    [
        (
            HOST_CALL,
            FuncSignature::new(vec![I32, I32, I32], vec![I64]),
        ),
        (RESPONSE_LEN, FuncSignature::new(vec![I32], vec![I32])),
        (
            RESPONSE_READ,
            FuncSignature::new(vec![I32, I32, I32], vec![I32]),
        ),
        (RESPONSE_RELEASE, FuncSignature::new(vec![I32], vec![])),
    ]
}

/// The signature of the `dash_alloc` export.
pub fn alloc_signature() -> FuncSignature {
    FuncSignature::new(vec![ValueType::I32], vec![ValueType::I32])
}

/// The signature of every entry export: `(args_ptr: i32, args_len: i32) -> i64`, the result
/// packed as `(ptr << 32) | len`.
pub fn entry_signature() -> FuncSignature {
    FuncSignature::new(vec![ValueType::I32, ValueType::I32], vec![ValueType::I64])
}

/// The signature of `dash_vm.trap` and of the generated accounting helpers: `(i32) -> ()`.
pub fn trap_signature() -> FuncSignature {
    FuncSignature::new(vec![ValueType::I32], vec![])
}
