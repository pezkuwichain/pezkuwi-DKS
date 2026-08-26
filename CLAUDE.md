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
python3 .github/scripts/durum.py           # the planned work, and its regressions
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

## The state of the work is measured, not written down

```
python3 .github/scripts/durum.py
```

Nineteen items, each answering its question by reading the source. This is the plan's status;
there is no file to keep in step with it, because a note describing the tree stops being true
the moment the tree moves and nothing tells you. It runs automatically at session start.

It is also a CI gate. An item that was closed and is not any more exits non-zero and the build
goes red. The backlog does not fail the build; work that came undone does.

When an item closes, `--record` moves the baseline. Do that in the same commit as the work.

## Before finishing a piece of work

Run the gate. If it reports something you did not intend to change, that is the answer, not
the tests you happened to run.

## The rules the gate cannot check

- **Identifiers are Kurdish, comments are English.** The gate checks this; it cannot check
  that a Kurdish word is the *right* one.
- **Nothing burns.** The supply is fixed and halving. `OnUnbalanced = ()` destroys tokens --
  route slashes to a treasury instead.
- **Both twins or neither.** Zagros and Pezkuwichain are the same chain at different stages.
  A change landing in one is a bug in the other. `check-twin-runtimes.py` holds the pallet
  index maps together; nothing holds the rest, so read both.
- **Heavy builds do not belong on this machine.** A pallet check is fine; a runtime test
  binary or a full workspace build is not. CI and the dedicated runner exist for that.
- **Do not edit sources while a build is running.** The result is neither a pass nor a
  failure, and a green from a half-written tree has been mistaken for proof here before.

## Where the reasoning lives

`res/plans/` holds *why* each item exists and what breaks if it is skipped -- never its
status. `res/` is outside the repository and stays there.
