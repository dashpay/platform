//! The per-function frame cost of the portable logical stack.
//!
//! The cost is a deterministic function of the function's type and body, computed at
//! preparation and burned into the prepared code, so every node charges the same bytes for the
//! same call and the charge never depends on how Cranelift laid out the native frame.

use crate::bundle::{FuncSignature, ValueType};

/// Fixed bytes charged per frame for the return address, the frame pointer and spill slots
/// the compiler needs regardless of the function's shape. Provisional (register entry A06).
pub const FRAME_BASE_BYTES: u64 = 32;

/// Frame bytes of a function: base plus every parameter, declared local and operand slot at
/// 4 bytes for 32-bit scalars and 8 bytes for 64-bit scalars and references.
///
/// `max_operand_slots` is the deepest operand stack the validator observed in the body, so the
/// cost covers the whole activation and not just what the caller passes.
pub fn frame_bytes(signature: &FuncSignature, locals: &[ValueType], max_operand_slots: u32) -> u64 {
    let params: u64 = signature.params.iter().map(|ty| ty.logical_bytes()).sum();
    let locals: u64 = locals.iter().map(|ty| ty.logical_bytes()).sum();
    // Operand slots have no single type; every slot is charged at the widest scalar so the
    // charge is an upper bound whatever the body pushes.
    let operands = u64::from(max_operand_slots) * 8;
    FRAME_BASE_BYTES + params + locals + operands
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_charge_base_plus_typed_slots() {
        let signature =
            FuncSignature::new(vec![ValueType::I32, ValueType::I64], vec![ValueType::I32]);
        let locals = [ValueType::F32, ValueType::F64, ValueType::FuncRef];
        assert_eq!(
            frame_bytes(&signature, &locals, 3),
            FRAME_BASE_BYTES + (4 + 8) + (4 + 8 + 8) + 3 * 8
        );
    }

    #[test]
    fn should_charge_only_the_base_for_a_leaf_with_no_state() {
        assert_eq!(
            frame_bytes(&FuncSignature::new(vec![], vec![]), &[], 0),
            FRAME_BASE_BYTES
        );
    }
}
