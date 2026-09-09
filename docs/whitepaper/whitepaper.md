# PezkuwiChain

### A state that runs as software

**Whitepaper v6.1 — Testnet Edition**
Dijital Kurdistan Tech Institute

*The chain described here runs on a test network. Six months live, 2.7 million blocks
finalised, no mainnet genesis yet. §10.1 lists what is specified but not yet built, in full
and without softening — a reader who finds a gap there before finding it in the code is
reading the document this was written to be.*

---

## Abstract

PezkuwiChain is a sovereign public blockchain built to carry the functions of a state:
a citizen register, elected institutions, a treasury, a currency, and courts. It is not a
company issuing a token. Its constitution is its runtime, its statute book is its pallet
set, and its separation of powers is enforced by the type system rather than by promise.

Two properties distinguish it. First, **there are two electorates and they are deliberately
different.** Matters of the state — citizenship, offices, the register — are decided by
counting citizens, one person one vote, on a chain where a token balance buys no vote — the
deposits that make a proposal or a candidacy serious are reserved in HEZ, and reserved is not
spent. Matters of the network's administration — staking, leases, auctions, the treasury of
last resort — are decided by stake. Neither electorate votes in the other's ballot.

They are not equals, and this document does not pretend otherwise. The civil layer holds the
only door into the consensus layer's root; the reverse door does not exist. Section 2.1 says
what that means and what it does not.

Second, **money and the authority to move it live on different chains.** Every fund sits on
the Asset Hub. The relay holds no fund of its own — only the escrow mirroring what the Asset
Hub carries, and the founder's allocation, which is property rather than a fund. Every authority to draw from it sits on the People chain. A payment is a
cross-chain message from an office to a vault, and the vault's own configuration names the
one chain it will listen to. An officeholder cannot reach the money by holding a key; they
reach it by holding an office, and the office is an entry in a register that citizens elect.

This document describes the system as it is written, and every figure in it is read from the
source. Where a figure here and the code disagree, the code is correct and this document is
the defect.

---

## 1. Why a state, and not a platform

Most chains are platforms looking for applications. PezkuwiChain begins from the opposite
end: a nation without a state apparatus, and a question about which parts of one can be
built from software.

Some cannot. A blockchain does not hold territory, and it does not enforce a judgment with
anything but the consent of those who run it. What it can do is hold a register that nobody
can quietly edit, run an election whose count is reproducible by anyone, and hold a treasury
whose every movement carries the name of the office that authorised it. Those three — the
register, the ballot, and the ledger — are what a state must get right before anything else,
and they are precisely what software is good at.

So the design is narrow on purpose. PezkuwiChain does not attempt to be a world computer. It
attempts to be a **civil service that cannot be captured by whoever holds the most coins**,
and it spends its architecture on that single problem.

### 1.1 The stateless advantage

A nation building institutions for the first time inherits no legacy system to migrate, no
ministry to placate, and no incumbent to compensate. What is a disadvantage in every other
respect is, here, a clean slate. Rules that established states must retrofit — a register
that cannot be forged, an election whose count is public arithmetic, a budget that cannot
be moved without a named authority — can be written into the foundation rather than bolted
on afterwards.

---

## 2. Architecture

PezkuwiChain is a relay chain with system chains attached to it. The relay provides shared
security and finality; each system chain carries one function of the state and nothing else.
This is not a scaling decision. It is a **separation-of-powers decision expressed as
topology**: the chain that holds the register is not the chain that holds the money, and
neither is the chain that produces blocks.

| Chain | Id | Carries |
|---|---|---|
| **Pezkuwichain** (relay) | — | Consensus, finality, the validator session, cross-chain routing, and the HEZ escrow |
| **Asset Hub** | 1000 | Every fund. HEZ, PEZ, wHEZ and wUSDT. Staking and validator elections. |
| **People** | 1004 | The citizen register, every office, the courts, trust, and the validator pool |
| **Bridge Hub** | 1002 | Bridges to other consensus systems, including Ethereum |
| **Coretime** | 1005 | Blockspace allocation |

**Five chains are specified; two of them run from the first block.** The relay schedules
exactly two cores at genesis, one for the Asset Hub and one for People — the two chains a
state cannot run without. Bridge Hub and Coretime are written and will be seated when there
is traffic for them to carry; every figure in this document about those two describes a
specification rather than a running chain.

HEZ is native on all three running chains and moves between them by teleport, against the
escrow the relay holds. Two wrapped assets also live on the Asset Hub and are not part of
that mechanism: **wHEZ** (asset 2) is HEZ wrapped one-for-one so that it can be traded by
pallets that handle assets rather than the native balance, and **wUSDT** (asset 1000) is the
custodial bridge's representation of USDT. Neither is a second HEZ, and neither is minted by
a teleport.

### 2.1 The one door into the relay's root

The relay chain has no `root` governance track. There is no referendum that can dispatch as
root, and no collective that can. Root arrives from exactly one place: a message from the
People chain, carried as `OriginKind::Superuser`, converted by a single origin converter
that matches that chain's identifier and nothing else.

This is the constitutional core of the design, and it is eleven lines of code. The consensus
layer is subordinate to the civil layer, structurally, and no amount of stake on the relay
can reverse the direction.

**What root can do, stated plainly.** Root in this system is not an office; it is a seat, and
two things sit in it. One is the People chain's own referendum, on the twenty-eight-day track,
counted by citizens. The other is a sudo key held for the founding period. Until
it retires it is absolute, and this document would be worth less if it said otherwise.

**When it retires is a measurement, not a date.** The referendum seat is only occupied once
the register can actually fill it: the support floor in §3.1 means a question needs two
thousand citizens voting aye, and a roll that cannot produce them has a civil path in name
only. Retiring the key before then would not hand authority to the people — it would leave the
chain with no working authority at all. So the key goes when three things are true together:
the chain has been proved end to end on the test network, the roll can carry the floor, and a
referendum has actually decided something under it. The third is the one that is easy to skip
and the one that matters: an authority that has never been exercised is not known to work.

Root can upgrade a runtime, and a runtime is where every rule in this document lives —
including the origin filters that make the vaults refuse. So every claim below of the form
*"X cannot reach this money"* means **"X cannot reach it short of a runtime upgrade."** There
is no formulation that would make it stronger, and a document that implied one would be
describing a different kind of machine.

What protects the invariants is therefore not impossibility. It is that the only path runs
through an upgrade; that an upgrade is a constitutional amendment rather than an
administrative act; and that the path is slow, public, and counted by people rather than by
holdings. Two things are deliberately kept off even that path: the register's own admission
rules, which no root arm administers (§3.1), and PEZ's supply, which a call filter refuses
over any cross-chain message whatever origin it carries (§8.2).

---

## 3. Two electorates

The most consequential decision in this system is that **the franchise is split**, and the
two halves count differently.

### 3.1 The People chain — one citizen, one vote

Referenda on the People chain are tallied by a citizen count. Support is measured as *ayes
divided by the entire citizen roll*, not by tokens, and approval as ayes over ayes-plus-nays.
The roll is the number of citizens in the register. A wallet holding a billion HEZ has
exactly the weight of a wallet holding none: one, if it belongs to a citizen, and zero if it
does not.

**Support is measured against the roll, or against a hundred thousand, whichever is larger.**
The support thresholds below fall to two percent, and two percent of a young register is a
handful of people — the curves are written for a state with millions on the roll, and applied
to a few hundred they would let a couple of dozen citizens amend the constitution. So the
denominator has a floor. Below it a question is not cheap to carry; it is refused, in the same
way a stratum with too few members is refused a seat in §7.2. The floor is not a quorum a
referendum has to assemble on top of its own rules — it is the same rule, honestly denominated.

**And the roll can fall as well as rise.** A denominator that only grows has the same end as
a captured one: every lost key, every death and everybody who registered once and never came
back would stay in it for good, so the share a question needs would climb for ever while the
people who could supply it did not. A citizen who has taken no part in anything for two years
stops counting towards it. Nothing is taken away — the citizenship, the standing and the vote
all remain, and casting one puts them back in the count in the same block — and anybody may
mark a silence that the chain can already see, because a body that chose whose absence counted
could shrink the electorate before a vote it cared about.

Two things follow. Nothing can pass before the roll reaches two thousand, because a question
needs two thousand ayes and there is nobody else to cast them — and that is a necessary size
rather than a sufficient one, since two thousand citizens must actually vote aye, not merely
exist. And the floor **retires itself**:
once the register is larger than a hundred thousand the denominator is the roll again, and the
rule has no further effect for the rest of the chain's life.

Five tracks exist, each dispatching a different authority:

| Track | Decision period | Confirms | For |
|---|---|---|---|
| `root` | 28 days | 24 h | Anything on this chain except the register's own rules |
| `welati_election` | 14 days | 12 h | Electoral machinery |
| `welati_admin` | 7 days | 3 h | Routine administration |
| `citizenship_admin` | 14 days | 6 h | The register's own administration |
| `qeyd_rules` | **90 days** | 7 days | The rules governing the register itself |

**Voting costs nothing.** Answering a referendum, casting a ballot and endorsing a candidacy
are free the first time each citizen does them — the fee is skipped in the runtime rather than
refunded — because a franchise that costs money is not one this document can claim. A repeat is
paid for, which is what stops a free call being a free call twice.

The last one is the notable entry. The parameters that decide who may vouch for a new
citizen, how many people one citizen may vouch for, and what suspends that right, are held
in a parameter store whose only administrator is a referendum on the ninety-day track. Not
root. Not the court. Not the president. Changing the rules of admission takes three months
of deliberation by the people already admitted, and there is no faster path.

That exclusion is literal: the twenty-eight-day root track is an arm of nothing in this
store, and neither is the relay. An optional slow path is a fast path — nobody takes the long
road when the short one arrives at the same place — so the short road was not built. The
escape hatch is the one named in §2.1 and no other: a runtime upgrade can change the
defaults, and changing them is what amending a constitution ought to feel like.

### 3.2 The relay chain — stake, with conviction

The relay uses conviction voting over HEZ. Turnout is measured against votable issuance,
which deliberately excludes the escrow account holding the Asset Hub's mirror of the supply —
180 million HEZ that exists on both sides of a teleport and must not be counted twice.

Eight tracks exist for network matters: whitelisted upgrades, staking administration, lease
and auction administration, general administration, and the two cancellation tracks. There is
no track for root, because root is not the relay's to give.

### 3.3 The citizens' initiative

Citizens may open a referendum without any office's involvement. One percent of the roll,
recomputed live against the current register, backing a proposal within fourteen days, opens
it on the track it names. The deposit is ten HEZ and the cooldown is thirty days.

---

## 4. Citizenship

Citizenship is a non-transferable NFT in collection zero. Holding it is what makes an account
a *welatî*, and every office, every vote, and every trust score is downstream of it.

Admission has three steps and no gatekeeper — nobody sits between an applicant and the
register whose approval must be sought, and no office can admit at will:

1. **Apply.** The applicant reserves a one-HEZ deposit and registers an identity hash. The
   hash is globally unique and is claimed at application time, so two applications cannot
   describe the same person.
2. **Vouch.** An existing citizen approves the referral. A citizen begins with five vouching
   places and earns one more for every three that settle, to a ceiling of fifty. Vouching is
   not free of consequence: a voucher whose referrals are revoked three or more times, and
   whose revoked share passes twenty percent, is suspended.
3. **Confirm.** The applicant confirms, the NFT is minted, and the roll increases by one.

Two hands do touch the register, and both are named rather than implied. If nobody vouches
within ninety days, the **founding account** may admit the applicant and becomes their
referrer of record — the fallback exists so that having no connections is not a permanent
bar, and admissions made this way are stored apart from ordinary vouches, so standing in for
somebody is never counted as having chosen them. And the **court**, as the register
authority, can revoke a citizenship or a vouch after the fact. Neither is an administrator
of admission: one cannot refuse, the other cannot admit.

**What the identity hash does and does not prove.** Uniqueness of the hash stops the same
document being registered twice. It does not stop one person registering twice with two
documents, and this system claims no proof of personhood. The defence against a manufactured
population is the vouching chain and its cost: a fabricated citizen needs an existing citizen
to stand for them, vouching places are finite, and a voucher whose referrals are revoked
loses the right to vouch at all. That is accountability rather than impossibility, and it is
priced accordingly — a hostile register-stuffer must either buy citizens who have something
to lose or burn their own standing at three revocations. We state the bound rather than
overclaim the mechanism: the register is as honest as the citizens who fill it, which is a
weaker guarantee than cryptography and a stronger one than an administrator's judgement.

**Losing the key.** A citizenship NFT is non-transferable and its identity hash is claimed
for good, so an applicant who loses their key cannot simply register again — the hash would
collide with the one they already own. The remedy is judicial rather than automatic: the
court, sitting as the register authority at two thirds, may reissue a citizenship to a new
account. It is deliberately not a self-service path. A recovery anybody can invoke is a theft
mechanism wearing a helpful name, and the register is the one asset in this system with no
market price and no way to be made whole after the fact.

The old account is retired for good. It can be neither reissued again nor reissued *to*, which
is what stops a chain of accounts being used to launder standing through a series of court
orders. It stays on the record as superseded rather than being erased: history keeps the name it
was written under, and only what a person is owed or answerable for follows them forward.

---

## 5. The institutions

Every office in PezkuwiChain is a *tiki* — an entry in the register attached to an account.
Fifty-six exist. What matters is not the list but the four ways a tiki is obtained:
**automatic** (citizenship itself), **elected**, **earned** (by contribution, at published
thresholds), and **appointed**. An office may never be granted by the same route that grants
a community badge, and the code refuses it.

### 5.1 Serok — the President

One seat, four years, elected by every citizen. To stand, a candidate needs an approved
identity, a trust score of at least 250, a hundred-HEZ deposit, and endorsements from citizens with
a trust score of at least 40 — a thousand of them once the register is grown, and the same
*share* of a smaller one before that. Written flat, a thousand was half the population of a
young roll, each endorser needing standing that comes mostly from education nobody could have
earned yet; the first election would have been winnable only from inside whatever circle could
hand out the rest. The threshold now scales with the register up to the figure named here and
never past it, with a floor so that a candidacy in a small country still costs somebody else's
reputation. Nobody may serve more than two
consecutive terms.

The election requires fifty percent turnout — waived only after one failed attempt, so that
a boycott delays rather than vetoes. A candidate wins outright with more than half the valid
votes; otherwise the top two go to a runoff whose campaign is one third the length.

### 5.2 Meclis — the Parliament

Two hundred and one seats, four years, elected across ten districts. A candidate needs a
trust score of at least 100 and a hundred endorsements, and the election requires forty
percent turnout.

**The first parliament sits for half a term.** This is deliberate: it staggers the
legislature against the presidency permanently, so that no single election ever renews the
whole state at once.

### 5.3 Serokê Meclisê — the Speaker

Elected, but only from among sitting members of parliament, and requiring a trust score of
at least 200. The Speaker holds no term of their own — the office is vacated whenever a new
house is seated, because a speaker without a house is not a speaker.

### 5.4 Dîwan — the Constitutional Court

Eleven seats, nine years — the longest term in the system, and longer than any body that
appoints to it.

**Six are elected by the Parliament and five are appointed by the President.** Neither can
seat a majority. The split is not written as "five"; it is derived, as *total minus elected*,
so that changing the size of the court cannot silently change the balance between the two
powers that fill it.

Elected members need a trust score of at least 275. Appointed members must already hold one
of fourteen qualifying professional tikis — jurist, judge, prosecutor, engineer, cyber-security
specialist, network operator, economist, accountant, planner, electoral officer, statistician,
auditor, scholar, or cultural custodian. A president may choose, but only from people the
register already recognises as qualified.

**There is no call to dismiss a member of the court.** The absence is the point: a court
that can be removed by the powers it rules on is not a court, and nine years is longer than
any body that appoints to it precisely so that no single alignment outlives it.

**The seats are not staggered, and that is a choice rather than an omission.** A staggered
bench is harder for one election to capture, and giving that up is a real cost — but a
staggered court is one whose seats are always about to fall vacant, so its members always sit
in front of the body that will next fill them. This court is meant to be eleven people the
nation already follows, seated once, irremovable, owing nothing to whoever seated them; a
rolling appointment would let the calendar back in exactly what the missing dismissal call
keeps out. The capture worry is answered by arithmetic instead: taking all eleven needs the
house *and* the presidency in one moment, and both run for four years against the court's nine
— so the alignment that seated it faces the electorate twice before a single seat turns over.

**Dismissal and vacancy are different questions, and only the first is closed.** Death, a lost
key, and permanent silence are not misconduct, but they subtract from the same two-thirds a
decision needs; four silent seats out of eleven make that threshold unreachable for good, and
every authority the court carries — the register, the validator pool, and the fast upgrade
path in §10.1 — freezes with it. So the court's seats are vacated on measured inactivity, and
the duty that goes with a seat is one signature every hundred and eighty days. It decides
nothing — a check-in is not a vote and says nothing about how a member would rule — because a
duty that required agreeing with anybody would make the court answer to whoever set the test.
Anyone may then vacate a seat that has missed it; the condition is arithmetic that anybody can
check for themselves, and no body is given the power to choose whose absence counts, because
every body that could hold it is one the court exists to rule on.

Vacating is itself the repair: two thirds is counted over the members who sit, so a bench of
seven needs five rather than eight, and the court resumes the moment the silent seats leave it.
Refilling is the slower half — the President may appoint into a vacancy at once, while an
elected seat waits for the court's next scheduled election. Removal for cause remains
impossible; absence is not cause, and treating it as such would be the loophole this rule
exists to refuse.

The court is not decorative. Two thirds of it constitutes the *register authority*, which
governs the citizen register itself, administers the validator pool, and can strip an elected
or earned office. It is also the fraud origin for education credentials and, together with
the council, the slashing origin for staking scores.

**The council** is the parliament's own standing body: a collective of up to a hundred whose
roster is written from the sitting Meclis rather than elected separately, so it holds no
mandate of its own and cannot outlive the house that seats it. It matters in three places and
nowhere else — it is an alternative arm where the court or the president would otherwise act
alone, it is half of the slashing origin for staking scores, and a single member of it may
freeze a suspicious staking-score submission pending review. That last one is deliberately
cheap to use and cannot take anything: freezing is not slashing, and only the two bodies
together can slash.

### 5.5 Serokwezîran — the Prime Minister, and the cabinet

The President nominates; the Parliament confirms. Neither alone suffices, and either may end
it. Once confirmed, the Prime Minister appoints and dismisses the cabinet alone — seven named
ministries (finance, defence, justice, education, health, infrastructure, culture) plus
general ministers without portfolio.

Two ministries carry spending authority, and they are the subject of Section 9.

### 5.6 The civil service

Twenty-four professional offices, from judge and prosecutor to notary, registrar, tax
collector, ambassador and teacher. Any minister or the President may nominate; nobody may
nominate themselves; every nomination needs a trust score of at least 75 and lapses in seven
days.

**Five of the twenty-four cannot be seated by the President alone** — judge, treasurer,
cyber-security specialist, inspector, and ambassador require parliamentary confirmation. And
the list of which five is itself amendable **only by the Parliament**. The executive cannot
shorten the list of offices it does not control.

---

## 6. Trust

Trust is a single number between zero and a thousand, recomputed per citizen, and it is the
currency of standing in this system — for candidacy, for endorsement, for the validator pool,
and for the citizens' share of the reward pool.

It is composed of four measured parts, each normalised against its own maximum and weighted:

| Part | Weight | Measures | Maximum |
|---|---|---|---|
| **Perwerde** (education) | 30 | Points from completed, certified courses | 50,000 |
| **Referral** | 25 | Citizens vouched for, net of revocations | The score at the current vouching ceiling |
| **Tiki** | 25 | Community and contribution badges held | 1,000 |
| **Staking** | 20 | Size and duration of stake | 100 |

The weights sum to one hundred, and the runtime asserts it. Each part is divided by what is
*attainable*, not by a number written beside it: the education maximum is every rewarded
course taken at full value, and the referral maximum is the score a citizen reaches at the
vouching ceiling the register's rules currently set. A weight that says twenty-five is
therefore twenty-five, and stays twenty-five if a ninety-day referendum moves the ceiling —
a component cannot quietly stop being able to reach its own top.

Two properties follow from the arithmetic, and both are intentional.

**Zero stake is zero trust.** The staking part is not merely weighted; it is a gate. A
citizen with no economic exposure scores zero however educated or well-connected. Standing
requires something at risk.

**But capital is the smallest component, and it is not the only thing that can be given.**
Stake carries the lowest weight of the four, and its own scale saturates: the amount tiers stop rewarding size above 750 HEZ, and the largest
remaining multiplier comes from *holding for twelve months*, not from holding more. Beyond a
modest threshold, patience buys more standing than wealth does.

**Offices are excluded.** Holding an office adds nothing to the tiki component — the code
filters offices out before summing. Otherwise power would compound: an office would raise
trust, trust would qualify for more offices, and the register would drift toward whoever
already held it.

---

## 7. TNPoS — the validator pool

Nominated proof-of-stake elects the wealthiest set that nomination can assemble. Over time
that is the same set. **TNPoS breaks the correlation by construction**: it fills the committee
from nine independent strata, and gives each stratum the same number of seats regardless of
how much stake sits behind it.

### 7.1 The nine strata

| Stratum | Admits a citizen who has | Answers to |
|---|---|---|
| **Stake** | Any staking score above zero | The market |
| **Meclis** | A seat in the elected house | The house |
| **Dîwan** | A seat on the court | The court |
| **Perwerde** | Any education score above zero | Accredited institutions |
| **Tiki** | Any community score above zero | The community |
| **Welatî lottery** | Trust above forty — more than the cheapest act | Nobody; the pool dilutes |
| **Geography** | An attested belonging to a part of the nation | A notary, undone by the court |
| **Tenure** | A year of unbroken, offence-free membership | Nobody; only elapsed time |
| **Infrastructure** | Forty-eight sessions validated, and no pattern of failing with others | Nobody; the chain's own record |

The third column is the one the security argument counts. Two strata answering to the same
institution are one stratum, and eight distinct answers is what "nine independent gates" has to
mean before it can be claimed.

Each stratum seats **three** validators. A full committee is **twenty-seven**.

The stratum named Dîwan is the court of §5.4, and a seat there confers no judicial
office in the other direction: a judge who validates is a judge who runs a node, and the court
still decides as a body of two thirds. Its floor is the one exception in §7.3, and the court's
size is exactly the reason for it.

The security argument rests on the strata being gated by *different* authorities: two strata
answering to the same institution are one stratum, not two, and the committee's independence
is counted from that number. All nine gates are measured on this chain today, and
they are nine different measurements. Stake, education and community tikis each read their own
score. The parliamentary and judicial strata read the membership they are named after, so a
citizen with a perfect trust score and no seat is refused by both. Geography reads an attested belonging — where a citizen is
*from*, not where they are sitting. Somebody from Rojava living in Germany marks Rojava: this is
the register of a dispersed nation, and what is worth spreading seats across is which part of it
a person comes from. Six are recognised: **Başûr, Bakur, Rojava, Rojhilat, the diaspora and the
Caucasus**, the last two for citizens whose belonging is to neither of the four. A citizen may
ask for a mark once twenty-five settled referrals stand behind them, a notary confirms the
region they claimed and no other, the court can cancel it — and the citizen can take it back,
because it is volunteered and nothing else asks for it. Where a validator's machine physically
stands is a different question, and the ninth stratum answers it by measurement rather than by
asking. The three seats rotate across the regions
rather than pooling — six complete a cycle in two eras — because three seats drawn from one pool
would go to whichever region is most populous and the label would decide nothing. Tenure
reads unbroken, offence-free time in the pool and answers to no authority at all: it is the one
qualification that cannot be granted, bought or manufactured. Moving between strata does not
break it — a member never stops being in the pool — but leaving does, and so does an offence, so
a validator who equivocates begins that record again rather than waiting out a ban. The open lottery asks only that a
citizen has done more than the cheapest possible act — its bar is exactly what an account that
stakes the smallest tier and does nothing else scores — because that stratum's security is the
size of the pool rather than the height of its bar, and a hard gate there would duplicate one of
the other eight and shut out the ordinary citizens it exists to seat.

Infrastructure is the ninth, and it is the only gate that asks for *work done*. Forty-eight
sessions seated, no ban, and no repeated pattern of failing alongside others. The other eight
are passed with money, an identity, a vouch, a seat or a signature; this one is passed by having
validated, which nobody can grant and no manufactured account arrives holding.

**It measures failing together rather than being apart, and that is deliberate.** Location
matters only because validators in one place go down at the same moment, and that moment is the
only consequence of shared infrastructure that reaches a chain at all. So the relay reports, at
the end of each session, who was seated and who authored nothing; one name is an operator's own
outage, eight names is eight operators who share something. A session where more than half the
committee is down marks nobody — that is the network having a bad day, and counting it would
mark every honest operator at once.

**It disqualifies; it does not certify.** A cluster that has never had an outage looks exactly
like independent operators, and no measurement from inside a chain can separate them — two
validators that have never diverged leave no trace of their difference. A declaration would not
solve that, only hide it, and would put an authority and a lie where there is currently
neither. What this gate does is make shared ground detectable once it costs anything, and it
says so rather than promising more. The
figure to hold onto is therefore this: **nine strata are specified, and the count of
independent gates is what the network should be judged on at any given moment.** It is
published on chain, and it is not nine yet.

### 7.2 Membership is a gate, not a ranking

This is the part that most distinguishes TNPoS from anything score-weighted. Inside a
stratum, a higher trust score buys **no advantage whatsoever**. The score decides whether you
are in the pool; a uniform random draw decides whether you sit. The wealthiest citizen and
the barely-qualified citizen have the same chance in the same stratum.

The draw is seeded by commit–reveal across the era: commitments in the first half, reveals in
the second, each era's seed derived from the previous one and the revealed preimage. No
single participant chooses the seed, and the seed for an era does not exist until that era
is underway.

### 7.3 The floors that refuse to seat a weak committee

A stratum with fewer than **fifty** eligible members is not seated at all, and **its seats are
not redistributed**. The court is the single exception and carries its own floor of three, which
is its seat count: fifty is sized against an adversary who can *manufacture* eligible members,
and a seat on an eleven-member court cannot be manufactured — taking six of them means taking
the house and the presidency together, at which point three validator seats are not what stopped
anybody. Applying fifty there would have left the judicial stratum permanently unseatable, since
eleven can never be fifty. A committee is refused if it draws from fewer than five strata, or has
fewer than fifteen members, or more than sixty-four.

Refusing to fill a committee is a safer failure than filling it from whoever happens to be
available, and the code treats it that way: a thin field produces a smaller committee, never
a captured one.

### 7.4 What the committee needs to act

| Committee | Quorum | Halt | Fork |
|---|---|---|---|
| 27 (full) | 19 | 9 | 11 |

Quorum is two thirds plus one. The halt threshold is the number who can stop the chain by
abstaining; the fork threshold is the number who would have to collude to split it. Both are
derived from the committee size rather than fixed, so a smaller committee is honest about
being easier to disrupt.

### 7.5 Misconduct costs standing, and separately costs money

TNPoS itself touches no funds. Its sanction is exclusion: unavailability bans a validator for
twenty-four eras, equivocation for three hundred and sixty. A ban may only ever be extended,
never shortened, and removal from the committee is immediate.

Economic slashing is the Asset Hub's business, and **nothing is burned**. Slashed HEZ is
resolved to the treasury. Burning an inflating token would hand the confiscated value to
everyone still holding it — a quiet dividend paid by the victim to the bystanders. A penalty
should become something the state can spend.

---

## 8. Two tokens

HEZ and PEZ are not two flavours of the same thing. They answer to different authorities,
live on different chains, and behave in opposite directions.

### 8.1 HEZ — the currency

The native token of the relay, the Asset Hub and People. One HEZ is 10¹² TYR. It pays fees,
it secures the network, and it inflates.

**Two hundred million HEZ exist at genesis**, and the split is:

| Allocation | Amount | Held on | By |
|---|---|---|---|
| Presale | 100,000,000 (50%) | Asset Hub | A keyless pot |
| Treasury | 40,000,000 (20%) | Asset Hub | A keyless pot |
| Airdrop | 40,000,000 (20%) | Asset Hub | A keyless pot |
| Founder | 20,000,000 (10%) | Relay | The founding account, liquid |

Three of the four are keyless: no seed produces the account, so the balance leaves only
through an authorised spend. The fourth is the founder's, and it is property rather than a
fund — held on a key, and liquid from the first block. That is deliberate, and it is stated
here rather than left to be discovered.

It is liquid because it is the network's launch capital. A chain secured by stake cannot seat
the validators that produce its blocks until somebody has staked; at genesis there is no
market and no one else holding HEZ, so the founder's share is what the first validators are
bonded with and what the founding team is paid from. The undertaking is that it is **lent to
the network's security rather than sold** — put behind validators so the chain has weight
defending it, and returned to circulation as the roll and the market can carry it.

That is an undertaking and not a lock, and the difference is the point of this document: no
runtime rule enforces it. What makes it checkable is that the account is public and the
ledger is public: every transfer out of the founding account is on the chain, with its
destination, its amount and its block. Whether the HEZ went to a validator's stash or
somewhere else is not a matter of trust — it is a query anyone can run, against an address
published here. An undertaking that can be audited is a different thing from one that has to
be believed, and this is the only kind this document is willing to make.

What the runtime does enforce is on the other side of the ledger — the founder's PEZ.

The founder's **PEZ** is the share that waits, and there the rule is in code. It is minted
into a keyless pot and leaves only when the population gate fires — the same latch, in the
same call, that starts the citizens' payments. If the roll never reaches the threshold the
founder's share stays locked as permanently as the citizens' does. The two allocations are
treated differently because they do different jobs: the HEZ brings the network up, and the
PEZ is a share of what it becomes.

The treasury's share is minted into the treasury pallet's own account on the Asset Hub, which
is where the pallet that spends it lives. The relay has no treasury pallet, so money held
there would have had authority nowhere: a pot with no governance path, reachable only by a
key. Splitting the money from the authority that spends it is exactly the failure this
architecture exists to prevent, and it is not excused by the two halves belonging to the same
state.

The relay mints the founder's twenty million, the validators' initial stashes, and a hundred
and eighty million of **escrow** — the mirror of what the Asset Hub holds, so that a teleport
moves a token rather than creating one. The escrow is not supply: it is the same HEZ, held
here and represented there, and the relay's turnout figure excludes it so that governance is
not distorted by its size. The runtime carries a test that builds the genesis and adds up
what is actually in it, asserting owned plus escrow equals exactly two hundred million —
constants are not what a chain mints, so the test reads the genesis rather than the
constants.

**Inflation is bounded and its base is fixed.** The rate is a governance parameter, eight
percent by default, hard-capped at ten percent by a constant no parameter can exceed. It is
applied to a *fixed base of two hundred million*, not to total issuance — so the emission
does not compound, and at the default it is sixteen million HEZ a year, of which fifteen
percent goes to the treasury and the rest to those securing the chain. Only the Treasurer, an
office on the People chain, may change the rate — never HEZ holders, and never by more than
one percentage point at a time, no more often than every ninety days.

**What a fixed base means for a holder.** Because the rate applies to the fixed two hundred
million rather than to circulating supply, emission does not compound: the same number of HEZ
arrives every year, and the *effective* rate falls as the supply grows — about eight percent
in year one, four and a half in year ten, under three by year twenty-five. Supply is
nonetheless unbounded. It doubles in roughly twelve and a half years and triples in
twenty-five. A genesis holder who never stakes is therefore diluted linearly and permanently,
and this is a property of the design rather than an oversight: the emission buys validator
security, which a state cannot pay for out of fees it does not yet collect. Stated plainly so
that nobody has to derive it from the schedule.

Emission is not the only income. Transaction fees on the relay split **eighty percent to the
treasury and twenty percent to the block's author**; on the teyrchains the whole fee goes to
the collator pot, which is a collator's only income, since inflation pays the relay's
validators and not them.

### 8.2 PEZ — the franchise

An asset on the Asset Hub, asset id one, **five billion units, fixed forever**. No inflation,
no mint path, no burn path.

Its owner, issuer, admin and freezer are all one keyless account derived from a pallet
identifier. **No seed produces it, so nobody holds it.** PEZ cannot be minted, force-frozen,
or destroyed, because there is no account that could sign it. Beyond that, a call filter
refuses `mint`, `burn`, `force_create`, `force_asset_status` and `start_destroy` for asset one
arriving over a cross-chain message — so even the relay's superuser cannot reach it.

| Allocation | Amount | Held by |
|---|---|---|
| Treasury + rewards pool | 4,812,500,000 (96.25%) | Keyless treasury pot |
| Founder | 93,750,000 (1.875%) | The founding account, locked |
| Presale | 93,750,000 (1.875%) | Presale custody |

The founder's PEZ is locked on the same schedule as the founder's HEZ and released to the
same gate described in §8.3: none of it moves before the state has the citizens it exists to
pay. The symmetry is complete rather than convenient — if the roll never reaches the
threshold, the founder's share stays locked as permanently as the citizens' does. That is the
point of binding the two to one latch instead of two schedules: the people who built this
cannot be paid by a state that never came into being. The presale's share is held by a custody account rather than a pallet because it is sold
and therefore has to move; it answers to a board rather than to a single key.

### 8.3 The halving

The rewards pool is not distributed by decision. It is released by arithmetic, monthly, and
the amount halves every forty-eight releases — approximately four years.

The first period releases half the pool across forty-eight months: about **50,130,208 PEZ**
per month. Release forty-eight pays half that, release ninety-six half again, and the amount
reaches zero when halving has consumed the last unit of the smallest denomination — around
the sixty-sixth halving, which is roughly two hundred and sixty years out. Each release is
derived from the release index rather than accumulated, so no drift is possible and no missed
release can be double-paid.

Every release splits the same way: **seventy-five percent to the incentive pot** (the
citizens' share, distributed by trust) and **twenty-five percent to the government pot** (the
state's budget). Nobody signs a release. Once the schedule is running it happens on block
initialisation, and no office can bring one forward or hold one back.

**But the schedule does not start at genesis. It starts at a hundred thousand citizens.**

Nothing is released — not the citizens' share, not the state's budget — until the register
reports that the roll has passed a hundred thousand. The report is automatic, it is made by
the chain that holds the register rather than by anyone who could be asked to make it, and it
latches: once crossed, a later fall in population does not stop the payroll, because a state
that stopped paying its citizens the month its population dipped would be worse than one that
never started.

The reason is arithmetic. The first month pays about fifty million PEZ. Divided among two
hundred citizens that is a founding distribution wearing a payroll's clothes; divided among a
hundred thousand it is what it says it is. The threshold is the point at which a single
month's share stops being large enough to be worth forging the register for — which is the
same security argument §4 makes, applied to the money instead of the roll.

Two consequences follow, and both are written into the chain rather than promised here. The
**founder's allocation is bound to the same gate**, so the first tokens that move for the
people who built this are not earlier than the first tokens that move for the people it was
built for. And if the threshold turns out to be wrong, a **ninety-day referendum of the
citizens already admitted may lower it once** — the same track that governs the register's own
rules, on the same reasoning: a number that could lock the pool forever should be answerable
to the people the pool belongs to, and to no office at all.

---

## 9. The four funds, and who may move them

This is the section the architecture exists for. Every fund is on the Asset Hub, and every
authority over the state's money is on the People chain. The single exception is the HEZ
treasury, whose authority is the economic franchise itself and therefore sits with the
holders — §9.2 says why that one is different. Read each row as a sentence: *this office
proposes, this body decides, this vault pays.*

### 9.1 The map

| Fund | Token | Who may propose | Who decides | How it pays |
|---|---|---|---|---|
| **Treasury** | HEZ | Network governance, by spender track, at five tiers from 250 to 1,000,000 HEZ | The referendum on that track | Payout within 30 days |
| **Airdrop pot** | HEZ | **The Prime Minister** | **The President** — and the **Treasurer** as a second signature above 1,000,000 HEZ, with a seven-day delay | Payout within 30 days |
| **Presale pot** | HEZ | **The Finance Minister** | **The Parliament**, by simple majority | Payout within 365 days, after the lock |
| **Government pot** | PEZ | **The Finance Minister**, bounded by the approved budget | The Parliament, when it passed the budget | Immediate transfer |
| **Incentive pot** | PEZ | No proposal — a citizen claims | The trust score, arithmetically | Immediate transfer |

### 9.2 What the vaults refuse

**A ceiling on one payment is not a ceiling on spending.** The airdrop pot pays on two
signatures below a million HEZ and needs the Treasurer above it, which bounded any single
payment and nothing else: a million could be moved, and then another million, until a
forty-million pot was empty and the office being skipped was the only one that would have
noticed. The pot now remembers. Recent payments drain away over a month rather than resetting
on a boundary — a window that resets is worth twice its ceiling to whoever reads the clock —
and once three million have moved inside one, the Treasurer signs and the week's wait applies
however small the next payment is.


Four of the five vaults name **exactly one chain** they will accept instruction from: the
People chain. Not the relay. Not root. Not a key. The airdrop pot, the presale pot and both
PEZ pots are configured with an origin that matches the People chain's location and has no
root arm at all — the arm was never built, which is a stronger statement than one that was
built and then disabled.

The consequence, with §2.1's caveat carried forward: **the relay's superuser cannot spend the
airdrop, the presale, or either PEZ pot without replacing the runtime that refuses it.** It
can halt the chain and it can reject a proposed spend, but there is no call it can make that
pays it. To move that money by an ordinary act it would have to become the People chain, and
the People chain is a register of elected offices.

The HEZ treasury is the exception, and it is the exception on purpose. Its five spender tracks
are conviction voting over HEZ — the economic franchise deciding an economic question — and
root is an additional arm above them. It is the fund of last resort, and the one place where
the network's own governance rather than the state's holds the purse. It is also the only
vault whose ceiling is a track rather than a chain, which is why it is the one a reader should
watch.

### 9.3 A payment, end to end

The airdrop path shows the whole shape:

1. The **Prime Minister** proposes an amount and a beneficiary.
2. The **President** approves. If the amount exceeds one million HEZ, the **Treasurer** must
   also sign, and the payment cannot execute for seven days after the last signature — a
   cooling period proportional to the size.
3. **Anyone** may then execute. Execution is permissionless because every discretionary
   decision has already been made and recorded; what remains is arithmetic, and arithmetic
   should not wait on a signature.
4. The People chain sends a message to the Asset Hub naming the pot, the beneficiary and the
   amount. The pot's origin check confirms the sender is the People chain.
5. The beneficiary collects, within the payout window.

Every step is an event. The proposal names its proposer, the approval names its approver, and
the amount is on the wire in the clear.

### 9.4 The budget

The government pot is not spent proposal by proposal. The Parliament passes a budget, which
credits an approved figure; the Finance Minister then spends against that figure and cannot
exceed it. This is the ordinary shape of public finance, and it is enforced by a bound rather
than by an audit after the fact.

### 9.5 The citizens' share

The incentive pot is distributed per epoch, weighted by trust: a citizen's share is their
trust score over the network's total active trust, times the epoch's pool. Ten percent of each
epoch is reserved for holders of role badges. Unclaimed rewards are clawed back after a week
so that the pool cannot silently drain into abandoned accounts.

No office signs a citizen's reward. It is claimed, and the arithmetic is the authority.

---

## 10. Security

**At the implementation layer**, the runtime is Rust compiled to WebAssembly, and upgrades are
forkless — a defect is patched by a runtime upgrade, not by asking the network to migrate.

**And a defect can be patched in hours, on two keys.** The ordinary route to the relay's root
is the twenty-eight-day referendum, which is the right speed for a constitutional amendment and
the wrong one for something being exploited — and the call that fixes an unknown defect cannot
have been approved in advance. So the court, at two thirds, may put a call hash on the relay's
whitelist, and a whitelisted call confirms in ten minutes. Neither key is enough alone: the
court cannot dispatch what it lists, and what it lists is public for the whole of its
confirmation. The court holds it rather than a ministry because this is the one authority that
has to be used while something is going wrong, and the body holding it must not be the body
most likely to be the reason.

**At the consensus layer**, block production and finality are separate mechanisms, so that a
chain that stops finalising still produces blocks and a chain that stops producing does not
finalise garbage. Equivocation and disputes are reported on-chain.

**At the economic layer**, slashing removes stake, and the removed stake becomes treasury
rather than vanishing.

**At the civil layer** — the one this design adds — misconduct costs standing. A banned
validator loses the committee seat, the reward weight, and the candidacy threshold that trust
confers. Trust is not unbuyable — a fifth of it is stake, and another quarter is badges that
institutions award — but the two largest paths into it, education and a record of vouching
that survived revocation, are paid for in time by definition. That is the property the
security rests on: not that standing cannot be bought, but that it cannot be bought
*quickly*, and an attacker who has lost it is starting from where everyone else started.

**And structurally**, the separations are real: the money is on a chain that only accepts
instruction from the register; the register is governed by a court that neither the president
nor the parliament can seat alone; the rules for admission to the register can only be changed
by a ninety-day referendum of the people already in it; and the relay's root can be reached
from exactly one place.

### 10.1 What is specified but not yet built

A design document that lists only what works is an advertisement. This section is the other
half, and it is written to be complete rather than reassuring: everything below is a gap a
reader could otherwise find in the code and conclude we had hidden. The chain is on a test
network precisely so that these close before a genesis freezes them — a structural fix costs
one commit today and a coordinated operation across every operator afterwards.

**Nothing is left in this half.** Every gap the previous edition listed as agreed-and-unbuilt
has landed; what remains below is the part nobody has decided.

**Closed since the last edition, and described where they belong.** Listed here so that a
reader who was given the previous version can see what moved: the emergency upgrade path (§10),
vacancy on silence and judicial reissue (§4, §5.4), the airdrop's memory (§9.2), the dormancy
rule, the candidacy bar as a share of the roll and free voting (§3.1) — and all nine validator
gates, which now read nine different things rather than six of them reading one (§7.1). Each is
a mechanism and not a promise; the sections named are where it is described, and the code is
what settles it.

**Open, and not yet decided.** These are judgements rather than engineering, and this document
will not pretend they are settled.

| Question | Why it is hard |
|---|---|
| **Whether membership proofs should be zero-knowledge** | No personal data is on chain — an identity is a hash and the documents behind it never leave the applicant. What *is* public, and permanently so, is the graph: who vouched for whom. Its protection is that an account is a number rather than a name, which holds until one node is identified and then unwinds outward along the vouches. Deleting the edges later would not help, because the vouch was an extrinsic and the blocks keep it. Only a zero-knowledge membership proof closes this, and that is a project rather than a change |

**And one thing that is not a gap.** There is no proof of personhood, by decision rather than
by omission. §4 says what the vouching chain does and does not buy. We would rather state a
weaker guarantee accurately than a stronger one loosely.

---

## 11. Heritage and independence

PezkuwiChain is built on the Polkadot SDK, and says so.

The framework — its consensus, its cross-chain messaging format, its runtime machinery — is
the work of Parity Technologies and the wider Polkadot community, released as free software.
That inheritance is not incidental; it is the reason a small institute could build a state
layer at all rather than spending a decade on a consensus engine. The debt is acknowledged in
every file: four thousand eight hundred and thirty source files carry a copyright line naming
Parity Technologies alongside the Dijital Kurdistan Tech Institute, and the files this project
has not modified carry Parity's alone.

What is ours is the layer above: the citizen register, the offices and their elections, the
courts, the trust computation, the validator pool, the treasuries and the authority chains
that reach them. Those are original work, and they are what this document describes.

Independence is technical as well as legal. PezkuwiChain is not a Polkadot parachain; it is a
sovereign relay chain with its own validators, its own token, and its own governance. It
shares an ancestry with Polkadot in the way two states may share a legal tradition — visibly,
and without either governing the other.

---

## 12. Licence and legal position

The project is free software, multi-licensed in the pattern its heritage requires:

| Layer | Licence |
|---|---|
| Framework libraries | Apache-2.0 |
| Node and runtime | GPL-3.0-or-later with the Classpath exception |
| Documentation examples | MIT-0 |
| Project templates | Unlicense |

The workspace default is GPL-3.0-or-later with the Classpath exception, and the repository
carries the full text of each licence it uses. Copyright is jointly attributed to Parity
Technologies (UK) Ltd. and the Dijital Kurdistan Tech Institute.

PezkuwiChain is a public infrastructure project of the **Dijital Kurdistan Tech Institute**.
HEZ and PEZ are utility tokens of a functioning network. They are not securities, not shares,
and not claims on the assets or revenue of any entity. Nothing in this document is an offer,
a solicitation, or investment advice.

---

## 13. Appendix A — Glossary

| Term | Meaning |
|---|---|
| **welatî** | Citizen; the holder of a citizen NFT |
| **tiki** | An office, role or badge recorded against a citizen |
| **Serok** | President |
| **Meclis** | Parliament |
| **Serokê Meclisê** | Speaker of the Parliament |
| **Dîwan** | Constitutional Court |
| **Serokwezîran** | Prime Minister |
| **Wezîr** | Minister |
| **Wezîrê Darayiyê** | Minister of Finance |
| **Xezinedar** | Treasurer |
| **perwerde** | Education |
| **qeyd** | The register, and the rules governing it |
| **teyrchain** | A system chain secured by the relay |
| **council** | The parliament's standing collective; its roster is written from the sitting Meclis |
| **escrow** | The relay-held mirror of the HEZ the Asset Hub carries; not supply, and excluded from turnout |
| **root** | A seat, not an office: the People chain's referendum, and a sudo key for the founding period |
| **stratum** | One of the nine pools the validator committee is drawn from, each gated by a different authority |
| **dormancy** | Having taken no part for two years; it removes a citizen from the support denominator and from nothing else |
| **geographic mark** | Which part of the nation a citizen belongs to — not where they live: claimed by the citizen, confirmed by a notary, cancellable by the court, and withdrawable by the citizen |
| **tenure** | Unbroken, offence-free membership of the validator pool; the one qualification no authority can grant |
| **Noter** | Notary; the office whose attestation the register relies on |
| **whitelist** | The relay's fast path: a call the court has listed can be enacted in hours instead of twenty-eight days |
| **wHEZ** | HEZ wrapped one-for-one as an asset so that asset-handling pallets can trade it |
| **wUSDT** | The custodial bridge's representation of USDT on the Asset Hub |
| **bizinikiwi** | The framework layer |
| **HEZ** | The native currency; 1 HEZ = 10¹² TYR |
| **TYR** | The smallest unit of HEZ |
| **PEZ** | The fixed-supply asset backing the citizens' reward pool and the state budget |

---

## 14. Appendix B — Figures at a glance

| | |
|---|---|
| HEZ genesis supply | 200,000,000 |
| Held on the Asset Hub / on the relay | 180,000,000 / 20,000,000 — less the validators' initial stashes, which are carved out of the treasury's share and minted on the relay, so the four allocations still sum to exactly 200,000,000 |
| HEZ inflation, default / ceiling | 8% / 10% of a fixed 200M base |
| PEZ supply | 5,000,000,000, fixed |
| PEZ halving period | 48 monthly releases (~4 years) |
| PEZ release split | 75% citizens / 25% state |
| PEZ schedule starts at | 100,000 citizens on the roll |
| Founder allocation | HEZ liquid at genesis, undertaken as launch capital for validator security; PEZ bound to the population gate in code |
| Presidential term | 4 years, maximum 2 consecutive |
| Parliamentary seats / term | 201 / 4 years (first term halved) |
| Constitutional Court | 11 seats — 6 elected, 5 appointed — 9 years |
| Register-rules referendum | 90-day decision period |
| Support denominator | The active roll, or 100,000, whichever is larger |
| Dormancy — leaves the denominator after | 2 years with no participation; one vote returns |
| Ayes needed to reach root | 2,000 |
| Citizens' initiative threshold | 1% of the roll |
| TNPoS committee | 9 strata × 3 seats = 27 |
| TNPoS quorum / halt / fork | 19 / 9 / 11 |
| Minimum eligible per stratum | 50 — except the court's, which is 3, its seat count |
| Independent gates measured today | 9 of 9 |
| Court seat vacated after | 180 days without a signature |
| Tenure — unbroken pool membership | 1 year; admits on trust until the chain is that old |
| Open lottery — trust floor | 40, exactly what the cheapest account scores |
| Infrastructure — sessions seated | 48 (≈8 eras of being drawn) |
| Infrastructure — co-failure | disqualified by 3 sessions in 90 days failing with more than 3 others |
| Geographic regions | 6 — Başûr, Bakur, Rojava, Rojhilat, diaspora, Caucasus |
| Geographic mark — referrals to apply | 25 settled |
| Airdrop — one payment / a month's payments | 1,000,000 HEZ / 3,000,000 HEZ before the Treasurer signs |
| Chains specified / running at genesis | 5 / 2 |
