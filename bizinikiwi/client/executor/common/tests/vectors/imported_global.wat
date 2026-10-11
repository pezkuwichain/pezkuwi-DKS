;; Imports a global next to a defined one and has no export section at all: the
;; memory import has to become an export in a newly inserted export section.
(module
  (import "env" "g" (global $ig i32))
  (import "env" "memory" (memory 1))
  (global $dg (mut i32) (i32.const 0))
  (func $a (result i32) (global.get $ig) (global.set $dg (i32.const 1)) (call $b) (i32.add))
  (func $b (result i32) (global.get $dg)))
