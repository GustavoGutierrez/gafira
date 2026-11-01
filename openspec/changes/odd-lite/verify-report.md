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

# Verify report: odd-lite, delivery `chore/odd-lite-phase-d`

## Round 1

Commit 235d0ce, branch `chore/odd-lite-phase-d`, base `develop` (d3148bf). Mode light, depth V1, `mutation: off`, tier T0, no `mutants` report. Acceptance source: Light spec, AC10 to AC14, with AC2 to AC9 checked for regression. BLOCKED did not arise: every critical check ran.

Verdict: PASS. CRITICAL 0, WARNING 4, SUGGESTION 7. Per class: implementation 4.

Finding ids continue from the phases A and B round, so W1 to W4 keep their meaning.

## CRITICAL

None.

## WARNING

All four are gaps in rule text that the first folderless run will hit. None breaks an AC as written, and none weakens a mode, gate, floor or owner stop.

- W5. class: `implementation`. `verification.json` cannot carry what the entry must copy from it. `coordinator.md:85` and `AGENTS.md:68` copy `follow_up` from that file and close findings by the closure verdict, and `coordinator.md:151` says to read that file, not the long report. Its schema (`verify.md:44-57`) has no follow-up mark and no per-id closed or open result; `verify.md:16` only says to "report" them. Worse, no rule says whether a finding still open in a closure round is re-listed in that round. `findings` ids must be unique across rounds and `classes` holds the classes of "its" findings, so a round 2 FAIL with only a still-open C1 can yield a row with no class, and the two-FAIL stop would not fire from `rounds`. Within a session the coordinator can read the report; after a session loss the row stays wrong. Suggested fix: add optional `follow_up` to findings and a `closure` list of `{id, status}` to the round (schema stays 1), and count still-open findings in `classes`.
- W6. class: `implementation`. Resume needs an id that a new session may not hold. G0 commits the entry on the outcome's branch (`coordinator.md:83`) and a FAIL never reaches `develop`, so `.odd/state.json` on `develop` has no such entry. `coordinator.md:17` and `AGENTS.md:73` start from "branches matching the outcome id". A session started on `develop` and told only "continue" has no rule that yields the id. The match is also a substring: outcome `foo` matches `feat/foo-bar-d1`, and "newest commit touching that file" can then pick another outcome's branch. AC11 holds for a named outcome. Suggested fix: list all unmerged branches when no id is given, and pick the branch whose `.odd/state.json` holds the id.
- W7. class: `implementation`. Raises S3 of the phases A and B round, since the entry now carries the stops. `coordinator.md:125` deletes an entry with no covered IDs or follow-ups "on the last branch before its merge", and `coordinator.md:151` runs T4 after that merge. An L1 fix on critical code has mandatory T4 and can have no product IDs. If T4 finds a survivor, the fix round has no entry: `rounds` is gone, so the "T4 fix cycle counts as one verify round" rule has no input, and the closure `verify` meets "a missing outcome entry" (`verify.md:29`), a CRITICAL. Suggested fix: delete the entry after T4, in its own commit on a short branch.
- W8. class: `implementation`. The L0 row was only partly aligned with W2. `coordinator.md:24` and `AGENTS.md:28` now give the pack's depth, but still say `one writer` and `mutation: off` with no qualifier. A style change in critical code is L0 by its signals and Small refactor by its pack (`tdd-apply`, `verify`, T4). The row then reads wider than the exception at `coordinator.md:20`. `coordinator.md:63` and `AGENTS.md:99` settle it correctly, so the outcome is right if read in full, and the exception itself is not wider than before. Suggested fix: "one writer or the pack's agents" and "`mutation: off` when no critical code is touched".

## SUGGESTION

- S7. `coordinator.md:84` says "after each `verify`" with no folder condition, while `AGENTS.md:62` keeps `verify` for entries that are the G0 artifact. A folder change with an entry (this one) gets contradictory instructions. The wording predates this delivery; it now also covers FAIL and BLOCKED and the findings. Add "with no folder".
- S8. Id collisions. `coordinator.md:85` wants "the same ids" and ids "unique". Round 1 of a next delivery gets no open findings as input, so its W1 can collide with a kept follow-up. A BLOCKED re-run can do the same with findings of its replaced row. Tell `verify` to read the entry on every folderless round.
- S9. `verify.md:12` round fallback: rows reset only at the first update on the next delivery's branch, so a missing round before that gives 3 instead of 1. The coordinator normally passes the round, so this applies only when it does not.
- S10. `coordinator.md:20` says "pack row of the chosen mode", but a pack row is picked by request type. For L2 on a light pack the depth comes from "Mode defaults" (`coordinator.md:65`). `AGENTS.md:24` repeats it.
- S11. `product-specs/SKILL.md:72` still expects a `verify-report.md` listing covered IDs. Step 3d now requires the list, but with no folder it stays in the scratchpad and the entry does not hold it. W1 of the first round named this line.
- S12. The amended Light spec plan has 11 steps; the template says at most 7 (`coordinator.md:100`). No effect on any AC.
- S13. No phase D section in `apply-progress.md`, so there is no recorded AC evidence or measured size. G3 applies to code modes, so this is not a gate failure; this report's table fills the gap.

Still open from the first round: S1 (L0 `passed` with V0), S2, S4, S5, S6 first part. S3 became W7. S6's second part is resolved for the fix-round case by "last implementer commit".

## Note, not a finding

- Size: 56 insertions, 38 deletions, 11 files from d3148bf; 41 and 29 without `.opencode` and `openspec`. Below the ~300 signal.
- `layout-core` is archived, yet ADR 0011 says `proposed` until its D3 pilot ends, and `AGENTS.md:44` and `coordinator.md:29` still call it in progress. AC14 asks to stay `proposed`, so this is the owner's call at acceptance.
- The entry's `next_action` still reads "W1 first", and AC10 to AC14 are not in its criteria. Both are the coordinator's to update.
- The `specs/` consolidation stays in the ADR and `next_action` but has no AC in the Light spec, so it does not count as a remaining delivery here. `next: archive` follows the acceptance source; if the owner wants it inside `odd-lite`, it needs a Light spec amend first.

## Gates

No file in `git diff --name-only d3148bf HEAD` matches `.rs`, `Cargo`, `.toml`, `.cargo`, `deny` or `.github` (grep exit 1).

| Gate | Result |
|---|---|
| `cargo fmt`, clippy, test | not applicable, no Rust or Cargo file changed |
| `cargo deny` | not applicable, same reason |
| CI policy | not applicable, no step scans the changed paths |
| `python3 tools/sync_agents.py --check` | exit 0 |
| `python3 -m json.tool .odd/state.json` | exit 0 |
| Disk | 49 GB free; nothing built |

## Artifact integrity (step 3b)

- `50f59cb` (`docs(odd-lite): light spec amend`) touches only `openspec/changes/odd-lite/apply-progress.md`, is coordinator-owned, states why in its body and precedes 4321915.
- It adds AC10 to AC14 and extends the Plan line; AC1 to AC9 are unchanged.
- `git diff 50f59cb HEAD -- openspec/changes/odd-lite/` is empty, so no later commit changed the block.
- No commit in the range carries AI attribution. Commit titles are conventional.

## Criteria

| AC | State | Evidence |
|---|---|---|
| AC10 | covered | `AGENTS.md:64-71`, `coordinator.md:84-85`; schema has all seven fields and `{round, verdict, classes}`; commit before routing on PASS, FAIL and BLOCKED. Weak spot W5 |
| AC11 | covered | Read location `coordinator.md:17`, `AGENTS.md:73`; routing and stops `coordinator.md:87`, `:169`; closure input `verify.md:12`, `:16`; follow-ups `archive.md:24`. Weak spot W6 |
| AC12 | covered | `coordinator.md:20`, `:34`, `:63`; `AGENTS.md:24`, `:99`. Weak spot W8 |
| AC13 | covered | `verify.md:32` names the `git grep <ID>` check and classes |
| AC14 | covered | sync exit 0; json.tool exit 0; `schema` 1 and the file has no diff; fields optional (`AGENTS.md:64`) and the current entry has no `verify` field; ADR "Phase D: requirements" marks W1 to W3 resolved; ADR and index stay `proposed` |
| AC2 | no regression | Word diff of `AGENTS.md`: each removed fragment is reworded in place; no rule line deleted |
| AC3 | no regression | Floors and gates tables have no diff; the only table edit is the L0 Path cell, which raises depth |
| AC4, AC5 | no regression | Folder rule unchanged; `.odd/state.json` tracked, not ignored, no diff |
| AC6 | no regression | BLOCKED text unchanged, gains the stored row and the commit before routing |
| AC7, AC8 | no regression | 9 agent files; ADR brief names, retained flows, rollback and metrics intact; T4 rule unchanged |
| AC9 | no regression | Forbidden-path grep exit 1; no absolute or user path in added lines (grep exit 1) |

## Consistency across the six files

| Topic | Result |
|---|---|
| Modes, gates, floors, owner stops | none weakened. G5, loop limits, "a FAIL never reaches `develop`" unchanged; the before-routing commit is stricter than the old before-merge one |
| W2 exception | same scope as before in `coordinator.md`, `AGENTS.md` and the ADR: L0 and L1, no critical code. Now also named in the general rule. See W8 for the row |
| Depth and agents | a level raises only, in all three; L0 depth fixed; `one writer` left (W8) |
| Entry schema | `AGENTS.md` and `coordinator.md` agree on fields, follow-up rule, BLOCKED rows and reset. `verify.md` lacks the fields to feed them (W5) |
| Folderless notes | `AGENTS.md`, `coordinator.md`, `tdd-apply.md`, `verify.md` and the ADR all say "last implementer commit" |
| ADR | removed "new work keeps a folder until W1 is resolved", correct now it is resolved; audit trail risk updated |

## Simulated folderless outcome

Outcome `foo`, L1, light, V2, `mutation: off`, branch `fix/foo`, using rule text only.

| Step | Rule | Answered |
|---|---|---|
| Round 1 FAIL, C1 and W1 | `coordinator.md:84-85`: commit findings and one row before routing | yes; the follow-up mark has no source in the file (W5) |
| Session lost, scratchpad empty | `coordinator.md:17`: read the entry on the branch | yes for a named outcome or a checkout on the branch; no for "continue" on `develop` (W6) |
| Session lost before the record | entry shows the prior round; `verify` re-runs with the same number | yes |
| Route after resume | `coordinator.md:87`, `:168`: class of open findings, upstream first | yes |
| Fix, then round 2 | `verify.md:12`, `:16`: open findings as input, no earlier report needed | yes |
| Round 2 FAIL, C1 still open | same-class stop from `rounds` | partly: does C1 count for round 2? (W5) |
| Second FAIL in light | upgrade one mode, recorded by `outcome amend` | yes |
| BLOCKED during a round | row kept, not counted, re-run replaces it | yes |
| Round PASS | open WARNINGs become follow-ups, entry committed, merge | yes |
| Archive | `coordinator.md:125`, `archive.md:13`, `:16`, `:24-25`: follow-ups, then delete; no follow-ups means delete before merge | yes with `mutation: off`; with T4, W7 |

## Hand mutants and mutation

V1, `mutation: off`: none run, no report, not a finding.

next: archive

# Verify report: odd-lite, delivery `chore/odd-lite-phase-d`, closure

## Round 2

Commit d21847e, branch `chore/odd-lite-phase-d`, base `develop` (d3148bf). Closure round, mode light, depth V1, `mutation: off`, tier T0, no `mutants` report. Scope: W5 to W8, the commits e533750, 1a539d7 and d21847e, and the simulation points round 1 could not answer. BLOCKED did not arise: every critical check ran.

Verdict: PASS. CRITICAL 0, WARNING 1 (new, follow-up), SUGGESTION 3. Per class: implementation 1. W5 to W8 are closed.

## CRITICAL

None.

## Closure of the earlier findings

| Id | Status | Evidence |
|---|---|---|
| W5 | closed | `verify.md` schema gains optional `follow_up` and `status`; `verify.md:60` makes a closure round list each earlier finding by id and count a still-open one for that round. `coordinator.md:85` and `AGENTS.md:68-69` take `status` from that list and include still-open findings in `classes` |
| W6 | closed | `coordinator.md:17`, `AGENTS.md:73`: unmerged local branches and `develop` are all read, a branch matches by exact `outcome_id`, and with no outcome named and more than one active the owner is asked. Commands checked on this repo and in a scratch repo. A gap the fix opened is W9 |
| W7 | closed | `coordinator.md:125`, `AGENTS.md:103`: with `archive` or `delivery` the entry stays until T4 closes the change, and only `off` deletes it before the merge. `archive.md:13` reports `blocked` until then. The entry is on `develop` after the merge, so a resume during T4 finds it |
| W8 | closed | `coordinator.md:24`, `AGENTS.md:28`: "one writer (or the pack's agents)" and "`mutation: off` where allowed (no critical code)". The row is no wider than the exception at `coordinator.md:20` |

S3 of the first round closes with W7.

## WARNING

- W9. class: `implementation`, `follow_up: true`, status open. New in the W6 fix. An inherited copy of an entry can look active. Resume now reads every unmerged branch by file content, and 1a539d7 filters inherited copies only "with several branches for one outcome". Two cases pass that filter (scratch repo, rule text applied by hand). Case 1: outcome X is closed on `develop` and an unmerged branch cut earlier still holds its entry. X is then the only active entry, the "more than one active" question does not fire, and a bare "continue" resumes a closed outcome. Case 2: X has advanced on `develop` and the only unmerged branch holds an older copy. `AGENTS.md:73` says to trust "the delivery branch", and `coordinator.md:17` does not say whether `develop` joins the comparison, so the older copy can win. This is not hypothetical here: `chore/pi-agent-sync` and `backup/pi-agent-sync-preverify` hold `odd-lite` today with no scoped commit, so case 1 applies once `odd-lite` closes if they remain unmerged. "If it disagrees with git, git wins" limits the harm but names no check. Suggested fix: include `develop` in the comparison, and treat an entry as inherited when the branch has no scoped commit of its own beyond `develop` (`git log develop..<branch> --grep='(<outcome>)' -- .odd/state.json` is empty).

## SUGGESTION

- S14. `verify.md:60` leaves two details open. It does not say whether `counts` covers a listed `closed` finding, and "PASS only with zero CRITICAL" could be misread against a closed CRITICAL that is listed. This round counts open findings only. It also says "the round's `classes`", but `classes` exists only in the entry, and the coordinator derives it.
- S15. The resume list went from `git branch -a` to `refs/heads`, so a pushed delivery branch is missed on another clone. A session in the same clone is unaffected, and AC11 holds.
- S16. State commits from before 1a539d7 use `chore(odd)`, so the scoped lookup returns nothing for them (`develop` has no `(odd-lite)` commit today, nor do `chore/pi-agent-sync` or `backup/pi-agent-sync-preverify`). No rule says how an empty result compares. Also, `--grep` matches the commit body as well as the subject. Neither changes the answer on this repo today: `chore/odd-lite-phase-d` is the only branch with a scoped commit.

Still open from earlier rounds, unchanged by these commits: S1, S2, S4, S5, the first part of S6, and S7 to S13. S8 and S9 were not touched by the new rules, and `coordinator.md:84` still has no folder condition (S7).

## Note, not a finding

- Size: 22 insertions, 21 deletions, 7 files from 8bc9a76 to 1a539d7; 16 and 15 without `.opencode`.
- The `.opencode` files are thin adapters holding source hashes, so a grep for the new rule text finds nothing there, as expected. `sync_agents.py --check` compares those hashes.
- d21847e reformats `.odd/state.json` with `json.tool`, so its 76-line diff looks larger than it is. The only data changes are AC10 to AC14 as `pending` and `next_action`.
- AC10 to AC14 can now move to `passed`, which is the coordinator's update.
- As in round 1, `next: archive` follows the acceptance source. `next_action` names the `specs/` consolidation, which has no AC in the Light spec; if it belongs inside `odd-lite`, it needs a Light spec amend first.
- W1 to W4 belong to the phases A and B delivery and are not re-listed here. ADR 0011 records W1 to W3 as resolved.

## Gates

No file in `git diff --name-only d3148bf HEAD` matches `.rs`, `Cargo`, `.toml`, `.cargo`, `deny` or `.github` (grep exit 1).

| Gate | Result |
|---|---|
| `cargo fmt`, clippy, test | not applicable, no Rust or Cargo file changed |
| `cargo deny` | not applicable, same reason |
| CI policy | not applicable, no step scans the changed paths |
| `python3 tools/sync_agents.py --check` | exit 0 |
| `python3 -m json.tool .odd/state.json` | exit 0 |
| Path check (AC9) | no `crates/`, `apps/`, `docs/specs/` or `openspec/changes/archive/` path (grep exit 1); no absolute or user path in added lines (grep exit 1) |
| Disk | 49 GB free; nothing built |

## Artifact integrity (step 3b)

- `git log d3148bf..HEAD -- openspec/changes/odd-lite/apply-progress.md` lists only `50f59cb`, and `git diff 50f59cb HEAD` on that file is empty, so the Light spec block is intact.
- d21847e touches only `.odd/state.json`, carries the outcome scope and keeps `schema: 1`.
- No commit in the range carries AI attribution. Commit titles are conventional.

## Criteria

| AC | State | Evidence |
|---|---|---|
| AC10 | covered | Schema unchanged in its fields; W5 gives `verification.json` the source for them; commit before routing unchanged |
| AC11 | covered | `coordinator.md:17`, `:87`, `:169`; `verify.md:16`; `archive.md` 2c. Weak spot W9, where the outcome is not named |
| AC12 | covered | `coordinator.md:20`, `:24`, `:63`; `AGENTS.md:24`, `:28` |
| AC13 | covered | Step 3d has no diff since round 1 |
| AC14 | covered | Both commands exit 0; `schema: 1`; new `verification.json` fields are optional; ADR W1 to W3 resolved and status `proposed`, no diff since round 1 |
| AC2 to AC9 | no regression | Sentence diff below; 9 agent files; path check above |

## No mode, gate, floor or owner stop weakened

- Every line removed from `AGENTS.md`, `coordinator.md`, `verify.md` and `archive.md` since 235d0ce was read. No gate row, floor row, pack row or loop-limit line is among them.
- The only table cells changed are the two L0 rows and the two state field rows. The L0 change narrows `mutation: off` and allows the pack's agents.
- `archive` gained a `blocked` condition, and the entry now lives longer, so both changes are stricter.
- "Report each by its id" left `verify.md:16` and now lives, more strictly, in the schema text at `verify.md:60`.
- A FAIL still never reaches `develop`, G5 is unchanged, and BLOCKED still costs no round.

## Simulation re-walk

Outcome `foo`, L1, light, V2, using rule text only. Points 2 and 3 were also run in a scratch git repo.

| Point left open in round 1 | Rule now | Answered |
|---|---|---|
| 1. Follow-up mark has no source in the file | `follow_up` in the `verification.json` schema, copied at `coordinator.md:85` | yes |
| 2. Round 2 FAIL with C1 still open: does it count? | C1 is re-listed `open` (`verify.md:60`), its class joins that row's `classes` (`coordinator.md:85`), so the two-FAIL stop fires from `rounds` | yes |
| 3. "Continue" on `develop` with an empty scratchpad | The list finds `fix/foo` by exact id; `feat/foo-bar-d1`, holding only `bar`, no longer matches `foo`; two active means ask the owner | yes, except W9 |
| Archive with T4 (the fourth partial row) | Entry stays through T4; the fix round has `rounds`, and closure `verify` finds its entry | yes |

## Hand mutants and mutation

V1 closure round, `mutation: off`: none run, no report, not a finding.

next: archive
