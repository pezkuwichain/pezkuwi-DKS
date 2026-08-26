#!/usr/bin/env python3
"""Reads the state of the pre-genesis work out of the tree, and fails on regression.

Every line here answers a question by measuring the source rather than by reading a note.
A note describing the tree goes stale the moment the tree moves; this cannot.

`durum-baseline.json` records which items were closed. An item going from closed to open is
a regression and exits non-zero, which is what makes this a gate rather than a report. Items
that were never closed are the backlog and do not fail the build.

    python3 .github/scripts/durum.py            # report, and fail on regression
    python3 .github/scripts/durum.py --record   # take today's state as the baseline
"""
import json, os, re, subprocess, sys
from pathlib import Path

BASELINE = Path(__file__).resolve().parent / "durum-baseline.json"
ROOT = Path(__file__).resolve().parents[2]
PEOPLE = [ROOT / f"pezcumulus/teyrchains/runtimes/people/people-{c}" for c in ("zagros", "pezkuwichain")]
AH = [ROOT / f"pezcumulus/teyrchains/runtimes/assets/asset-hub-{c}" for c in ("zagros", "pezkuwichain")]
RELAY = [ROOT / f"pezkuwi/runtime/{c}" for c in ("pezkuwichain", "zagros")]

def read(p):
    try:
        return p.read_text()
    except OSError:
        return ""

def both(paths, rel, needle):
    """Her iki ikizde de var mı."""
    return all(needle in read(p / rel) for p in paths)

def count_all(paths, rel, needle):
    return sum(read(p / rel).count(needle) for p in paths)

CHECKS = []
def check(sira, ad):
    def deco(fn):
        CHECKS.append((sira, ad, fn))
        return fn
    return deco

@check("Paralel 1", "Democracy 72 + Elections 73 kaldırıldı")
def _():
    gone = all("pezpallet_democracy" not in read(p / "src/lib.rs") for p in PEOPLE)
    return gone, "iki People'da da yok" if gone else "hâlâ bağlı"

@check("Paralel 2", "Relay→Root dönüştürücüsü")
def _():
    ok = all("StateRegisterAsRoot" in read(p / "src/xcm_config.rs") for p in RELAY)
    return ok, "iki relay'de de" if ok else "eksik"

@check("Paralel 3", "AH'de WeightInfo = () kalmadı")
def _():
    n = count_all(AH, "src/lib.rs", "type WeightInfo = ();")
    return n == 0, f"{n} adet duruyor (Treasury/Bounties/ChildBounties/AssetRate)"

@check("Sıralı 1", "Kişi sayımlı sayaç + oy verme yüzeyi")
def _():
    w = read(ROOT / "pezcumulus/teyrchains/pezpallets/welati/src/lib.rs")
    ok = "fn answer_referendum" in w and "CitizenTally" in read(
        ROOT / "pezcumulus/teyrchains/pezpallets/welati/src/types.rs")
    wired = both(PEOPLE, "src/people.rs", "type Polls = Referenda;")
    return ok and wired, "extrinsic + runtime bağlı" if ok and wired else "yarım"

@check("Sıralı 2", "Preimage 64 + SendXcmOrigin + Scheduler 71")
def _():
    pre = both(PEOPLE, "src/lib.rs", "Preimage: pezpallet_preimage = 64,")
    snd = all("EnsureXcmOrigin<RuntimeOrigin, GovernanceToPlurality>" in read(p / "src/xcm_config.rs")
              for p in PEOPLE)
    return pre and snd, f"Preimage={pre} SendXcmOrigin={snd}"

@check("Sıralı 3", "Referenda 62 + Origins 63 + track listesi")
def _():
    r = both(PEOPLE, "src/lib.rs", "Referenda: pezpallet_referenda = 62,")
    o = both(PEOPLE, "src/lib.rs", "Origins: pezpallet_custom_origins = 63,")
    t = all((p / "src/governance/tracks.rs").exists() for p in PEOPLE)
    return r and o and t, f"Referenda={r} Origins={o} tracks={t}"

@check("Sıralı 4", "AH ekonomi kütüğü 74–78")
def _():
    need = ["Scheduler: pezpallet_scheduler = 74,", "Preimage: pezpallet_preimage = 75,",
            "ConvictionVoting: pezpallet_conviction_voting = 76,",
            "Referenda: pezpallet_referenda = 77,", "Origins: pezpallet_custom_origins = 78,"]
    ok = all(both(AH, "src/lib.rs", n) for n in need)
    return ok, "beş pallet iki ikizde de" if ok else "eksik"

@check("Sıralı 5", "Kesişmezlik testi")
def _():
    ok = all("state_and_economic_origins_do_not_overlap" in read(p / "tests/tests.rs") for p in AH)
    return ok, "iki AH testinde de" if ok else "yok"

@check("Sıralı 6a", "AH Treasury::SpendOrigin yönetişime açık")
def _():
    ok = all("governance::Spender" in read(p / "src/lib.rs") for p in AH)
    return ok, "Spender bağlı" if ok else "yalnız Root"

@check("Sıralı 6b", "S2 — hiçbir slash sahipsiz bir hesaba ödemiyor")
def _():
    """Aranan sey "relay hazinesine odeme yok" DEGIL.

    Odemenin yanlis olmasi, hesabin sahipsiz kalmasiyla baslar. Relay hazinesi durdugu
    surece bu hedefler dogrudur; kalem, relay hazinesi emekli edildigi an acilir. Kapinin
    tuttugu sey bu baglanti: ikisi birlikte hareket etmeli, ve hangisinin once gittigi
    onemli.
    """
    used = count_all(PEOPLE, "src/people.rs", "RelayTreasuryAccount") - len(PEOPLE)
    treasury_var = any("Treasury: pezpallet_treasury = 18," in read(p / "src/lib.rs")
                       for p in RELAY)
    if used == 0:
        return True, "hiçbir hedef relay hazinesine bakmıyor"
    ok = treasury_var
    return ok, (f"{used} hedef relay hazinesine ödüyor, hazine yerinde"
                if ok else
                f"{used} hedef ÖDEME YAPIYOR ama relay hazinesi kaldırılmış — sahipsiz hesap")

@check("Sıralı 7", "Yönetişim adresi tutarlı")
def _():
    """Yazilis bicimini degil, COZULEN DEGERI karsilastirir.

    Ilk hali `Location::parent()` yazanlarla constants'tan alanlari ayri sayiyordu; ikisi de
    relay'i gosterdigi halde tutarsiz raporluyordu. Aranan sey adresin kendisi.
    """
    def resolve(src):
        if "GovernanceLocation: Location = Location::parent()" in src:
            return "relay"
        if "locations::GovernanceLocation" in src or "GovernanceLocation;" in src:
            for cons in ("zagros", "pezkuwichain"):
                c = read(ROOT / f"pezcumulus/teyrchains/runtimes/constants/src/{cons}.rs")
                if "GovernanceLocation: Location = Location::parent()" in c:
                    return "relay"
                if "GovernanceLocation = AssetHubLocation" in c:
                    return "asset-hub"
        return None

    vals = {}
    for d in ROOT.glob("pezcumulus/teyrchains/runtimes/*/*/src/xcm_config.rs"):
        src = read(d)
        if "GovernanceLocation" not in src:
            continue
        v = resolve(src)
        vals.setdefault(v or "?", []).append(d.parent.parent.name)
    ok = set(vals) == {"relay"}
    return ok, "; ".join(f"{k}: {len(v)} runtime" for k, v in sorted(vals.items()))

@check("Sıralı 8a", "S1 — relay SendXcmOrigin hâlâ dolu")
def _():
    ok = all("LocalPalletOriginToLocation" in read(p / "src/xcm_config.rs") for p in RELAY)
    return ok, "dolu (emeklilik sırasında () olmamalı)" if ok else "boşalmış — S1 gerçekleşti"

@check("Sıralı 8b", "Sudo emekli edilebilir: yönetişim relay Root'una ulaşıyor")
def _():
    """Olculen sey "sudo dustu mu" DEGIL. Ne zaman dusecegi isletme karari; tehlikeli olan,
    dustugunde ulasilamaz kalacak bir sey birakmak. Relay'in ayricalikli yuzeyinin tamami
    `EnsureRoot` ve buraya tek yol sudoydu. `StateRegisterAsRoot` ikinci yolu aciyor:
    kutugun referandumu relay Root'u olur. Kapinin tuttugu sart bu.
    """
    yol = all("StateRegisterAsRoot" in read(p / "src/xcm_config.rs") for p in RELAY)
    sudo = any("Sudo: pezpallet_sudo" in read(p / "src/lib.rs") for p in RELAY)
    return yol, ("yönetişim yolu açık" + (", sudo hâlâ duruyor (255)" if sudo else ", sudo emekli")
                 if yol else "yönetişimin relay Root'una yolu YOK — sudo düşerse anayasa ulaşılamaz")

@check("S3", "Hazine hesabı türeten sabit, var olan bir hazineye çözülüyor")
def _():
    """Plan bunu "TREASURY_PALLET_ID = 18" diye yazmisti; olculdu, indeks degil PalletId
    (`py/trsry`). Yani risk indeks kaymasi degil, S2'nin ayni baglantisi: bu sabitten hesap
    tureten her sey relay hazinesinin varligina bagli. Tuketici saymak bir sey olcmuyordu.
    """
    treasury_var = any("Treasury: pezpallet_treasury = 18," in read(p / "src/lib.rs")
                       for p in RELAY)
    out = subprocess.run(["grep", "-rl", "TREASURY_PALLET_ID", "--include=*.rs", "."],
                         cwd=ROOT, capture_output=True, text=True).stdout
    n = len([f for f in out.splitlines() if "/target/" not in f])
    return treasury_var or n == 0, (
        f"{n} dosya türetiyor, relay hazinesi yerinde" if treasury_var
        else f"{n} dosya hesap türetiyor ama relay hazinesi YOK")

# --- Genesis öncesi temel sağlamlık (res/plans/temel-saglamlik-genesis-oncesi.md) ---

STORED_ENUMS = [
    ("Tiki", "pezcumulus/teyrchains/pezpallets/tiki/src/lib.rs"),
    ("RoleAssignmentType", "pezcumulus/teyrchains/pezpallets/tiki/src/lib.rs"),
    ("ElectionType", "pezcumulus/teyrchains/pezpallets/welati/src/types.rs"),
    ("OfficialRole", "pezcumulus/teyrchains/pezpallets/welati/src/types.rs"),
    ("GovernmentPosition", "pezcumulus/teyrchains/pezpallets/welati/src/types.rs"),
    ("StakingSource", "pezcumulus/teyrchains/pezpallets/staking-score/src/lib.rs"),
    ("KycLevel", "pezcumulus/teyrchains/pezpallets/identity-kyc/src/types.rs"),
    ("EpochState", "pezcumulus/teyrchains/pezpallets/pez-rewards/src/lib.rs"),
]

@check("Temel 1", "Depolanan enum'ların varyant sırası sabit")
def _():
    eksik = []
    for name, rel in STORED_ENUMS:
        s = read(ROOT / rel)
        m = re.search(rf"^(\s*)pub enum {name} \{{$", s, re.M)
        if not m:
            eksik.append(f"{name}(bulunamadı)"); continue
        indent = m.group(1); start = m.end()
        close = re.search(rf"^{indent}\}}$", s[start:], re.M)
        body = s[start:start + close.start()]
        idx = re.findall(r"#\[codec\(index = (\d+)\)\]", body)
        # her varyantin bir indeksi olmali: buyuk harfle baslayan satir sayisi kadar
        # sondaki yorumu da kabul et: `Piştrastkar,        // KYC verifier`
        var = re.findall(r"^\s*[^\W\d_][\w]*\s*(?:=\s*\d+\s*)?,\s*(?://.*)?$", body, re.M | re.U)
        if len(idx) != len(var) or len(idx) == 0:
            eksik.append(f"{name}({len(idx)}/{len(var)})")
    return not eksik, "8 enum, hepsi işaretli" if not eksik else "eksik: " + ", ".join(eksik)

@check("Temel 2", "Özgün pallet'lerde StorageVersion")
def _():
    UPSTREAM = {"ping", "teyrchain-info"}  # upstream'de de yok, eklemek sapma olur
    eksik = []
    for d in sorted((ROOT / "pezcumulus/teyrchains/pezpallets").iterdir()):
        if not (d / "src/lib.rs").exists() or d.name in UPSTREAM:
            continue
        if "STORAGE_VERSION" not in read(d / "src/lib.rs"):
            eksik.append(d.name)
    return not eksik, "hepsinde var" if not eksik else "eksik: " + ", ".join(eksik)

@check("Temel 3", "Bir makamın sahibini söyleyen tek bir sicil var")
def _():
    """Plan bunu "dort taksonomi" diye yazmisti. Olculdu: mesele enum sayisi degil, KAYIT
    sayisi. `tiki::TikiHolder` zaten `Map<Tiki, AccountId>` -- sicil var. `welati`'nin
    `CurrentOfficials` ve `AppointedOfficials` haritalari ayni seyin kopyasi, ayni makam
    listesinin iki alt kumesiyle anahtarlanmis. Uc sicil birbiriyle celisebilir; tiki
    pallet'inin kendi yorumu bunu soyluyor: iki yerde kayitli bir makam, kimin sorduguna
    gore iki farkli kiside olabilir.
    """
    w = read(ROOT / "pezcumulus/teyrchains/pezpallets/welati/src/lib.rs")
    kopya = [n for n in ("CurrentOfficials", "AppointedOfficials")
             if f"pub type {n}<T: Config> =" in w]
    return not kopya, ("tek sicil: tiki::TikiHolder" if not kopya
                       else f"{len(kopya)+1} sicil — welati'de {', '.join(kopya)} hâlâ ayrı")

@check("Temel 4", "\"Meclis karar verdi\" origin'i var")
def _():
    # Yorumda anilmasi sayilmaz: silinen origin'in yerinde neden silindigini anlatan bir
    # yorum duruyor. Aranan sey bir TIP bildirimi.
    # `any` degil `all`: bir ikizde durup otekinde olmamasi tam da aranan kusur.
    ok = all(re.search(r"^pub type RootOrParliament\b", read(p / "src/lib.rs"), re.M)
             for p in PEOPLE)
    return ok, "tip bildirimi var" if ok else "yok — heyet olarak Meclis'i temsil eden origin yok"



ROOTS = [ROOT / "pezcumulus/teyrchains/pezpallets",
         ROOT / "pezcumulus/teyrchains/runtimes",
         ROOT / "pezkuwi/runtime"]

TR_ONLY = "ıİğĞ"

# ASCII ile yazilmis Turkce kokler. Yalniz tanimlayici icinde aranir.
TR_STEMS = [
    # "Meclis" YOK: Kurtcede de kullaniliyor (Serokî Meclisê). Turkceye ozgu olan "Baskan".
    "Advalet", "Adalet", "Denetim", "Teknoloji", "Baskan", "Bakanlik",
    "Kurul", "Yetki", "Karar", "Secim", "Gorev", "Odeme", "Hesap", "Deger",
    "Durum", "Kayit", "Belge", "Onay", "Talep", "Rapor", "Islem", "Yonetim",
    "Vatandas", "Oylama", "Uyelik", "Baslangic", "Sonuc", "Ayar",
]
# Kurtcede/Ingilizcede de gecen, yanlis pozitif uretenler
ALLOW = {"Mela", "Noter", "Balyoz", "Bazargan", "Karguzar", "Hesabdar"}

IDENT = re.compile(r"\b([A-Z][\w]*)\b", re.UNICODE)
COMMENT = re.compile(r"^\s*(//|///|//!)")
TR_WORDS = re.compile(
    r"\b(için|olarak|değil|çünkü|olmalı|yapılır|edilir|yetkili|kullanılır|kararları|"
    r"devri|sahibi|gerekir|sadece|ancak|böylece|tanımları|işlemleri|geçerli|zorunlu|"
    r"Kullanım|yetkisi|VEYA|aşaması|sonrası|çoğunluk|üzerinden)\b")

def _dil_tara():
    ident_hits, comment_hits = [], []
    for root in ROOTS:
        for f in root.rglob("*.rs"):
            if "/target/" in str(f):
                continue
            try:
                lines = f.read_text().splitlines()
            except OSError:
                continue
            for i, ln in enumerate(lines, 1):
                if COMMENT.match(ln):
                    if TR_WORDS.search(ln):
                        comment_hits.append(f"{f}:{i}")
                    continue
                for name in IDENT.findall(ln.split("//")[0]):
                    if name in ALLOW:
                        continue
                    if any(c in name for c in TR_ONLY):
                        ident_hits.append(f"{f}:{i}: {name}")
                    elif any(s in name for s in TR_STEMS):
                        ident_hits.append(f"{f}:{i}: {name}")
    return ident_hits, comment_hits


@check("Dil", "Tanımlayıcılar Kürtçe, yorumlar İngilizce")
def _():
    ident, comment = _dil_tara()
    ok = not ident and not comment
    return ok, "temiz" if ok else f"{len(ident)} tanımlayıcı, {len(comment)} yorum"


def main():
    kaydet = "--record" in sys.argv
    onceki = json.loads(BASELINE.read_text()) if BASELINE.exists() else {}

    print(f"{'kalem':<12} {'durum':<6} açıklama")
    print("-" * 78)
    simdi, gerileme, acik = {}, [], 0
    for sira, ad, fn in CHECKS:
        ok, note = fn()
        simdi[sira] = ok
        if not ok:
            acik += 1
            if onceki.get(sira) is True:
                gerileme.append(f"{sira} — {ad}")
        print(f"{sira:<12} {'✅' if ok else '❌':<6} {ad} — {note}")
    print("-" * 78)
    print(f"kapalı {len(simdi)-acik}/{len(simdi)}, açık {acik}")

    if kaydet:
        BASELINE.write_text(json.dumps(simdi, ensure_ascii=False, indent=1, sort_keys=True) + "\n")
        print(f"baseline written: {BASELINE}")
        return 0

    if gerileme:
        print("\nGERİLEME — kapanmış bir kalem yeniden açıldı:")
        for g in gerileme:
            print("  ✗", g)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
