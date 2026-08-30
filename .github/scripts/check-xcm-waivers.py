#!/usr/bin/env python3
"""Twins must waive the same locations, or say in writing why they do not.

Two lists decide who may talk to a chain for free. `AllowExplicitUnpaidExecutionFrom` in the
barrier decides whether a message runs at all; `WaivedLocations` decides whether it is charged.
They are not the same list and they fail differently: a missing waiver costs a fee, a missing
barrier entry turns the message away before any of it executes.

That second failure has no symptom on either side. It happened here: `bridge-hub-zagros` lost
the Asset Hub from its barrier while still waiving its fees, so the Snowbridge export path was
dead on Zagros and the money side looked correct. Nothing found it for weeks -- the one test
that would have, `unpaid_transfer_token_to_ethereum_should_work`, never ran, because the job
died earlier on a weights check.

`check-twin-runtimes.py` compares pallet index maps and cannot see any of this.

So: for each twin pair, the two lists must match. A difference is allowed only if it is
recorded below with a reason, and the record is checked -- a recorded difference that no longer
exists is reported too, because a stale exemption is how a real one gets waved through.

Usage: check-xcm-waivers.py [--verbose]
Exit 1 on an unrecorded difference, a stale record, or a runtime whose lists cannot be parsed.
"""

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]

# Twin pairs, by the directory that holds each runtime.
TWINS = [
    ("assets/asset-hub-pezkuwichain", "assets/asset-hub-zagros"),
    ("bridge-hubs/bridge-hub-pezkuwichain", "bridge-hubs/bridge-hub-zagros"),
    ("coretime/coretime-pezkuwichain", "coretime/coretime-zagros"),
    ("people/people-pezkuwichain", "people/people-zagros"),
]
BASE = REPO / "pezcumulus/teyrchains/runtimes"

# Differences that are decided rather than accidental. Keyed by (pair, list, side, entry).
# Every one has to say why, and every one is checked for still being real.
RECORDED = {
    ("asset-hub", "barrier", "zagros", "Equals<FellowshipSalaryLocation>"):
        "Fellowship lives on Zagros only; mainnet has no fellowship to pay.",
    ("asset-hub", "barrier", "zagros", "Equals<FellowshipTreasuryLocation>"):
        "Same body, same reason.",
    ("asset-hub", "barrier", "zagros", "Equals<SecretarySalaryLocation>"):
        "Secretary is a Fellowship office; Zagros only.",
    ("bridge-hub", "barrier", "zagros", "Equals<SnowbridgeFrontendLocation>"):
        "Snowbridge's frontend pallet sits on the Zagros side of the bridge.",
    ("bridge-hub", "barrier", "zagros", "Equals<GovernanceLocation>"):
        "Zagros governance addresses this hub directly; on mainnet it arrives as Parent.",
    ("bridge-hub", "barrier", "pezkuwichain", "Equals<SiblingPeople>"):
        "Named People explicitly; the Zagros twin spells the same chain PeopleLocation.",
    ("bridge-hub", "barrier", "zagros", "Equals<PeopleLocation>"):
        "The same chain as the twin's SiblingPeople, under the constants module's name.",
    ("bridge-hub", "barrier", "pezkuwichain", "Equals<AssetHubPezkuwichainLocation>"):
        "The same chain as the twin's AssetHubLocation, under the bridge primitives' name.",
    ("bridge-hub", "barrier", "zagros", "Equals<AssetHubLocation>"):
        "The same chain as the twin's AssetHubPezkuwichainLocation.",
    ("coretime", "barrier", "zagros", "FellowsPlurality"):
        "Fellowship is Zagros-only.",
    ("coretime", "barrier", "zagros", "Equals<GovernanceLocation>"):
        "Zagros governance addresses coretime directly; on mainnet it arrives as Parent.",
    # The same three bodies again, in the fee list rather than the barrier. Recorded twice on
    # purpose: the two lists are separate decisions and a body can plausibly be let through
    # without being waived, or waived without being let through. Sharing one record would hide
    # exactly the mismatch that broke the Zagros bridge hub.
    ("asset-hub", "waived", "zagros", "Equals<FellowshipSalaryLocation>"):
        "Fellowship lives on Zagros only; mainnet has no fellowship to pay.",
    ("asset-hub", "waived", "zagros", "Equals<FellowshipTreasuryLocation>"):
        "Same body, same reason.",
    ("asset-hub", "waived", "zagros", "Equals<SecretarySalaryLocation>"):
        "Secretary is a Fellowship office; Zagros only.",
}

MARKER = "AllowExplicitUnpaidExecutionFrom<"
WAIVED_MARKER = "pub type WaivedLocations = ("


def balanced(text, start, open_ch, close_ch):
    """The body between `start` and its matching close, counting nesting.

    A regex cannot do this: `AllowExplicitUnpaidExecutionFrom<(A, Equals<B>)>` ends at the
    third `>`, and a non-greedy match stops at the first -- which is how the first version
    reported `Equals<PeopleLocation` with the bracket shorn off.
    """
    depth, i = 0, start
    while i < len(text):
        if text[i] == open_ch:
            depth += 1
        elif text[i] == close_ch:
            depth -= 1
            if depth == 0:
                return text[start + 1:i]
        i += 1
    return None


def entries(blob):
    """Split a type-tuple body into entries.

    Comments come out first, before the split. They were stripped afterwards in the first
    version, and a comma inside a comment then cut an entry in half -- the gate reported
    "only pezkuwichain has the" -- and I nearly took that for a finding.
    """
    blob = "\n".join(re.sub(r"//.*$", "", l) for l in blob.splitlines())
    out, depth, cur = [], 0, ""
    for ch in blob:
        if ch in "<(":
            depth += 1
        elif ch in ">)":
            depth -= 1
        if ch == "," and depth == 0:
            out.append(cur)
            cur = ""
        else:
            cur += ch
    out.append(cur)
    return [" ".join(e.split()) for e in out if e.strip()]


def lists_of(rel):
    src = BASE / rel / "src/xcm_config.rs"
    if not src.is_file():
        return None
    t = src.read_text(errors="replace")

    barrier = None
    i = t.find(MARKER)
    if i >= 0:
        body = balanced(t, i + len(MARKER) - 1, "<", ">")
        if body is not None:
            body = body.strip()
            # One entry needs no tuple: `AllowExplicitUnpaidExecutionFrom<ParentOrParents...>`.
            if body.startswith("("):
                body = balanced(body, 0, "(", ")") or body
            barrier = entries(body)

    waived = None
    j = t.find(WAIVED_MARKER)
    if j >= 0:
        body = balanced(t, j + len(WAIVED_MARKER) - 1, "(", ")")
        if body is not None:
            waived = entries(body)

    return {"barrier": barrier, "waived": waived}


def main():
    verbose = "--verbose" in sys.argv
    failed = False
    used = set()

    for a, b in TWINS:
        family = Path(a).name.rsplit("-", 1)[0]
        la, lb = lists_of(a), lists_of(b)
        if la is None or lb is None:
            print(f"  {family}: a runtime is missing; the pair cannot be compared")
            failed = True
            continue

        for kind in ("barrier", "waived"):
            ea, eb = la[kind], lb[kind]
            if ea is None or eb is None:
                print(f"  {family} {kind}: not found in "
                      f"{'pezkuwichain' if ea is None else 'zagros'} -- the parser stopped "
                      f"seeing it, which is not the same as it being absent")
                failed = True
                continue

            for side, mine, theirs in (("pezkuwichain", ea, eb), ("zagros", eb, ea)):
                for e in mine:
                    if e in theirs:
                        continue
                    key = (family, kind, side, e)
                    if key in RECORDED:
                        used.add(key)
                        if verbose:
                            print(f"  ok       {family} {kind} {side}: {e}")
                    else:
                        print(f"  {family} {kind}: only {side} has `{e}`")
                        print(f"           record it with a reason, or add it to the twin")
                        failed = True

    stale = set(RECORDED) - used
    for key in sorted(stale):
        print(f"  recorded difference no longer exists: {key[0]} {key[1]} {key[2]} `{key[3]}`")
        print(f"           remove the record -- a stale exemption waves a real one through")
        failed = True

    if failed:
        print()
        print("Bir ikiz diğerinin waive etmediği bir yeri waive ediyor. Bariyerdeki bir eksik")
        print("mesajı hiç çalıştırmadan geri çevirir ve iki uçta da hiçbir belirti bırakmaz --")
        print("`bridge-hub-zagros` Asset Hub'ı bariyerden düşürmüştü, ücretini waive etmeye")
        print("devam ediyordu, ve Snowbridge ihracat yolu Zagros'ta ölüydü.")
        return 1
    print(f"{len(TWINS)} twin pairs agree on both lists ({len(RECORDED)} recorded differences)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
