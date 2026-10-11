use ark_serialize::*;
use ark_std::vec::Vec;

use crate::{
    bw6::BW6Config,
    short_weierstrass::{Affine, Projective},
    AffineRepr, CurveGroup,
};

pub type G2Affine<P> = Affine<<P as BW6Config>::G2Config>;
pub type G2Projective<P> = Projective<<P as BW6Config>::G2Config>;

#[derive(CanonicalSerialize, CanonicalDeserialize)]
pub struct G2Prepared<P: BW6Config>(pub G2Affine<P>);

impl<P: BW6Config> Copy for G2Prepared<P> {}

impl<P: BW6Config> Clone for G2Prepared<P> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<P: BW6Config> PartialEq for G2Prepared<P> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<P: BW6Config> Eq for G2Prepared<P> {}

impl<P: BW6Config> core::fmt::Debug for G2Prepared<P> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("G2Prepared").field(&self.0).finish()
    }
}

impl<P: BW6Config> From<G2Affine<P>> for G2Prepared<P> {
    fn from(other: G2Affine<P>) -> Self {
        G2Prepared(other)
    }
}

impl<P: BW6Config> From<G2Projective<P>> for G2Prepared<P> {
    fn from(q: G2Projective<P>) -> Self {
        q.into_affine().into()
    }
}

impl<'a, P: BW6Config> From<&'a G2Affine<P>> for G2Prepared<P> {
    fn from(other: &'a G2Affine<P>) -> Self {
        G2Prepared(*other)
    }
}

impl<'a, P: BW6Config> From<&'a G2Projective<P>> for G2Prepared<P> {
    fn from(q: &'a G2Projective<P>) -> Self {
        q.into_affine().into()
    }
}

impl<P: BW6Config> G2Prepared<P> {
    pub fn is_zero(&self) -> bool {
        self.0.infinity
    }
}

impl<P: BW6Config> Default for G2Prepared<P> {
    fn default() -> Self {
        G2Prepared(G2Affine::<P>::generator())
    }
}
