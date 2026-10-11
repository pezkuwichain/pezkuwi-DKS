;; Each module below uses one post-MVP feature. The executor has only ever
;; accepted MVP modules (plus mutable globals), so each must be rejected --
;; except the last: a second memory is well-formed MVP syntax, which the
;; parser accepts and the engine's validation rejects.
;; The vector generator splits this file on the `;; --` separator lines.
;; -- signext
(module (func (result i32) (i32.extend8_s (i32.const 1))))
;; -- multivalue
(module (func (result i32 i32) (i32.const 1) (i32.const 2)))
;; -- bulk
(module (memory 1) (func (memory.copy (i32.const 0) (i32.const 1) (i32.const 1))))
;; -- satconv
(module (func (result i32) (i32.trunc_sat_f32_s (f32.const 1))))
;; -- reftypes
(module (table 1 externref))
;; -- multimemory
(module (memory 1) (memory 1))
