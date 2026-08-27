#!/usr/bin/env python3
"""The pre-genesis engineering plan, derived rather than remembered.

A checklist of things somebody thought of is not a plan. This is the cross product: every
subject the chain ships, against every invariant that must hold before genesis. Completeness
comes from construction -- if a pallet exists, it has a row; if an invariant exists, it has a
column -- so a gap cannot hide by never having been written down.

Each cell is one of:
    ok    the invariant holds
    GAP   it does not, and that is work
    n/a   the invariant does not apply to this subject
    ?     cannot be decided by reading the source; needs a person

Why each invariant is here, and what breaks after genesis if it is missing:

  enum-pin   SCALE numbers a fieldless variant by position. Three of these enums are storage
             *keys*. A variant inserted in the middle silently renames every key already
             written. Unfixable after genesis.
  storage-v  Without a declared version the in-code and on-chain numbers are both an implicit
             zero, and a first migration cannot tell "never migrated" from "migrated to 0".
  weight     `WeightInfo = ()` is zero, not unmeasured: the call is free, and free calls are
             a denial-of-service surface.
  no-burn    The supply is fixed and halving. `OnUnbalanced = ()` drops the imbalance, which
             destroys tokens.
  twin       Zagros and Pezkuwichain are one chain at two stages. Something landing in one
             and not the other is a fault in the other, and the passing twin hides it.
  language   Identifiers Kurdish, comments English. Two enums named the same office in two
             languages and nobody noticed for months.
  one-record A fact stored twice can disagree with itself. An office recorded in two places
             can be held by two people depending on who asks.

Order matters in one place only, and it is recorded: renaming a variant is safe *after*
enum-pin and unsafe before, because until the number is pinned it follows the name.

    python3 .github/scripts/plan.py           # the matrix
    python3 .github/scripts/plan.py --gaps    # only what is left
    python3 .github/scripts/plan.py --flows   # the cross-chain paths, gate by gate
    python3 .github/scripts/plan.py --arch    # which chain carries which pallet
    python3 .github/scripts/plan.py --phases  # the sequence, and what each phase must show
"""
import re, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

def read(p):
    try:
        return p.read_text()
    except OSError:
        return ""

# --- subjects, enumerated from the tree rather than listed by hand ---

def pallets():
    d = ROOT / "pezcumulus/teyrchains/pezpallets"
    return sorted(p for p in d.iterdir() if (p / "src/lib.rs").exists())

def runtimes():
    out = []
    for g in ("pezcumulus/teyrchains/runtimes/*/*", "pezkuwi/runtime/*"):
        for p in sorted(ROOT.glob(g)):
            if (p / "src/lib.rs").exists() and "construct_runtime!" in read(p / "src/lib.rs"):
                out.append(p)
    return out

# Upstream's, so ours is measured against upstream rather than against a preference.
UPSTREAM_PALLETS = {"ping", "teyrchain-info", "collective-content"}

TWINS = [("zagros", "pezkuwichain")]

def twin_of(name):
    for a, b in TWINS:
        if name.endswith(a):
            return name[: -len(a)] + b
        if name.endswith(b):
            return name[: -len(b)] + a
    return None

# --- invariants ---

def inv_enum_pin(p, kind):
    src = read(p / "src/lib.rs") + read(p / "src/types.rs")
    enums = re.findall(r"^\s*pub enum (\w+) \{", src, re.M)
    stored = [e for e in enums if re.search(rf"StorageMap<[^>]*\b{e}\b|StorageDoubleMap<[^>]*\b{e}\b", src)]
    if not stored:
        return "n/a", ""
    missing = []
    for e in stored:
        m = re.search(rf"^(\s*)pub enum {e} \{{$", src, re.M)
        if not m:
            continue
        st = m.end()
        cl = re.search(rf"^{m.group(1)}\}}$", src[st:], re.M)
        if not cl or "#[codec(index" not in src[st:st + cl.start()]:
            missing.append(e)
    return ("GAP", ", ".join(missing)) if missing else ("ok", f"{len(stored)} stored")

def inv_storage_v(p, kind):
    if kind != "pallet":
        return "n/a", ""
    if p.name in UPSTREAM_PALLETS:
        return "n/a", "upstream's"
    return ("ok", "") if "STORAGE_VERSION" in read(p / "src/lib.rs") else ("GAP", "")

# Pallets upstream itself leaves at zero, measured against `polkadot-stable2606-1` rather
# than waved through. The consensus three take unsigned equivocation reports, which do not
# pass a weight gate; glutton's sudo is on a chain whose purpose is to burn block space.
UPSTREAM_ZERO_WEIGHT = {"pezpallet_babe", "pezpallet_grandpa", "pezpallet_beefy",
                        "pezpallet_sudo"}

# Runtimes that never carry value: emulated harnesses and load generators.
NOT_SHIPPED = {"test-runtime", "penpal"}


def inv_weight(p, kind):
    if kind != "runtime":
        return "n/a", ""
    if p.name in NOT_SHIPPED:
        return "n/a", "never launched"
    src = read(p / "src/lib.rs")
    cur, hits = None, []
    for ln in src.split("\n"):
        m = re.match(r"impl (pez\w+)::Config(?:<[^>]*>)? for Runtime \{", ln)
        if m:
            cur = m.group(1)
        if ln.strip() == "type WeightInfo = ();" and cur not in UPSTREAM_ZERO_WEIGHT:
            hits.append(cur or "?")
    return ("ok", "") if not hits else ("GAP", f"{len(hits)} at zero: " + ", ".join(hits[:3]))

def inv_no_burn(p, kind):
    """Whether any supply is destroyed, which is not the same as whether a handler is `()`.

    `BurnDestination = ()` only destroys anything when `Burn` is above zero, and the rate is
    the thing that decides. Reading the handler alone reported the fault correctly here but
    for the wrong reason, and would have called `Burn = 0` a gap forever.
    """
    if p.name in NOT_SHIPPED:
        return "n/a", "never launched"
    src = read(p / "src/lib.rs") + read(p / "src/people.rs")
    hits = []

    # Treasury: a rate with nowhere to send it is a leak out of total issuance.
    if "type BurnDestination = ();" in src:
        m = re.search(r"pub const Burn: Permill = ([^;]+);", src)
        rate = m.group(1).strip() if m else "?"
        if "zero()" not in rate and "from_percent(0)" not in rate:
            hits.append(f"treasury burns {rate} to nowhere")

    # Slash handlers: `()` drops the imbalance outright, with no rate to soften it.
    n = len(re.findall(r"type (?:Slash|Slashed|OnSlash) = \(\);", src))
    if n:
        hits.append(f"{n} slash handler(s) drop to nowhere")

    return ("ok", "") if not hits else ("GAP", "; ".join(hits))

def inv_twin(p, kind):
    """Defers to the placement sheet rather than answering the same question differently.

    Two sheets disagreeing about one fact is worse than either being wrong: the placement
    sheet knows which asymmetries are deliberate -- a bridge named after the other chain, an
    entry recorded with a reason -- and this one was calling all of them drift.
    """
    # Upstream ships one variant of these and it is the test network's. Glutton exists to
    # eat block space in load tests and has no business on a live chain; the collectives
    # chain is where the fellowship sits while the network is a testnet. Measured against
    # `polkadot-stable2606-1`, which carries `glutton-westend` and `collectives-westend` and
    # no other variant of either. One variant here too, on our testnet, is the same shape.
    TESTNET_ONLY = {"collectives", "glutton"}

    t = twin_of(p.name)
    if t is None:
        return "n/a", "no twin"
    if any(p.name.startswith(f + "-") for f in TESTNET_ONLY):
        return "n/a", "testnet only, as upstream"
    if not (p.parent / t).exists():
        return "GAP", f"{t} missing"
    table = placement()
    stray = []
    for pal, row in table.items():
        if p.name in row and t not in row and (pal, p.name) not in DELIBERATE:
            mirrored = None
            for a, b in TWINS:
                if a.capitalize() in pal:
                    mirrored = pal.replace(a.capitalize(), b.capitalize())
                elif b.capitalize() in pal:
                    mirrored = pal.replace(b.capitalize(), a.capitalize())
            if mirrored and mirrored in table and t in table[mirrored]:
                continue
            stray.append(pal)
    return ("ok", "") if not stray else ("GAP", ", ".join(sorted(stray)[:4]))

TR_ONLY = "ıİğĞ"
TR_STEMS = ["Advalet", "Adalet", "Denetim", "Teknoloji", "Baskan", "Bakanlik", "Yetki",
            "Karar", "Secim", "Gorev", "Odeme", "Durum", "Kayit", "Onay", "Islem"]
TR_WORDS = re.compile(r"\b(için|olarak|değil|çünkü|olmalı|yetkili|kararları|gerekir|sadece|"
                      r"ancak|zorunlu|Kullanım|yetkisi|üzerinden)\b")
ALLOW = {"Mela", "Noter", "Balyoz", "Bazargan", "Karguzar", "Hesabdar"}

def inv_language(p, kind):
    hits = []
    for f in (p / "src").rglob("*.rs"):
        for i, ln in enumerate(read(f).splitlines(), 1):
            if re.match(r"^\s*(//|///|//!)", ln):
                if TR_WORDS.search(ln):
                    hits.append(f"{f.name}:{i} comment")
                continue
            for name in re.findall(r"\b([A-Z][\w]*)\b", ln.split("//")[0], re.U):
                if name in ALLOW:
                    continue
                if any(c in name for c in TR_ONLY) or any(s in name for s in TR_STEMS):
                    hits.append(f"{f.name}:{i} {name}")
    return ("ok", "") if not hits else ("GAP", f"{len(hits)}: {hits[0]}")

def inv_one_record(p, kind):
    """A fact stored in more than one place. Only checkable where we know the pair."""
    if p.name != "welati":
        return "?", ""
    w = read(p / "src/lib.rs")
    dup = [n for n in ("CurrentOfficials", "AppointedOfficials")
           if f"pub type {n}<T: Config> =" in w]
    return ("ok", "") if not dup else ("GAP", "duplicates tiki::TikiHolder: " + ", ".join(dup))

INVARIANTS = [
    ("enum-pin", inv_enum_pin), ("storage-v", inv_storage_v), ("weight", inv_weight),
    ("no-burn", inv_no_burn), ("twin", inv_twin), ("language", inv_language),
    ("one-record", inv_one_record),
]


# --- the wiring sheet: cross-chain paths, end to end ---
#
# An origin check on the receiving side is the last of three gates, not the only one. The
# barrier runs first and turns away a message it does not recognise; the fee policy runs
# next and charges a sovereign account that may hold nothing. Both halves of a path can be
# correct on their own while the path is dead, and that is not visible from either end.
#
# It happened twice here. The Asset Hub gated the government spend behind
# `EnsureXcm<Equals<PeopleLocation>>` and its barrier did not name People, so the message was
# refused before the origin check was ever reached. Nothing reported it: the sender only
# learns that the router accepted the message.
#
# Every `EnsureXcm<Equals<X>>` in the tree declares a path somebody depends on. This walks
# them and reports the three gates for each.

# What each matcher actually admits. A matcher only counts as covering a location if the
# kinds line up: `ParentOrParentsPlurality` admits the relay, and reading it as coverage for
# a sibling is how this sheet first reported a shut door as open.
COVERING = {
    "AllSiblingSystemTeyrchains": ("sibling", "any sibling system chain"),
    "RelayOrOtherSystemTeyrchains": ("sibling", "relay or sibling system chain"),
    "SystemTeyrchains": ("sibling", "system chains"),
    "IsChildSystemTeyrchain": ("child", "any child system chain"),
    "ParentOrParentsPlurality": ("parent", "the relay and its bodies"),
}


def location_kind_and_aliases(name, runtime):
    """Resolve a location alias to what it *is*, and to the other names for the same thing.

    `WelatiTreasuryChain` and `AssetHubLocation` are the same place under two names, and a
    check written against one does not see a barrier entry written against the other.
    """
    src = "".join(read(runtime / "src" / f) for f in ("lib.rs", "people.rs", "xcm_config.rs"))
    src += read(ROOT / "pezcumulus/teyrchains/runtimes/constants/src/zagros.rs")
    src += read(ROOT / "pezcumulus/teyrchains/runtimes/constants/src/pezkuwichain.rs")
    aliases = {name}
    body = None
    for _ in range(4):  # `A = B::get()` zincirini takip et; aynı yerin iki adı olabiliyor
        m = re.search(rf"pub {name}: Location = ([^;]+);", src)
        if not m:
            break
        body = m.group(1).strip()
        chain = re.fullmatch(r"(\w+)::get\(\)", body)
        if not chain:
            break
        name = chain.group(1)
        aliases.add(name)
    if body is None:
        return "?", aliases
    kind = ("parent" if "Location::parent()" in body or "(1, [])" in body else
            "sibling" if "Location::new(1," in body and "Teyrchain" in body else
            "child" if "Location::new(0," in body and "Teyrchain" in body else "?")
    for other in re.finditer(r"pub (\w+): Location = ([^;]+);", src):
        if other.group(2).strip() == body:
            aliases.add(other.group(1))
    return kind, aliases


def _admits(listing, loc, kind, aliases):
    if any(f"Equals<{a}>" in listing for a in aliases):
        return "ok"
    for matcher, (mkind, label) in COVERING.items():
        if matcher in listing and mkind == kind:
            return f"via {label}"
    return "GAP"

def flows():
    rows = []
    for rt in runtimes():
        lib = read(rt / "src/lib.rs")
        xcm = read(rt / "src/xcm_config.rs")
        for m in re.finditer(r"type (\w+) = EnsureXcm<Equals<(\w+)>>;", lib + read(rt / "src/people.rs")):
            check, loc = m.group(1), m.group(2)

            # 1. converter: OriginKind::Xcm has to become pezpallet_xcm::Origin::Xcm
            conv = "ok" if "XcmPassthrough" in xcm else "GAP"

            kind, aliases = location_kind_and_aliases(loc, rt)

            # 2. barrier: named outright, or covered by a matcher that admits this kind
            bar = re.search(r"AllowExplicitUnpaidExecutionFrom<\(?([^)]*)\)?>", xcm, re.S)
            barrier = _admits(bar.group(1) if bar else "", loc, kind, aliases)

            # 3. fee: the same question of the waiver list
            wv = re.search(r"pub type WaivedLocations =\s*\(?([^;]*)\)?;", xcm, re.S)
            fee = _admits(wv.group(1) if wv else "", loc, kind, aliases)

            rows.append((rt.name, check, f"{loc} ({kind})", conv, barrier, fee))
    return rows


def print_flows():
    rows = flows()
    print(f"{'receiving chain':<24} {'check':<22} {'from':<22} {'converter':<10} "
          f"{'barrier':<28} fee")
    print("-" * 128)
    gaps = 0
    for rt, check, loc, conv, bar, fee in rows:
        if "GAP" in (conv, bar, fee):
            gaps += 1
        print(f"{rt:<24} {check:<22} {loc:<22} {conv:<10} {bar:<28} {fee}")
    print()
    print(f"{len(rows)} declared paths, {gaps} with a gate that turns the message away")
    return gaps



# --- the architecture sheet: which chain carries what, and who owns each fact ---
#
# Placement is a decision that outlives everything else: a pallet's index is written into
# genesis and every offline signer reads it. The sheet is the cross product of pallet against
# chain, taken from `construct_runtime!` rather than from a description of the design, so a
# pallet that quietly appears on a chain it was never meant for has a cell.
#
# Two things it looks for beyond placement:
#   - a pallet on one twin and not the other, which is the twin's bug rather than a choice,
#     unless it is recorded here as deliberate;
#   - the same index meaning two different pallets across chains, which is not itself wrong
#     but is worth seeing, because a reader who learns "62 is Referenda" learns it once.

# Placements that differ on purpose, with the reason. Anything not here that differs is a gap.
DELIBERATE = {
    ("Revive", "asset-hub-zagros"): "Zagros carries contracts; the mainnet hub does not yet",
    ("Sudo", "asset-hub-zagros"): "testnet only",
    ("AhOps", "asset-hub-zagros"): "testnet only",
    ("ToPezkuwichainXcmRouter", "asset-hub-zagros"): "the bridge points the other way",
    ("ToZagrosXcmRouter", "asset-hub-pezkuwichain"): "the bridge points the other way",
    ("EthereumInboundQueueV2", "bridge-hub-zagros"): "Snowbridge v2, testnet only",
    ("EthereumOutboundQueueV2", "bridge-hub-zagros"): "Snowbridge v2, testnet only",
    ("EthereumSystemV2", "bridge-hub-zagros"): "Snowbridge v2, testnet only",
    ("AssetsPrecompilesPermit", "asset-hub-zagros"): "testnet only",
    # Upstream declares these for Rococo's hub alone, and ours mirrors that. Measured
    # against `polkadot-stable2606-1` rather than assumed: they appear in
    # bridge-hub-rococo and in no other hub.
    ("BridgePezkuwiBulletinGrandpa", "bridge-hub-pezkuwichain"): "upstream: Rococo's hub only",
    ("BridgePezkuwiBulletinMessages", "bridge-hub-pezkuwichain"): "upstream: Rococo's hub only",
    ("XcmOverPezkuwiBulletin", "bridge-hub-pezkuwichain"): "upstream: Rococo's hub only",
    ("BridgeRelayersForPermissionlessLanes", "bridge-hub-pezkuwichain"):
        "upstream: Rococo's hub only",
}


def placement():
    """pallet -> {chain: index}"""
    table = {}
    for rt in runtimes():
        # Anchor on the invocation, not the word. The name also appears in the import list
        # and in a `recursion_limit` comment, and matching either takes a body that stops
        # before the macro even starts -- eight runtimes read as empty that way.
        src = read(rt / "src/lib.rs")
        for name, idx in re.findall(r"^\s*(\w+): [a-z]\w+(?:::<\w+>)? = (\d+),", src, re.M):
            table.setdefault(name, {})[rt.name] = int(idx)
    return table


def print_arch():
    table = placement()
    chains = sorted({c for v in table.values() for c in v})
    short = {c: c.replace("asset-hub-", "AH-").replace("bridge-hub-", "BH-")
              .replace("coretime-", "CT-").replace("people-", "PE-")
              .replace("collectives-", "CO-").replace("glutton-", "GL-")[:14] for c in chains}
    w = max(len(p) for p in table) + 2
    print(f"{'pallet':<{w}}" + "".join(f"{short[c]:<16}" for c in chains))
    print("-" * (w + 16 * len(chains)))
    gaps = []
    for pal in sorted(table):
        row = table[pal]
        cells = []
        for c in chains:
            cells.append(str(row[c]) if c in row else "·")
        print(f"{pal:<{w}}" + "".join(f"{x:<16}" for x in cells))
        # twin asymmetry, minus the two shapes that are asymmetric by design:
        #   - a pallet named after the *other* chain, which its twin carries under the
        #     mirrored name: a bridge to Zagros lives on the Pezkuwichain hub and the other
        #     way round, and calling that drift would flag the bridge for existing;
        #   - anything recorded in DELIBERATE with a reason.
        for c in list(row):
            t = twin_of(c)
            if not t or t not in chains or t in row or (pal, c) in DELIBERATE:
                continue
            mirrored = None
            for a, b in TWINS:
                if a.capitalize() in pal:
                    mirrored = pal.replace(a.capitalize(), b.capitalize())
                elif b.capitalize() in pal:
                    mirrored = pal.replace(b.capitalize(), a.capitalize())
            if mirrored and mirrored in table and t in table[mirrored]:
                continue
            gaps.append(f"{pal}: on {c}, not on {t}")
        # one index, two meanings
    idx_use = {}
    for pal, row in table.items():
        for c, i in row.items():
            idx_use.setdefault(i, set()).add(pal)
    reused = {i: p for i, p in idx_use.items() if len(p) > 1}

    print()
    print(f"{len(table)} pallets across {len(chains)} chains")
    for g in sorted(set(gaps)):
        print(f"  GAP  {g}")
    if reused:
        print(f"  note {len(reused)} indices mean different pallets on different chains "
              f"(not wrong, but a reader learns each one twice)")
    return len(set(gaps))



# --- the sequence sheet: phases, what blocks what, and what counts as done ---
#
# The hard part of a phase plan is not the order, it is the acceptance criterion. "Zagros is
# validated" is not a decision anyone can check, and a phase whose exit is a judgement call
# is a phase that ends whenever somebody is tired.
#
# So the criteria are derived from the design rather than invented: the other sheets already
# enumerate what the chain depends on -- eight cross-chain paths, thirteen governance tracks,
# a citizen initiative, a spend that crosses two chains. "Validated" means each of those has
# happened once on a running chain. That number is not a matter of taste; it is however many
# the design declares.
#
# Two of the gates are irreversible and that is why they are gates rather than milestones:
#
#   pre-genesis  Enum indices, storage versions and pallet indices are frozen the moment a
#                chain has a genesis. Everything the static sheet lists has to be closed
#                *before* FAZ 2, not before mainnet -- Zagros is a real chain with real
#                storage keys and a testnet's keys are no easier to renumber.
#   FAZ 3        Mainnet is meant to mirror a validated Zagros. Launching before Zagros has
#                exercised the design means the mirror has nothing to copy.

GENESIS_FILE = "pezkuwi/xcm/src/v5/junction.rs"


def phases():
    src = read(ROOT / GENESIS_FILE)
    zagros_born = "pub const ZAGROS_GENESIS_HASH: [u8; 32] = [0; 32];" not in src
    # `[^;]*` stops inside `[u8; 32]`. The type annotation has a semicolon in it.
    mainnet_hash = re.search(
        r"pub const PEZKUWICHAIN_GENESIS_HASH[\s\S]{0,80}?hex!\[\"(\w+)\"\]", src)
    mainnet = mainnet_hash.group(1)[:12] if mainnet_hash else "?"

    # the static sheet, reused rather than restated
    subjects = [(q, "pallet") for q in pallets()] + [(q, "runtime") for q in runtimes()]
    static_gaps = sum(1 for q, k in subjects for _, fn in INVARIANTS if fn(q, k)[0] == "GAP")
    dead_paths = sum(1 for r in flows() if "GAP" in (r[3], r[4], r[5]))

    # what FAZ 3 has to have exercised, counted from the design
    # FAZ 3 validates Zagros, so it is Zagros's own tracks that have to be exercised --
    # its relay, its Asset Hub and its People chain. Summing both families counted the
    # mainnet twin's as well, which nothing in this phase runs.
    paths = len([r for r in flows() if "zagros" in r[0]])
    tracks = 0
    for rt in runtimes():
        if not rt.name.endswith("zagros"):
            continue
        tracks += len(re.findall(r"pezpallet_referenda::Track \{",
                                 read(rt / "src/governance/tracks.rs")))

    return [
        # Not measurable from the tree: the criterion is a green run on the branch, and a
        # sheet that reports it from here would be reporting a guess.
        ("FAZ 0", "framework sync", "evidence", None,
         "green CI on `framework-sync-stable2606`, then merged"),
        ("pre-genesis", "invariants frozen at genesis", "GATE",
         static_gaps == 0 and dead_paths == 0,
         f"{static_gaps} static gaps, {dead_paths} dead paths — must be 0 BEFORE FAZ 2"),
        ("FAZ 2", "Zagros from genesis", "milestone",
         zagros_born, "ZAGROS_GENESIS_HASH is still [0; 32]" if not zagros_born
         else "genesis hash set"),
        ("FAZ 3", "Zagros validated", "GATE", False,
         f"needs, live and once each: {paths} cross-chain paths, {tracks} governance tracks, "
         f"one citizen initiative start to finish, one spend crossing People to the Asset Hub, "
         f"and nothing done by sudo that governance is meant to do"),
        ("FAZ 4", "mainnet from genesis, mirroring Zagros", "milestone", False,
         f"PEZKUWICHAIN_GENESIS_HASH is {mainnet}… — the old chain's; the reset writes a new one"),
    ]


def print_phases():
    print(f"{'phase':<14} {'kind':<10} {'state':<7} what")
    print("-" * 118)
    for name, what, kind, done, note in phases():
        state = "?" if done is None else ("done" if done else "open")
        print(f"{name:<14} {kind:<10} {state:<7} {what} — {note}")
    print()
    print("blocks:  pre-genesis → FAZ 2 → FAZ 3 → FAZ 4.  A gate skipped is not a delay, it is")
    print("         a decision taken by default: after FAZ 2 the indices are frozen, and after")
    print("         FAZ 4 the mirror has copied whatever Zagros was.")
    return 0


def main():
    if "--phases" in sys.argv:
        return print_phases()
    if "--arch" in sys.argv:
        return 1 if print_arch() else 0
    if "--flows" in sys.argv:
        return 1 if print_flows() else 0
    only_gaps = "--gaps" in sys.argv
    subjects = [(p, "pallet") for p in pallets()] + [(p, "runtime") for p in runtimes()]
    names = [n for n, _ in INVARIANTS]
    w = max(len(p.name) for p, _ in subjects) + 2

    if not only_gaps:
        print(" " * w + "  ".join(f"{n:<10}" for n in names))
        print("-" * (w + 12 * len(names)))
    gaps, unknown = [], []
    for p, kind in subjects:
        cells = []
        for n, fn in INVARIANTS:
            state, note = fn(p, kind)
            cells.append(state)
            if state == "GAP":
                gaps.append((p.name, n, note))
            elif state == "?":
                unknown.append((p.name, n))
        if not only_gaps:
            print(f"{p.name:<{w}}" + "  ".join(f"{c:<10}" for c in cells))

    print()
    print(f"{len(subjects)} subjects x {len(INVARIANTS)} invariants = "
          f"{len(subjects)*len(INVARIANTS)} cells, {len(gaps)} gaps, {len(unknown)} undecidable")
    for name, inv, note in gaps:
        print(f"  GAP  {name:<26} {inv:<11} {note}")
    for name, inv in unknown:
        print(f"  ?    {name:<26} {inv:<11} needs a person")
    return 1 if gaps else 0

if __name__ == "__main__":
    sys.exit(main())
