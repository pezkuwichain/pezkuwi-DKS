# TNPoS — Tasarım Spesifikasyonu

**Tarih:** 2026-08-28 · **Durum:** onaylandı, uygulanmadı
**Yerine geçtiği:** `pezpallet-validator-pool` (tamamen silinir, yamalanmaz)
**Hedef pallet:** `pezpallet-tnpos`

---

## 1. İddia — ve iddianın sınırı

TNPoS, komite üyeliğinin **sermayeyle satın alınamadığı**, **kestirilemez biçimde
örneklendiği** ve **hesabı sorulabilir** olduğu bir kesin-sonluluk (deterministic finality)
konsensüsüdür.

**Doğru cümle:**
> Deterministik kesinlik ve hesap verebilir güvenlik; komite üyeliği sermayeyle satın
> alınmaz, kazanılır; ve komite önceden kestirilemeyecek biçimde örneklenir.

**Kullanılmayacak cümle:** *"PoS'un enerji verimliliği + PoW'un güvenliği."* Tutmaz.
PoW'un güvenliği dışsal ve sahtelenemez maliyetten gelir, karşılığında olasılıksal
kesinlik verir. Komite BFT'sinin güvenliği kuorum kesişimi ve cezalandırılabilirlikten
gelir, karşılığında deterministik kesinlik verir. İkisi farklı garantiler; biri diğerini
kapsamaz. Aşırı-iddia, ilk ciddi dış denetimde geri teper.

Enerji verimliliği iddiası tartışmasızdır ve ayrıca söylenebilir: blok başına 27 imza.

---

## 2. Tehdit modeli

Kapsam içinde (Serok kararı, 2026-08-28):

| Saldırgan | Yeteneği | Tasarımdaki karşılığı |
|---|---|---|
| **Devlet aktörü** (TR/IR/SY/IQ) | Hedefli DDoS, ISP kesintisi, **validatörün fiziksel tutuklanması**, kurum üzerinde baskı | Komite kestirilemezliği · takma adlı üyelik · ileri-güvenli anahtarlar · coğrafya katmanı · otomatik yeniden örnekleme |
| **Sermaye** | Sınırsız para, kimlik yok | Katmanlaştırma: satın alınabilen katman toplam koltuğun 1/9'u |
| **Sybil** | Bir insan, çok kimlik | Katman içi tabanlar · kimlik/KYC katmanı taşıyıcı varsayım |

**Türetilmiş ve zorunlu hale gelen:** **iç kolüzyon.** Kapsam dışı bırakılamaz, çünkü
bir devlet aktörünün en ucuz hamlesi 27 kişiyi tutuklamak değil, bir kurumu satın almaktır.
Bölüm 5'teki bütçenin tamamı bu senaryo üzerine kuruludur.

**Kapsam dışı:** ağ katmanı kimlik sızıntısı bir protokol açığı değildir; sentry node / Tor /
altyapı dağıtımı ile karşılanır ve **operasyon gereğidir** — kriptografi tek başına yeterli
değildir (Bölüm 13, R7).

---

## 3. Konsensüs parametreleri

| Parametre | Değer | Gerekçe |
|---|---|---|
| Katman sayısı `k` | **9** | Bölüm 5 |
| Katman başına koltuk | **3** | Katmanlar arası eşitlik; hiçbir erk tek başına eşiğe yaklaşamaz |
| Komite `n` | **27** | `k × 3` |
| Kuorum `q` | **19** | GRANDPA'nın yapısal 2/3 eşiği. Değiştirilmez |
| **Durdurma eşiği** | **≥ 9 koltuk** | `n − q + 1` |
| **Çatallama eşiği** | **≥ 11 koltuk** | `2q − n` |

**Çıkan mülkler:**
- Tek bir erk: 3 koltuk → **zararsız**
- İki erk: 6 koltuk → **zararsız**
- Üç erk: 9 koltuk → **durdurabilir, çatallayamaz**
- Dört erk: 12 koltuk → çatallayabilir

**17/21 reddedildi.** Kuorumu 2/3'ün üstüne çıkarmak çatal direncini artırır ama canlılığı
düşürür; devlet aktörü tehdit modelinde beş kaybın zinciri durdurması kabul edilemez.
Ayrıca GRANDPA'nın eşiği yapısaldır — 17/21 formel doğrulanmış bir gadget'ı değiştirmeyi
gerektirirdi. Tehdit modeli ve mühendislik kısıtı aynı yeri gösteriyor.

---

## 4. Dokuz erk

Her katmanın kapısı **farklı bir yetki kaynağına** ait olmak zorundadır. Matematiğin gizli
şartı budur: aynı kurumun kapısını tuttuğu iki katman, hesapta **tek katman** sayılır.

| # | Katman | Kapı | Bağımsızlık gerekçesi |
|---|---|---|---|
| 1 | **Stake** | Bonded HEZ, AH'de Phragmén ile iç sıralama | Piyasa. Tek satın alınabilir katman |
| 2 | **Meclis** | Seçilmiş milletvekilliği | Yasama |
| 3 | **Divan** | Mahkeme üyeliği | Yargı — ⚠️ korelasyon riski, aşağı bkz. |
| 4 | **Perwerde** | W3 University + **Kafkas Üniversitesi** (Tiflis, akredite) | **Yabancı, bölge dışı kurum.** Bölgedeki hiçbir devletin erişemediği tek kapı — yapısal varlık |
| 5 | **Tiki** | Topluluk verilen tikiler | Sosyal graf — ⚠️ makam tikileri hariç tutulmalı |
| 6 | **Welati kurası** | Yalnız vatandaşlık; tüm vatandaşlardan kura | Antik Atina sortition'ı. Kurumu yok, dolayısıyla ele geçirilecek kurumu da yok. Saldırganı Sybil'dir, kolüzyon değil |
| 7 | **Coğrafya / diaspora** | Bölge dışı ikamet tanıklığı | Diğer sekizine dik eksen. **Devlet aktörü tehdidine doğrudan yazılmış** |
| 8 | **Kıdem** | Kesintisiz havuz üyeliği ≥ 120 era (~30 gün), ihlalsiz | Zaman bir kurum değildir. Satın alınamaz, bağışlanamaz, sahtelenemez — yalnız beklenir |
| 9 | **Bağımsız altyapı** | Ölçülmüş çalışma kaydı + ayrık ASN/barındırma tanıklığı | Teknik liyakat; tüm sosyal/politik kapılardan bağımsız |

### İki korelasyon riski — kayda geçirilir, gizlenmez

**(a) Divan, Meclis ve Serok'a bağımlı.** Mahkeme 5 Serok + 6 Meclis atamasıyla kuruluyor
(bkz. kuvvetler ayrılığı kaydı). Bu, 3 numaralı katmanı 2 numaradan tam bağımsız yapmıyor
ve **etkin `k`'yı 9'un altına düşürüyor.** Düzeltmesi anayasaldır, kod değildir: yargı
atamasının bağımsızlaştırılması. Kapanana kadar bütçe hesabı Divan'ı yarım katman saymalıdır.

**(b) Makam tikileri, Tiki katmanını Meclis'e bağlar.** Düzeltmesi kod: **12 makam tikisi
Tiki katmanının uygunluk ölçütünden hariç tutulur.** Yalnız topluluk kaynaklı tikiler sayılır.

---

## 5. Güvenlik bütçesi — ölçülmüş, varsayılmamış

`pezkuwi-tnpos-primitives::security` içinde saf fonksiyon; çok değişkenli hipergeometrik
(katman başına 3 koltuk, 9 katmanın konvolüsyonu). Katman başına 200 uygun üye varsayımı,
4 era/gün. **200, tablonun modelleme varsayımıdır; 50 ise katmanın oturtulması için sert tabandır (aşağı bkz.).**

**Bütçenin dayandığı senaryo: bir erk tamamen ele geçirilmiş, kalan her katmanda saldırganın o katmanın uygun üyelerinin %5'ini tuttuğu varsayılır.**

| Senaryo | P(durdurur)/era | P(çatallar)/era | Çatal aralığı |
|---|---|---|---|
| Sybil %2, her katmanda | 8.7e-10 | 6.5e-13 | ~10⁹ yıl |
| Sybil %5, her katmanda | 3.2e-06 | 2.1e-08 | 32.600 yıl |
| Sybil %10, her katmanda | 8.1e-04 | 2.5e-05 | 28 yıl |
| Sybil %20, her katmanda | 7.3e-02 | 1.1e-02 | **< 1 yıl** |
| **1 erk TAM + %5** | **8.8e-04** | **1.2e-05** | **60 yıl** |
| 1 erk TAM + %10 | 2.7e-02 | 1.6e-03 | **< 1 yıl** |
| 2 erk TAM + %5 | 8.4e-02 | 3.0e-03 | **< 1 yıl** |
| 3 erk TAM | 1.00 | 0 | durdurur, çatallayamaz |

**Okunuşu:** tasarım Sybil'e karşı çok geniş marjlı; baskın risk kurumsal ele geçirmedir.
Katmansız (basit rastgele) örneklemede aynı komite, saldırgan havuzun %20'sini tutuyorsa
69 era'da bir çatallardı — katmanlaştırmanın satın aldığı şey budur.

**Durdurma ile çatallama simetrik değildir:** durdurma otomatik yeniden örneklemeyle
kurtarılabilir, çatallama kurtarılamaz. Bu yüzden Faz 2'deki otomatik yeniden örnekleme
bir iyileştirme değil, **güvenlik bütçesinin bileşenidir.**

### Katman içi taban

`N` uygun üyeden 3 çekildiğinde saldırganın `a` üyesiyle o katmanın üçünü de alma olasılığı
`C(a,3)/C(N,3)`:

| N | a=3 | a=5 | a=10 |
|---|---|---|---|
| 20 | 8.8e-04 | 8.8e-03 | 1.1e-01 |
| 50 | 5.1e-05 | 5.1e-04 | 6.1e-03 |
| 100 | 6.2e-06 | 6.2e-05 | 7.4e-04 |
| 200 | 7.6e-07 | 7.6e-06 | 9.1e-05 |

**Asgari uygun üye sayısı katman başına 50'dir**; altında katman oturtulmaz (Bölüm 7.1).

### İspat bir belge değil, bir runtime kısıtıdır

> Pallet, güvenlik bütçesini aşan bir yapılandırmayla **era açmayı reddeder.**

`integrity_test` (derleme zamanı) ve `try_state` (çalışma zamanı) ile zorlanır. AH
`staking.rs`'teki "anayasa koddur, politika depodadır" deseninin güvenlik bütçesine
uygulanması. Bir katman kotasını güvenle taşıyamıyorsa **oturtulmaz** — sessizce güvensiz
hale gelmez.

---

## 6. Mimari

### 6.1 Crate sınırları

| Crate | Sorumluluk | Neden ayrı |
|---|---|---|
| `pezkuwi-tnpos-primitives` | Tipler, skor trait'leri, güvenlik matematiği **saf fonksiyon olarak** | Runtime'sız test edilir. **P-1'i kapatır**: skor trait'leri bugün dört pallet'te bayt bayt kopyalanmış |
| `pezpallet-tnpos` | Havuz, uygunluk, 9 katman, kotalar, skor önbelleği, komite teslimi, slashing | Çekirdek |
| `pezpallet-tnpos-sortition` | Ring-VRF bilet implementasyonu | **Kritik sınır:** `Sortition` trait'inin arkasında. Faz 1 basit implementasyonla çıkar, Faz 2'de ring-VRF çekirdeğe dokunmadan takılır. `ark-vrf` ağırlığı yalnız buraya girer |
| `pezpallet-tnpos-people-client` | People chain'de skorları paketleyip XCM ile relay'e yollar | `staking-async-rc-client` deseninin aynısı — repoda kanıtlanmış |

`Sortition` trait sınırı bu tasarımın en önemli mühendislik kararıdır: fazlama onun
sayesinde yeniden yazma gerektirmez.

### 6.2 Nerede yaşar

- **Relay:** havuz, uygunluk, katmanlar, örnekleme, komite teslimi (`SessionManager`)
- **AssetHub:** stake katmanının **iç** sıralaması — mevcut `staking-async` + `MultiBlockElection`.
  Nominatör, exposure, slashing, ödül makinesine **dokunulmaz**
- **People:** kimlik ve kredi kaynakları (trust, tiki, perwerde, referral, staking-score)

### 6.3 Üç halka

1. **Uygunluk halkası.** Havuza giren her üye bir bandersnatch anahtarı kaydeder; havuz bir
   *ring* oluşturur (≤1024; `MaxPoolSize` 1000 ile uyumlu). Ring açıktır.
2. **Bilet penceresi.** Era'nın ilk yarısında uygun üyeler bir sonraki era için ring-VRF
   bileti üretir ve **başka bir üye üzerinden anonim röleyle** zincire gönderir. Bilet
   *"bu ring'in üyesiyim ve VRF çıktım şu"* der; hangi üye olduğunu söylemez. Katman bilet
   gövdesinde açık, kimlik değil.
3. **Örnekleme.** Era sınırında her katmanın biletleri VRF çıktısına göre sıralanır, 3'ü
   alınır. VRF çıktısı ne seçilebilir ne önceden hesaplanabilir — eğilemezlik ve
   kestirilemezlik buradan gelir, ayrı beacon gerekmez.

**Elde edilen mülk:** komite üyesi, kimliğine hiç bağlanmamış **taze bir session key** ile
ortaya çıkar. Zincir "27 meşru havuz üyesi" olduğunu ispatlı bilir, hangi 27 kişi olduğunu
bilmez — ne önceden ne sonradan. Hedef listesi çıkarılamaz, çünkü liste yoktur.

### 6.4 Ölçülmüş kısıtlar

- Ring-VRF ispat doğrulaması **~11 ms/bilet**; 27 üye = ~300 ms. Bir era'ya yayıldığında
  ihmal edilebilir, tek blokta toplandığında blok bütçesini aşar → **gönderim penceresi zorunludur**
- Ring doğrulayıcı anahtarını yeniden kurmak **~50 ms** (domain 1024) — era başına bir kez
- `bandersnatch-experimental` bayrağı **hiçbir runtime'da açık değil**; açılması gerekir
- `sc-consensus-sassafras` **yok** (ne bizde ne upstream'de). BABE kalır. Ring-VRF yalnız
  runtime içinde, komite seçimi için kullanılır — **node tarafında sıfır değişiklik**

---

## 7. Bozulma ve kurtarma

### 7.1 Katman küçülürse

- ❌ **Koltuk katmanlar arası transfer edilmez.** Anayasal değişmez — tam da savunduğumuz
  yoğunlaşmayı kendi elimizle yapmak olur
- ❌ Zinciri durdurmak: bir devlet zinciri için kabul edilemez
- ✅ **Komite küçülür**, `q` gerçek boyutun 2/3'ü olarak yeniden hesaplanır, güvenlik kısıtı
  **küçülmüş yapılandırma için yeniden çalıştırılır**. Sert taban (**≥ 15 üye / ≥ 5 katman**)
  altında genesis'te tanımlı acil durum kümesi oturur ve yönetişim alarmı çalar

### 7.2 Canlılık kurtarma

İlke: *kurtarma, koruduğu şeye bağlı olmasın.* İki ayrı durum, karıştırılmaz:

- **Finality durursa (GRANDPA):** BABE blok üretmeye devam eder, runtime koşar → finality
  N blok geride kalırsa **runtime taze tohumla komiteyi yeniden örnekler.** Otomatik,
  yönetişim döngüde değil
- **Blok üretimi durursa (9+ üye çevrimdışı):** hiçbir şey koşmaz, zincir üstü kurtarma
  imkânsızdır. **Zincir-dışı prosedür:** genesis'te tanımlı yedek küme + yönetişim yolu,
  **tatbikatı yapılmış**. Keşfedilerek değil, yazılarak çözülür

---

## 8. Skorlar sınırı geçerken

**(a) Bayat skor eski değeriyle kullanılamaz.** Her önbelleklenmiş skor `last_updated`
taşır; 4 era'dan (~1 gün) eski skor **süresi dolmuş** sayılır ve son değeriyle değil, **uygunsuz**
olarak işlenir — fail-closed. Sessiz duran bir aboneliğin bedeli daha önce ölçüldü; burada
konsensüs ele geçirtir.

**(b) `staking_score` oracle'ı bugün bir bottur ve TNPoS onu konsensüs-kritik yapar.**
Staking verisi relay→People'a kriptografik ispatla değil, noter/bot ile geçiyor.
**Faz 0'ın bloke edici kalemi:** XCM/ispat yoluna ya da M-of-N tanıklığa taşınmadan Faz 1
başlamaz.

---

## 9. Slashing — anonim üye nasıl cezalandırılır

**Stake katmanı:** mevcut `staking-async` slashing'i, değişmez.

**Diğer sekiz katman — iki katmanlı ceza:**
1. **Küçük katılım bondu.** Spam/DoS maliyetini karşılar; engel oluşturacak kadar büyük
   olamaz, yoksa sermaye kapısı arka kapıdan geri gelir
2. **Trust'ın kendisi slashable varlıktır.** İhlal → trust cezası + havuz yasağı (hafif 24 era, ağır 360 era) +
   ağır durumda makam/tiki iptali. Standingi varlığı olan biri için paradan caydırıcı, ve
   zincirin felsefesiyle tutarlı olan ceza budur

**Anonim üyenin bondu (Faz 3):** bond kimliğe değil, **bilete ait bir taahhüde** kilitlenir;
nullifier'a bağlı anonim emanet. İhlal ispatlanırsa emanet yakılır — üye **hiç teşhis
edilmeden** parasını kaybeder. Mekanizma Semaphore'un nullifier'ı; Ethereum'da üretimde.
*Anonimlik cezasızlık değildir.*

---

## 10. Silinen — `validator-pool`'dan taşınmayanlar

| Silinen | Neden |
|---|---|
| **Shadow mode'un tamamı** | Hiç çalışmadı: `SessionManager` implementasyonu hiçbir runtime'a bağlı değildi, `new_session` bir kez bile çağrılmadı. Metrikleri uydurma (`tnpos_total_stake: 0` sabit, `project_tnpos_blocks` model uydurması). Yerine geçen: Zagros'ta gerçek dağıtım + `try-runtime` |
| Beyan edilen stake kontrolü | `min_stake` çağıranın verdiği argümandı; gerçek bakiye hiç okunmuyordu. Sıfır maliyetli konsensüs ele geçirme |
| Hash sırasına göre seçim | `PoolMembers::iter()` ile ilk gelenler alınıp *sonra* karıştırılıyordu; trust ve stake sıralamada hiç kullanılmıyordu |
| Geçmişe dayalı rotasyon kuralı | VRF örnekleme rotasyonu bedava ve eğilemez biçimde verir. Eski kural havuzu kilitleyip `on_initialize`'ı her blok tekrar deneyen sessiz döngüye sokuyordu |
| `reputation ≥ 70` kapısı | Yerine katman uygunluğu + offence kaydı |

---

## 11. Doğrulama planı

1. **Property test** (proptest) — saf matematik üzerinde, rastgele katman yapılandırmalarında
2. **Monte Carlo** — 10⁶ era, çeşitli saldırgan modelleri; ampirik ele geçirme oranı analitik
   sınırla karşılaştırılır. Uyuşmazlarsa **model yanlıştır**
3. **Formel spesifikasyon** (Quint veya TLA+), makine-kontrollü değişmezler:
   koltuk katmanlar arası transfer edilmez · komite taban altına düşmez · süresi dolmuş
   skorla era açılmaz · kurtarma daima sonlanır
4. **Zagros'ta ≥ 3 ay**, güvenlik kısıtı canlı
5. **Ücretli dış denetim:** ring-VRF entegrasyonu **ve** kimlik/KYC katmanı
6. **Benchmark'lar CI runner'larında** ölçülür, WSL'de değil

---

## 12. Fazlar

| Faz | İçerik | Kabul ölçütü |
|---|---|---|
| **0 — ön koşul, bloke edici** | `staking_score` oracle'ının kriptografik yola taşınması · People→Relay skor XCM kanalı · genesis reset takvimine hizalama | Kanal canlı, oracle bot değil |
| **1 — çekirdek** | primitives + havuz + 9 katman + kotalar + güvenlik kısıtı + bozulma + `SessionManager` teslimi + katman-özel slashing + basit eğilemez tohum (commit-reveal). Ring-VRF **yok** | Zagros'a dağıtıldı, `try_state` yeşil |
| **2 — sortition sertleştirme** | `Sortition` trait'ine ring-VRF · **gerçek SRS** (bugün `new_testing()`) · era içi alt-turlar · otomatik finality kurtarma | Komite kestirilemez; kurtarma tatbikatı geçti |
| **3 — anonimlik** | Nullifier'a bağlı anonim emanet bond · takma adlı havuz üyeliği · **ileri-güvenli geçici katılım anahtarları** (Algorand'dan alınan ders: ele geçirilen anahtar geçmişi yeniden imzalayamaz) | Devlet aktörü tehdit modeli karşılandı |
| **4 — Ar-Ge, paralel** | SAFROLE / `sc-consensus-sassafras` portu, **yalnız Zagros** | Mainnet taahhüdü yok |

---

## 13. Riskler ve açık kalemler

| # | Risk | Durum |
|---|---|---|
| R1 | `bandersnatch-experimental` **hiçbir üretim zincirinde koşmuyor**. Kriptografi (bandersnatch, ark-vrf) akademik olarak sağlam; *entegrasyon* sahada değil | Faz 2'nin taşıdığı risk. Ücretli denetim zorunlu |
| R2 | `staking_score` oracle'ı bir bot; TNPoS onu konsensüs-kritik yapıyor | **Faz 0 bloke edici** |
| R3 | **Sybil direnci tüm modelin taşıyıcı varsayımı.** Kimlik katmanı çökerse dokuz katman da çöker | KYC, konsensüs pallet'inden daha sert denetlenmeli |
| R4 | **Kod, hiçbir katmanın 3 koltuğu aşmamasını garanti eder; dokuz erkin gerçekten bağımsız olduğunu garanti edemez** | Anayasa meselesi. Whitepaper'da da böyle yazılmalı |
| R5 | Divan, Meclis+Serok atamasıyla kuruluyor → etkin `k` < 9 | Anayasal düzeltme bekliyor; o zamana kadar Divan yarım katman sayılır |
| R6 | Gerçek SRS edinilmedi; genesis `RingContext::new_testing()` kullanıyor | Faz 2. Ethereum KZG seremonisi transkripti aday — doğrulanmalı |
| R7 | Ağ katmanı (IP) kimlik sızdırabilir; kriptografi tek başına yetmez | Sentry/Tor altyapı politikası tasarımın parçası sayılır |
