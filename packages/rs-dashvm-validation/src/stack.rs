//! The portable logical-stack contract between prepared code and the runtime.
//!
//! Native stack exhaustion differs between architectures and compilers, so the guest never
//! sees it: preparation generation 0 injects its own accounting and the runtime keeps the
//! native stack far larger than anything the logical limit lets a guest reach. The accounting
//! is two host-owned mutable `i32` globals imported by every module of a bundle
//! (`dash_vm.stack_bytes` and `dash_vm.stack_depth`, see [`crate::abi_names`]) and one
//! trap import. Both globals count **down**: the runtime initialises them to the invocation's
//! remaining budget (the protocol limit, clamped to `i32::MAX`) and the instrumentation
//! subtracts on entry and adds back on exit, so the prepared bytes carry no limit value and are
//! a pure function of the canonical bytes and the preparation generation. Because the globals
//! are host objects shared by every instance of an invocation, a chain of calls across modules
//! and a nested contract call all drain one counter.
//!
//! On every guest-to-guest call the accounting helper charges the callee's frame cost (see
//! [`crate::instrumentation::frame_cost`]) and one activation. When a counter would go
//! negative the helper calls `dash_vm.trap` with one of the codes below and follows the call
//! with `unreachable`, so a host that returned from the trap by mistake still ends the guest.

/// Trap code: the logical-stack byte budget is exhausted.
pub const TRAP_CODE_STACK_BYTES: i32 = 1;
/// Trap code: the activation-depth budget is exhausted.
pub const TRAP_CODE_STACK_DEPTH: i32 = 2;
