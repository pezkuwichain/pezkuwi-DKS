#!/usr/bin/env python3
"""No token-weighted ballot may reach the register.

The state has two ballots. On the People chain a referendum is counted by `CitizenTally` --
one citizen, one vote. On the Asset Hub it is counted by holdings, HEZ weighted by conviction.
Which subjects belong to which ballot is the whole separation: if a holding can reach a
register power, the register is for sale.

A sister check already holds the two *track lists* apart by name
(`no_subject_is_decided_by_two_different_electorates`). Names catch the obvious form of the
fault -- the same subject on both lists -- and miss the disguised one: a track under some other
name whose origin a register pallet happens to accept. This one asks the question from the
other end. For every origin-typed binding on a pallet that writes the register, it resolves the
type expression through the runtime's own aliases down to leaves, and classifies each leaf by
what can actually produce it.

It fails closed. A leaf this script does not recognise is reported as a failure, not waved
through: the last gate written here went quietly green for three separate reasons, and the
reason it could was that it treated "I did not match anything" as "there was nothing to match".

Usage: check-register-origins.py [--verbose]
Exit 1 on a violation or an unrecognised leaf.
"""

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]

# The chains that hold the register. Both, because they are twins and a rule enforced on one is
# not enforced at all.
RUNTIMES = [
    REPO / "pezcumulus/teyrchains/runtimes/people/people-pezkuwichain/src",
    REPO / "pezcumulus/teyrchains/runtimes/people/people-zagros/src",
]

# Pallets whose storage *is* the register: who is a person, who holds an office, who sits, and
# what standing they carry. A pallet added here that has no origin bindings costs nothing; one
# left out is a hole, so the list is deliberately wider than the strict definition.
REGISTER_PALLETS = {
    "pezpallet_identity_kyc",
    "pezpallet_referral",
    "pezpallet_tiki",
    "pezpallet_welati",
    "pezpallet_trust",
    "pezpallet_perwerde",
    "pezpallet_staking_score",
}

# Leaf classifications. `ok` means the producer is head-counted, a seated body, or a named
# officeholder; `bad` means a holding or an unauthenticated caller can reach it.
OK_LEAF = [
    (re.compile(r"^EnsureRoot(WithSuccess)?\b"), "root"),
    (re.compile(r"^pezframe_system::EnsureRoot"), "root"),
    (re.compile(r"^pezpallet_collective::Ensure\w+<[^>]*?(\w+Collective)"), "collective"),
    # Single-holder offices only, and which those are is read out of `is_unique_role` below
    # rather than listed here. One person holds the tiki, so "the holder decided" and "the
    # office decided" are the same sentence.
    (re.compile(r"^pezpallet_welati::Ensure(Serok|SerokWeziran)\b"), "single-holder office"),
]
BAD_LEAF = [
    (re.compile(r"EnsureSigned"), "any signed account may write the register"),
    # One member of a body is not the body. Not hypothetical: an alias in this tree was named
    # `RootOrParliament` and accepted a single member of parliament, and its *name* is the only
    # reason anybody caught it. A collective decides by proportion, and `EnsureProportion*` is
    # the origin that says so. If a register power really is meant to rest with any single
    # member, it should say so here with a reason rather than ride on the resemblance.
    (re.compile(r"Ensure(Parlementer|EndameDiwane|Endam)\w*"),
     "one member of a body, standing in for the body"),
    (re.compile(r"pezpallet_custom_origins::"), "a referendum track; check its tally"),
    (re.compile(r"governance::(WelatiElection|WelatiAdmin|CitizenshipAdmin)"),
     "a referendum track; check its tally"),
    (re.compile(r"EnsureXcm"), "another chain; the register is not writable from abroad"),
]

SPLIT = re.compile(r"^(EitherOf|EitherOfDiverse|EnsureOneOf)\s*<(.*)>$", re.S)

TIKI = REPO / "pezcumulus/teyrchains/pezpallets/tiki/src"


def tiki_facts():
    """Which tikis one person holds, and which tiki each `*Role` marker names.

    Read out of the pallet, not listed here. A list in this file would be a second copy of
    `is_unique_role`, and the copy is what goes stale -- the same fault the franchise sentinel
    had before it was moved to read the real track lists.
    """
    lib = (TIKI / "lib.rs").read_text()
    m = re.search(r"pub fn is_unique_role\(tiki: &Tiki\) -> bool \{(.*?)\n\t\t\}", lib, re.S)
    unique = set(re.findall(r"Tiki::(\w+)", m.group(1))) if m else set()

    ens = (TIKI / "ensure.rs").read_text()
    roles = {}
    for rm in re.finditer(r"pub struct (\w*Role);(.*?)crate::Tiki::(\w+)", ens, re.S):
        roles[rm.group(1)] = rm.group(3)

    # The convenience aliases -- `pub type EnsureSerok<T> = EnsureTiki<T, SerokRole>;` -- read
    # from the pallet too. Naming them here instead would be a third copy of the same fact, and
    # the one most likely to be forgotten when a new office gets an alias.
    for am in re.finditer(r"pub type (Ensure\w+)<T> = EnsureTiki<T, (\w*Role)>;", ens):
        if am.group(2) in roles:
            roles[am.group(1)] = roles[am.group(2)]
    return unique, roles


def runtime_roles(text):
    """Role markers a runtime declares for itself.

    They do not all live in the pallet: `EducationMinisterRole` is declared in `people.rs` and
    resolves to `WezireBelaw`. Looking in only one of the two places a thing can be defined is
    how a check reports a gap that is really its own blind spot -- this one did, until this.
    """
    out = {}
    for m in re.finditer(r"pub struct (\w*Role);\s*\nimpl [\w:]*GetTiki for \1\b.*?"
                         r"Tiki::(\w+)", text, re.S):
        out[m.group(1)] = m.group(2)
    return out


UNIQUE_TIKIS, ROLE_TIKI = tiki_facts()


def top_level_split(s):
    """Split a generic argument list on commas that are not inside <> or ()."""
    out, depth, cur = [], 0, ""
    for ch in s:
        if ch in "<(":
            depth += 1
        elif ch in ">)":
            depth -= 1
        if ch == "," and depth == 0:
            out.append(cur.strip())
            cur = ""
        else:
            cur += ch
    if cur.strip():
        out.append(cur.strip())
    return out


def read(src):
    return "".join((src / f).read_text() for f in ("lib.rs", "people.rs") if (src / f).exists())


def aliases(text):
    """`pub type Name = <expr>;` at the top level of the runtime."""
    found = {}
    for m in re.finditer(r"\npub type (\w+)\s*=\s*(.*?);\n", text, re.S):
        found[m.group(1)] = " ".join(m.group(2).split())
    return found


def bindings(text):
    """Origin-typed config items on the register's pallets."""
    out = []
    for m in re.finditer(r"impl\s+([a-z_0-9]+)::Config(?:<[^>]*>)?\s+for\s+Runtime\s*\{(.*?)\n\}",
                         text, re.S):
        pallet, body = m.group(1), m.group(2)
        if pallet not in REGISTER_PALLETS:
            continue
        for om in re.finditer(r"\n\ttype\s+(\w*Origin\w*)\s*=\s*([^;]+);", body):
            out.append((pallet, om.group(1), " ".join(om.group(2).split())))
    return out


def hand_written(text):
    """`pub struct X;` with a hand-rolled `impl EnsureOrigin ... for X`.

    Resolved by reading the body rather than trusting the name. A hand-written converter is
    exactly where an unwanted origin would hide -- `RegisterAuthority` admits Root, but only
    while the court has empty seats, and no alias would have shown that. The leaves are the
    origin types the body mentions; the surrounding logic can only ever narrow them, so taking
    every one it names is the safe direction.
    """
    out = {}
    for m in re.finditer(r"\npub struct (\w+);\s*\nimpl [\w:<>, ]*?EnsureOrigin<[^>]*>\s+for "
                         r"\1\b(.*?)\n\}\n", text, re.S):
        name, body = m.group(1), m.group(2)
        found = re.findall(
            r"(pezpallet_collective::Ensure\w+::?<[^>]*?\w+Collective[^>]*>"
            r"|pezpallet_collective::Ensure\w+<[^>]*?\w+Collective[^>]*>"
            r"|pezframe_system::ensure_root"
            r"|EnsureRoot::<\w+>"
            r"|EnsureRoot<\w+>"
            r"|pezpallet_welati::Ensure\w+"
            r"|pezpallet_tiki::ensure::Ensure\w+"
            r"|EnsureSigned\w*"
            r"|EnsureXcm<[^>]*>"
            r"|pezpallet_custom_origins::\w+)", body)
        out[name] = [f.replace("pezframe_system::ensure_root", "EnsureRoot<AccountId>")
                     .replace("::<", "<") for f in found] or ["<body names no origin>"]
    return out


def leaves(expr, alias, hand, seen=None):
    """Flatten an origin expression to its leaf terms, following the runtime's own aliases."""
    seen = seen or set()
    expr = expr.strip().removeprefix("crate::").strip()
    if expr in hand and expr not in seen:
        out = []
        for sub in hand[expr]:
            out += leaves(sub, alias, hand, seen | {expr})
        return out
    if expr in alias and expr not in seen:
        return leaves(alias[expr], alias, hand, seen | {expr})
    m = SPLIT.match(expr)
    if m:
        out = []
        for part in top_level_split(m.group(2)):
            out += leaves(part, alias, hand, seen)
        return out
    return [expr]


def classify(leaf, local_roles):
    # `EnsureTiki<_, XRole>` -- resolve the marker to its tiki and ask the pallet whether one
    # person holds it. A member of a body standing in for the body is what this catches.
    m = re.search(r"EnsureTiki<[^,]*,\s*(\w*Role)\s*>", leaf)
    if m is None:
        # The convenience aliases, but only when the name really is one -- matching every
        # `EnsureX<..>` here would swallow `EnsureRoot` and `EnsureProportionAtLeast` and turn
        # the whole report into "unknown", which is how a check drowns its own signal.
        a = re.search(r"(?:^|::)(Ensure\w+)<[^>]*>$", leaf)
        if a and (a.group(1) in local_roles or a.group(1) in ROLE_TIKI):
            m = a
    if m:
        role = m.group(1)
        tiki = local_roles.get(role) or ROLE_TIKI.get(role)
        if tiki is None:
            return "UNKNOWN", f"`{role}` names no tiki in the pallet or this runtime"
        if tiki in UNIQUE_TIKIS:
            return "ok", f"single-holder office ({tiki})"
        return "BAD", (f"`{tiki}` is a seat in a body, not an office -- one member is standing "
                       f"in for the body")

    for pat, kind in BAD_LEAF:
        if pat.search(leaf):
            return "BAD", kind
    for pat, kind in OK_LEAF:
        if pat.match(leaf):
            return "ok", kind
    return "UNKNOWN", "not recognised -- classify it rather than widening the pattern"


def tally_is_head_counted(text):
    """The People chain's referenda must count citizens, not balances.

    Checked because every `ok` verdict above rests on it: a collective or an officeholder is a
    safe producer only while the ballot that seats them counts heads. Swap the tally and every
    row in this report becomes wrong at once, without any of them changing.
    """
    m = re.search(r"impl pezpallet_referenda::Config for Runtime \{(.*?)\n\}", text, re.S)
    if not m:
        return None
    t = re.search(r"\n\ttype Tally = ([^;]+);", m.group(1))
    return " ".join(t.group(1).split()) if t else None


def main():
    verbose = "--verbose" in sys.argv
    failed = False

    for src in RUNTIMES:
        name = src.parent.name
        text = read(src)
        alias = aliases(text)
        hand = hand_written(text)
        local_roles = runtime_roles(text)

        tally = tally_is_head_counted(text)
        if tally is None or "CitizenTally" not in tally:
            print(f"  {name}: referenda tally is `{tally}`, not a citizen tally")
            failed = True
        elif verbose:
            print(f"  {name}: tally = {tally}")

        rows = bindings(text)
        if not rows:
            print(f"  {name}: no register origin bindings found -- the parser stopped seeing "
                  f"them, which is not the same as there being none")
            failed = True
            continue

        for pallet, item, expr in rows:
            for leaf in leaves(expr, alias, hand):
                verdict, why = classify(leaf, local_roles)
                if verdict == "ok":
                    if verbose:
                        print(f"  ok       {name} {pallet}::{item} -> {leaf[:60]} ({why})")
                    continue
                print(f"  {verdict:<8} {name} {pallet}::{item}")
                print(f"           {leaf[:90]}")
                print(f"           {why}")
                failed = True

    if failed:
        print()
        print("Sicili yazan bir yetki, jeton ağırlıklı bir sandıktan ya da tanımadığım bir")
        print("yerden geliyor. Tanımadığım bir yaprak da hatadır: sınıflandır, deseni")
        print("genişletme -- bir kapının sessizce yeşile dönmesi tam böyle olur.")
    else:
        print("sicil yetkileri temiz: hiçbiri jeton ağırlıklı bir sandığa dayanmıyor")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
