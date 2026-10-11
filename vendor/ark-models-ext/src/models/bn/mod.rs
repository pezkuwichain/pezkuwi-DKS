pub use ark_ec::models::bn::TwistType;
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

pub trait BnConfig: 'static + Sized {
    /// The absolute value of the BN curve parameter `X`
    /// (as a multiple of six the `X` used in the paper).
    const X: &'static [u64];
    /// Whether or not `X` is negative.
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
    ) -> MillerLoopOutput<Bn<Self>>;

    fn final_exponentiation(f: MillerLoopOutput<Bn<Self>>) -> Option<PairingOutput<Bn<Self>>>;
}

pub mod g1;
pub mod g2;

pub use self::{
    g1::{G1Affine, G1Prepared, G1Projective},
    g2::{G2Affine, G2Prepared, G2Projective},
};

pub struct Bn<P: BnConfig>(PhantomData<fn() -> P>);

impl<P: BnConfig> Copy for Bn<P> {}

impl<P: BnConfig> Clone for Bn<P> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<P: BnConfig> PartialEq for Bn<P> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<P: BnConfig> Eq for Bn<P> {}

impl<P: BnConfig> core::fmt::Debug for Bn<P> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("Bn").field(&self.0).finish()
    }
}

impl<P: BnConfig> core::hash::Hash for Bn<P> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        core::hash::Hash::hash(&self.0, state)
    }
}

impl<P: BnConfig> Pairing for Bn<P> {
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
