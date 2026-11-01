# Verify report: odd-lite

## Round 1

Commit 5fc7659, branch `chore/odd-lite`, base `develop` (ffb0836). Mode light, depth V1, `mutation: off`, tier T0, `decided-by: owner`. Rules: `develop` version of `verify.md`, as the coordinator asked. BLOCKED did not arise: every critical check ran.

Verdict: PASS. CRITICAL 0, WARNING 4, SUGGESTION 6.

## CRITICAL

None.

## WARNING

All four concern instructions that future folderless or L0 work will hit. None breaks an AC as written, and none affects `layout-core`, which keeps its folder.

- W1. class: `implementation`. A folderless FAIL does not survive the session. `coordinator.md:84-85` and `verify.md:35` put the report and `verification.json` under the scratchpad, and the entry keeps only `{round, commit, verdict, next}`. A later session therefore cannot do three things: `verify` cannot "check that each earlier finding is closed" (`verify.md:16`), the coordinator cannot route the findings, and it cannot apply "two consecutive FAILs of the same class" (`coordinator.md:166`, `AGENTS.md:107`), an owner stop that needs the classes of past rounds. `product-specs/SKILL.md:72` also expects a durable `verify-report.md` with the covered IDs. ADR 0011 lists a thinner audit trail but not this resume gap, and the D3 pilot keeps a folder, so it will not exercise it. Suggested fix: add the open findings and their classes to the entry's `verify` field. Fix before the first folderless run.
- W2. class: `implementation`. Two statements on lowering pack defaults collide. `coordinator.md:20` says a level can "never lower a pack default (only the owner does)", and `coordinator.md:34` says the same. `coordinator.md:63` and `AGENTS.md:90` let the coordinator set `mutation: off` "even where the pack says archive". The specific exception can be read as winning, but the general rule has no carve-out. The same gap applies to depth and agents: the "heaviest of" rule names only the mode, while the L0 row fixes V0 and one writer. An agent-definition change is L0 by its signals but V1 by its pack. A Rust style change is L0 but V2 with T4 under Small refactor. No floor is affected. Suggested fix: state that the level never lowers pack depth or agents, and that `mutation: off` is the single exception.
- W3. class: `implementation`. Product ID traceability has no clear home on the new default path. `verify.md:31` (3c) holds the `git grep <ID>` check, the proof of each ID's criterion, and the classes for non-compliance, but it is "Fast lane only" and written around the `slice card` commit. `verify.md:32` (3d) says "the same checks" and then only "every criterion has a test or command". `AGENTS.md:139` and `coordinator.md:107` still require test naming for outcome entries. Suggested fix: make 3d name the ID check and the finding classes.
- W4. class: `implementation`. The ADR overstates what stayed the same. `docs/adr/0011-odd-lite.md:46` gives the check "tables unchanged" for modes, packs, floors, gates, tiers and depths, and line 38 says "floors, gates and owner stops unchanged". The floors and packs tables have no diff, but G0 and G4 changed and the Spec and Runs cells of the `AGENTS.md` modes table changed. `apply-progress.md` states the G0 and G4 exception correctly. The edits are additive, so nothing is lost; only the ADR's check is false as written.

## SUGGESTION

- S1. `coordinator.md:84`: "Only a `verify` PASS marks a criterion `passed`", but L0 runs V0 with no `verify` agent, so an L0 entry can never reach `passed`. It is harmless today because the entry is deleted (`coordinator.md:122`). Say how V0 evidence counts.
- S2. `product-specs/SKILL.md:9` and `:78`, plus `references/change-template.md:3`, still give the Slice card as the default. Line 55 overrides this for section 4 only. `AGENTS.md`, `coordinator.md`, `specifier.md`, `tdd-apply.md`, `verify.md`, `archive.md` and `explorer.md` agree on where acceptance lives.
- S3. `coordinator.md:122` deletes a no-archive entry "on the last branch before its merge", but `coordinator.md:148` runs T4 after that merge. An L1 fix with `mutation: archive` and no product IDs loses its entry before a possible T4 survivor. Delete it after T4.
- S4. ADR 0011 nits. In "Rollback", step 3 must happen before steps 1 and 2 (it says "first" but is listed last). `cargo-deny` is a step of the `policy` job (`ci.yml:219`), not a job.
- S5. `mutants.md:14` says `verify` returns BLOCKED when a T4 cannot run, but the route at `coordinator.md:148` calls `verify` after T4 only for a survivor. G5 still stops the close. Name the verify step for this case.
- S6. `tdd-apply.md:15` step 1 does not list the outcome entry as a checklist source. "The delivery's last commit" (step 6, `verify.md:25`) is unclear after a fix round, and an interrupted delivery has no notes yet.

## Note, not a finding

- Size: 311 insertions, 52 deletions, 13 files from `6e549ed` to `6513da4` (re-measured). That is above the ~300 light signal; the mode is owner-decided. The measure excludes the brief (352), the Light spec (18) and the progress notes (26, 337 with them).
- Known and accepted: `docs/specs/README.md` "Cómo continuar el documento en curso" and `docs/manuals/tecnico/chapters*/10-methodology.md` stay stale. ADR 0011 names both (lines 92 and 124), and both paths exist.

## Gates

No file in `git diff --name-only develop...HEAD` matches `.rs`, `Cargo.*`, `.toml`, `.cargo`, `deny` or `.github` (grep exit 1).

| Gate | Result |
|---|---|
| `cargo fmt`, clippy, test | not applicable, no Rust or Cargo file changed |
| `cargo deny` | not applicable, same reason |
| CI policy | not applicable, no step scans the changed paths |
| Disk | 67 GB free; nothing built |

## Artifact integrity (step 3b)

- `2c134a7` touches only `openspec/changes/odd-lite/apply-progress.md`, and it precedes every delivery commit, so G0 holds.
- Only two commits touch that file: `2c134a7` and `5fc7659`. `git diff 2c134a7 HEAD` removes no line, so the block is intact. There is no amend commit.
- `8ae0d51` touches only `.odd/state.json`.
- No commit carries AI attribution.

## Criteria

| AC | State | Evidence |
|---|---|---|
| AC1 | covered | ADR "Inventory" has brief §3 items 1 to 7; "Disposition" has the required columns |
| AC2 | covered | See "Rule preservation" below. "ODD lite" section at `AGENTS.md:5` |
| AC3 | covered | `coordinator.md:19-27`; floors and pack tables have no diff; see "Floors and stops" below |
| AC4 | covered | `AGENTS.md` "Change folder rule"; `coordinator.md:29`; L3 and full keep a folder |
| AC5 | covered | `git ls-files` lists the file, `git check-ignore` exit 1, `json.tool` exit 0; resume at `coordinator.md:15-17`; `layout-core` keeps its folder (`AGENTS.md:44`) |
| AC6 | covered | `verify.md:59` and `:61`, step 3d; `coordinator.md:78` and `:148`; schema stays 1, the `blocked` field is optional, PASS and FAIL text kept |
| AC7 | covered | ADR "Brief names"; 9 agent files present; `mutants.md:14`, `coordinator.md:63` |
| AC8 | covered | ADR status and index `proposed`; "Retained flows"; "Rollback"; "Metrics" with "not measured" |
| AC9 | covered | Forbidden-path grep exit 1. The only `/home/` in added lines is the AC9 evidence text naming the pattern, not a path. No user name, `$HOME` or `C:/` |

### Rule preservation (AC2)

Word diff: 10 removed fragments in `AGENTS.md` and `CLAUDE.md`, matching `apply-progress.md`. An independent sentence-level comparison against `develop` covered `AGENTS.md`, `CLAUDE.md`, and the coordinator, verify, tdd-apply, specifier, archive, mutants, explorer and `product-specs` files. It found 45 changed sentences and read each one: all are reworded in place to add the no-folder alternative, BLOCKED or `off`. No safety, architecture, TDD, testing, delivery or Git rule is lost, including Strict TDD, Code rules, Integration, the PR rule and G0 "not waivable".

### Floors and stops (AC3)

- Mode is the heaviest of floor, level and pack default, so no level goes below a floor.
- The G0 to G5 stops, owner stops, loop limits, Integration rules and escalation table all remain. G0 and G4 only gained text.
- The one rule that relaxes is intended and stated: L0 and L1 with no critical code may use `mutation: off` (ADR Decision 4 and amendment to ADR 0001). T4 and hand mutants stay on critical code.
- Weak spots are W1 (same-class stop after a resume) and W2 (depth).

### Internal consistency

| Topic | Result |
|---|---|
| Acceptance without a folder | consistent, except W3 and S2 |
| G0 with an outcome entry | consistent: `coordinator.md:74` and `:83`, `AGENTS.md:88`, `verify.md:29` and `:32` |
| Output without a folder | consistent, followable within one session; W1 and S6 across sessions |
| `layout-core` keeps its folder | consistent: `AGENTS.md:42-44`, `coordinator.md:29` and `:107`, ADR, `product-specs/SKILL.md:55` |

`openspec/config.yaml`, `architect.md` and `manual-writer.md` are unchanged and contradict nothing.

### `.odd/state.json`

Valid JSON, 1261 bytes, 37 lines. All field values fit the `AGENTS.md` schema. `e5e5576` and `ffb0836` exist, and D1 to D7 match the Slice card. `spec_id` of `odd-lite` names the brief, not a spec; the schema allows it. Only IDs and status are held, so nothing repeats the Slice card or Light spec.

### Repo facts re-counted (ADR 0011)

| Claim | Recount |
|---|---|
| 10 archived changes, 3 to 12 files, median 8, 135 to 4598 lines | confirmed |
| Three fast-lane-era changes, 3 files each, 135 to 352 lines | confirmed |
| `layout-core` 7 files, 1121 lines | confirmed |
| 911 `#[test]` in 85 files; 8 crates, 1 app | confirmed |
| 9 agents, 5 skills, symlinks, no commands, no hooks, no nested instructions | confirmed |
| D1 two rounds; D2 FAIL, FAIL, PASS | confirmed (git log, `verification.json`) |
| ADR 0001 item 3 and section 6; ADR 0002 sections 3 and 6 anchors | confirmed |
| CI jobs include `cargo-deny` | not exact, see S4 |

## Hand mutants and mutation

V1, `mutation: off`: none run, no report, not a finding.

next: archive
