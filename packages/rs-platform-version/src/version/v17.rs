use crate::version::fee::v3::FEE_VERSION3;
use crate::version::protocol_version::PlatformVersion;
use crate::version::system_limits::v5::SYSTEM_LIMITS_V5;
use crate::version::v16::PLATFORM_V16;
use crate::version::ProtocolVersion;

pub const PROTOCOL_VERSION_17: ProtocolVersion = 17;

/// The 5.0 protocol version, the one that introduces smart contracts. Provisional number from
/// the DashVM allocation register (15 for 4.3, 16 for 4.4, 17 for 5.0).
///
/// Two changes over v16 so far, both tables and both still unread by any dispatching code path:
///
/// * `SYSTEM_LIMITS_V5` sets `smart_contract_computation`: at most 25 million computation
///   units per outer contract invocation and 250 million per block, counted by one
///   contract-only counter separate from every native budget.
/// * `FEE_VERSION3` prices those units at 1 credit each (`dashvm`); its number stays 1 because
///   no storage rate changes.
///
/// Both numbers are provisional register values, measured and revised before any network is
/// asked to propose this version. Enforcement (the per-block ledger on the block execution
/// context, the not-executed classification for a full block, CheckTx affordability) arrives
/// with the tasks that wire the runtime in; until then a node at v17 behaves exactly like one
/// at v16.
pub const PLATFORM_V17: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_17,
    fee_version: FEE_VERSION3, // changed: prices smart-contract computation (dashvm group)
    system_limits: SYSTEM_LIMITS_V5, // changed: per-invocation and per-block smart-contract computation limits
    ..PLATFORM_V16
};
