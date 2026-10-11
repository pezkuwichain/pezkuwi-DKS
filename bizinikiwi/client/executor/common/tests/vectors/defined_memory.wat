;; Defines (not imports) a memory with a maximum and exports it; no globals, no
;; table, no start, no custom sections. One function is called but not exported.
(module
  (memory (export "memory") 1 32)
  (func $helper (param i32 i32) (result i32)
    (i32.add (local.get 0) (local.get 1)))
  (func (export "run") (result i32) (local i32)
    (loop $l (result i32)
      (local.set 0 (call $helper (local.get 0) (i32.const 1)))
      (br_if $l (i32.lt_u (local.get 0) (i32.const 10)))
      (call $helper (local.get 0) (local.get 0)))))
