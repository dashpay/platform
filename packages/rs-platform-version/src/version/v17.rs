use crate::version::dashvm_versions::v1::DASHVM_VERSION_V1;
use crate::version::protocol_version::PlatformVersion;
use crate::version::v16::PLATFORM_V16;
use crate::version::ProtocolVersion;

pub const PROTOCOL_VERSION_17: ProtocolVersion = 17;

/// The 5.0 protocol version, the one that introduces smart contracts. Provisional number from
/// the DashVM allocation register (15 for 4.3, 16 for 4.4, 17 for 5.0).
///
/// One change over v16 so far, a table that no dispatching code path reads yet:
///
/// * `dashvm` is `Some(DASHVM_VERSION_V1)`: engine profile 0 (the pinned Wasmtime release with
///   Cranelift), preparation generation 0 (the admitted feature set, structural bounds and
///   logical-stack instrumentation of `dashvm-validation`), metering generation 0 (audited
///   Wasmtime fuel plus weighted host charges), and the register's provisional limits and
///   weights.
///
/// Every number is provisional, measured and revised before any network is asked to propose
/// this version. Preparation of contract code becomes possible at 17 because
/// `PreparationProfile` can only be built from a version whose table is `Some`; nothing
/// executes until the runtime and the state transitions that call it land, so a node at 17
/// behaves exactly like one at 16 until then.
pub const PLATFORM_V17: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_17,
    dashvm: Some(DASHVM_VERSION_V1), // changed: the DashVM engine, preparation and metering table
    ..PLATFORM_V16
};
