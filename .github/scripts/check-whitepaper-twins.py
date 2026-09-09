#!/usr/bin/env python3
"""`whitepaper.md` and `whitepaper.html` are published side by side and nothing compared them.

Both are hand-maintained, both are served from `docs/whitepaper/`, and every publishing
surface now feeds from one or the other: the landing page builds its table of contents from
the markdown, the canonical page serves the HTML, the wiki fetches the markdown. So a heading
that moves in one document and not the other is not a cosmetic difference -- it is two of our
own sites describing the same institution by two different names, which is precisely what an
outside reviewer reads as carelessness.

The first run of this check found exactly that: §9 was "The four funds" in the markdown and
"The Vaults" in the HTML, and the runtimes call it neither -- 194 occurrences of `funds` and
none of `vault`. The heading was corrected to follow the code.

What it compares is the numbered spine, not the prose. The HTML is a designed document with
figures the markdown cannot carry, so their word counts will never match; their section
numbers and titles must. Case, dashes and HTML entities are normalised away -- the HTML is
title-case by design and that is not drift.

Usage: check-whitepaper-twins.py [--verbose]
"""

import html
import re
import sys
from pathlib import Path

DOCS = Path(__file__).resolve().parents[2] / "docs" / "whitepaper"

# Sections that exist in one document on purpose. Each names why, because a section present
# in only one place is the default failure of this check and silence here would restore the
# blind spot it was written to close.
ONLY_IN = {
    # The narrative opening is written twice for two audiences: the markdown leads with a
    # dense abstract for readers arriving from the repository, the HTML with a longer essay
    # for readers arriving from the web. Serok has the call on whether they converge; until
    # then neither is drift.
    "abstract": "md",
    "our philosophy": "html",
    # A link list for a reader who cannot click the surrounding page. Meaningless in the
    # markdown, which is itself served from the repository the list points at.
    "resources": "html",
}


def norm(s):
    """Title case, em-dash spelling and entity encoding are presentation, not content."""
    s = html.unescape(s)
    s = re.sub(r"<[^>]+>", "", s)
    s = s.replace("—", "-").replace("–", "-").replace(":", "-")
    return re.sub(r"[\s-]+", " ", s).strip().lower()


def split(title):
    """`9. The four funds` -> (9, 'the four funds'); unnumbered -> (None, title)."""
    m = re.match(r"^(\d+)\.\s*(.+)$", title)
    return (int(m.group(1)), norm(m.group(2))) if m else (None, norm(title))


def main():
    verbose = "--verbose" in sys.argv
    md_file, ht_file = DOCS / "whitepaper.md", DOCS / "whitepaper.html"
    for f in (md_file, ht_file):
        if not f.is_file():
            print(f"  {f.name} is missing -- the published documents are not both here")
            return 1

    md = [split(h) for h in re.findall(r"^##\s+(.+?)\s*$", md_file.read_text(), re.M)]
    ht = [split(h) for h in
          re.findall(r"(?is)<h2[^>]*>(.*?)</h2>", ht_file.read_text())]

    # Numbered sections are compared by number; a title is one-sided when only one document
    # carries it, whether or not it happens to be numbered there. Both cases end at the same
    # question: is this waived above, and for the right document.
    md_by_n = {n: t for n, t in md if n is not None}
    ht_by_n = {n: t for n, t in ht if n is not None}
    md_titles = {t for _, t in md}
    ht_titles = {t for _, t in ht}

    bad, reported = [], set()
    for n in sorted(set(md_by_n) | set(ht_by_n)):
        a, b = md_by_n.get(n), ht_by_n.get(n)
        if a is not None and b is not None and a != b:
            bad.append(f"  §{n} is '{a}' in the markdown and '{b}' in the HTML")
            # Both titles are one-sided by definition; saying so again below would print
            # one drift three times and bury the sentence that names it.
            reported |= {a, b}
        elif a is not None and b is not None and verbose:
            print(f"  both  §{n} {a}")

    for title in sorted((md_titles ^ ht_titles) - reported):
        side = "md" if title in md_titles else "html"
        if ONLY_IN.get(title) == side:
            if verbose:
                print(f"  {side:<5} {title}  (waived)")
        else:
            bad.append(f"  '{title}' is in the {side} only, and is not waived above")

    if bad:
        print("\n".join(bad))
        print()
        print("İki belge de yayında ve ikisi de elle bakımlı. Birinde düzelen bir başlık")
        print("ötekinde eskisiyle kalırsa iki sitemiz aynı kurumu iki adla anlatır.")
        return 1

    print(f"whitepaper.md and whitepaper.html agree on all "
          f"{len([k for k in md if k is not None])} numbered sections")
    return 0


if __name__ == "__main__":
    sys.exit(main())
