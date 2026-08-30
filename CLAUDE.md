# Working on this tree

## The plan

Five sheets. Each answers one question, and each derives its own completeness rather than
listing what somebody remembered -- a pallet that exists has a row, an invariant that exists
has a column, so a gap cannot hide by never having been written down.

```
python3 .github/scripts/plan.py            # what must be true at genesis   (subject x invariant)
python3 .github/scripts/plan.py --flows    # do the cross-chain paths work  (path x gate)
python3 .github/scripts/plan.py --arch     # which chain carries what       (pallet x chain)
python3 .github/scripts/plan.py --phases   # the order, and what each phase must show
python3 .github/scripts/plan.py --work     # has the agreed design landed; fails on regression
```

The wiring sheet is the one that earns its keep. A cross-chain path has three gates and the
origin check is the last of them; both ends can be right while the path is dead, and neither
end can see it. Two paths sat broken that way, in opposite directions, and neither was found
by reading.

Order matters in two places and both are irreversible:

- **Everything the static sheet lists closes before FAZ 2, not before mainnet.** Enum indices,
  storage versions and pallet indices freeze the moment a chain has a genesis, and a testnet's
  storage keys are no easier to renumber than a mainnet's.
- **A variant may be renamed only after its index is pinned.** Until then the encoding follows
  the name.

The work sheet runs at session start and in CI. When an item lands, `--record` moves the
baseline; do that in the same commit as the work.

## Before finishing a piece of work

Run the gate. If it reports something you did not intend to change, that is the answer, not
the tests you happened to run.

## The rules the gate cannot check

- **Identifiers are Kurdish, comments are English.** The gate checks this; it cannot check
  that a Kurdish word is the *right* one.
- **Nothing burns, and the two tokens are not the same token.** `OnUnbalanced = ()` destroys
  tokens -- route slashes and forfeits to a treasury instead. This line used to read "the
  supply is fixed and halving", with no token named, and that sentence is true of PEZ and
  false of HEZ: PEZ is an asset on the Asset Hub, five billion, fixed, its rewards pool
  halving every 48 months; HEZ is the native token of the relay, the Asset Hub and People
  alike, and it inflates -- `MAX_INFLATION_RATE` caps it at 10% a year. The unnamed sentence
  got copied into eleven comments as a reason for not burning HEZ.
  The reason for HEZ is different and stronger: burning an inflating token hands the
  confiscated value to everyone holding it. A penalty should become something the state can
  spend, not a quiet dividend. `check-token-claims.py` keeps the two apart.
- **Both twins or neither.** Zagros and Pezkuwichain are the same chain at different stages.
  A change landing in one is a bug in the other. `check-twin-runtimes.py` holds the pallet
  index maps together; nothing holds the rest, so read both.
- **Heavy builds do not belong on this machine.** A pallet check is fine; a runtime test
  binary or a full workspace build is not. CI and the dedicated runner exist for that.
- **Do not edit sources while a build is running.** The result is neither a pass nor a
  failure, and a green from a half-written tree has been mistaken for proof here before.

## Where the reasoning lives

**One file: `res/plans/PLAN.md`.** It is the plan, the checklist and the open items, and it
is the only one -- `res/plans/arsiv/` is history, not policy. Do not start a second plan
file; eight of them accumulated here once, each written and then never reopened, and the
work went into deciding which was true.

It holds *why* each item exists, who has to close it, and what breaks if it is skipped. It
does **not** hold a status the sheets can measure -- it names the command instead. A status
carried by hand goes stale, and a stale mark is worse than no mark. `res/` is outside the
repository and stays there.
