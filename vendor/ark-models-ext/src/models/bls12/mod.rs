pub use ark_ec::models::bls12::TwistType;
use ark_ec::{
    models::CurveConfig,
    pairing::{MillerLoopOutput, Pairing, PairingOutput},
};
use ark_ff::{
    fields::{
        fp12_2over3over2::{Fp12, Fp12Config},
        fp2::Fp2Config,
        fp6_3over2::Fp6Config,
        Fp2,
    },
    PrimeField,
};
use ark_std::marker::PhantomData;

use crate::models::short_weierstrass::SWCurveConfig;

pub trait Bls12Config: 'static + Sized {
    /// Parameterizes the BLS12 family.
    const X: &'static [u64];
    /// Is `Self::X` negative?
    const X_IS_NEGATIVE: bool;
    /// What kind of twist is this?
    const TWIST_TYPE: TwistType;

    type Fp: PrimeField + Into<<Self::Fp as PrimeField>::BigInt>;
    type Fp2Config: Fp2Config<Fp = Self::Fp>;
    type Fp6Config: Fp6Config<Fp2Config = Self::Fp2Config>;
    type Fp12Config: Fp12Config<Fp6Config = Self::Fp6Config>;
    type G1Config: SWCurveConfig<BaseField = Self::Fp>;
    type G2Config: SWCurveConfig<
        BaseField = Fp2<Self::Fp2Config>,
        ScalarField = <Self::G1Config as CurveConfig>::ScalarField,
    >;

    fn multi_miller_loop(
        a_vec: impl IntoIterator<Item = impl Into<G1Prepared<Self>>>,
        b_vec: impl IntoIterator<Item = impl Into<G2Prepared<Self>>>,
    ) -> MillerLoopOutput<Bls12<Self>>;

    fn final_exponentiation(f: MillerLoopOutput<Bls12<Self>>)
        -> Option<PairingOutput<Bls12<Self>>>;
}

pub mod g1;
pub mod g2;

pub use self::{
    g1::{G1Affine, G1Prepared, G1Projective},
    g2::{G2Affine, G2Prepared, G2Projective},
};

pub struct Bls12<P: Bls12Config>(PhantomData<fn() -> P>);

impl<P: Bls12Config> Copy for Bls12<P> {}

impl<P: Bls12Config> Clone for Bls12<P> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<P: Bls12Config> PartialEq for Bls12<P> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<P: Bls12Config> Eq for Bls12<P> {}

impl<P: Bls12Config> core::fmt::Debug for Bls12<P> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("Bls12").field(&self.0).finish()
    }
}

impl<P: Bls12Config> core::hash::Hash for Bls12<P> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        core::hash::Hash::hash(&self.0, state)
    }
}

impl<P: Bls12Config> Pairing for Bls12<P> {
    type BaseField = <P::G1Config as CurveConfig>::BaseField;
    type ScalarField = <P::G1Config as CurveConfig>::ScalarField;
    type G1 = G1Projective<P>;
    type G1Affine = G1Affine<P>;
    type G1Prepared = G1Prepared<P>;
    type G2 = G2Projective<P>;
    type G2Affine = G2Affine<P>;
    type G2Prepared = G2Prepared<P>;
    type TargetField = Fp12<P::Fp12Config>;

    fn multi_miller_loop(
        a: impl IntoIterator<Item = impl Into<Self::G1Prepared>>,
        b: impl IntoIterator<Item = impl Into<Self::G2Prepared>>,
    ) -> MillerLoopOutput<Self> {
        P::multi_miller_loop(a, b)
    }

    fn final_exponentiation(f: MillerLoopOutput<Self>) -> Option<PairingOutput<Self>> {
        P::final_exponentiation(f)
    }
}
