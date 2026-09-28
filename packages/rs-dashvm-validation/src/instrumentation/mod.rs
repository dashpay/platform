//! Portable logical-stack instrumentation, the required transformation of preparation
//! generation 0.
//!
//! The instrumenter rewrites a submitted module so that every guest-to-guest call is accounted
//! against two host-owned counters (see [`crate::stack`]). It adds three imports at the front
//! of the import section:
//!
//! 1. `dash_vm.stack_bytes`: mutable `i32` global, remaining logical-stack bytes.
//! 2. `dash_vm.stack_depth`: mutable `i32` global, remaining activation frames.
//! 3. `dash_vm.trap(i32)`: function, raised on exhaustion.
//!
//! The function import shifts every defined function index by one and the two global imports
//! shift every global index by two; the [`rewrite`] module carries those shifts through every
//! index reference with the `Reencode` hooks. Then:
//!
//! * every direct `call` to a defined function is wrapped: `enter(callee)` before the call and
//!   `leave(callee)` after it, where `enter` charges the callee's frame bytes and one activation
//!   and traps on exhaustion, and `leave` refunds both;
//! * every defined function that can be reached other than by a direct call (through the table
//!   or an export) gets a thunk, a generated function of the same type that performs
//!   `enter`, calls the original, then `leave`. Table entries and exports are redirected to the
//!   thunk, so the accounting is complete and `return` inside the original body stays correct
//!   (the thunk's `leave` runs after the original returns, whichever path it took);
//! * calls to imported functions (host envelope, bundle bindings) are not wrapped here: host
//!   calls do not consume guest stack, and a bundle binding lands on the target module's thunk,
//!   which accounts for the callee.
//!
//! The `enter` and `leave` helpers are two generated functions of type `(i32) -> ()` that take
//! the byte cost as their parameter; every site passes the callee's cost as an `i32.const`, so
//! the code size grows by a bounded constant per call site and per thunk. The output carries no
//! limit value: both counters count down from what the runtime sets, so the prepared bytes are
//! a pure function of the canonical bytes and this generation.

pub mod frame_cost;
pub mod rewrite;
pub mod thunks;

/// What the instrumenter added, for the provenance check and for the runtime's metering audit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstrumentationReport {
    /// Direct call sites wrapped with `enter` and `leave`.
    pub wrapped_call_sites: u32,
    /// Thunks generated (one per defined function reachable by table or export).
    pub thunks: u32,
    /// Frame bytes of every canonical defined function, in function-section order.
    pub frame_bytes: Vec<u64>,
    /// Frame bytes of the largest frame.
    pub max_frame_bytes: u64,
}

/// Number of imports the instrumenter prepends.
pub const INJECTED_IMPORTS: u32 = 3;
/// Number of function imports the instrumenter prepends (the trap).
pub const INJECTED_FUNCTION_IMPORTS: u32 = 1;
/// Number of global imports the instrumenter prepends.
pub const INJECTED_GLOBAL_IMPORTS: u32 = 2;
/// Generated helper functions (`enter`, `leave`) defined after the canonical functions.
pub const HELPER_FUNCTIONS: u32 = 2;
