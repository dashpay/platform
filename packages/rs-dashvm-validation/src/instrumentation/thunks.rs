//! Generated function bodies: the `enter` and `leave` accounting helpers and the per-function
//! thunks that route table and export references through the accounting.

use crate::bundle::FuncSignature;
use crate::stack::{TRAP_CODE_STACK_BYTES, TRAP_CODE_STACK_DEPTH};
use wasm_encoder::{BlockType, Function, Instruction, ValType};

/// Index of the injected `dash_vm.trap` import in the prepared function index space.
pub const TRAP_FUNCTION_INDEX: u32 = 0;
/// Index of the injected `dash_vm.stack_bytes` global.
pub const STACK_BYTES_GLOBAL_INDEX: u32 = 0;
/// Index of the injected `dash_vm.stack_depth` global.
pub const STACK_DEPTH_GLOBAL_INDEX: u32 = 1;

/// Operators (including the closing `end`) in the `enter` helper body.
pub const ENTER_OPERATORS: u64 = 25;
/// Operators (including the closing `end`) in the `leave` helper body.
pub const LEAVE_OPERATORS: u64 = 9;
/// Operators a wrapped call site adds around the original `call`.
pub const WRAP_OPERATORS: u64 = 4;
/// Operators a thunk has beyond one `local.get` per parameter.
pub const THUNK_FIXED_OPERATORS: u64 = 6;

/// `enter(cost: i32)`: charge `cost` bytes and one activation, trapping on exhaustion.
pub fn enter_body() -> Function {
    let mut f = Function::new([]);
    // Bytes: trap when cost > remaining.
    f.instruction(&Instruction::LocalGet(0));
    f.instruction(&Instruction::GlobalGet(STACK_BYTES_GLOBAL_INDEX));
    f.instruction(&Instruction::I32GtS);
    f.instruction(&Instruction::If(BlockType::Empty));
    f.instruction(&Instruction::I32Const(TRAP_CODE_STACK_BYTES));
    f.instruction(&Instruction::Call(TRAP_FUNCTION_INDEX));
    f.instruction(&Instruction::Unreachable);
    f.instruction(&Instruction::End);
    f.instruction(&Instruction::GlobalGet(STACK_BYTES_GLOBAL_INDEX));
    f.instruction(&Instruction::LocalGet(0));
    f.instruction(&Instruction::I32Sub);
    f.instruction(&Instruction::GlobalSet(STACK_BYTES_GLOBAL_INDEX));
    // Depth: trap when no activation remains.
    f.instruction(&Instruction::GlobalGet(STACK_DEPTH_GLOBAL_INDEX));
    f.instruction(&Instruction::I32Const(0));
    f.instruction(&Instruction::I32LeS);
    f.instruction(&Instruction::If(BlockType::Empty));
    f.instruction(&Instruction::I32Const(TRAP_CODE_STACK_DEPTH));
    f.instruction(&Instruction::Call(TRAP_FUNCTION_INDEX));
    f.instruction(&Instruction::Unreachable);
    f.instruction(&Instruction::End);
    f.instruction(&Instruction::GlobalGet(STACK_DEPTH_GLOBAL_INDEX));
    f.instruction(&Instruction::I32Const(1));
    f.instruction(&Instruction::I32Sub);
    f.instruction(&Instruction::GlobalSet(STACK_DEPTH_GLOBAL_INDEX));
    f.instruction(&Instruction::End);
    f
}

/// `leave(cost: i32)`: refund `cost` bytes and one activation.
pub fn leave_body() -> Function {
    let mut f = Function::new([]);
    f.instruction(&Instruction::GlobalGet(STACK_BYTES_GLOBAL_INDEX));
    f.instruction(&Instruction::LocalGet(0));
    f.instruction(&Instruction::I32Add);
    f.instruction(&Instruction::GlobalSet(STACK_BYTES_GLOBAL_INDEX));
    f.instruction(&Instruction::GlobalGet(STACK_DEPTH_GLOBAL_INDEX));
    f.instruction(&Instruction::I32Const(1));
    f.instruction(&Instruction::I32Add);
    f.instruction(&Instruction::GlobalSet(STACK_DEPTH_GLOBAL_INDEX));
    f.instruction(&Instruction::End);
    f
}

/// A thunk of the callee's type: `enter(cost)`, forward every parameter, call the callee,
/// `leave(cost)`, return the callee's results.
pub fn thunk_body(
    signature: &FuncSignature,
    cost: i32,
    callee: u32,
    enter: u32,
    leave: u32,
) -> Function {
    let mut f = Function::new([]);
    f.instruction(&Instruction::I32Const(cost));
    f.instruction(&Instruction::Call(enter));
    for param in 0..signature.params.len() as u32 {
        f.instruction(&Instruction::LocalGet(param));
    }
    f.instruction(&Instruction::Call(callee));
    f.instruction(&Instruction::I32Const(cost));
    f.instruction(&Instruction::Call(leave));
    f.instruction(&Instruction::End);
    f
}

/// The value type of the two accounting globals.
pub const COUNTER_TYPE: ValType = ValType::I32;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::ValueType;

    fn count_operators(function: &Function) -> u64 {
        let body = function.clone().into_raw_body();
        let reader = wasmparser::BinaryReader::new(&body, 0);
        let function_body = wasmparser::FunctionBody::new(reader);
        let mut count = 0;
        let mut ops = function_body
            .get_operators_reader()
            .expect("generated body has locals");
        while !ops.eof() {
            ops.read().expect("generated body decodes");
            count += 1;
        }
        count
    }

    #[test]
    fn should_pin_the_operator_counts_the_provenance_check_relies_on() {
        assert_eq!(count_operators(&enter_body()), ENTER_OPERATORS);
        assert_eq!(count_operators(&leave_body()), LEAVE_OPERATORS);
        let signature =
            FuncSignature::new(vec![ValueType::I32, ValueType::I64], vec![ValueType::I64]);
        assert_eq!(
            count_operators(&thunk_body(&signature, 40, 7, 8, 9)),
            THUNK_FIXED_OPERATORS + 2
        );
    }
}
