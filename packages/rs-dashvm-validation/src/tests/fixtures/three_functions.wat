(module
  (import "dash_host" "host_call" (func $host_call (param i32 i32 i32) (result i64)))
  (table 2 2 funcref)
  (elem (i32.const 0) $leaf $mid)
  (memory (export "memory") 1)
  (global $counter (mut i32) (i32.const 0))
  (func (export "dash_alloc") (param i32) (result i32)
    global.get $counter
    local.get 0
    i32.add
    global.set $counter
    global.get $counter)
  (func $leaf (param i32) (result i32)
    (local i64)
    local.get 0
    i32.const 1
    i32.add)
  (func $mid (param i32) (result i32)
    local.get 0
    call $leaf
    i32.const 0
    call_indirect (param i32) (result i32))
  (func (export "run") (param i32 i32) (result i64)
    local.get 0
    call $mid
    local.get 1
    call $leaf
    i32.add
    i64.extend_i32_u
    i32.const 0 i32.const 0 i32.const 0
    call $host_call
    i64.add)
)
