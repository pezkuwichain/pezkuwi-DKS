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
    (re.compile(r"^pezpallet_tiki::ensure::Ensure(\w+)"), "office"),
    (re.compile(r"^pezpallet_welati::Ensure(\w+)"), "office"),
    (re.compile(r"^Ensure(Serok|Diwan|Parlementer|Wezir)\w*"), "office"),
]
BAD_LEAF = [
    (re.compile(r"EnsureSigned"), "any signed account may write the register"),
    (re.compile(r"pezpallet_custom_origins::"), "a referendum track; check its tally"),
    (re.compile(r"governance::(WelatiElection|WelatiAdmin|CitizenshipAdmin)"),
     "a referendum track; check its tally"),
    (re.compile(r"EnsureXcm"), "another chain; the register is not writable from abroad"),
]

SPLIT = re.compile(r"^(EitherOf|EitherOfDiverse|EnsureOneOf)\s*<(.*)>$", re.S)


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


def classify(leaf):
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
                verdict, why = classify(leaf)
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
