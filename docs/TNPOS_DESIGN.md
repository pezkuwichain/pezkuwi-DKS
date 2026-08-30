# TNPoS — Design Specification

**Date:** 2026-08-28 · **Status:** approved, not implemented
**Supersedes:** `pezpallet-validator-pool` (deleted entirely, not patched)
**Target pallet:** `pezpallet-tnpos`

---

## 1. The claim — and the limit of the claim

TNPoS is a deterministic-finality consensus in which committee membership **cannot be bought
with capital**, is **sampled unpredictably**, and is **accountable**.

**The accurate sentence:**
> Deterministic finality and accountable security; committee membership is not bought with
> capital, it is earned; and the committee is sampled in a way that cannot be predicted in
> advance.

**The sentence not to use:** *"the energy efficiency of PoS + the security of PoW."* It does
not hold. PoW's security comes from an external, unforgeable cost, and in return it gives
probabilistic finality. The security of committee BFT comes from quorum intersection and
punishability, and in return it gives deterministic finality. These are different guarantees;
neither subsumes the other. An overclaim backfires at the first serious external audit.

The energy-efficiency claim is uncontested and can be stated on its own: 27 signatures per block.

---

## 2. Threat model

In scope (Serok decision, 2026-08-28):

| Adversary | Capability | What answers it in the design |
|---|---|---|
| **State actor** (TR/IR/SY/IQ) | Targeted DDoS, ISP cut-off, **physical arrest of a validator**, pressure on an institution | Committee unpredictability · pseudonymous membership · forward-secure keys · the geography stratum · automatic resampling |
| **Capital** | Unlimited money, no identity | Stratification: the buyable stratum is 1/9 of the total seats |
| **Sybil** | One human, many identities | Within-stratum floors · the identity/KYC layer is a load-bearing assumption |

**Derived, and therefore mandatory:** **internal collusion.** It cannot be placed out of
scope, because a state actor's cheapest move is not to arrest 27 people but to buy one
institution. The whole budget in Section 5 is built on this scenario.

**Out of scope:** network-layer identity leakage is not a protocol flaw; it is answered by
sentry nodes / Tor / infrastructure distribution, and it is **an operational requirement** —
cryptography alone is not enough (Section 13, R7).

---

## 3. Consensus parameters

| Parameter | Value | Rationale |
|---|---|---|
| Number of strata `k` | **9** | Section 5 |
| Seats per stratum | **3** | Equality between strata; no single power can approach a threshold on its own |
| Committee `n` | **27** | `k × 3` |
| Quorum `q` | **19** | GRANDPA's structural 2/3 threshold. Not changed |
| **Halt threshold** | **≥ 9 seats** | `n − q + 1` |
| **Fork threshold** | **≥ 11 seats** | `2q − n` |

**Resulting properties:**
- A single power: 3 seats → **harmless**
- Two powers: 6 seats → **harmless**
- Three powers: 9 seats → **can halt, cannot fork**
- Four powers: 12 seats → can fork

**17/21 was rejected.** Raising the quorum above 2/3 increases fork resistance but lowers
liveness; under the state-actor threat model it is unacceptable for the loss of five members
to halt the chain. GRANDPA's threshold is also structural — 17/21 would require modifying a
formally verified gadget. The threat model and the engineering constraint point to the same
place.

---

## 4. The nine powers

Each stratum's gate must belong to **a different source of authority**. This is the hidden
precondition of the mathematics: two strata whose gate is held by the same institution count
as **one stratum** in the arithmetic.

| # | Stratum | Gate | Independence rationale |
|---|---|---|---|
| 1 | **Stake** | Bonded HEZ, ranked internally on AH with Phragmén | The market. The only buyable stratum |
| 2 | **Meclis** | Elected parliamentary membership | Legislature |
| 3 | **Divan** | Court membership | Judiciary — ⚠️ correlation risk, see below |
| 4 | **Perwerde** | W3 University + **Caucasus University** (Tbilisi, accredited) | **A foreign institution outside the region.** The only gate no state in the region can reach — a structural asset |
| 5 | **Tiki** | Community-granted tikis | Social graph — ⚠️ office tikis must be excluded |
| 6 | **Welati lottery** | Citizenship alone; a draw among all citizens | Ancient Athenian sortition. It has no institution, and therefore no institution to capture. Its adversary is Sybil, not collusion |
| 7 | **Geography / diaspora** | Attestation of residence outside the region | An axis orthogonal to the other eight. **Written directly against the state-actor threat** |
| 8 | **Tenure** | Uninterrupted pool membership ≥ 120 eras (~30 days), with no offence | Time is not an institution. It cannot be bought, granted or forged — only waited out |
| 9 | **Independent infrastructure** | A measured uptime record + attestation of a distinct ASN/hosting | Technical merit; independent of every social and political gate |

### Two correlation risks — recorded, not hidden

**(a) Divan depends on Meclis and Serok.** The court is constituted by 5 Serok appointments
plus 6 Meclis appointments (see the separation-of-powers record). This does not make stratum
3 fully independent of stratum 2, and it **pushes the effective `k` below 9.** The fix is
constitutional, not code: making judicial appointment independent. Until it closes, the budget
calculation must count Divan as half a stratum.

**(b) Office tikis tie the Tiki stratum to Meclis.** The fix is code: **the 12 office tikis
are excluded from the Tiki stratum's eligibility criterion.** Only community-sourced tikis
count.

> **Where this exclusion lives matters** (measured during the Task 8 review): `pezpallet-tnpos`
> asks only `tiki_of` for the Tiki gate and looks at nothing else — that is the correct
> boundary. But it also means **the pallet does not enforce the exclusion; whoever implements
> `ScoreProvider` does.** So the independence of the nine gates rests not in the consensus
> pallet but in the **score bridge** (M7.1). If that bridge does not filter out office tikis,
> two strata silently become one, and all the probabilities in §5 start describing a different
> chain — without a single test breaking.

---

## 5. Security budget — measured, not assumed

A pure function inside `pezkuwi-tnpos-primitives::analysis`; multivariate hypergeometric
(3 seats per stratum, a convolution over 9 strata). It assumes 200 eligible members per
stratum and 4 eras/day. **200 is the table's modelling assumption; 50 is the hard floor for
seating a stratum (see below).**

**The scenario the budget rests on: one power is fully captured, and in every remaining stratum the adversary is assumed to hold 5% of that stratum's eligible members.**

| Scenario | P(halt)/era | P(fork)/era | Fork interval |
|---|---|---|---|
| Sybil 2%, in every stratum | 8.7e-10 | 6.5e-13 | ~10⁹ years |
| Sybil 5%, in every stratum | 3.2e-06 | 2.1e-08 | 32,600 years |
| Sybil 10%, in every stratum | 8.1e-04 | 2.5e-05 | 28 years |
| Sybil 20%, in every stratum | 7.3e-02 | 1.1e-02 | **< 1 year** |
| **1 power FULL + 5%** | **8.8e-04** | **1.2e-05** | **60 years** |
| 1 power FULL + 10% | 2.7e-02 | 1.6e-03 | **< 1 year** |
| 2 powers FULL + 5% | 8.4e-02 | 3.0e-03 | **< 1 year** |
| 3 powers FULL | 1.00 | 0 | halts, cannot fork |

**How to read it:** the design has a very wide margin against Sybil; the dominant risk is
institutional capture.

**Stratification buys nothing against a spread adversary.** If an adversary distributes 20%
of the 1800 eligible members (360) evenly across the nine strata, a flat unstratified sample
of 27 forks with probability 1.04e-2 per era — once every 96 eras. The table's own stratified
"Sybil 20%" row is 1.1e-02 — roughly once every 91 eras: almost the same number. Against an
evenly spread adversary the two designs say the same thing, because the mean is the same and
stratification only clips the variance.

What stratification actually buys appears when the adversary **concentrates** — which is also
the realistic scenario: capital can buy one stratum outright, and cannot buy the remaining
eight. `analysis.rs`'s own test (`stratification_bounds_a_concentrated_adversary...`) measures
this: when the same ninety-member adversary is gathered into a single stratum, the fork
probability is **exactly zero** — three seats is a ceiling, not a low likelihood. Under flat
sampling those same ninety people still have a real chance. The value of stratification is not
to shrink a probability but to make a concentrated adversary hit a deterministic ceiling.

**Halting and forking are not symmetric:** a halt can be recovered by automatic resampling, a
fork cannot. Automatic resampling in Phase 2 is therefore not an improvement but **a component
of the security budget.**

### The within-stratum floor

When 3 are drawn from `N` eligible members, the probability that an adversary with `a` members
takes all three seats of that stratum is `C(a,3)/C(N,3)`:

| N | a=3 | a=5 | a=10 |
|---|---|---|---|
| 20 | 8.8e-04 | 8.8e-03 | 1.1e-01 |
| 50 | 5.1e-05 | 5.1e-04 | 6.1e-03 |
| 100 | 6.2e-06 | 6.2e-05 | 7.4e-04 |
| 200 | 7.6e-07 | 7.6e-06 | 9.1e-05 |

**The minimum number of eligible members is 50 per stratum**; below that the stratum is not
seated (Section 7.1).

### The proof is a runtime constraint, not a document

> The pallet **refuses to open an era** with a configuration that exceeds the security budget.

It is enforced by `integrity_test` (compile time) and `try_state` (run time). This applies the
"the constitution is code, the policy is in storage" pattern from AH `staking.rs` to the
security budget. If a stratum cannot carry its quota safely it is **not seated** — it does not
quietly become unsafe.

---

## 6. Architecture

### 6.1 Crate boundaries

| Crate | Responsibility | Why separate |
|---|---|---|
| `pezkuwi-tnpos-primitives` | Types, score traits, the security mathematics **as pure functions** | Tested without a runtime. **Closes P-1**: the score traits are today copied byte for byte across four pallets |
| `pezpallet-tnpos` | Pool, eligibility, the 9 strata, quotas, score cache, committee handover, slashing | The core |
| `pezpallet-tnpos-sortition` | Ring-VRF ticket implementation | **Critical boundary:** it sits behind the `Sortition` trait. Phase 1 ships with a simple implementation; in Phase 2 ring-VRF is plugged in without touching the core. The weight of `ark-vrf` enters here and nowhere else |
| `pezpallet-tnpos-people-client` | Packages the scores on the People chain and sends them to the relay over XCM | Exactly the `staking-async-rc-client` pattern — already proven in this repository |

The `Sortition` trait boundary is the most important engineering decision in this design: it
is what lets the phasing proceed without a rewrite.

### 6.2 Where it lives

- **People:** the pool, eligibility, the strata, sampling, and the committee — together with
  the identity and credential sources they read (trust, tiki, perwerde, referral,
  staking-score). It sits here because the scores it draws on are local: on the relay, five
  of them arrived by XCM and could go stale independently, and three-of-five stale seats a
  committee whose stratum proportions are silently wrong — worse than five-of-five, which
  seats nobody
- **AssetHub:** the **internal** ranking of the stake stratum — the existing `staking-async` +
  `MultiBlockElection` — and the payment. The nominator, exposure and slashing machinery is
  left untouched; what changes is the **list it is fed** (decision of 2026-08-29: People
  elects, AssetHub pays, the relay validates)
- **Relay:** validates with the committee it is handed, through `ah_client`. No TNPoS code
  runs here and the pallet has **no `SessionManager` implementation** — one was written while
  it lived on the relay and removed on 2026-08-29, because a dead implementation reads as a
  claim that the committee seats itself somewhere

### 6.3 The three rings

1. **The eligibility ring.** Every member joining the pool registers a bandersnatch key; the
   pool forms a *ring* (≤1024; consistent with a `MaxPoolSize` of 1000). The ring is public.
2. **The ticket window.** In the first half of an era, eligible members produce a ring-VRF
   ticket for the next era and submit it to the chain **by anonymous relay through another
   member**. The ticket says *"I am a member of this ring and my VRF output is this"*; it does
   not say which member. The stratum is in the clear in the ticket body; the identity is not.
3. **Sampling.** At the era boundary each stratum's tickets are ordered by VRF output and 3
   are taken. The VRF output can neither be chosen nor computed in advance — unbiasability and
   unpredictability come from this, and no separate beacon is needed.

**The resulting property:** a committee member appears with **a fresh session key** that has
never been linked to their identity. The chain knows with proof that these are "27 legitimate
pool members"; it does not know which 27 people — neither in advance nor afterwards. No target
list can be produced, because there is no list.

### 6.4 Measured constraints

- Ring-VRF proof verification is **~11 ms/ticket**; 27 members = ~300 ms. Spread over an era
  it is negligible; gathered into a single block it exceeds the block budget → **a submission
  window is mandatory**
- Rebuilding the ring verifier key takes **~50 ms** (domain 1024) — once per era
- The `bandersnatch-experimental` flag is **not enabled in any runtime**; it will have to be
- `sc-consensus-sassafras` **does not exist** (neither here nor upstream). BABE stays. Ring-VRF
  is used only inside the runtime, for committee selection — **zero changes on the node side**

---

## 7. Degradation and recovery

### 7.1 If a stratum shrinks

- ❌ **Seats are not transferred between strata.** A constitutional invariant — it would mean
  producing with our own hands exactly the concentration we defend against
- ❌ Halting the chain: unacceptable for a state chain
- ✅ **The committee shrinks**, `q` is recomputed as 2/3 of the actual size, and the security
  constraint is **re-run for the shrunken configuration**. Below the hard floor (**≥ 15 members
  / ≥ 5 strata**) the emergency set defined at genesis is seated and a governance alarm is
  raised

### 7.2 Liveness recovery

The principle: *recovery must not depend on the thing it protects.* Two distinct cases, never
conflated:

- **If finality stalls (GRANDPA):** BABE keeps producing blocks and the runtime keeps running
  → if finality falls N blocks behind, **the runtime resamples the committee with a fresh
  seed.** Automatic; governance is not in the loop
- **If block production stops (9+ members offline):** nothing runs, and on-chain recovery is
  impossible. **Off-chain procedure:** the fallback set defined at genesis plus a governance
  path, **rehearsed**. It is solved by being written down, not by being discovered on the day

---

## 8. Scores crossing the boundary

**(a) A stale score may not be used at its old value.** Every cached score carries
`last_updated`; a score older than 4 eras (~1 day) counts as **expired** and is treated as
**ineligible** rather than at its last value — fail-closed. The cost of a silently stalled
subscription has been measured before; here it hands over consensus.

**(b) The `staking_score` oracle is a bot today, and TNPoS makes it consensus-critical.**
Staking data crosses relay→People through a notary/bot, not through a cryptographic proof.
**Phase 0's blocking item:** Phase 1 does not start until this moves onto an XCM/proof path or
onto M-of-N attestation.

---

## 9. Slashing — how an anonymous member is punished

**Stake stratum:** the existing `staking-async` slashing, unchanged.

**The other eight strata — a two-layer penalty:**
1. **A small participation bond.** It covers the cost of spam/DoS; it cannot be large enough to
   be a barrier, or the capital gate returns through the back door
2. **Trust itself is the slashable asset.** An offence → a trust penalty + a pool ban (24 eras
   for minor, 360 eras for severe) + revocation of office/tiki in the severe case. For someone
   whose standing is their asset this deters more than money does, and it is the penalty
   consistent with the chain's philosophy

**The anonymous member's bond (Phase 3):** the bond is locked not to an identity but to **a
commitment belonging to the ticket**; an anonymous escrow bound to a nullifier. If an offence
is proven the escrow is forfeited to the treasury — the member loses their money **without
ever being identified**. Forfeited, not burned: the bond is HEZ and HEZ inflates, so destroying
it would spread the penalty across everyone still holding some rather than funding the state.
The forfeit has to become something the state can spend. The mechanism is Semaphore's nullifier; in production on Ethereum.
*Anonymity is not impunity.*

---

## 10. Deleted — what does not carry over from `validator-pool`

| Deleted | Why |
|---|---|
| **All of shadow mode** | It never ran: the `SessionManager` implementation was not wired into any runtime, and `new_session` was never called even once. Its metrics were fabricated (`tnpos_total_stake: 0` constant, `project_tnpos_blocks` a made-up model). Replaced by: real deployment on Zagros + `try-runtime` |
| The declared-stake check | `min_stake` was an argument supplied by the caller; the real balance was never read. Zero-cost consensus capture |
| Selection by hash order | The first entries from `PoolMembers::iter()` were taken and shuffled *afterwards*; trust and stake were never used in the ordering |
| The history-based rotation rule | VRF sampling gives rotation for free and unbiasably. The old rule locked the pool and put `on_initialize` into a silent loop that retried it every block |
| The `reputation ≥ 70` gate | Replaced by stratum eligibility + the offence record |

---

## 11. Validation plan

1. **Property test** (proptest) — over the pure mathematics, on random stratum configurations
2. **Monte Carlo** — 10⁶ eras, several adversary models; the empirical capture rate is compared
   against the analytic bound. If they disagree, **the model is wrong**
3. **Formal specification** (Quint or TLA+), machine-checked invariants:
   seats are not transferred between strata · the committee does not fall below the floor · no
   era opens with an expired score · recovery always terminates
4. **≥ 3 months on Zagros**, with the security constraint live
5. **A paid external audit:** the ring-VRF integration **and** the identity/KYC layer
6. **Benchmarks are measured on the CI runners**, not on WSL

---

## 12. Phases

| Phase | Content | Acceptance criterion |
|---|---|---|
| **0 — precondition, blocking** | Moving the `staking_score` oracle onto a cryptographic path · the People→Relay score XCM channel · alignment with the genesis reset schedule | The channel is live, the oracle is not a bot |
| **1 — core** | primitives + pool + the 9 strata + quotas + the security constraint + degradation + committee export to the validating chain + stratum-specific slashing + a commit-reveal seed (**unpredictable, but biasable by withholding** — see the note below). **No** ring-VRF | Deployed to Zagros, `try_state` green |
| **2 — hardening the sortition** | Ring-VRF behind the `Sortition` trait · **a real SRS** (today `new_testing()`) · sub-rounds within an era · automatic finality recovery | The committee is unpredictable; the recovery drill passed |
| **3 — anonymity** | An anonymous escrow bond bound to a nullifier · pseudonymous pool membership · **forward-secure ephemeral participation keys** (the lesson taken from Algorand: a captured key cannot re-sign the past) | The state-actor threat model is answered |
| **4 — R&D, in parallel** | A SAFROLE / `sc-consensus-sassafras` port, **Zagros only** | No mainnet commitment |

---

## 13. Risks and open items

| # | Risk | Status |
|---|---|---|
| R0 | **Phase 1's commit-reveal seed is NOT unbiasable.** It is unpredictable (nobody can know it before the reveal window closes), but a participant who **withholds** their contribution changes the outcome: an adversary holding k accounts gets 2^k choices of seed. An early draft said "unbiasable"; that was wrong. Three things make this defensible for Phase 1: contribution is limited to pool members (the grinding set is no larger than the pool), an unopened commitment is **visible and identifiable on chain**, and — correcting myself — withholding **cannot be punished automatically** in Phase 1: the `Offence` enum has no variant corresponding to withholding and there is no on-chain detection mechanism; the only route is for `ManagerOrigin` to report it by hand and map it onto `Offence::Unavailable`, whose own documentation says "seated but did not vote", not "withheld". The real answer is Phase 2: in ring-VRF withholding has no counterpart, because the seed does not depend on a contribution | **Phase 2 is mandatory before mainnet** — Phase 1 is for Zagros only |
| R1 | `bandersnatch-experimental` **is not running on any production chain**. The cryptography (bandersnatch, ark-vrf) is academically sound; the *integration* has no field record | The risk Phase 2 carries. A paid audit is mandatory |
| R2 | The `staking_score` oracle is a bot; TNPoS makes it consensus-critical | **Phase 0, blocking** |
| R3 | **Sybil resistance is the load-bearing assumption of the whole model.** If the identity layer collapses, all nine strata collapse with it | KYC must be audited harder than the consensus pallet |
| R4 | **The code guarantees that no stratum exceeds 3 seats; it cannot guarantee that the nine powers are genuinely independent** | A constitutional matter. The whitepaper must say so as well |
| R5 | Divan is constituted by Meclis+Serok appointment → effective `k` < 9 | Awaiting a constitutional fix; until then Divan counts as half a stratum |
| R6 | No real SRS has been obtained; genesis uses `RingContext::new_testing()` | Phase 2. The Ethereum KZG ceremony transcript is a candidate — to be verified |
| R7 | The network layer (IP) can leak identity; cryptography alone is not enough | The sentry/Tor infrastructure policy counts as part of the design |
| R8 | **On Zagros six of the nine strata go through `trust_of`, and `StubScores::trust_of` returns the same fresh value for every account.** So the deployed Phase 1 configuration has not nine effective independent gates but **one** — the central assumption of the security budget in Section 5 (nine independent powers) is never exercised on the testnet. A green Zagros is NOT evidence of stratum independence; what makes `k = 9` real is the genuine score channel the People-chain bridge will bring (M7.1) | Open until the People-chain bridge lands; must close before mainnet |
