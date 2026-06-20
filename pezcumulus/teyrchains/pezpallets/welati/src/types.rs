// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use codec::{Decode, Encode, MaxEncodedLen};
use pezframe_support::pezpallet_prelude::*;
use pezframe_system::{pezpallet_prelude::BlockNumberFor, Config as SystemConfig};
use pezpallet_tiki::Tiki;
use pezsp_runtime::RuntimeDebug;
use pezsp_std::prelude::*;
use scale_info::TypeInfo;

#[derive(RuntimeDebug, Eq, PartialEq)]
pub enum ElectionOutcome<AccountId> {
	/// Kazananlar belirlendi.
	Winners(BoundedVec<AccountId, ConstU32<201>>),
	/// İkinci tur gerekli, bunlar da adaylar.
	RunoffRequired(BoundedVec<AccountId, ConstU32<2>>),
}

/// Devlet pozisyonları (seçimle gelinen makamlar)
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum GovernmentPosition {
	/// Cumhurbaşkanı (Serok)
	Serok,
	/// Parlamenter (Parlementer)
	Parlementer,
	/// Meclis Başkanı (SerokiMeclise)
	MeclisBaskanı,
	/// Dîwan Üyesi (EndameDiwane)
	EndameDiwane,
}

/// Devlet memuru rolleri (atama ile gelinen pozisyonlar)
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum OfficialRole {
	// Adalet Bakanlığı altında
	Dadger,
	Dozger,
	Hiquqnas,
	Noter,

	// Hazine Bakanlığı altında
	Xezinedar,
	Bacgir,
	GerinendeyeCavkaniye,

	// Teknoloji ve Altyapı Bakanlığı altında
	OperatoreTore,
	PisporeEwlehiyaSiber,
	GerinendeyeDaneye,

	// İçişleri ve İletişim Bakanlığı altında
	Berdevk,
	Qeydkar,

	// Dışişleri Bakanlığı altında
	Balyoz,
	Navbeynkar,
	ParezvaneCandi,

	// Denetim Bakanlığı altında
	Mufetis,
	KaliteKontrolker,

	// Ekonomi ve Ticaret Bakanlığı altında
	Bazargan,
	RêvebereProjeyê,

	// Milli Eğitim ve Diyanet Bakanlığı altında
	Feqi,
	Perwerdekar,
	Rewsenbir,
	Mamoste,

	// İstisnai atama (doğrudan Serok)
	Mela,
}

/// Bakan pozisyonları (Wezîr alt kategorileri)
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum MinisterRole {
	/// Adalet Bakanı
	AdvaletWeziri,
	/// Hazine Bakanı
	XezineWeziri,
	/// Teknoloji ve Altyapı Bakanı
	TeknolojîWeziri,
	/// İçişleri ve İletişim Bakanı
	NavxweWeziri,
	/// Dışişleri Bakanı
	DerveWeziri,
	/// Denetim Bakanı
	DenetimWeziri,
	/// Ekonomi ve Ticaret Bakanı
	AbûrîWeziri,
	/// Milli Eğitim ve Diyanet Bakanı
	PerwerdeDiyanetWeziri,
}

/// Seçim türleri
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum ElectionType {
	/// Cumhurbaşkanlığı seçimi (özel kurallar)
	Presidential,
	/// Parlamento seçimi (201 kişi)
	Parliamentary,
	/// Meclis başkanlığı seçimi (parlamenterler arası)
	SpeakerElection,
	/// Dîwan üyesi seçimi
	ConstitutionalCourt,
}

/// Oy türleri
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum VoteType {
	/// Normal vatandaş oyu
	Citizen,
	/// Ağırlıklı oy (Trust Puanı bazlı)
	Weighted,
	/// Delegasyon oyu
	Delegated,
}

/// Nominasyon bilgilerini tutan yapı
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct NominationInfo<T: pezframe_system::Config> {
	/// Nominasyonu yapan (Bakan)
	pub nominator: T::AccountId,
	/// Aday gösterilen kişi
	pub nominee: T::AccountId,
	/// Nominasyonun yapıldığı blok
	pub nominated_at: BlockNumberFor<T>,
	/// Onaylanıp onaylanmadığı
	pub approved: bool,
	/// Onaylayan (genellikle Serok)
	pub approver: Option<T::AccountId>,
	/// Onaylanma tarihi
	pub approved_at: Option<BlockNumberFor<T>>,
	/// Nominasyon durumu
	pub status: NominationStatus,
}

/// Nominasyon durumları
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum NominationStatus {
	/// Bekleyen nominasyon
	Pending,
	/// Onaylanmış
	Approved,
	/// Reddedilmiş
	Rejected,
	/// İptal edilmiş
	Cancelled,
	/// Süresi dolmuş
	Expired,
}

/// Kolektif karar türleri
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum CollectiveDecisionType {
	/// Parlamento kararı (basit çoğunluk - %50+1)
	ParliamentSimpleMajority,
	/// Parlamento süper çoğunluk kararı (2/3)
	ParliamentSuperMajority,
	/// Parlamento mutlak çoğunluk (3/4 - anayasa değişikliği)
	ParliamentAbsoluteMajority,
	/// Dîwan kararı (anayasal denetim - 2/3)
	ConstitutionalReview,
	/// Dîwan ittifak kararı (tüm üyeler)
	ConstitutionalUnanimous,
	/// Karma karar (Parlamento + Serok onayı)
	HybridDecision,
	/// Cumhurbaşkanı tek başına karar
	ExecutiveDecision,
	/// Veto override (Parlamento 2/3 ile veto aşma)
	VetoOverride,
}

/// Kolektif oylamanın durumu
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum ProposalStatus {
	/// Taslak halinde (henüz oylamaya sunulmamış)
	Draft,
	/// Aktif oylama
	Active,
	/// Kabul edildi
	Approved,
	/// Reddedildi
	Rejected,
	/// İptal edildi
	Cancelled,
	/// Zaman aşımı
	Expired,
	/// Veto edildi (Serok tarafından)
	Vetoed,
	/// Anayasal denetimde (Dîwan'da)
	UnderConstitutionalReview,
}

/// Kolektif teklif bilgileri
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct CollectiveProposal<T: pezframe_system::Config> {
	/// Teklif ID'si
	pub proposal_id: u32,
	/// Teklif sahibi
	pub proposer: T::AccountId,
	/// Teklif başlığı
	pub title: BoundedVec<u8, ConstU32<100>>,
	/// Teklif açıklaması
	pub description: BoundedVec<u8, ConstU32<1000>>,
	/// Teklif tarihi
	pub proposed_at: BlockNumberFor<T>,
	/// Oylama başlangıç tarihi
	pub voting_starts_at: BlockNumberFor<T>,
	/// Bitiş tarihi
	pub expires_at: BlockNumberFor<T>,
	/// Karar türü
	pub decision_type: CollectiveDecisionType,
	/// Mevcut durum
	pub status: ProposalStatus,
	/// Olumlu oylar
	pub aye_votes: u32,
	/// Olumsuz oylar
	pub nay_votes: u32,
	/// Çekimser oylar
	pub abstain_votes: u32,
	/// Gerekli minimum oy sayısı
	pub threshold: u32,
	/// Oy veren üye sayısı
	pub votes_cast: u32,
	/// Öncelik seviyesi
	pub priority: ProposalPriority,
	/// GÜNCELLENDİ: Teklif kabul edilirse çalıştırılacak olan çağrı (extrinsic).
	#[codec(skip)]
	pub call: Option<Box<<T as SystemConfig>::RuntimeCall>>,
}

/// Teklif öncelik seviyeleri
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum ProposalPriority {
	/// Düşük öncelik
	Low,
	/// Normal öncelik
	Normal,
	/// Yüksek öncelik
	High,
	/// Acil (24 saat içinde)
	Urgent,
	/// Kritik (anında)
	Critical,
}

/// Kollektif oy bilgisi
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct CollectiveVote<T: pezframe_system::Config> {
	/// Oy veren
	pub voter: T::AccountId,
	/// Teklif ID'si
	pub proposal_id: u32,
	/// Oy türü
	pub vote: VoteChoice,
	/// Oy verme zamanı
	pub voted_at: BlockNumberFor<T>,
	/// Oy gerekçesi (opsiyonel)
	pub rationale: Option<BoundedVec<u8, ConstU32<500>>>,
}

/// Oy seçenekleri
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum VoteChoice {
	/// Evet
	Aye,
	/// Hayır
	Nay,
	/// Çekimser
	Abstain,
}

/// Parlamento üye bilgisi
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	TypeInfo,
	MaxEncodedLen,
	Default,
	RuntimeDebug,
)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct ParliamentMember<T: pezframe_system::Config> {
	/// Üye hesabı
	pub account: T::AccountId,
	/// Seçilme tarihi
	pub elected_at: BlockNumberFor<T>,
	/// Görev süresi bitiş tarihi
	pub term_ends_at: BlockNumberFor<T>,
	/// Katıldığı oylama sayısı
	pub votes_participated: u32,
	/// Toplam oy hakkı sayısı
	pub total_votes_eligible: u32,
	/// Katılım oranı (yüzde)
	pub participation_rate: u8,
	/// Özel komiteler
	pub committees: BoundedVec<CommitteeType, ConstU32<5>>,
}

/// Komite türleri
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum CommitteeType {
	/// Bütçe Komisyonu
	Budget,
	/// Dışişleri Komisyonu
	ForeignAffairs,
	/// Adalet Komisyonu
	Justice,
	/// Teknoloji Komisyonu
	Technology,
	/// Eğitim Komisyonu
	Education,
	/// Sağlık Komisyonu
	Health,
	/// Anayasa Komisyonu
	Constitutional,
}

/// Dîwan üye bilgisi
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct DiwanMember<T: pezframe_system::Config> {
	/// Üye hesabı
	pub account: T::AccountId,
	/// Atanma tarihi
	pub appointed_at: BlockNumberFor<T>,
	/// Görev süresi (9 yıl)
	pub term_ends_at: BlockNumberFor<T>,
	/// Atayan makam (Parlamento/Serok)
	pub appointed_by: AppointmentAuthority<T>,
	/// Uzmanlık alanı
	pub specialization: ConstitutionalSpecialization,
	/// Verdiği karar sayısı
	pub decisions_made: u32,
}

/// Atama yetkisi
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub enum AppointmentAuthority<T: pezframe_system::Config> {
	/// Parlamento tarafından atanan (6 kişi)
	Parliament,
	/// Serok tarafından atanan (5 kişi)
	President(T::AccountId),
}

/// Anayasal uzmanlık alanları
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum ConstitutionalSpecialization {
	/// Temel haklar ve özgürlükler
	FundamentalRights,
	/// Devlet organizasyonu
	StateOrganization,
	/// Ekonomik düzen
	EconomicOrder,
	/// Sosyal haklar
	SocialRights,
	/// Yargı bağımsızlığı
	JudicialIndependence,
	/// Yerel yönetimler
	LocalGovernment,
	/// Uluslararası hukuk
	InternationalLaw,
}

/// Atama süreci bilgisi
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct AppointmentProcess<T: pezframe_system::Config> {
	/// Süreç ID'si
	pub process_id: u32,
	/// Atama yapılacak pozisyon
	pub position: OfficialRole,
	/// İlgili bakan (aday gösteren)
	pub nominating_minister: T::AccountId,
	/// Aday
	pub nominee: T::AccountId,
	/// Başlatılma tarihi
	pub initiated_at: BlockNumberFor<T>,
	/// Son karar tarihi
	pub deadline: BlockNumberFor<T>,
	/// Mevcut durum
	pub status: AppointmentStatus,
	/// Ek belgeler/gerekçe
	pub documents: BoundedVec<BoundedVec<u8, ConstU32<1000>>, ConstU32<10>>,
}

/// Atama süreci durumları
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum AppointmentStatus {
	/// Bakan nominasyonu bekliyor
	WaitingNomination,
	/// Serok onayı bekliyor
	WaitingPresidentialApproval,
	/// Parlamento onayı bekliyor (bazı pozisyonlar için)
	WaitingParliamentaryApproval,
	/// Onaylandı
	Approved,
	/// Reddedildi
	Rejected,
	/// Süre doldu
	Expired,
}

/// Yönetişim metrikleri
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct GovernanceMetrics<T: pezframe_system::Config> {
	/// Toplam aktif teklif sayısı
	pub active_proposals: u32,
	/// Bu dönem geçen yasa sayısı
	pub laws_passed_this_term: u32,
	/// Parlamento katılım oranı
	pub parliament_attendance_rate: u8,
	/// Dîwan karar sayısı
	pub constitutional_decisions: u32,
	/// Ortalama karar süresi (blok cinsinden)
	pub average_decision_time: BlockNumberFor<T>,
	/// Veto edilen yasa sayısı
	pub vetoed_laws: u32,
	/// Veto aşılan sayı
	pub veto_overrides: u32,
}

/// Seçim durumları
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum ElectionStatus {
	/// Aday kayıt dönemi
	CandidacyPeriod,
	/// Kampanya dönemi
	CampaignPeriod,
	/// Oy verme dönemi
	VotingPeriod,
	/// Tamamlandı
	Completed,
	/// İptal edildi
	Cancelled,
}

/// Aday bilgileri
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct CandidateInfo<T: pezframe_system::Config> {
	pub account: T::AccountId,
	pub district_id: Option<u32>,
	pub registered_at: BlockNumberFor<T>,
	pub endorsers: BoundedVec<T::AccountId, ConstU32<100>>,
	pub vote_count: u32,
	pub deposit_paid: u128,
	pub campaign_data: BoundedVec<u8, ConstU32<500>>,
}

/// Seçim sonuçları
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct ElectionResult<T: pezframe_system::Config> {
	pub election_id: u32,
	pub winners: BoundedVec<T::AccountId, ConstU32<201>>, // Max 201 for Parliament
	pub total_votes: u32,
	pub turnout_percentage: u8,
	pub finalized_at: BlockNumberFor<T>,
}

/// Seçim bölgesi bilgileri
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Default,
)]
#[codec(mel_bound())]
pub struct ElectoralDistrict {
	pub district_id: u32,
	pub name: BoundedVec<u8, ConstU32<50>>,
	pub seat_count: u32,
	pub voter_population: u32,
	pub geographic_bounds: Option<BoundedVec<u8, ConstU32<200>>>,
}

/// Seçim bilgilerini tutan yapı - Genişletilmiş versiyon
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct ElectionInfo<T: pezframe_system::Config> {
	/// Seçim ID'si
	pub election_id: u32,
	/// Seçim türü
	pub election_type: ElectionType,
	/// Seçimin başlangıç bloğu
	pub start_block: BlockNumberFor<T>,
	/// Aday kayıt son tarihi
	pub candidacy_deadline: BlockNumberFor<T>,
	/// Kampanya başlangıcı
	pub campaign_start: BlockNumberFor<T>,
	/// Oy verme başlangıcı
	pub voting_start: BlockNumberFor<T>,
	/// Seçimin bitiş bloğu
	pub end_block: BlockNumberFor<T>,
	/// Adayların listesi
	pub candidates: BoundedVec<T::AccountId, ConstU32<500>>, // Geniş limit
	/// Toplam oy sayısı
	pub total_votes: u32,
	/// Seçim durumu
	pub status: ElectionStatus,
	/// Seçim bölgeleri
	pub districts: BoundedVec<ElectoralDistrict, ConstU32<50>>,
	/// Minimum katılım oranı (yüzde olarak)
	pub minimum_turnout: u8,
}

/// Oy bilgilerini tutan yapı
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct ElectionVoteInfo<T: pezframe_system::Config> {
	/// Oy veren kişi
	pub voter: T::AccountId,
	/// Oy verilen adaylar (çoklu oy için)
	pub candidates: BoundedVec<T::AccountId, ConstU32<10>>,
	/// Oyun verildiği blok
	pub vote_block: BlockNumberFor<T>,
	/// Oyun ağırlığı (Trust Puanı bazlı olabilir)
	pub vote_weight: u32,
	/// Oy türü (gizli/açık)
	pub vote_type: VoteType,
	/// Seçim bölgesi
	pub district_id: Option<u32>,
}

/// Seçim güvenlik önlemleri
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum SecurityMeasure {
	/// Çift oy kontrolü
	DuplicateVoteDetection,
	/// Kimlik doğrulama
	IdentityVerification,
	/// Oy gizliliği
	VotePrivacy,
	/// Manipülasyon koruması
	ManipulationPrevention,
}

/// Oy gizlilik düzeyi
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum VotePrivacyLevel {
	/// Tamamen açık
	FullyOpen,
	/// Kısmi gizli (sadece sonuç görünür)
	PartiallyPrivate,
	/// Tamamen gizli
	FullyPrivate,
}

/// Çift oy önleme yöntemi
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum DuplicateVoteMethod {
	/// Hesap bazlı kontrol
	AccountBased,
	/// Kimlik bazlı kontrol
	IdentityBased,
	/// Çoklu katman kontrol
	MultiLayered,
}

/// Şeffaflık düzeyi
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum TransparencyLevel {
	/// Minimum şeffaflık
	Minimal,
	/// Standart şeffaflık
	Standard,
	/// Maksimum şeffaflık
	Maximum,
}

/// Denetim gereksinimleri
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
pub struct AuditRequirements {
	/// İç denetim gerekli mi?
	pub internal_audit_required: bool,
	/// Dış denetim gerekli mi?
	pub external_audit_required: bool,
	/// Gerçek zamanlı izleme
	pub real_time_monitoring: bool,
	/// Denetim raporu gerekli mi?
	pub audit_report_required: bool,
}

/// Oy ağırlık sistemi
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum VoteWeightMethod {
	/// Eşit ağırlık
	Equal,
	/// Trust puanı bazlı
	TrustScoreBased,
	/// Pozisyon bazlı
	PositionBased,
}

/// Kimlik doğrulama yöntemi
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum VoterAuthMethod {
	/// KYC bazlı
	KycBased,
	/// Biometric
	Biometric,
	/// Çoklu faktör
	MultiFactor,
}

/// Kampanya düzenlemeleri
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct CampaignRegulations<T: pezframe_system::Config> {
	/// Kampanya süresi (blok sayısı)
	pub duration_blocks: BlockNumberFor<T>,
	/// Maksimum harcama limiti
	pub spending_limit: Option<u128>,
	/// İzin verilen etkinlik türleri
	pub allowed_activities: BoundedVec<CampaignActivityType, ConstU32<20>>,
	/// Yasaklanan etkinlik türleri
	pub prohibited_activities: BoundedVec<CampaignActivityType, ConstU32<20>>,
}

/// Kampanya etkinlik türleri
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum CampaignActivityType {
	/// Kitle toplantısı
	PublicRally,
	/// Medya reklamı
	MediaAdvertisement,
	/// Kapı kapı ziyaret
	DoorToDoorCanvassing,
	/// Dijital kampanya
	DigitalCampaign,
	/// Bağış toplama
	FundraisingEvent,
}

/// Aday olma kuralları
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
pub struct CandidacyRules {
	/// Minimum yaş gereksinimi
	pub minimum_age: Option<u32>,
	/// Eğitim gereksinimleri
	pub education_requirements: Option<EducationLevel>,
	/// Geçmiş deneyim gereksinimleri
	pub experience_requirements: Option<BoundedVec<u8, ConstU32<500>>>,
	/// Yasaklı geçmiş koşulları
	pub disqualifying_conditions: BoundedVec<DisqualifyingCondition, ConstU32<10>>,
}

/// Eğitim seviyesi
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum EducationLevel {
	/// İlkokul
	Elementary,
	/// Ortaokul
	MiddleSchool,
	/// Lise
	HighSchool,
	/// Üniversite
	University,
	/// Yüksek lisans
	MastersDegree,
	/// Doktora
	Doctorate,
}

/// Diskalifiye edici koşullar
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum DisqualifyingCondition {
	/// Suç kaydı
	CriminalRecord,
	/// Mali suistimal
	FinancialMisconduct,
	/// Etik ihlal
	EthicsViolation,
	/// Çifte vatandaşlık
	DualCitizenship,
	/// Zihinsel yetersizlik
	MentalIncapacity,
}

/// Parlamento komite üyeliği detayları
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct CommitteeMembership<T: pezframe_system::Config> {
	pub committee: CommitteeType,
	pub role: CommitteeRole,
	pub joined_at: BlockNumberFor<T>,
	pub term_ends_at: Option<BlockNumberFor<T>>,
}

/// Komitedeki rol
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum CommitteeRole {
	/// Başkan
	Chairman,
	/// Başkan yardımcısı
	ViceChairman,
	/// Üye
	Member,
	/// Raportör
	Rapporteur,
}

/// Yasama süreci aşamaları
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum LegislativeStage {
	/// Taslak aşaması
	Draft,
	/// Komite incelemesi
	CommitteeReview,
	/// Genel kurul ilk görüşme
	FirstReading,
	/// Komiteye iade
	CommitteeRevision,
	/// Genel kurul ikinci görüşme
	SecondReading,
	/// Üçüncü görüşme
	ThirdReading,
	/// Cumhurbaşkanına gönderildi
	SentToPresident,
	/// Onaylandı
	Approved,
	/// Veto edildi
	Vetoed,
	/// Kanunlaştı
	Enacted,
}

/// Yasa türleri
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum LawType {
	/// Anayasa değişikliği
	ConstitutionalAmendment,
	/// Organik kanun
	OrganicLaw,
	/// Olağan kanun
	OrdinaryLaw,
	/// Bütçe kanunu
	BudgetLaw,
	/// Uluslararası anlaşma onayı
	InternationalAgreement,
}

/// Anayasal denetim türleri
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum ConstitutionalReviewType {
	/// Ön denetim (kanun çıkmadan önce)
	PreliminaryReview,
	/// Sonraki denetim (kanun çıktıktan sonra)
	SubsequentReview,
	/// Bireysel başvuru
	IndividualApplication,
	/// Soyut norm denetimi
	AbstractNormControl,
}

/// Veto türleri
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum VetoType {
	/// Tam veto
	AbsoluteVeto,
	/// Kısmi veto
	LineItemVeto,
	/// Geciktirici veto
	SuspensiveVeto,
}

/// Meclis oturum türleri
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum SessionType {
	/// Olağan oturum
	RegularSession,
	/// Olağanüstü oturum
	ExtraordinarySession,
	/// Gizli oturum
	ClosedSession,
	/// Acil oturum
	EmergencySession,
}

/// Oturum durumu
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum SessionStatus {
	/// Planlandı
	Scheduled,
	/// Aktif
	Active,
	/// Ertelendi
	Postponed,
	/// Tamamlandı
	Completed,
	/// İptal edildi
	Cancelled,
}

/// Meclis oturumu bilgileri
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct ParliamentSession<T: pezframe_system::Config> {
	pub session_id: u32,
	pub session_type: SessionType,
	pub scheduled_start: BlockNumberFor<T>,
	pub actual_start: Option<BlockNumberFor<T>>,
	pub end_time: Option<BlockNumberFor<T>>,
	pub status: SessionStatus,
	pub agenda: BoundedVec<u32, ConstU32<50>>, // Proposal ID'leri
	pub attendees: BoundedVec<T::AccountId, ConstU32<201>>,
	pub decisions_made: BoundedVec<u32, ConstU32<20>>, // Alınan karar ID'leri
}

/// Devlet bütçesi kategorileri
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum BudgetCategory {
	/// Personel giderleri
	Personnel,
	/// Mal ve hizmet alımları
	GoodsAndServices,
	/// Yatırım harcamaları
	CapitalExpenditures,
	/// Transfer ödemeleri
	TransferPayments,
	/// Borç ödemeleri
	DebtService,
	/// Yedek ödenekler
	Contingency,
}

/// Bütçe onay durumu
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum BudgetStatus {
	/// Taslak
	Draft,
	/// Meclis'te
	InParliament,
	/// Onaylandı
	Approved,
	/// Uygulanıyor
	InExecution,
	/// Tamamlandı
	Completed,
}

/// Helper trait'ler için yardımcı yapılar
pub trait GovernmentPositionInfo {
	fn required_trust_score(&self) -> u128;
	fn required_tiki(&self) -> Option<Tiki>;
	fn term_length_blocks(&self) -> u32;
}

impl GovernmentPositionInfo for GovernmentPosition {
	fn required_trust_score(&self) -> u128 {
		match self {
			GovernmentPosition::Serok => 600,
			GovernmentPosition::Parlementer => 300,
			GovernmentPosition::MeclisBaskanı => 400,
			GovernmentPosition::EndameDiwane => 750,
		}
	}

	fn required_tiki(&self) -> Option<Tiki> {
		match self {
			GovernmentPosition::Serok => Some(Tiki::Welati),
			GovernmentPosition::Parlementer => Some(Tiki::Welati),
			GovernmentPosition::MeclisBaskanı => Some(Tiki::Parlementer),
			GovernmentPosition::EndameDiwane => Some(Tiki::Welati),
		}
	}

	fn term_length_blocks(&self) -> u32 {
		match self {
			GovernmentPosition::Serok => 4 * 365 * 24 * 60 * 10, // 4 yıl
			GovernmentPosition::Parlementer => 4 * 365 * 24 * 60 * 10, // 4 yıl
			GovernmentPosition::MeclisBaskanı => 2 * 365 * 24 * 60 * 10, // 2 yıl
			GovernmentPosition::EndameDiwane => 9 * 365 * 24 * 60 * 10, // 9 yıl
		}
	}
}

pub trait OfficialRoleInfo {
	fn required_trust_score(&self) -> u128;
	fn nominating_minister(&self) -> MinisterRole;
	fn requires_parliament_approval(&self) -> bool;
}

impl OfficialRoleInfo for OfficialRole {
	fn required_trust_score(&self) -> u128 {
		75 // Anayasada belirtilen genel şart
	}

	fn nominating_minister(&self) -> MinisterRole {
		match self {
			OfficialRole::Dadger
			| OfficialRole::Dozger
			| OfficialRole::Hiquqnas
			| OfficialRole::Noter => MinisterRole::AdvaletWeziri,

			OfficialRole::Xezinedar | OfficialRole::Bacgir | OfficialRole::GerinendeyeCavkaniye => {
				MinisterRole::XezineWeziri
			},

			OfficialRole::OperatoreTore
			| OfficialRole::PisporeEwlehiyaSiber
			| OfficialRole::GerinendeyeDaneye => MinisterRole::TeknolojîWeziri,

			OfficialRole::Berdevk | OfficialRole::Qeydkar => MinisterRole::NavxweWeziri,

			OfficialRole::Balyoz | OfficialRole::Navbeynkar | OfficialRole::ParezvaneCandi => {
				MinisterRole::DerveWeziri
			},

			OfficialRole::Mufetis | OfficialRole::KaliteKontrolker => MinisterRole::DenetimWeziri,

			OfficialRole::Bazargan | OfficialRole::RêvebereProjeyê => MinisterRole::AbûrîWeziri,

			OfficialRole::Feqi
			| OfficialRole::Perwerdekar
			| OfficialRole::Rewsenbir
			| OfficialRole::Mamoste => MinisterRole::PerwerdeDiyanetWeziri,

			// Mela özel durum - doğrudan Serok atar
			OfficialRole::Mela => MinisterRole::AdvaletWeziri, // Placeholder
		}
	}

	fn requires_parliament_approval(&self) -> bool {
		match self {
			// Yüksek düzey pozisyonlar Parlamento onayı gerektirir
			OfficialRole::Dadger
			| OfficialRole::Xezinedar
			| OfficialRole::PisporeEwlehiyaSiber
			| OfficialRole::Mufetis
			| OfficialRole::Balyoz => true,
			// Diğerleri sadece Serok onayı
			_ => false,
		}
	}
}
