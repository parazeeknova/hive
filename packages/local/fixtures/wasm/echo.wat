;; Hive JSON ABI v1: alloc(length)->pointer; run(pointer,length)->(pointer<<32)|length.
;; This capability dynamically returns the current input JSON, with no host access.
(module
  (memory (export "memory") 1 16)
  (func (export "alloc") (param i32) (result i32) i32.const 0)
  (func (export "run") (param $ptr i32) (param $len i32) (result i64)
    local.get $ptr
    i64.extend_i32_u
    i64.const 32
    i64.shl
    local.get $len
    i64.extend_i32_u
    i64.or))
