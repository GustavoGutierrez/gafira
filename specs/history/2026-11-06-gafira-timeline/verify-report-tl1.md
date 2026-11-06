# Verify report: gafira-timeline — TL1 (retroactive round 1)

## Current state

Round 1 of delivery `TL1`, **retroactive**. `TL1` was merged into `develop` as `a47ce03`
without a verify round, and this report was written afterwards, against a detached worktree
at that merge. It **did not gate the merge** and cannot: the delivery was already in
`develop` when it ran, and two later deliveries were built on it. What it can do is make the
archive's PASS claim true, or replace it with what the merge hid.

Commit `a47ce03` (`Merge branch 'feat/gafira-timeline-tl1' into develop`), depth **V1** (the
Slice card says `verify: V1`), mode `full` (the card's Flow line), tier **T0**,
`mutation: off` (owner, card). No `mutants` report exists and none is owed; the T4 row is
`null`.

Verdict: **FAIL** — 1 CRITICAL, 2 WARNING, 2 SUGGESTION.

Per class: ambiguity 1 (1 CRITICAL), implementation 2 (2 WARNING), architecture 0, claim 0.

The single CRITICAL is not a code defect in the narrow sense: the delivery implements its
Slice card exactly. It is a **contradiction in the requirement artifacts**. The card's AC3
and AC5 require the new field to be omitted when `None` and cite `ADR 0006 rule 3`, and that
rule says the opposite ("Every field is always written. No `skip_serializing_if` in
`gafira-doc`"). The delivery added the repository's only `skip_serializing_if`, the ADR was
not amended, and the ADR's own enforcing test (`page_wire_form_writes_every_field`) still
passes because its fixture never sets a duration. The owner must reconcile the two; the code
is the smaller half of the fix.

`next: specifier` (C1, ambiguity), then `tdd-apply` for W1 and W2.

## Findings

| id | severity | class | ref | summary |
|---|---|---|---|---|
| C1 | CRITICAL | ambiguity | `crates/gafira-doc/src/page.rs:47`; `docs/adr/0006-format-evolution-before-first-release.md:21`; `apply-progress.md:8` (AC3) | Card AC3/AC5 require the field omitted when `None` and cite ADR 0006 rule 3, which forbids `skip_serializing_if` in `gafira-doc` outright. The delivery ships the repo's only one; the ADR is unamended; `page_wire_form_writes_every_field` stays green because its fixture has no duration. |
| W1 | WARNING | implementation | `crates/gafira-doc/tests/visit_contract.rs:47,244` | `TIMELINE_SOURCES = ["page.rs"]` skips the whole file, so `Instant`, `SystemTime` and `Duration` are now legal in the domain's largest source. ARQ-136's row allows only `Flicks`/`FrameRate`, and only in five named types. Probe G2 below. |
| W2 | WARNING | implementation | `crates/gafira-doc/src/page.rs:48`; `design.md:456` | The design's critical-code rule "a page duration is a `Flicks` value ≥ 0" has no validation and no test. `Flicks` is documented as *signed*; `"duration":-5` loads and re-saves unchanged. |
| S1 | SUGGESTION | ambiguity | `design.md:396,463` | The design still says the crate-wide scan is "removed" and that "all seven" goldens move. The delivery kept a scoped scan and moved one golden (recorded in the notes). TL2/TL3 read this design. |
| S2 | SUGGESTION | implementation | `.github/workflows/ci.yml` V5 step | The V5 step checks tree→row only, never row→tree. This delivery's "both directions" check is manual and CI cannot reproduce it. |

```json
{ "schema": 1, "change": "gafira-timeline", "delivery": "TL1", "rounds": [ {
  "round": 1,
  "commit": "a47ce03",
  "mode": "full",
  "tier": "T0",
  "depth": "V1",
  "verdict": "FAIL",
  "next": "specifier",
  "counts": {"critical": 1, "warning": 2, "suggestion": 2},
  "classes": {"ambiguity": 1, "implementation": 2},
  "findings": [
    {"id": "C1", "severity": "CRITICAL", "class": "ambiguity",
     "ref": "crates/gafira-doc/src/page.rs:47; docs/adr/0006-format-evolution-before-first-release.md:21; apply-progress.md:8",
     "summary": "Card AC3/AC5 require the field omitted when None and cite ADR 0006 rule 3, which forbids skip_serializing_if in gafira-doc; the ADR is unamended and its enforcing test stays green.",
     "follow_up": false, "status": "open"},
    {"id": "W1", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-doc/tests/visit_contract.rs:47,244",
     "summary": "TIMELINE_SOURCES = [page.rs] skips the whole file: Instant, SystemTime and Duration are now legal there, while the ARQ-136 row allows only Flicks/FrameRate in five named types.",
     "follow_up": true, "status": "open"},
    {"id": "W2", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-doc/src/page.rs:48; specs/changes/gafira-timeline/design.md:456",
     "summary": "The design's rule that a page duration is a Flicks value >= 0 has no validation and no test; Flicks is signed and \"duration\":-5 loads and re-saves unchanged.",
     "follow_up": true, "status": "open"},
    {"id": "S1", "severity": "SUGGESTION", "class": "ambiguity",
     "ref": "specs/changes/gafira-timeline/design.md:396,463",
     "summary": "The design still says the crate-wide scan is removed and that all seven goldens move; the delivery kept a scoped scan and moved one golden.",
     "follow_up": false, "status": "open"},
    {"id": "S2", "severity": "SUGGESTION", "class": "implementation",
     "ref": ".github/workflows/ci.yml (V5 step)",
     "summary": "The V5 step checks tree->row only, never row->tree; the delivery's both-directions check is manual and not reproducible by CI.",
     "follow_up": false, "status": "open"}
  ],
  "gates": {"fmt": 0, "clippy": 0, "test": 0, "deny": 0, "ci_policy": 0},
  "mutation": null,
  "hand_mutants": {"run": 0, "killed": 0},
  "criteria": {"total": 7, "covered": 7},
  "size": {"lines": 278, "forecast": 200},
  "report": "verify-report-tl1.md" } ] }
```

## What this round proves, and what it does not

V1 is the gates at HEAD plus the map of each acceptance criterion to a test or a command.
This round goes a little past V1 because the incident asks it to settle the record: it
re-ran the RED commit, and it ran five probes on a **scratch export outside the repo**
(`/tmp/tl1-probe`, `git archive` of `HEAD`; `/tmp/tl1-red`, `git archive` of `aee6122`).
The probes are negative controls of the guard and of the wire, not an independent hand-mutant
list of the delivery's code, which V1 does not owe and `mutation: off` does not ask for. The
implementer's own H1–H5 were not re-run.

Not done here, and why:

- No T4 or T3 mutation: `mutation: off` for this delivery (owner, card Flow line).
- No per-commit re-gate. The notes record per-commit gates (`apply-progress.md:29-35`); this
  round re-ran the gates at the merge commit only. A retroactive round cannot reconstruct
  the state of a commit that was pushed and then built on.
- The full workspace suite is not owed by V1. It was run anyway (ADR 0017 asks for
  `--workspace` when a delivery touches a `Cargo.toml`, `Cargo.lock` or `ci.yml`, which this
  one does) and it is green; see the gate table.

## Gates at HEAD (`a47ce03`)

`df -h /` first: 159 GB free. `CARGO_TARGET_DIR=/tmp/gafira-tl1-v1-target`,
`CARGO_INCREMENTAL=0`, cleaned at the end.

| gate | command | exit | evidence |
|---|---|---|---|
| fmt | `cargo fmt --check` | 0 | clean |
| clippy | `cargo clippy -p gafira-doc --all-targets --locked -- -D warnings` | 0 | 15.4 s, cold |
| test | `cargo test -p gafira-doc --locked` | 0 | 20 test targets + `Doc-tests gafira_doc` = 21 binaries, **271 passed, 0 failed, 0 ignored** |
| deny | `cargo deny check` | 0 | advisories ok, bans ok, licenses ok, sources ok |
| CI policy | the nine `policy` steps of `.github/workflows/ci.yml`, run literally by `/tmp/gafira-policy-run.py` | 0 | all nine `exit=0`; script re-checked against this tree's `ci.yml` (it reads `jobs.policy.steps`, skips the toolchain install, and every step's `run:` is self-contained bash with no `${{ }}` or matrix value) |
| clippy (workspace, extra) | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | ADR 0017 scope for `Cargo.lock` + `ci.yml` |
| test (workspace, extra) | `cargo test --workspace --locked` | 0 | 194 targets, **2 427 passed, 0 failed, 0 ignored** |

The nine policy steps, by name: Format, CLI has no UI dependencies (ARQ-07), Domain crates
forbid unsafe code, Domain crates respect the layer table (V5), Domain clippy.toml copies are
in sync (V12), Layers 0-2 know no language (ARQ-107), EditToken only in the edit crates
(V13), Every core command has a property test (V14), Testkit ports feature compiles (ADR
0013). `cargo-deny` is an action in `ci.yml`, not a `run:` step, so it is the `deny` row.

## Criteria

| AC | proved by | would it fail without the code? |
|---|---|---|
| AC1 (ARQ-01, V-5) edge declared, crate builds | `tests/duration.rs:50` `arq_01_the_crate_declares_the_gafira_time_edge` (manifest via `include_str!`); the build; the V5 step | **Yes, observed.** At `aee6122` (test-only commit) the assertion fails against a manifest without `gafira-time.workspace = true`. |
| AC2 (V-2, ARQ-136) `Option<Flicks>` of `gafira-time` | `tests/duration.rs:117` `arq_136_the_duration_is_a_flicks_of_gafira_time`: `let none: Option<Flicks> = page.duration;` and `page.duration.map(Flicks::ticks)` | Not as a failing test: the field does not exist before the GREEN commit, so its RED is AC1's manifest assertion — which is what the notes say. The typed binding is the guard; an `Option<i64>` does not compile. |
| AC3 (ARQ-10, ARQ-13) absent in the wire when `None`; old document loads | `tests/duration.rs:66` `arq_13_a_page_without_a_duration_writes_no_field` (byte equality with the old bytes, `!resaved.contains("duration")`, a fresh `Page`); `tests/duration.rs:89` `arq_10_a_page_duration_round_trips_through_the_wire_form`; `tests/layout.rs` `adr_0006_old_bytes_resave_in_the_canonical_form` | **Partly observed.** The round trip fails at `aee6122` (`assert_eq!(resaved, DOC_WITH_DURATION)` drops the field). `arq_13` passes before the change too — as its own note says — and it is the guard against `null` or a default (implementer's H1/H2). |
| AC4 (V-5) every crate that reaches `gafira-time` has it in its row; step passes | the V5 policy step; `cargo tree -e normal,build --target all` per row | **Yes — recorded, not re-run here.** The notes record eight `::error::<crate> must not depend on gafira-time` lines, exit 1, from the manifest-without-rows state; that state was never a commit (edge and rows landed together). Implementer's H4 drops one row and the step fails. |
| AC5 (ARQ-10) the byte-pinned golden moves for that reason | `src/json.rs:278` `full_golden_equals_the_built_document` (`assert_eq!(d.to_json().unwrap(), golden)`, fixture sets the duration at `json.rs:338`) | **Yes.** The golden and the fixture changed together; without the field the built document cannot carry the token the golden now has. |
| AC6 (ARQ-02, ARQ-107) `forbid(unsafe_code)`, V5, V12, ARQ-107 | four policy steps | No RED by design: they pass before and after, which is their point. All exit 0. |
| AC7 (ARQ-10, ARQ-13) equality and canonical order | `tests/duration.rs:101` `arq_10_the_duration_takes_part_in_the_equality_of_the_document`; order pinned by AC3's byte round trip | **Yes, observed.** At `aee6122` the two documents compare equal and `assert_ne!` fails. |

7 of 7 covered. The two criteria whose RED is not a failing test (AC2, AC6) are recorded as
such in `apply-progress.md` and the reason holds: one is a type rule with no runtime
observable, the other is a "still holds" command.

## The wire: absent, not `null`

Probe `crates/gafira-doc/examples/wire_probe.rs` on the scratch export at HEAD (not
committed), printing bytes:

- `Page::new(...)` with `duration: None` → the JSON has **no `"duration"` key at all**; the
  substring `"duration"` does not occur. Not `"duration":null`.
- the same page with `Some(Flicks::from_ticks(3_528_000_000))` → `"duration":3528000000`,
  at offset 412, `"locked"` at 434: after `grid`, before `locked`, the order §3.1 gives.
- a document written before the field existed (`OLD_BYTES`, the old wire form) loads and
  re-saves **byte for byte** (`OLD_RESAVE_EQUALS_INPUT=true`), still with no key.

So the absent-versus-`null` question is answered on the bytes, not on the Rust value, and an
old document loads. The mechanism is `#[serde(default, skip_serializing_if =
"Option::is_none")]` at `page.rs:47-48` — which is exactly what C1 is about.

## The eight V5 rows

`git diff 4f79cb1..HEAD -- .github/workflows/ci.yml` changes exactly eight rows, all adding
`gafira-time`: `gafira-doc`, `gafira-edit`, `gafira-testkit`, `gafira-pathops`,
`gafira-layout`, `gafira-text`, `gafira-raster`, `gafira-segment`. `gafira-render-cpu`
already listed it, and the notes say why (its tree reaches `gafira-time` through
`gafira-render`).

Checked against `cargo tree -p <crate> -e normal,build --target all` for **every** row, in
both directions:

- every crate whose tree reaches `gafira-time` lists it in its row (14 rows besides
  `gafira-time`'s own `-` row; the V5 step enforces this direction);
- no row lists a `gafira-*` crate its tree does not reach (the step does *not* enforce this
  direction — S2; the manual check is clean).

The step passes at this tree. The delivery's stated reason for eight rather than the design's
nine is correct.

## The ARQ-136 guard

`visit_contract.rs` now keeps the five names, keeps the exact `MeasureCx` assertion
(`page` and `dpi`, no time field) and the module-coverage loop, and skips the sources listed
in `TIMELINE_SOURCES` (`:47`, today `["page.rs"]`) from the crate-wide name scan (`:244`).

Three controls on the scratch export at HEAD:

| # | control | result |
|---|---|---|
| G1 | a real `gafira_time::Flicks` parameter added to `src/document.rs` | **caught**: "the domain names the time type Flicks in src/document.rs, outside the data of the timeline" (`visit_contract.rs:252`), test FAILED |
| G3 | a new `src/probe_module.rs` declared with `mod probe_module;` and not added to `DOMAIN_SOURCES` | **caught**: "src/lib.rs declares the module probe_module, which the list of sources does not cover: add it" (`:238`), test FAILED |
| G2 | `std::time::Instant` named in `src/page.rs`, the skipped file | **not caught**: the test passes (5 passed), which is W1 |

**Judgement: right for `page.rs`'s field, coarser than the requirement, and not narrowed
merely to let the delivery through.** The narrowing was necessary — `Page.duration:
Option<Flicks>` cannot exist while `Flicks` is banned in `page.rs` — and it is authorized:
the ARQ-136 row was rewritten to say exactly this in `4f79cb1`, the immediate parent of the
delivery's first commit, from the decision in `design.md` P10 ("a spec change, not a
test-only change"). The delivery also went *further* than the design's P10, which wanted the
crate-wide scan dropped; the delivery kept it scoped, which is stricter.

The gap is the granularity: the row allows `Flicks` and `FrameRate`, and only "where the data
of the timeline lives — `Page.duration`, `Sequence`, `Track`, `Clip`, `TimeRange`". The guard
allows all five names anywhere in the whole file. `Instant`, `SystemTime` and `Duration` are
the ones the row still bans outright, and they are now legal in `page.rs`, the domain's
largest source. G2 is not hypothetical: `std::time::Instant::now()` in `page.rs` passes the
only test that enforces ARQ-136. W1, `implementation`, follow-up.

What the guard does keep, and what this round confirms: a new source cannot slip in unseen
(G3), and a time type outside the timeline data is caught (G1).

## The golden

`tests/golden/full.json` moved by one line and one token: `,"duration":3528000000` inserted
in page 2, after `grid`, before `locked`. It is **the required consequence of the new field**:
the fixture at `src/json.rs:338` sets the same value on the same page, and the assertion
compares the whole serialized document to the golden.

It was **edited by hand**, not regenerated: `gafira-doc` has no bless mechanism
(`GAFIRA_BLESS_GOLDEN` exists only in `gafira-anim/tests/golden.rs`). The reason is recorded
(`apply-progress.md:71-76`) and the edit is safe because three tests pin the bytes —
`src/json.rs:278` (the built document equals the golden), `tests/golden.rs`
`canonical_goldens_round_trip_byte_for_byte`, and `tests/serde.rs:107`
(`assert_eq!(out, FULL)`) — while `tests/layout.rs` compares the old `pre_layout/full.json`
bytes to the golden with the key removed, and `tests/properties.rs` only feeds the golden as
input. All of them pass. The other six goldens did not move, which is the
consequence of omitting the field when `None`, not an omission of the golden update. AC5 is
satisfied; its disagreement with `design.md:463` ("all seven move") is S1 and C1.

## Artifact integrity

- The card commit is `5220b6f docs(gafira-timeline): open the change with the TL1 slice
  card`, and it touches only `specs/changes/gafira-timeline/apply-progress.md`.
- No delivery commit changed the card: for each of `aee6122`, `d023416`, `8a5b71c`,
  `93172c8`, `b0f6c44`, `526595b` the diff adds no line to the `Flow:`, `Product IDs`,
  `Scope:`, `Acceptance:`, `Plan:` or `Owner decisions` blocks (checked commit by commit).
  The three notes commits touch `apply-progress.md` only.
- The ARQ-136 product row was rewritten in `4f79cb1`, one commit before the delivery's first
  commit, not after it; it is the design's P10 decision and it is what makes the guard's
  narrowing legal. Recorded here as context, not as a finding.
- `arq_137_the_domain_depends_on_no_crate_above_it` drops `gafira-time` from its ban list.
  That is correct: ARQ-137 bans edges to crates **above** the domain, `gafira-time` is layer
  0, and ADR 0007 names the edge as expected. The test still bans `gafira-render`,
  `gafira-media`, `gafira-layout` and `gafira-ports`.

## Product IDs in the card

`ARQ-01` (`duration.rs:50`), `ARQ-02` (AC6's four policy steps), `ARQ-10` (`duration.rs:89`,
`:101`, the golden), `ARQ-13` (`duration.rs:66`, `:89`), `ARQ-136` (`duration.rs:117`,
`visit_contract.rs`). `ARQ-30` has no AC of its own in the card; it is named by the existing
`crates/gafira-edit/tests/ui.rs` (a `Document` cannot be mutated outside `History`), and this
delivery does not touch the write path, so it is covered by the existing suite rather than by
this delivery. Every ID the card names has a named proof or a command.

## Sizes

`git diff --shortstat 4f79cb1..HEAD`: 11 files changed, 278 insertions, 17 deletions. The
notes' own measurement (`apply-progress.md:120-123`) matches exactly, of which this file
(`apply-progress.md`) is 95 insertions and the delivery's code and tests 183. The design's
forecast is 200 (280 with tests); measured 278 is inside it and far below the ~700 ceiling.

## Risks this round leaves

1. C1 is a format rule, not a preference: the archive's unqualified PASS for `TL1` rests on
   a delivery that contradicts an accepted ADR. Until the owner decides, every later
   delivery inherits the question (TL3 writes sequences and assets to the same wire).
2. W1 leaves a hole in the only test that enforces "the domain owns no clock". It is cheap to
   close (scan `page.rs` for `Instant`, `SystemTime`, `Duration` and allow only `Flicks` and
   `FrameRate`), and cheap to forget.
3. W2 means a negative page duration is representable and persists. No AC asks for the
   check, but the design's critical-code row does.
4. This round did not re-gate per commit. The merge commit is green; the intermediate commits
   are taken from the notes.

## Next

C1 is the blocker: `next: specifier` — reconcile the Slice card's AC3/AC5 with ADR 0006 rule
3. Either the ADR is amended to permit omitting an `Option` field (defensible: `None` is the
only default and it cannot drift), in which case the owner's decision is recorded and the
card's citation is corrected; or the card is corrected and the field is always written, in
which case all seven goldens move and `page_wire_form_writes_every_field` regains its teeth.
The code follows whichever is chosen.

After that: `tdd-apply` for W1 (tighten the guard to the row's granularity — per-type or a
per-name allowlist inside `page.rs`) and W2 (a validation and a test for a negative
duration, or an explicit decision to defer it to TL3). S1 and S2 are documentation/tooling
follow-ups that do not block.

**What the archive should say.** It cannot claim a bare PASS for `TL1`. The honest record is
one CRITICAL of class `ambiguity` on the wire rule, two open implementation WARNINGs, seven
of seven criteria proved at the gates, and a note that this round ran after the merge, on
`a47ce03`. The delivered behaviour is the card's behaviour and is sound; what is missing is
the ADR amendment that makes it legal.

next: specifier

---

# Verify report: gafira-timeline — TL1 round 2 (closure)

```json
{ "schema": 1, "change": "gafira-timeline", "delivery": "TL1", "rounds": [ {
  "round": 2, "commit": "24318330665328153398c5e4cd3be8637aada94c", "mode": "standard", "tier": "T0", "depth": "V1",
  "verdict": "PASS", "next": "archive",
  "counts": {"critical": 0, "warning": 2, "suggestion": 3},
  "findings": [
    {"id": "C1", "severity": "CRITICAL", "class": "ambiguity",
     "ref": "docs/adr/0006-format-evolution-before-first-release.md:21-39,46-48",
     "summary": "Closed: rule 3 now permits only an Option<T> whose None is semantic, omitted and never null, listed with its reason; Page.duration is the entry; the two tests exist and a mutation of either side is killed.",
     "follow_up": false, "status": "closed"},
    {"id": "W1", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-doc/tests/visit_contract.rs:43-58,120-230,391-441",
     "summary": "Closed: the guard refuses the three clock types in every source; std::time::Instant in src/page.rs now fails with `src/page.rs names the time type(s) [\"Instant\"]`. Residual holes are the new W3 and W4.",
     "follow_up": false, "status": "closed"},
    {"id": "W2", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-doc/src/page.rs:67-86; src/layout.rs:35,49; tests/duration.rs:136",
     "summary": "Closed: \"duration\":-5 fails to load with the existing OutOfRange { field: DurationTicks, value: -5.0 }; 0 loads as Some(Flicks(0)); check_layout refuses -1 and Document::from_parts rejects it.",
     "follow_up": false, "status": "closed"},
    {"id": "W3", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-doc/tests/visit_contract.rs:137-194,182-194",
     "summary": "code_only reads a '\"' char literal as a string opener and strips the rest of the file: `const Q: char = '\"'; fn f(_: Instant)` yields []. No such literal exists in src/ today, so no clock is hidden now.",
     "follow_up": true, "status": "open"},
    {"id": "W4", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-doc/tests/visit_contract.rs:120-134,363-383",
     "summary": "A type reachable only through a string is invisible: include!(\"clock.rs\") and #[serde(with = \"...\")] yield []; include! also bypasses the module-coverage loop, so the file it adds is never scanned.",
     "follow_up": true, "status": "open"},
    {"id": "S1", "severity": "SUGGESTION", "class": "ambiguity",
     "ref": "specs/changes/gafira-timeline/design.md:396-397,463-464",
     "summary": "Untouched: the design still says the crate-wide name scan is removed, that the wire writes the duration always, and that all seven goldens move — the amended rule 3 and the code say the opposite.",
     "follow_up": false, "status": "open"},
    {"id": "S2", "severity": "SUGGESTION", "class": "implementation",
     "ref": ".github/workflows/ci.yml, step 'Domain crates respect the layer table (V5)'",
     "summary": "Untouched: the V5 step still checks tree->row only, never row->tree; the row->tree direction stays a manual check.",
     "follow_up": false, "status": "open"},
    {"id": "S3", "severity": "SUGGESTION", "class": "implementation",
     "ref": "docs/adr/0006-format-evolution-before-first-release.md:25-26; src/page.rs:48",
     "summary": "Rule 3's third part — the field is listed in the ADR — has no test: a new Option field with an unlisted skip_serializing_if would pass every gate and leave the goldens untouched.",
     "follow_up": false, "status": "open"} ],
  "gates": {"fmt": 0, "clippy": 0, "test": 0, "deny": 0, "ci_policy": 0},
  "mutation": null,
  "hand_mutants": {"run": 3, "killed": 3},
  "criteria": {"total": 7, "covered": 7},
  "size": {"lines": 447, "forecast": 280}, "report": "verify-report-tl1.md" } ] }
```

## Round 2 — TL1 closure

- Delivery `TL1`, round **2**, a **closure round**. The fix branch is cut from `develop` at
  `316d247` (`docs(adr): rule 3 says what the wire does, with one listed exception`), the
  coordinator's amendment of `C1`; verified at the branch tip
  `24318330665328153398c5e4cd3be8637aada94c`, worktree `/tmp/gafira-tl1-fix`, branch
  `fix/gafira-timeline-tl1-verdict`, clean. Four commits over that base: `04ebea9` (`W1`, the
  guard), `418b300` (`W2`, the range), `73690c1` (the amended ADR's present side), `2431833` (the
  record).
- Mode `standard`, depth **V1**, `mutation: off`, tier `T0` as the card records (inert here: no T4
  is owed and none is read). No independent mutant budget was spent; the three reproductions below
  are the spot-check and the two ADR mutations, not a delivery mutant list.
- Verdict: **PASS** — `C1`, `W1` and `W2` are closed, every gate exits 0, and the two new WARNINGs
  (`W3`, `W4`) are follow-up test gaps in the guard, not a clock in the domain today. `S1`–`S3` are
  suggestions. Per class: implementation 2 (both follow-up), ambiguity 0.
- Toolchain: cargo/rustc 1.93.1. Disk at start: 156 GB free.
- This file carries round 1 verbatim from the copy written in `/tmp/gafira-tl1-v1` (the change
  folder had not been copied to `develop` when that round ran) and appends this round; the file did
  not exist in this branch.

## Gates (branch tip, run once)

| Gate | Exit | Evidence |
|---|---|---|
| `cargo fmt --check` | 0 | no diff |
| `cargo clippy -p gafira-doc --all-targets --locked -- -D warnings` | 0 | `Finished dev profile in 9.96s` |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | extra, for `gafira-edit` and the testkit that consume the crate |
| `cargo test -p gafira-doc --locked` | 0 | 23 test targets (21 `tests/` files, the lib unit target, `Doc-tests gafira_doc`); 24 `test result` lines because the doctest target reports twice; **300 passed, 0 failed, 0 ignored**; `tests/visit_contract.rs` **7/7**, `tests/duration.rs` **6/6** |
| `cargo deny check` | 0 | `advisories ok, bans ok, licenses ok, sources ok` |
| CI policy, job `policy` | 0 | 9/9 steps exit 0 by name: Format; CLI has no UI dependencies (ARQ-07); Domain crates forbid unsafe code; Domain crates respect the layer table (V5); Domain clippy.toml copies are in sync (V12); Layers 0-2 know no language (ARQ-107); EditToken only in the edit crates (V13); Every core command has a property test (V14); Testkit ports feature compiles (ADR 0013) |
| `cargo test --workspace --locked` | 0 | 200 test targets, **2 522 passed, 0 failed, 0 ignored** |

The extractor `/tmp/gafira-policy-run.py` was re-checked against this tree's `ci.yml` before use:
the `policy` job has 12 steps, 10 with a `run:`; the extractor skips `Install toolchain
(rust-toolchain.toml)`, the `cargo-deny-action` step is covered by the `cargo deny check` row, and
the nine remaining checks ran literally under `bash -euo pipefail`.

Build hygiene: `CARGO_TARGET_DIR=/tmp/gafira-tl1-v2-target` for the worktree and
`/tmp/tl1r2-target` for the scratch export (`/tmp/tl1r2-export`, `git archive` of `HEAD`), both
with `CARGO_INCREMENTAL=0`, both released with `cargo clean --target-dir` when the round ended.

## Closure table

| Id | Severity | Class | Status | Evidence |
|---|---|---|---|---|
| C1 | CRITICAL | ambiguity | **closed** | Rule 3 amended at `316d247`; `Page.duration` is the listed entry; both sides have a test, and a mutation of each side is killed (§C1). |
| W1 | WARNING | implementation | **closed** | The guard partitions the five names and refuses the three clock types in the four timeline sources; the round-1 hole is reproduced and now refused (§W1). |
| W2 | WARNING | implementation | **closed** | `-5` fails with the crate's range error, `0` loads; the check is in `Page::check_layout`, which `Document::validate`, `from_parts`, `History` and `gafira-edit`'s `check_pages` share (§W2). |
| S1 | SUGGESTION | ambiguity | open | `design.md:396-397` still says the crate-wide scan is removed and `:463-464` that the field is always written and all seven goldens move. Now also contradicts the amended rule 3. |
| S2 | SUGGESTION | implementation | open | The V5 step still checks one direction only; no commit of the fix touches `ci.yml`. |
| W3 | WARNING | implementation | open, follow-up | `code_only` is blinded by a `'"'` char literal (new, §W1). |
| W4 | WARNING | implementation | open, follow-up | A type reachable only through a string (`include!`, `serde(with)`) is invisible (new, §W1). |
| S3 | SUGGESTION | implementation | open | Rule 3's "listed here" part has no test (new, §C1). |

## C1 — the ADR now describes the wire

Rule 3 (`docs/adr/0006-format-evolution-before-first-release.md:21-27`) now reads: every field is
always written **except an `Option` whose `None` says something**; no `skip_serializing_if` in
`gafira-doc` for any other kind of field; the exception has three parts — typed `Option<T>`, a
**semantic** `None` that is not a default a reader could reconstruct, and **listed in the ADR with
its reason**; an excepted field is **omitted** when `None` and **never written as `null`**. The
Consequences (`:46-48`) require a **two-sided** test: absent when `None`, present and in canonical
position when `Some`, never `null`. That is exactly what the round asked for. `Page.duration` is
the single row of the table (`:37-39`), with the reason ("a page either has a duration or plays for
as long as its content; `null` would claim a duration the page does not have").

The claim is true of the tree: `skip_serializing_if` occurs **once** in `gafira-doc` and nowhere
else in the workspace (`src/page.rs:48`), and it is the listed field, so the amended rule describes
the code and nothing else.

The two tests exist and are two-sided — proved by mutation on the scratch export, not by reading:

| Mutation | `page_wire_form_writes_every_field` (absent side) | `adr_0006_a_present_duration_sits_between_grid_and_locked` (present side) |
|---|---|---|
| A: `skip_serializing_if` removed, so the field is always written and `None` becomes `null` | **FAILED** | passed |
| B: `skip_serializing_if = "skip_always"`, so the field is never written | passed | **FAILED** |

So `null` and "always" are caught by the absent side, "never" by the present side, and the pair is
not one test wearing two names. The absent side has a second witness one level up,
`tests/duration.rs:69 arq_13_a_page_without_a_duration_writes_no_field`, which compares the whole
document's bytes. The present side pins the whole page's bytes, the exact `,"duration":705600000,`
run after the grid's closing brace and before `locked`, `!json.contains("null")`, and the round
trip.

**Can the next field be added without re-reading the argument?** Yes. An author adds a table row
with the field, its source and why `None` is semantic, keeps `#[serde(default,
skip_serializing_if = "Option::is_none")]` and adds the pair of tests. The three conditions are
mechanical and the exception does not have to be re-litigated. One part is unenforced, though — the
ADR itself does not test that a field with a skip is **listed** (S3).

## W1 — the guard

The mechanism (`tests/visit_contract.rs:43-58,120-230`): one list of five names is partitioned into
`CLOCK_TYPES` (`Instant`, `Duration`, `SystemTime`) and `TIMELINE_TYPES` (`Flicks`, `FrameRate`),
and a test asserts the two lists are exactly the five. In a `TIMELINE_SOURCES` file (`page.rs`,
`timeline/{clip,sequence,track}.rs`) only the three clock types are banned; every other source bans
the five. The search runs on `code_only` — line comments, nested block comments, string literals and
raw strings stripped, `r#type` kept as code — and `names_type` splits on every non-identifier
character, so the field `duration`, the identifier `durations` and `DurationTicks` are not the
type.

**The round-1 hole, reproduced end to end.** On the export, `#[cfg(any())] fn probe_time_type(_now:
std::time::Instant) {}` appended to `src/page.rs`:

```text
test arq_136_the_measure_context_carries_no_time ... FAILED
src/page.rs names the time type(s) ["Instant"], outside what ARQ-136 allows there
```

It was the exact failure round 1 could not produce. The probe was removed; the export is scratch.

**Attacking the new mechanism** (probe inputs to `time_violations("page.rs", …)`, run on the
export; `page.rs` is a timeline source, so the ban is the three clock types):

| Attack | Result |
|---|---|
| a type named in an attribute path — `#[some_macro(Duration)]` | **caught** `["Duration"]` |
| a type named only in an attribute string — `#[serde(with = "std::time::Duration")]`, `#[doc = "Duration"]` | **missed** `[]` |
| a macro body — `macro_rules! m { () => { std::time::Instant::now() } }` | **caught** `["Instant"]` |
| a doc-test string — `/// ``` … let _: std::time::Duration … ``` ` | **missed** `[]` |
| a nested block comment — `/* a /* Duration */ b */` then `Instant` after it | comment stripped, `["Instant"]` after it caught |
| a closed comment pair — `/* /* */ */` then `Instant` | **caught** `["Instant"]` |
| a raw string — `r##"a "Instant" b"##` | stripped `[]` (correct: it is prose) |
| `type Duration = i64;` | **caught** `["Duration"]` |
| a fully-qualified path split across lines — `std::time::\n    Instant` | **caught** `["Instant"]` |
| a raw identifier — `fn f(r#Instant: u8)` | **caught** `["Instant"]` |
| `include!("clock.rs");` | **missed** `[]` |
| a char literal then code — `const Q: char = '"'; fn f(_: Instant)` | **missed** `[]` |
| a byte char literal then code — `const Q: u8 = b'"'; fn f(_: Duration)` | **missed** `[]` |
| a byte string and an escaped quote — `b"Instant"`, `"a \"Instant\""` | stripped `[]` (correct) |

**What it still misses, and whether it matters.** Judged against ARQ-136's purpose — a clock in the
domain — and not its letter:

- **Doc-test strings and names in doc text.** Missed, and it does not matter: a doc test is not the
  library, cannot add an item, and a `std::time::Duration` in an example is bad style, not a clock
  the domain carries.
- **Names that exist only in a string path** (`#[serde(with = "…")]`, `include!("…")`). Missed, and
  this one is worth a warning (`W4`): `include!` adds a whole file the scan never reads and the
  module-coverage loop never sees, so "a new source cannot slip in unseen" has an exception. A
  `serde(with)` path to a clock serializer in another crate is the same class, lower risk. Neither
  is in the tree.
- **A `'"'` (or `b'"'`) char literal anywhere above the clock** (`W3`). The stripper takes the
  literal's quote for an opening string and drops the rest of the file, so the guard silently stops
  checking below it — a one-character, idiomatic edit disables the only enforcement of ARQ-136 for
  that file. No such literal exists in `src/` today (`grep` over the crate finds none), so nothing
  is hidden now; the finding is that the guard's silence is indistinguishable from its agreement.
- **`Flicks` and `FrameRate` are allowed file-wide in the four timeline sources**, while ARQ-136's
  row says "only where the timeline's data lives — `Page.duration`, `Sequence`, `Track`, `Clip`,
  `TimeRange`". Still coarser than the letter for the two **data** types; round 1 judged that
  acceptable under the owner's Q6 and this round agrees. It cannot hide a clock, which is what the
  row is for.

Everything that puts a clock into **compiled library code by naming it** — bare, aliased,
qualified, split across lines, in an attribute path, in a macro body, after a nested comment, as a
raw identifier — is caught. **Verdict on the mechanism: it serves the purpose, and it is a
selector over text, so it is limited by not being a parser.** The two follow-ups are the honest
boundary of that choice.

## W2 — the negative duration

Probe on the export, on the canonical bytes of the card's five-second document with the token
replaced:

```text
-5 load => Display: invalid document: page 1: duration is -5, out of range
           Debug: Invalid(PageLayout { page: PageId(1), error: OutOfRange { field: DurationTicks, value: -5.0 } })
0  load => Ok(Some(Flicks(0)))
check_layout(-1) => Err(OutOfRange { field: DurationTicks, value: -1.0 })
check_layout(0)  => Ok(())
from_parts(-1)   => Some(PageLayout { page: PageId(1), error: OutOfRange { field: DurationTicks, value: -1.0 } })
```

- **`-5` fails, `0` loads**, and the error is the crate's **existing** range error:
  `PageLayoutError::OutOfRange`, reported as `ValidationError::PageLayout` — no new error type.
  Zero is a duration (`Some(Flicks(0))`), not an absence (`None`).
- **The check is where a page's other ranges are.** `Page::check_layout` (`src/page.rs:67`) runs
  bleed, margins, guides, grid, then `check_duration` (`:77-86`). The load reaches it through
  `Document::validate` (`src/validate.rs:97`), which `from_json` runs, and `Document::from_parts`
  validates too, so an invalid document cannot be constructed through the public API.
- **`gafira-edit` reaches it**, by two paths: `History::with_budget` calls `doc.validate()`
  (`crates/gafira-edit/src/history.rs:66`) and `debug_valid` asserts the same after every command
  (`:309-316`); `check_pages` calls `page.check_layout` (`set_property.rs:207-220`) and guards the
  DPI commands (`:367`, `:380`). The record's "so a negative duration set through a command is
  refused there too" is forward-looking: there is no command that sets a duration yet, and
  `check_pages` is on the DPI path only. The substance holds — the shared `check_layout` is the
  entry and the validation is not a load-only rule.
- **`PageField::DurationTicks` does not leak the banned identifier.** `src/layout.rs` is outside the
  timeline sources and is scanned against all five names; `names_type` reads whole words, so
  `DurationTicks` is not `Duration`. The guard passes on the real file, which is the check rather
  than the trust: the only time-looking strings in `gafira-doc/src` are this variant and its
  `"duration"` display. The name also reads correctly — it is named for the unit `Flicks::ticks()`
  returns, the message is the user's `duration`, and the doc says "in flicks". **Verdict: the
  naming choice is sound**, and it is the cheaper half of keeping the guard an identifier rule
  rather than a special-case list. (The smell is real — a name chosen partly so a test passes — but
  the name is also the one a reader would pick for a field in `PageLayoutError`, and it costs no
  exception in the guard.)

## Artifact integrity (steps 3, 3b, 3c, 3d)

- `5220b6f docs(gafira-timeline): open the change with the TL1 slice card` touches only
  `specs/changes/gafira-timeline/apply-progress.md` (1 file, +28). Round 1 reached the same
  conclusion independently.
- The fix's diff over `apply-progress.md` contains **no removed line**, so the card block (`Flow:`,
  `Product IDs`, `Scope:`, `Acceptance:`, `Plan:`, `Owner decisions`) is unchanged; `2431833` is a
  pure append of the record. No implementer commit changed the card.
- Commit scopes: `04ebea9` → `tests/visit_contract.rs` only; `418b300` → `src/layout.rs`,
  `src/page.rs`, `tests/duration.rs`; `73690c1` → `src/page.rs` only; `2431833` →
  `apply-progress.md` only. Nothing exceeded its delivery.
- `316d247` (the `C1` amendment) touches only `docs/adr/0006-…` (1 file, +24/−2) and is an ancestor
  of `develop`; no commit after it touches `docs/adr`.
- Size: `git diff --shortstat 316d247 HEAD` → 5 files, **447 insertions, 20 deletions** — 313
  code, tests and guards (215 the guard, 58 the range, 40 the present side) and 134 the record.
  Against the TL1 card's 200 (280 with tests) it is ×1.12 of the ×1.4 forecast, and far under the
  ~700 ceiling.
- Local `develop` is **one commit ahead of `origin/develop`**: `316d247` is not pushed. Pushing it
  is the coordinator's step, and the fix branch is built on it.

## Criteria

The closure round re-maps nothing: the card's 7 ACs were mapped in round 1 (7 of 7 covered), and
this round adds no acceptance criterion. `criteria` stays 7/7 as round 1 left it. The two findings
closed here are the card's own record (`W2` is the design's "a page duration is a `Flicks` ≥ 0")
and the requirement guard (`W1` is ARQ-136); neither changes the map.

## Observations (not findings)

- **The design still argues the old wire rule.** `design.md:463-464` says the duration "writes it
  always" and that all seven goldens move; `:396-397` says the crate-wide scan is removed. The
  amended rule 3 and the code say the opposite on both. This is `S1`, already open from round 1 and
  now also a contradiction with the ADR the change just amended; if the archive keeps the design as
  the record, the specifier should correct those two paragraphs first.
- **`PageLayoutError` is now a page-field error**, not a layout one; the docs of the enum and of
  `OutOfRange.value` were widened and the record states the deviation. No new error type, and
  `gafira-edit`'s mapping is unchanged. The `ValidationError::PageLayout` name stays narrower than
  what it now carries — a naming debt the record accepts, not a defect.
- **The guard is a test, not a lint.** Nothing prevents a future `skip_serializing_if`; `S3` is
  about the same shape (`the field is listed`).
- **`-5` is echoed as `-5.0`** because `OutOfRange.value` is `f64`. The renderer prints `duration is
  -5, out of range`, so the user does not see the `.0`. Fine for flicks up to 2^53.
- **The record's RED evidence was not re-run**; the reproductions here are the closure of the two
  warnings (the guard hole and the `-5` boundary), which is what the round owes.

## Next

`C1`, `W1` and `W2` are closed, every gate exits 0, and the change's three deliveries were verified
PASS (TL3's last round already routes to the archive). This delivery is done.

`W3` and `W4` are follow-up test gaps in the guard (`implementation`, follow-up): neither hides a
clock today and neither blocks. `S1` (the stale design), `S2` (the one-direction V5 step) and `S3`
(the unenforced "listed" part of rule 3) are suggestions; `S1` is the one worth doing before the
archive, because it is the record of the change and it contradicts the ADR that just made the code
legal.

next: archive
