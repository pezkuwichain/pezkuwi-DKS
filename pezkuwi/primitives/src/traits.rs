// pezkuwi/primitives/src/traits.rs
#![cfg_attr(not(feature = "std"), no_no_std)]

use pezframe_support::pezpallet_prelude::*;
use pezsp_runtime::traits::BlockNumber as BlockNumberT;
use pezsp_std::prelude::*;
use codec::{Encode, Decode};
use scale_info::TypeInfo;
// `pezpallet-staking-score`'dan StakingDetails'ı doğrudan kullanabilmek için
// StakingDetails'ı burada yeniden tanımlıyoruz. Bu struct, pezpallet-staking-score'daki ile BİREBİR AYNI olmalı.
#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, Debug)]
pub struct StakingDetails<Balance> {
    pub staked_amount: Balance,
    pub nominations_count: u32,
    pub unlocking_chunks_count: u32,
}

/// Puanlamada kullanılacak ham skor tipi.
pub type RawScore = u32;

/// Staking skorunu sağlayan arayüz.
pub trait StakingScoreProvider<AccountId, B: BlockNumber> { // B ismini veriyoruz
    /// Belirtilen hesabın staking puanını ve hesaplamada kullanılan süreyi döndürür.
    /// (score, duration_in_blocks)
    fn get_staking_score(who: &AccountId) -> (RawScore, B); // B'yi kullanıyoruz
}

/// Referans skorunu sağlayan arayüz.
pub trait ReferralScoreProvider<AccountId> {
    /// Belirtilen hesabın referans puanını döndürür.
    fn get_referral_score(who: &AccountId) -> RawScore;
}

/// Vatandaşlık durumunu sağlayan arayüz.
pub trait CitizenshipStatusProvider<AccountId> {
    /// Belirtilen hesabın vatandaş olup olmadığını döndürür (Approved KYC).
    fn is_citizen(who: &AccountId) -> bool;
}

/// Eğitim/Perwerde skorunu sağlayan arayüz.
pub trait PerwerdeScoreProvider<AccountId> {
    /// Belirtilen hesabın eğitim puanını döndürür.
    fn get_perwerde_score(who: &AccountId) -> RawScore;
}

/// Tiki (Rol) skorunu sağlayan arayüz.
pub trait TikiScoreProvider<AccountId> {
    /// Belirtilen hesabın sahip olduğu Tiki'lerden gelen toplam bileşen puanını döndürür.
    fn get_tiki_score(who: &AccountId) -> RawScore;
}

// Trust skorunun güncellenmesi için bir arayüz.
// Bu trait'i Trust paleti implemente edecektir.
pub trait TrustScoreUpdater<AccountId> {
    /// Belirli bir hesabın Trust Puanını yeniden hesaplar ve günceller.
    fn update_trust_score(who: &AccountId);
    /// Tüm aktif hesapların Trust Puanlarını günceller (uzun sürebilir, batch işlenmelidir).
    fn update_all_trust_scores();
}