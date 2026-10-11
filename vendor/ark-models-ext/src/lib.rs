#![cfg_attr(not(feature = "std"), no_std)]

pub use ark_ec::{
    scalar_mul, scalar_mul::*, twisted_edwards, twisted_edwards::*, AffineRepr, CurveGroup,
    PrimeGroup, VariableBaseMSM,
};
pub mod models;
pub mod transmute;
pub use models::*;
