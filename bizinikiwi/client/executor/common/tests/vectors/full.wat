;; Imports functions and memory; defines a table, globals, a start function,
;; element and data segments, custom sections in three places and a name section.
;; The bodies cover every MVP instruction class the stack-height pass accounts for.
(module
  (type $t0 (func (param i32) (result i32)))
  (type $t1 (func))
  ;; Structurally equal to $t0: a thunk must reuse the *first* equal type.
  (type $t2 (func (param i32) (result i32)))
  (type $t3 (func (param i64 i32) (result i64)))
  (import "env" "ext_a" (func $ext_a (type $t0)))
  (import "env" "ext_b" (func $ext_b (type $t1)))
  (import "env" "memory" (memory 2 16))
  (table 4 4 funcref)
  (global $g0 (mut i32) (i32.const 1024))
  (global $g1 i64 (i64.const -7))
  (export "g1" (global $g1))
  (export "main" (func $main))
  (export "dup" (func $dup))
  (export "main_again" (func $main))
  (export "leaf" (func $leaf))
  (export "ext_a" (func $ext_a))
  (export "tbl" (table 0))
  (start $init)
  (elem (i32.const 0) $leaf $dup $main $leaf)
  (data (i32.const 8) "hello")
  (data (i32.const 64) "\00\01\02")
  (@custom "runtime_version" (after import) "\01\02\03")
  (@custom "runtime_apis" (before code) "\04\05")
  (@custom "trailer" (after data) "zz")

  (func $init (type $t1)
    (drop (call $ext_a (i32.const 1)))
    (call $ext_b))

  (func $dup (type $t2) (param i32) (result i32)
    local.get 0
    call $main)

  (func $main (type $t0) (param i32) (result i32) (local i64 f32 f32) (local f64)
    (block $outer (result i32)
      (block $b0
        (block $b1
          (block $b2
            local.get 0
            br_table $b0 $b1 $b2 $b0)
          (br $outer (i32.const 2)))
        (loop $l
          local.get 0
          i32.const 1
          i32.sub
          local.tee 0
          br_if $l
          global.get $g0
          i32.const 3
          i32.add
          global.set $g0))
      (if (result i32) (local.get 0)
        (then
          (i32.load offset=4 align=2 (i32.const 16)))
        (else
          (i32.load8_s (i32.const 1))
          (i32.load8_u (i32.const 1))
          i32.add
          (i32.load16_s (i32.const 2))
          i32.add
          (i32.load16_u (i32.const 2))
          i32.add))
      drop
      (i64.store (i32.const 0) (i64.load (i32.const 8)))
      (i64.store8 (i32.const 0) (i64.load8_s (i32.const 8)))
      (i64.store16 (i32.const 0) (i64.load8_u (i32.const 8)))
      (i64.store32 (i32.const 0) (i64.load16_s (i32.const 8)))
      (i64.store (i32.const 0) (i64.load16_u (i32.const 8)))
      (i64.store (i32.const 0) (i64.load32_s (i32.const 8)))
      (i64.store (i32.const 0) (i64.load32_u (i32.const 8)))
      (f32.store (i32.const 0) (f32.load (i32.const 8)))
      (f64.store (i32.const 0) (f64.load (i32.const 8)))
      (i32.store8 (i32.const 0) (i32.const 7))
      (i32.store16 (i32.const 0) (i32.const 7))
      (i32.store (i32.const 0) (i32.const 7))
      (drop (memory.grow (memory.size)))
      (local.set 1 (i64.const 0x7fffffffffffffff))
      (local.set 2 (f32.const 1.5))
      (local.set 3 (f32.neg (f32.sqrt (local.get 2))))
      (local.set 4 (f64.const -2.25))
      (drop (i32.eqz (i32.const 0)))
      (drop (i64.eqz (local.get 1)))
      (drop (i32.lt_s (i32.const 1) (i32.const 2)))
      (drop (i64.ge_u (local.get 1) (i64.const 3)))
      (drop (f32.lt (local.get 2) (local.get 3)))
      (drop (f64.ge (local.get 4) (f64.const 0)))
      (drop (i32.popcnt (i32.clz (i32.ctz (i32.const 9)))))
      (drop (i64.rotl (i64.mul (local.get 1) (i64.const 3)) (i64.const 5)))
      (drop (f64.copysign (f64.floor (local.get 4)) (f64.const 1)))
      (drop (i32.wrap_i64 (i64.extend_i32_u (i32.trunc_f32_s (local.get 2)))))
      (drop (f64.promote_f32 (f32.demote_f64 (local.get 4))))
      (drop (i64.reinterpret_f64 (f64.convert_i64_s (local.get 1))))
      (drop (f32.reinterpret_i32 (i32.reinterpret_f32 (local.get 2))))
      (drop (select (i32.const 1) (i32.const 2) (local.get 0)))
      (drop (call_indirect (type $t0) (i32.const 5) (i32.const 2)))
      (drop (call $leaf (i64.const 1) (i32.const 2)))
      (drop (call $ext_a (i32.const 3)))
      nop
      (if (local.get 0) (then (return (i32.const 9))))
      i32.const 1))

  (func $leaf (type $t3) (param i64 i32) (result i64)
    local.get 0
    (if (i32.eqz (local.get 1)) (then unreachable))
    (block (br 0) i32.const 1 i32.const 2 i32.const 3 drop drop drop)))
