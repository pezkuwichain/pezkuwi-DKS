;; Custom sections at the head, between known sections and at the tail, an
;; imported memory, no exports and one internal function.
(module
  (@custom "a" (before first) "1")
  (@custom "b" (after type) "22")
  (@custom "c" (after last) "333")
  (import "env" "memory" (memory 1 2))
  (func $f (param i32) (result i32) (local.get 0))
  (func $g (result i32) (call $f (i32.const 1))))
