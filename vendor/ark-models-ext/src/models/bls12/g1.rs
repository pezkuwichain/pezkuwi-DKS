use crate::models::{
    bls12::Bls12Config,
    short_weierstrass::{Affine, Projective},
};
use ark_ec::{AffineRepr, CurveGroup};
use ark_serialize::*;
use ark_std::vec::Vec;

pub type G1Affine<P> = Affine<<P as Bls12Config>::G1Config>;
pub type G1Projective<P> = Projective<<P as Bls12Config>::G1Config>;

#[derive(CanonicalSerialize, CanonicalDeserialize)]
pub struct G1Prepared<P: Bls12Config>(pub G1Affine<P>);

impl<P: Bls12Config> Copy for G1Prepared<P> {}

impl<P: Bls12Config> Clone for G1Prepared<P> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<P: Bls12Config> PartialEq for G1Prepared<P> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<P: Bls12Config> Eq for G1Prepared<P> {}

impl<P: Bls12Config> core::fmt::Debug for G1Prepared<P> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("G1Prepared").field(&self.0).finish()
    }
}

impl<P: Bls12Config> From<G1Affine<P>> for G1Prepared<P> {
    fn from(other: G1Affine<P>) -> Self {
        G1Prepared(other)
    }
}

impl<P: Bls12Config> From<G1Projective<P>> for G1Prepared<P> {
    fn from(q: G1Projective<P>) -> Self {
        q.into_affine().into()
    }
}

impl<'a, P: Bls12Config> From<&'a G1Affine<P>> for G1Prepared<P> {
    fn from(other: &'a G1Affine<P>) -> Self {
        G1Prepared(*other)
    }
}

impl<'a, P: Bls12Config> From<&'a G1Projective<P>> for G1Prepared<P> {
    fn from(q: &'a G1Projective<P>) -> Self {
        q.into_affine().into()
    }
}

impl<P: Bls12Config> G1Prepared<P> {
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

impl<P: Bls12Config> Default for G1Prepared<P> {
    fn default() -> Self {
        G1Prepared(G1Affine::<P>::generator())
    }
}
