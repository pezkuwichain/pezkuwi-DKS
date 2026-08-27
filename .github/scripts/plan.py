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
    python3 .github/scripts/plan.py --work    # has the agreed design landed; fails on regression
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
    # Both trees. Scanning only the teyrchain one left `pezkuwi/pezpallets/*` unmeasured, so
    # validator-pool sat outside every invariant while the sheet reported no gaps.
    out = []
    for d in (ROOT / "pezcumulus/teyrchains/pezpallets", ROOT / "pezkuwi/pezpallets"):
        if d.is_dir():
            out += [p for p in d.iterdir() if (p / "src/lib.rs").exists()]
    return sorted(out, key=lambda p: p.name)

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
    """Every enum whose bytes reach storage must have every variant pinned.

    Three ways the earlier version reported green while measuring nothing, all found by
    reading it against the tree rather than trusting it:

      - it looked only at `StorageMap`/`StorageDoubleMap` and only where the enum's name
        appeared literally in the type, so a `StorageValue` and anything reached through a
        struct field was invisible. Most of this tree's enums are reached that way.
      - it accepted the enum if `#[codec(index` appeared ANYWHERE in the body, so pinning
        four variants out of eight passed.
      - it read only `lib.rs` and `types.rs`.

    Reachability is transitive: a struct in storage drags in the enums it holds, and those
    drag in theirs. Anything named in a storage declaration is a root; the closure over
    struct and enum fields is what actually gets encoded.
    """
    src = "\n".join(read(f) for f in sorted((p / "src").rglob("*.rs"))
                     if f.name not in ("tests.rs", "mock.rs", "benchmarking.rs"))
    if not src:
        return "n/a", ""

    # A runtime's `Origin` enum never appears in a storage declaration, so the walk below
    # cannot reach it -- but the Scheduler's agenda and every open referendum hold encoded
    # `PalletsOrigin` bytes, which is the same exposure by a different route. Renumbering one
    # would hand a scheduled call to a different body.
    if kind == "runtime":
        gov = p / "src/governance/origins.rs"
        if not gov.exists():
            return "n/a", ""
        g = read(gov)
        m = re.search(r"^(\s*)pub enum Origin \{$", g, re.M)
        if not m:
            return "n/a", ""
        cl = re.search(rf"^{m.group(1)}\}}$", g[m.end():], re.M)
        body = re.sub(r"//[^\n]*", "", g[m.end():m.end() + cl.start()])
        variants = re.findall(rf"^{m.group(1)}\t(\w+)\s*[,({{=]?\s*$", body, re.M)
        pinned = len(re.findall(r"#\[codec\(index", body))
        if variants and pinned < len(variants):
            return "GAP", f"Origin {pinned}/{len(variants)}"
        return ("ok", f"Origin {len(variants)}") if variants else ("n/a", "")

    # Every type body, so we can walk from a storage root down through the fields.
    bodies = {}
    for m in re.finditer(r"^(\s*)pub (?:enum|struct) (\w+)[^\n{]*\{$", src, re.M):
        cl = re.search(rf"^{m.group(1)}\}}$", src[m.end():], re.M)
        if cl:
            # Keep the declaration's own indent: variants sit one level in from it. An anchored
            # count that assumes a fixed depth reads zero for anything nested in the pallet
            # module and then reports it as pinned -- the same blindness this rewrite is fixing.
            bodies[m.group(2)] = (m.group(1), src[m.end():m.end() + cl.start()])
    enums = {m.group(1) for m in re.finditer(r"^\s*pub enum (\w+)[^\n{]*\{$", src, re.M)}
    if not enums:
        return "n/a", ""

    roots = set()
    for decl in re.findall(r"Storage(?:Value|Map|DoubleMap|NMap)<[^;]*?>", src, re.S):
        roots |= {w for w in re.findall(r"\b(\w+)\b", decl) if w in bodies}

    reached, queue = set(), list(roots)
    while queue:
        name = queue.pop()
        if name in reached:
            continue
        reached.add(name)
        queue += [w for w in re.findall(r"\b(\w+)\b", bodies.get(name, ("", ""))[1]) if w in bodies]

    stored = sorted(e for e in reached if e in enums)
    if not stored:
        return "n/a", ""

    missing = []
    for e in stored:
        indent, body = bodies[e]
        # Count variants, not occurrences: partial pinning has to fail. Variants sit exactly
        # one level in from the enum's own indent, so the pattern is derived, not assumed --
        # and trailing `// ...` is stripped first, because twelve of Tiki's variants carry one
        # and a pattern anchored to end-of-line silently counted 44 of 56, which is enough
        # pins to pass. Found by pulling a pin out and watching the gate stay green.
        stripped = re.sub(r"//[^\n]*", "", body)
        variants = re.findall(rf"^{indent}\t(\w+)\s*[,({{=]?\s*$", stripped, re.M)
        pinned = len(re.findall(r"#\[codec\(index", body))
        if variants and pinned < len(variants):
            missing.append(f"{e} {pinned}/{len(variants)}")
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

TR_ONLY = "ıİğĞ"  # tr-ok: the detector's own alphabet
TR_STEMS = ["Advalet", "Adalet", "Denetim", "Teknoloji", "Baskan", "Bakanlik", "Yetki",
            "Karar", "Secim", "Gorev", "Odeme", "Durum", "Kayit", "Onay", "Islem"]
# Turkish in a comment, found by the alphabet rather than by a word list.
#
# A remembered vocabulary only catches what somebody thought of: the list here held thirteen
# words and missed `kullanilyor` sitting in two production runtimes. The two letters below
# are the discriminating ones: Kurdish uses ç, ş, î, ê and û and has neither. The
# apostrophe-suffix is a Turkish construction English does not make.
# Backticked spans are skipped, since a comment may legitimately quote a Kurdish identifier.
TR_COMMENT = re.compile(  # tr-ok: this is the vocabulary, not a use of it
    r"[ıİğĞ]|\b\w+'(?:leri|ları|nin|nın|nun|nün|de|da|ye|ya|yi|yı|si|sı)\b|"  # tr-ok
    r"\b(için|olarak|değil|çünkü|olmalı|yetkili|kararları|gerekir|sadece|"  # tr-ok
    r"ancak|zorunlu|kullanım|yetkisi|üzerinden|bir|bu)\b|"  # tr-ok
    r"(?<!')\bve\b")  # tr-ok: bare `ve`, but not the `'ve` of `you've`
ALLOW = {"Mela", "Noter", "Balyoz", "Bazargan", "Karguzar", "Hesabdar"}

def _tr_comment(ln):
    return bool(TR_COMMENT.search(re.sub(r"`[^`]*`", "", ln)))

def inv_language(p, kind):
    hits = []
    for f in (p / "src").rglob("*.rs"):
        for i, ln in enumerate(read(f).splitlines(), 1):
            if re.match(r"^\s*(//|///|//!)", ln):
                if _tr_comment(ln):
                    hits.append(f"{f.name}:{i} comment")
                continue
            for name in re.findall(r"\b([A-Z][\w]*)\b", ln.split("//")[0], re.U):
                if name in ALLOW:
                    continue
                if any(c in name for c in TR_ONLY) or any(s in name for s in TR_STEMS):
                    hits.append(f"{f.name}:{i} {name}")
    return ("ok", "") if not hits else ("GAP", f"{len(hits)}: {hits[0]}")

def inv_rip_index(p, kind):
    """A pallet index left behind by a removed pallet must never be handed to a new one.

    The tree already records these as `// RIP <Name> <index>` inside `construct_runtime!`, and
    that comment is load-bearing: the index is baked into the composite `RuntimeCall`,
    `RuntimeEvent` and `RuntimeOrigin` encodings, so reusing one makes old bytes decode as the
    new pallet's. It cost this project twelve and a half million HEZ once, in `Balances::Holds`
    entries left by a Staking pallet that used to sit at index 9.

    A comment is not a check. This makes it one.
    """
    if kind != "runtime":
        return "n/a", ""
    # Read declaration lines directly rather than carving out the macro body. The invocation
    # is `construct_runtime!(` with a paren on these runtimes, and a pattern expecting a brace
    # matches nothing and returns "no data" -- which reads as a pass. `placement()` below
    # already learned this; the comment there says so, and it got repeated here anyway.
    src = read(p / "src/lib.rs")
    rip = {}
    for c in re.finditer(r"//\s*RIP\s+(.*)", src):
        for name, idx in re.findall(r"(\w+)\s+(\d+)", c.group(1)):
            rip[int(idx)] = name
    if not rip:
        return "n/a", ""
    live = {int(i): n for n, i in re.findall(r"^\s*(\w+): [a-z]\w+(?:::<\w+>)? = (\d+),", src, re.M)}
    taken = [f"{live[i]} took {rip[i]}'s {i}" for i in sorted(rip) if i in live]
    return ("GAP", ", ".join(taken)) if taken else ("ok", f"{len(rip)} retired")

def inv_term_blind(p, kind):
    """Authority must be read through `tiki::current_holder`, never off the raw map.

    An office can carry a term. `TikiHolder` says who was seated; `current_holder` says who
    still holds it. Read the map directly and an officeholder whose term ran out keeps the
    office until somebody notices and removes them by hand -- which is the failure a term
    exists to prevent, and `tiki`'s own comment above `current_holder` says so.

    Seating and vacating are the exception and must use the raw map, because they act on
    whoever is physically recorded. Those sites say so in a comment on the line above.
    """
    if kind != "pallet" or p.name == "tiki":
        return "n/a", ""
    bad = []
    for f in sorted((p / "src").rglob("*.rs")):
        if f.name in ("tests.rs", "mock.rs", "benchmarking.rs"):
            continue
        lines = read(f).split("\n")
        for i, ln in enumerate(lines):
            if "TikiHolder::<T>::get" not in ln:
                continue
            window = "\n".join(lines[max(0, i - 5):i])
            if "raw map on purpose" not in window:
                bad.append(f"{f.name}:{i + 1}")
    if not bad:
        return ("n/a", "") if not any("TikiHolder" in read(f) for f in (p / "src").rglob("*.rs")) \
            else ("ok", "reads through current_holder")
    return "GAP", f"{len(bad)} raw reads: {bad[0]}"

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
    ("rip-index", inv_rip_index), ("term-blind", inv_term_blind), ("one-record", inv_one_record),
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
    for _ in range(4):  # follow an `A = B::get()` chain; one place can have two names
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



# --- franchise: which electorate decides which subject -----------------------------------
#
# State matters are head-counted on the People chain, one citizen one vote. Economic matters
# are token-weighted on the Asset Hub. The claim written above People's origins is that the
# two lists are disjoint, because "if an origin appeared in both, a holding could reach a state
# power and the register would be for sale."
#
# The runtime test that was supposed to hold that line reads the Asset Hub's own tracks and
# nothing else, so it cannot see the relay -- and the relay is token-weighted too
# (`ConvictionVoting::TallyOf`) and still runs three of the state tracks by name. No single
# runtime's tests can compare three chains; this can.
#
# Recorded as a breach rather than waived: the relay's three are a leftover from before the
# split, six months older than the People ballot box, and what to do about them is Serok's.
# Naming them here keeps the count from growing while that is decided.
KNOWN_FRANCHISE_BREACH = {
    ("pezkuwichain", "welati_election"), ("pezkuwichain", "welati_admin"),
    ("pezkuwichain", "citizenship_admin"),
    ("zagros", "welati_election"), ("zagros", "welati_admin"),
    ("zagros", "citizenship_admin"),
}

def track_names(rt):
    return set(re.findall(r'name: s\("([a-z_]+)"\)', read(rt / "src/governance/tracks.rs")))

def print_franchise():
    state, weighted = {}, {}
    for rt in runtimes():
        names = track_names(rt)
        if not names:
            continue
        (state if rt.name.startswith("people-") else weighted)[rt.name] = names
    subjects = set().union(*state.values()) - {"root"}

    print(f"{'chain':<26}{'state subject':<24}{'verdict'}")
    print("-" * 74)
    breaches = 0
    for chain in sorted(weighted):
        for name in sorted(weighted[chain] & subjects):
            known = (chain, name) in KNOWN_FRANCHISE_BREACH
            print(f"{chain:<26}{name:<24}{'recorded breach' if known else 'NEW BREACH'}")
            breaches += 0 if known else 1
    if not any(weighted[c] & subjects for c in weighted):
        print("(no token-weighted chain runs a state subject)")
    print()
    print(f"{len(subjects)} state subjects, {len(KNOWN_FRANCHISE_BREACH)} recorded breaches, "
          f"{breaches} new")
    return 1 if breaches else 0


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



# --- the work sheet: the placement decisions, and whether they have landed ---
#
# The other sheets ask whether the tree is sound. This one asks whether the design that was
# agreed has actually been built: the state's ballot on the chain that holds the register,
# the economy's on the chain that holds the money, and the paths between them.
#
# `plan-baseline.json` is what makes this a gate rather than a list. It records which of
# these had landed, and one going back exits non-zero. Work not yet started is the backlog
# and does not fail a build; work that came undone does.

BASELINE = Path(__file__).resolve().parent / "plan-baseline.json"

WORK_ITEMS = []


def work(name):
    def deco(fn):
        WORK_ITEMS.append((name, fn))
        return fn
    return deco


def _people():
    return [q for q in runtimes() if q.name.startswith("people-")]


def _asset_hubs():
    return [q for q in runtimes() if q.name.startswith("asset-hub-")]


def _relays():
    return [q for q in runtimes() if q.name in ("pezkuwichain", "zagros")]


@work("democracy retired")
def _():
    gone = all("pezpallet_democracy" not in read(q / "src/lib.rs") for q in _people())
    return gone, "72 and 73 left unused on purpose" if gone else "still wired in"


@work("state ballot")
def _():
    need = ["Referenda: pezpallet_referenda = 62,", "Origins: pezpallet_custom_origins = 63,",
            "Preimage: pezpallet_preimage = 64,"]
    ok = all(all(n in read(q / "src/lib.rs") for n in need) for q in _people())
    tracks = all((q / "src/governance/tracks.rs").exists() for q in _people())
    return ok and tracks, "Referenda 62, Origins 63, Preimage 64, with tracks" if ok and tracks \
        else "incomplete"


@work("citizens vote")
def _():
    w = read(ROOT / "pezcumulus/teyrchains/pezpallets/welati/src/lib.rs")
    surface = "fn answer_referendum" in w and "fn open_initiative" in w
    wired = all("type Polls = Referenda;" in read(q / "src/people.rs") for q in _people())
    return surface and wired, "a head-counted answer and a citizens' initiative" if surface and wired \
        else "half done"


@work("state speaks abroad")
def _():
    ok = all("EnsureXcmOrigin<RuntimeOrigin, GovernanceToPlurality>" in read(q / "src/xcm_config.rs")
             for q in _people())
    return ok, "the three offices speak as their body" if ok else "SendXcmOrigin is still ()"


@work("economic ballot")
def _():
    need = ["Scheduler: pezpallet_scheduler = 74,", "Preimage: pezpallet_preimage = 75,",
            "ConvictionVoting: pezpallet_conviction_voting = 76,",
            "Referenda: pezpallet_referenda = 77,", "Origins: pezpallet_custom_origins = 78,"]
    ok = all(all(n in read(q / "src/lib.rs") for n in need) for q in _asset_hubs())
    return ok, "five pallets, 74 to 78" if ok else "incomplete"


@work("franchises disjoint")
def _():
    ok = all("state_and_economic_origins_do_not_overlap" in read(q / "tests/tests.rs")
             for q in _asset_hubs())
    return ok, "held apart by a test on both hubs" if ok else "nothing holds them apart"


@work("treasury answers to a vote")
def _():
    ok = all("governance::Spender" in read(q / "src/lib.rs") for q in _asset_hubs())
    return ok, "each tier bounded by its track" if ok else "Root only"


@work("house has an origin")
def _():
    ok = all(re.search(r"^pub type RootOrParliament\b", read(q / "src/lib.rs"), re.M)
             for q in _people())
    return ok, "the body, not a member of it" if ok else "nothing stands for the house"


@work("sudo can retire")
def _():
    path = all("StateRegisterAsRoot" in read(q / "src/xcm_config.rs") for q in _relays())
    sudo = any("Sudo: pezpallet_sudo" in read(q / "src/lib.rs") for q in _relays())
    return path, ("a referendum reaches relay Root" + ("; sudo still at 255" if sudo else "; retired")) \
        if path else "retiring sudo would strand the constitution"


@work("one address for governance")
def _():
    seen = set()
    for q in runtimes():
        src = read(q / "src/xcm_config.rs")
        if "GovernanceLocation" not in src:
            continue
        seen.add("relay" if "GovernanceLocation: Location = Location::parent()" in src
                 or "GovernanceLocation: Location = Location::parent()" in
                 read(ROOT / "pezcumulus/teyrchains/runtimes/constants/src/zagros.rs")
                 else "other")
    return seen == {"relay"}, ", ".join(sorted(seen)) or "none"


@work("slashes have an owner")
def _():
    used = sum(read(q / "src/people.rs").count("RelayTreasuryAccount") for q in _people()) \
        - len(_people())
    exists = any("Treasury: pezpallet_treasury = 18," in read(q / "src/lib.rs") for q in _relays())
    if used == 0:
        return True, "nothing points at the relay treasury"
    return exists, (f"{used} targets, and the treasury they pay exists" if exists
                    else f"{used} targets pay a treasury that is gone")


def print_work(record=False):
    import json
    before = json.loads(BASELINE.read_text()) if BASELINE.exists() else {}
    now, regressed, open_ = {}, [], 0
    print(f"{'item':<28} {'state':<7} note")
    print("-" * 110)
    for name, fn in WORK_ITEMS:
        ok, note = fn()
        now[name] = ok
        if not ok:
            open_ += 1
            if before.get(name) is True:
                regressed.append(name)
        print(f"{name:<28} {'done' if ok else 'open':<7} {note}")
    print()
    print(f"{len(now) - open_}/{len(now)} landed")
    if record:
        BASELINE.write_text(json.dumps(now, indent=1, sort_keys=True) + "\n")
        print(f"baseline written: {BASELINE.name}")
        return 0
    if regressed:
        print("\nREGRESSION -- something that had landed is gone:")
        for r in regressed:
            print("  x", r)
        return 1
    return 0


def docs_language():
    """Turkish outside the pallet trees: documentation and the tools themselves.

    The per-subject check reads `<subject>/src/**.rs`, which is every place a pallet can hide
    Turkish and no place else. It never saw `.md`, so a document could say anything; it never
    saw `.py`, so these scripts could -- and did, until somebody read them. A gate whose scope
    is narrower than the rule it enforces reports green about the part it can see.
    """
    hits = []
    roots = [ROOT / "README.md", ROOT / "docs", ROOT / ".github/scripts", ROOT / "CLAUDE.md"]
    files = []
    for r in roots:
        if r.is_file():
            files.append(r)
        elif r.is_dir():
            files += [f for f in r.rglob("*") if f.suffix in (".md", ".py")]
    for f in sorted(files):
        for i, ln in enumerate(read(f).splitlines(), 1):
            # `# tr-ok` marks a line that carries Turkish on purpose -- the detector's own
            # vocabulary is the only such line, and it cannot describe itself in English.
            if "# tr-ok" in ln:
                continue
            if _tr_comment(ln):
                hits.append(f"{f.relative_to(ROOT)}:{i}")
    return hits

def main():
    if "--work" in sys.argv:
        return print_work(record="--record" in sys.argv)
    if "--phases" in sys.argv:
        return print_phases()
    if "--franchise" in sys.argv:
        return print_franchise()
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
    docs = docs_language()
    print(f"{len(subjects)} subjects x {len(INVARIANTS)} invariants = "
          f"{len(subjects)*len(INVARIANTS)} cells, {len(gaps)} gaps, {len(unknown)} undecidable"
          + (f", {len(docs)} Turkish in docs/tools" if docs else ""))
    for h in docs[:5]:
        print(f"  GAP  {'docs/tools':<26} {'language':<11} {h}")
    for name, inv, note in gaps:
        print(f"  GAP  {name:<26} {inv:<11} {note}")
    for name, inv in unknown:
        print(f"  ?    {name:<26} {inv:<11} needs a person")
    return 1 if gaps else 0

if __name__ == "__main__":
    sys.exit(main())
