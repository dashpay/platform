use crate::version::system_limits::v4::SYSTEM_LIMITS_V4;
use crate::version::system_limits::SystemLimits;

/// System limits for protocol version 17 (5.0) and above.
///
/// Identical to [`SYSTEM_LIMITS_V4`] except that the contract-code capable generations of the
/// contract create and update transitions are bounded: a raw envelope of up to 32 MiB decoded
/// under a 64 MiB bincode budget, carrying at most 16 modules and 16 MiB of canonical module
/// bytes in total. Every other family keeps the 20 KiB `max_state_transition_size`.
///
/// The numbers are the provisional values of the DashVM shared allocation register (the
/// "Signed contract create/update transition" and "Canonical WASM" rows; the module count is an
/// engineering guess the register does not fix). They are measured on the testnet rehearsal and
/// revised before any network is asked to propose this version.
pub const SYSTEM_LIMITS_V5: SystemLimits = SystemLimits {
    // provisional: allocation register "Signed contract create/update transition", 32 MiB
    max_contract_code_state_transition_size: Some(33_554_432),
    // provisional: the wire cap plus 32 MiB of headroom for the container claims bincode
    // charges while decoding the contract schema (`Value` maps and arrays claim their capacity)
    max_contract_code_state_transition_decode_budget: Some(67_108_864),
    // provisional: allocation register "Canonical WASM 16 MiB per version", read as the total
    // canonical bytes of one bundle
    max_contract_code_bundle_bytes: Some(16_777_216),
    // provisional: engineering guess, the register gives no module count
    max_contract_code_modules_per_bundle: Some(16),
    ..SYSTEM_LIMITS_V4
};

// Whatever the measured revision of the numbers above is: the contract-code cap must not fall
// below the ordinary cap (the v1 decoder would otherwise bound a contract envelope tighter than
// a v0 one), the bundle bytes must fit inside the envelope with at least 1 MiB left for the
// contract itself, the signature and the framing, and the decode budget must exceed the wire
// cap or a maximal envelope could never be decoded.
const _: () = assert!(
    match (
        SYSTEM_LIMITS_V5.max_contract_code_state_transition_size,
        SYSTEM_LIMITS_V5.max_contract_code_state_transition_decode_budget,
        SYSTEM_LIMITS_V5.max_contract_code_bundle_bytes,
        SYSTEM_LIMITS_V5.max_contract_code_modules_per_bundle,
    ) {
        (Some(transition_cap), Some(decode_budget), Some(bundle_bytes), Some(modules)) => {
            transition_cap >= SYSTEM_LIMITS_V5.max_state_transition_size
                && bundle_bytes + 1_048_576 <= transition_cap
                && decode_budget > transition_cap
                && modules > 0
        }
        _ => false,
    },
    "SYSTEM_LIMITS_V5 must carry consistent contract code limits"
);
