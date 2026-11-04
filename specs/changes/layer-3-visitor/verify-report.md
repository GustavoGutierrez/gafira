# Verify report: layer-3-visitor, DC1, round 1

Commit `4363413` on `feat/layer-3-visitor` (code in `df8ab59`), base `ed1dc7d` (the commit the branch was cut from; `develop` is now `65392f0`, which already carries `SC1`). Depth V2, mode full, level L2, tier T4, `mutation: archive` (no `mutants` report; per the contract that is not a finding). Date 2026-11-04.

**Verdict: PASS.** CRITICAL 0, WARNING 3, SUGGESTION 3. Classes: ambiguity 1, implementation 2, architecture 0, claim 0.

Probes and mutants ran on scratch exports outside the repo (`/tmp/gafira-verify-dc1/`), each with its own `CARGO_TARGET_DIR` and `CARGO_INCREMENTAL=0`. The worktree stayed clean apart from this report and `verification.json`.

## Gates

Each command ran alone, on commit `4363413`, with its own exit code read (no pipe). `CARGO_TARGET_DIR=/tmp/gafira-verify-dc1/target-gate`.

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (no `FAILED` in any binary) |
| `cargo deny check` | 0 (`advisories ok, bans ok, licenses ok, sources ok`) |
| CI policy: Format | 0 |
| CI policy: CLI has no UI dependencies (ARQ-07) | 0 |
| CI policy: Domain crates forbid unsafe code | 0 |
| CI policy: Domain crates respect the layer table (V5) | 0 |
| CI policy: Domain clippy.toml copies are in sync (V12) | 0 |
| CI policy: Layers 0-2 know no language (ARQ-107) | 0 |
| CI policy: EditToken only in the edit crates (V13) | 0 |
| CI policy: Every core command has a property test (V14) | 0 |

The policy steps were extracted from `.github/workflows/ci.yml` of the verified commit and run with `bash -e -o pipefail` on a scratch clone (V13 and V14 call `git`, so they need a repository). The `Check host matches matrix target` step is a GitHub expression (`${{ matrix.target }}`); it is not a policy check and cannot run outside Actions.

`gafira-doc` at this commit: 115 unit tests and 4 `visit_contract` tests pass. The delivery adds 16 tests (12 inline in `crates/gafira-doc/src/visit.rs`, 4 in `crates/gafira-doc/tests/visit_contract.rs`).

## Merge into develop (not required by the contract; run because the branch base is old)

The branch was cut from `develop` at `ed1dc7d`, before `SC1` merged (`65392f0`), so the delivery's own gates ran on a tree without `gafira-render`, `gafira-media` and the testkit `ports` feature, and the branch's `ci.yml` has no `ports` step. To discharge the risk of the merge target, `develop` + `feat/layer-3-visitor` was merged on a scratch clone: **no conflict**, 4 files and 803 insertions.

| Gate on the merged tree | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 |
| `cargo deny check` | 0 |
| Every CI policy step of `develop`'s `ci.yml`, including `Testkit ports feature compiles (ADR 0013)` | 0 each |

`cargo tree -p gafira-doc -e normal,build --target all` on the merged tree lists `gafira-geom` and `gafira-color` only: the edge `gafira-doc → gafira-render` does not exist, in a tree where `gafira-render` exists (ARQ-137, ARQ-01).

## Acceptance criteria

| AC | Status | Proof |
|---|---|---|
| AC1 | covered | `visit::tests::arq_134_every_node_is_visited_once`; probe `probe_ac1_ac2_visitor_with_only_visit_node_gets_one_call_per_node` |
| AC2 | covered | `arq_134_a_visitor_with_only_visit_node_receives_every_node`; same probe |
| AC3 | covered | `arq_134_the_trait_declares_one_method_per_node_kind` and `arq_134_the_dispatch_has_no_catch_all_arm`; probe `probe_arq134_the_dispatch_has_no_catch_all_arm`; and the compile-failure half, verified by adding a variant to `NodeKind` (E0004 at `crates/gafira-doc/src/visit.rs:244`). Wording caveat in S1 |
| AC4 | covered | `arq_135_the_view_borrows_the_page_and_its_arena`; probe `probe_ac4_the_walked_arena_is_the_borrowed_one` (Arc strong count and pointer identity for every node) |
| AC5 | covered | `arq_135_a_page_without_roots_walks_to_an_empty_vector`; probe `probe_ac5_a_page_without_roots_walks_to_an_empty_vector` (both orders) |
| AC6 | covered | `arq_135_the_walk_answers_with_owned_values`; probe `probe_ac6_the_output_outlives_the_page`, plus a compile-fail probe: a visitor whose `Out` borrows the node does not compile (`method was supposed to return data with lifetime 'a but it is returning data with lifetime '1`) |
| AC7 | covered | `arq_136_pre_order_visits_the_parent_before_its_children` and `arq_136_post_order_visits_the_parent_after_its_children`; probe `probe_ac7_pre_and_post_order_and_sibling_order` (three siblings and two roots) |
| AC8 | covered | `arq_136_a_cycle_ends_the_walk_and_visits_each_node_once` (own cycle and mutual cycle, both orders); probe `probe_ac8_a_cycle_ends_the_walk_and_visits_each_node_once` |
| AC9 | covered | `arq_136_a_missing_child_is_skipped_and_the_rest_is_walked`; probe `probe_ac9_a_missing_child_is_skipped_and_the_rest_is_walked` |
| AC10 | covered | `arq_136_an_unusable_dpi_degenerates_the_measurement` (NaN, +∞, −∞, 0, −1); probe `probe_ac10_an_unusable_dpi_degenerates_the_measurement`, plus `probe_ac10_a_usable_dpi_measures_the_node_box_in_page_coordinates` for the usable DPI |
| AC11 | covered | the `grep` of the card, `arq_137_the_domain_depends_on_no_crate_above_it` (reads the manifest), probe `probe_ac11_the_public_api_names_no_render_graph` (every `.rs` of the crate), probe `probe_arq137_the_manifest_has_no_edge_to_a_layer_above_the_domain`, and the `cargo tree` of the merged tree |
| AC12 | covered | every CI policy step exits 0, `cargo test --workspace --locked` exits 0, `crates/gafira-doc/src/lib.rs` keeps `#![forbid(unsafe_code)]`, and `Document::validate` (ARQ-13) is exercised by the workspace run |

### The two deviations the coordinator asked about

**Deviation 1, the instant as a type parameter (`NodeBehavior<T = ()>`, `NodeExtension<T = ()>`).** The rule of ARQ-136 is *satisfied by another route*, so this is not a contradiction of the product row and not CRITICAL.

- The normative half of the rule holds exactly: `MeasureCx` carries only what the domain owns, the page borrowed and the target DPI (`crates/gafira-doc/src/visit.rs:63-66`), and the instant is a separate argument of `local_bounds` and `hit_test` (`visit.rs:112`, `visit.rs:115`), never a field of the context.
- The literal half, `t: Flicks`, cannot be implemented in `gafira-doc`: `Flicks` lives in `crates/gafira-time/src/flicks.rs:21`, and `gafira-doc` may not depend on `gafira-time`. The product spec itself fixes that: 02 §2.1 (row of `gafira-doc`, line 92) and 02 §2.2 (line 138, "Hoy gafira-doc solo depende de geom y color"), and the CI layer table enforces it (`.github/workflows/ci.yml:83`, `gafira-doc gafira-geom gafira-color`, in the branch and in `develop` alike).
- The generic keeps the seam open for the layer that owns the time: `impl<T> NodeBehavior<T> for Node` (`visit.rs:118`) holds for every `T`, so `gafira-render` can use `NodeBehavior<Flicks>` without any edge from the domain to `gafira-time`.
- The row is still inconsistent as written and must be reconciled: W1.

**Deviation 3, the role of the DPI.** The delivery decides that the DPI only decides whether the measurement is defined. That is consistent with the product row, so no finding.

- ARQ-136's requirement text (02 §4.1, line 495) and the numeric case table (lines 505-506) state the DPI in exactly two cases: not finite, and negative or zero. Nothing states that a usable DPI scales or alters the box.
- The placement is already stored in logical px (`crates/gafira-doc/src/placement.rs:19-21`), so the node's own box is in page coordinates by construction, and the delivery returns it (`visit.rs:123-130`).
- Probe `probe_ac10_a_usable_dpi_measures_the_node_box_in_page_coordinates` confirms the box of a node placed at (10, 20, 30, 40) is exactly that box at DPI 96, and that `hit_test` takes the point in page coordinates.

## Independent probes (step 1c)

Derived from the card and from ARQ-134..ARQ-137 before reading the implementer's tests, run on a scratch export of `4363413` (`/tmp/gafira-verify-dc1/probe`), 17 tests plus two commands. All pass.

| Probe | Proves |
|---|---|
| `probe_ac1_ac2_visitor_with_only_visit_node_gets_one_call_per_node` | AC1, AC2: one call per node, no node twice, with only `visit_node` implemented |
| `probe_ac3_each_node_reaches_exactly_one_kind_method` | AC3: each node reaches exactly one kind method, the right one |
| `probe_ac4_the_walked_arena_is_the_borrowed_one` | AC4: the arena is not cloned and the visited node is the arena's node (pointer identity) |
| `probe_ac5_a_page_without_roots_walks_to_an_empty_vector` | AC5 in both orders |
| `probe_ac6_the_output_outlives_the_page` | AC6 at compile time |
| `probe_ac7_pre_and_post_order_and_sibling_order` | AC7: parent first / last, siblings in the saved order, two roots |
| `probe_ac8_a_cycle_ends_the_walk_and_visits_each_node_once` | AC8: own cycle and mutual cycle |
| `probe_ac9_a_missing_child_is_skipped_and_the_rest_is_walked` | AC9 |
| `probe_arq136_the_walk_does_not_filter_by_visible_or_locked` | ARQ-136: no filtering by `visible` or `locked` |
| `probe_ac10_an_unusable_dpi_degenerates_the_measurement` | AC10: NaN, ±∞, 0 and negative give a 0×0 box and `hit_test false` |
| `probe_ac10_a_usable_dpi_measures_the_node_box_in_page_coordinates` | ARQ-136: the box in page coordinates, the hit test in page coordinates |
| `probe_ac11_the_public_api_names_no_render_graph` | AC11 over every `.rs` of the crate |
| `probe_arq137_the_manifest_has_no_edge_to_a_layer_above_the_domain` | ARQ-137: the manifest names no crate above the domain |
| `probe_arq134_the_dispatch_has_no_catch_all_arm` | AC3: no `_` arm, one arm per `NodeKind` variant |
| `probe_arq136_the_measure_context_carries_no_instant` | ARQ-136: the context carries no instant |
| `probe_arq137_the_extension_trait_declares_no_render_operation` | ARQ-137: `bounds` and `inspector_schema`, no render reference |
| `probe_arq137_an_extension_can_be_implemented_outside_the_domain` | ARQ-137, ARQ-20: the seam is implementable from another crate |
| command: add `Probe(ShapeNode)` to `NodeKind` | ARQ-134: the build of `gafira-doc` breaks with E0004 at `crates/gafira-doc/src/visit.rs:244` |
| command: a visitor whose `Out` borrows the node | AC6: does not compile |

Comparing with the implementer's tests: every criterion of the card is proved by a real behavioural test, not only by a source-reading test. The two source-reading tests (`arq_134_the_trait_declares_one_method_per_node_kind`, `arq_134_the_dispatch_has_no_catch_all_arm`) cover a rule that no runtime test can cover, and the compile-failure half of that rule is verified above. The one criterion with no guard is W2.

## Product IDs (step 3c)

| ID | Proof | Named by |
|---|---|---|
| ARQ-13 | `Document::validate` (gafira-doc) and the `debug_assert` of gafira-edit; the delivery's cycle and missing-child tests name it | `crates/gafira-doc/src/visit.rs:462`, `:481`; pre-existing `crates/gafira-edit/src/history.rs:286`, `:308` |
| ARQ-20 | the visitor is written outside the domain: `crates/gafira-doc/tests/visit_contract.rs` is not `gafira-doc` | `visit_contract.rs:155` |
| ARQ-134 | 3 inline tests, 2 contract tests, plus the `NodeKind` variant probe | `visit.rs`, `visit_contract.rs` |
| ARQ-135 | 3 inline tests, plus the borrow probe | `visit.rs` |
| ARQ-136 | 7 inline tests, plus the order, cycle, missing-child and DPI probes | `visit.rs` |
| ARQ-137 | the manifest test, the grep over the crate's sources, the `cargo tree` of the merged tree | `visit_contract.rs:89` |

Left out, with the reason: the acceptance criterion of ARQ-13 ends in "un fuzzing de `gafira-format` no produce documentos inválidos aceptados"; `gafira-format` does not exist in the workspace (there is no `crates/gafira-format`), so that half of the criterion cannot be run in this cut and is not a gap of this delivery. The card's AC12 asks only that the invariants of ARQ-13 still hold, which the workspace run proves.

Two criteria of ARQ-137 name commands that do not exist yet: `cargo xtask check-layers` (no `xtask` crate; `ci.yml`'s ARQ-107 step says "Until `check-layers` exists") and "the output of `cargo public-api`, which CI already generates" (no `cargo public-api` step in `ci.yml`). The delivery substitutes the grep over the crate's sources and the V5 step, which runs `cargo tree` over the real dependency edges: the substitutes are stronger than the commands the row names, and the gap in `ci.yml` predates this delivery.

## Artifact integrity (step 3c)

- `a6d4bc5` "docs(layer-3-visitor): slice card" touches only `specs/changes/layer-3-visitor/apply-progress.md` (1 file, 33 insertions) and precedes the first implementation commit `df8ab59`.
- The card block (lines 1 to 19, including the `Flow:` line) is byte-identical between `a6d4bc5` and `4363413`: `diff` of both versions is empty. No implementer commit changed the card.
- `df8ab59` touches only `crates/gafira-doc/` (3 files). `4363413` touches only `apply-progress.md` and fills the `## DC1 (tdd-apply)` skeleton the card commit already held; the card block is untouched.
- Conventional commit subjects, no attribution lines.
- The worktree was clean before this report.

## Hand mutants (step 4b)

**Spot-check of the recorded item 1** ("the two orders are inverted"), applied on the scratch export: the recorded killer `visit::tests::arq_136_pre_order_visits_the_parent_before_its_children` fails with `left: [4, 2, 3, 1]`, `right: [1, 2, 4, 3]`, exactly the message recorded in `apply-progress.md`. The item is real. (In my run 7 tests of the lib and contract targets failed; the note says 5, which depends on how the mutant is written. The recorded killer and its message reproduce exactly.)

**Three independent mutants**, picked from the rules of the spec, none of them in the recorded list:

| Mutant | Wrong behaviour | Killed by |
|---|---|---|
| HM-1 | the siblings are walked in the reverse of the saved order (`crates/gafira-doc/src/visit.rs:213`) | `arq_136_pre_order_visits_the_parent_before_its_children`, `arq_136_post_order_visits_the_parent_after_its_children`, `arq_136_a_missing_child_is_skipped_and_the_rest_is_walked` |
| HM-2 | the dispatch over `NodeKind` gains a catch-all `_` arm (`visit.rs:247`) | `visit_contract::arq_134_the_dispatch_has_no_catch_all_arm` (the 115 lib tests all pass: only the source-reading test sees it) |
| HM-3 | the walk filters out the invisible nodes (`visit.rs:217`) | `arq_136_the_walk_visits_hidden_and_locked_nodes` |

4 run, 4 killed, 0 survivors. Every mutant was reverted; none was committed.

## Size

Measured on the delivery branch against its base `ed1dc7d` with `git diff --shortstat`:

| Scope | Result |
|---|---|
| Whole delivery | 4 files changed, 803 insertions(+), 0 deletions(-) |
| Code only (`crates/`) | 3 files changed, 720 insertions(+) |
| `crates/gafira-doc/src/visit.rs` | 551 |
| `crates/gafira-doc/tests/visit_contract.rs` | 168 |
| `crates/gafira-doc/src/lib.rs` | 1 |
| `specs/changes/layer-3-visitor/apply-progress.md` | 83 (33 of the card, 50 of the notes) |

The forecast was 430 (x1.4 = 600). The delivery's own figure of "about 720 authored lines" is confirmed: 720 code lines. That is above the x1.4 planning figure (600) and above the ~700 signal of `AGENTS.md`, which asks the coordinator to stop and ask (W3).

## Findings

### W1 — WARNING — class: ambiguity — `specs/02-arquitectura-software.md` §4.1, ARQ-136; `crates/gafira-doc/src/visit.rs:104`, `:146`

ARQ-136 sketches `fn local_bounds(&self, cx: &MeasureCx, t: Flicks) -> Rect` for a trait that lives in `gafira-doc`, but 02 §2.1 (line 92), 02 §2.2 (line 138) and the CI layer table (`.github/workflows/ci.yml:83`) forbid `gafira-doc → gafira-time`, the crate that owns `Flicks`. The row is therefore not implementable as written, and the delivery had to make the instant a type parameter. The rule it protects ("the time is passed apart, not in the context") holds in the delivered code, and the deviation was declared in `apply-progress.md`, so this is a gap in the product row and not a defect of the code: `specifier` must reconcile ARQ-136 with the layering, and the decision belongs in `design.md` (P6 area). If the owner reads the parenthetical `(t: Flicks)` as binding, this becomes CRITICAL (ambiguity) and the verdict turns FAIL with `next: specifier`; the evidence above is the whole of the argument for the other reading.

### W2 — WARNING — class: implementation — `crates/gafira-doc/src/visit.rs:63-66`

No test pins ARQ-136's "the time is not in the context" rule, the rule the delivery deviated on. The delivery guards the shape of the dispatch with a source-reading test, but nothing reads the shape of `MeasureCx`: adding a `t` field to the context would fail no test in the repository. `probe_arq136_the_measure_context_carries_no_instant` (this report's probes) is the check that is missing.

### W3 — WARNING — class: implementation — `AGENTS.md` "Delivery rules"; `specs/changes/layer-3-visitor/apply-progress.md:83`

The delivery is 720 authored code lines against a forecast of 430 (x1.4 = 600), over the ~700 ceiling that `AGENTS.md` says needs the owner's explicit OK. No such OK is recorded. The coordinator decides, and the number above is the measurement it needs.

## Suggestions

- **S1** — AC3 of the card reads "declares three `visit_*` methods"; the trait declares four names that start with `visit_` (`visit_node` plus the three kind-specific ones), which is what 02 §4.1 sketches. Reword the card so the count excludes the mandatory `visit_node`, or a later reader will read the delivery as failing AC3.
- **S2** — the Scope of the card enumerates `NodeVisitor`, `walk`, `WalkOrder`, `PageView` and `MeasureCx`; the delivery also adds two public traits, `NodeBehavior` and `NodeExtension`. Both are demanded by the rows the card cites (ARQ-136 names `local_bounds` and `hit_test`, ARQ-137 names `NodeExtension`), so the code is right and the enumeration is incomplete. Reword the card.
- **S3** — the gate evidence in `apply-progress.md` comes from the pre-`SC1` tree, because the branch was cut from `develop` at `ed1dc7d`. The merge target passes every gate and every policy step (see above); record that in the notes before the archive.

## next

archive

---

# Verify report: layer-3-visitor, DC1, round 2 (closure)

Commit `a17fe2e` (`Merge branch 'develop' into feat/layer-3-visitor`), the HEAD of `feat/layer-3-visitor`; it already carries the merge of `develop` (`65392f0`, with `SC1`). Integration base `65392f0`. Depth V2, mode full, level L2, tier T4, `mutation: archive` (no `mutants` report; per the contract that is not a finding). Date 2026-11-04.

**Verdict: PASS.** CRITICAL 0, WARNING 1 (still open), SUGGESTION 4 (still open). Classes of the open findings: ambiguity 1, implementation 0, architecture 0, claim 0.

Of the three findings of round 1: **W2 and W3 are closed, W1 is still open.**

This round ran the gates (step 2), the artifact-integrity checks (steps 3, 3b, 3c, 3d) and the closure probes for W2 only. It spent no independent-mutant budget (closure round) and ran no probe outside the closure of W2. The counts above are of the findings still open at this round; W2 and W3 are listed in `findings` with `status: closed`.

Every command ran in the worktree `/tmp/gafira-wt/dc1` at `a17fe2e`, with `CARGO_TARGET_DIR=/tmp/gafira-verify-dc1` and `CARGO_INCREMENTAL=0`. The closure probes ran on a scratch export (`/tmp/gafira-verify-dc1/w2probe`, from `git archive a17fe2e`), never in the worktree. The worktree stayed clean apart from this report and `verification.json`.

## Gates

Each command ran alone and its exit code was read without a pipe.

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (180 test binaries, 2 288 passed, 0 failed) |
| `cargo deny check` | 0 (`advisories ok, bans ok, licenses ok, sources ok`) |
| CI policy: Format | 0 |
| CI policy: CLI has no UI dependencies (ARQ-07) | 0 |
| CI policy: Domain crates forbid unsafe code | 0 |
| CI policy: Domain crates respect the layer table (V5) | 0 |
| CI policy: Domain clippy.toml copies are in sync (V12) | 0 |
| CI policy: Layers 0-2 know no language (ARQ-107) | 0 |
| CI policy: EditToken only in the edit crates (V13) | 0 |
| CI policy: Every core command has a property test (V14) | 0 |
| CI policy: Testkit ports feature compiles (ADR 0013) | 0 |

The nine policy steps were extracted from the `run:` of `.github/workflows/ci.yml` at `a17fe2e` and run with `bash -e -o pipefail` in the worktree (V13 and V14 call `git`, and the worktree is a repository, so they ran as they do in Actions). The step `Check host matches matrix target` is a GitHub expression (`${{ matrix.target }}`); it is not a policy check and cannot run outside Actions.

`Testkit ports feature compiles (ADR 0013)` exists in this branch now: the merge `a17fe2e` brought `develop`'s `ci.yml`, and the step compiles and tests `gafira-testkit` with the `ports` feature, exit 0. The round-1 note that the step was absent from the branch was true of `6672880` and is stale at `a17fe2e`.

`cargo deny check` prints one `warning[duplicate]` for `miniz_oxide` (`Cargo.lock`, via `png` ← `gafira-testkit`, from `develop`'s `SC1`). It is a warning of the deny tool, not an error: the command exits 0 and no advisory, ban, licence or source check fails. It is not a finding of this delivery.

`gafira-doc` at this commit: 115 unit tests and 5 `visit_contract` tests pass. The delivery names 17 tests (`arq_134` 4, `arq_135` 3, `arq_136` 8, `arq_137` 1, `arq_20` 1).

## Remote CI

The coordinator dispatched the workflow over the branch. Run `41878047458`, `head_sha` `a17fe2e76976ddccf993f5d6a4b2baec4deb9510` (the verified commit), `workflow_dispatch`, started 2026-11-04T17:30:06Z, ended 2026-11-04T17:51:08Z. **Conclusion: success.**

| Job | Status | Conclusion | Steps |
|---|---|---|---|
| `policy` | completed | success | Format, CLI/UI (ARQ-07), forbid unsafe code, V5, V12, ARQ-107, V13, V14, **Testkit ports feature compiles (ADR 0013)**, cargo-deny: all success |
| `test (x86_64-unknown-linux-gnu)` | completed | success | host check, Clippy, Test: all success |
| `test (x86_64-pc-windows-msvc)` | completed | success | host check, Clippy, Test: all success |
| `test (aarch64-apple-darwin)` | completed | success | host check, Clippy, Test: all success |

Four jobs, zero failures. The three product targets of the matrix (Windows, Linux, macOS 12+ aarch64) pass the whole suite, which the local run on Linux alone could not show.

## Artifact integrity (steps 3, 3b, 3c, 3d)

| Commit | Subject | Files | Verdict |
|---|---|---|---|
| `a6d4bc5` | `docs(layer-3-visitor): slice card` | `specs/changes/layer-3-visitor/apply-progress.md` only (1 file, 33 insertions) | correct: touches only the card file and precedes the first implementation commit `df8ab59` (09:49:48 vs 10:13:53) |
| `df8ab59` | `feat(gafira-doc): add the visitor contract over the node kinds` | `crates/gafira-doc/` only (3 files, 720 insertions) | implementer: no card, no spec, no other crate |
| `4363413` | `docs(layer-3-visitor): record the DC1 progress` | `apply-progress.md` only (53 insertions, 3 deletions) | implementer notes in the skeleton the card commit held; card block untouched |
| `17c2b90` | `docs(layer-3-visitor): record verify round 1` | `verification.json`, `verify-report.md` | coordinator: exactly the two files `verify` writes |
| `6672880` | `docs(layer-3-visitor): record the coordinator decisions on verify round 1` | `apply-progress.md` only (+6) | coordinator: adds the `### Coordinator decisions on verify round 1` section, states why, touches no other file |
| `fe6349e` | `test(gafira-doc): tie the measure context to the absence of a time` | `crates/gafira-doc/tests/visit_contract.rs` only (+141, −2) | implementer fix round: no source of the crate, no card |
| `a71b2b8` | `docs(layer-3-visitor): record the DC1 fix round 1` | `apply-progress.md` only (+92) | implementer notes of the fix round; adds the `## DC1 (fix round 1)` section, touches no other file |
| `a17fe2e` | `Merge branch 'develop' into feat/layer-3-visitor` | the `SC1` tree from `develop` | merge; touches neither the card nor `specs/02-arquitectura-software.md` |

- The Slice card block (lines 1 to 19, including the `Flow:` line) is **byte-identical** between `a6d4bc5` and `a17fe2e` (`diff` of both versions is empty). No implementer commit changed the card.
- The `Flow:` line was introduced by `a6d4bc5` and never modified: `git log -L2,2` on the file shows one commit for it.
- No `slice card amend` commit exists for this change, so the coordinator needed none.
- `specs/02-arquitectura-software.md` was touched by **no** commit of the branch: `git log ed1dc7d..a17fe2e -- specs/02-arquitectura-software.md` is empty.
- The sources of the delivery did not change since round 1: `git diff df8ab59 a17fe2e -- crates/gafira-doc/src/visit.rs crates/gafira-doc/src/lib.rs` is empty. The fix round and the merge changed no behaviour of the crate.
- Conventional commit subjects, no AI attribution lines.

## Closure of the findings of round 1

### W1 — WARNING — class: ambiguity — **STILL OPEN**

`specs/02-arquitectura-software.md` §4.1, ARQ-136 (line 495) still reads, verbatim: *"El tiempo se pasa aparte (`t: Flicks`), no en el contexto."* The row was **not** reconciled: no commit of the branch touched the file, and the merge brought no change to it. `Flicks` still lives in `gafira-time` (`crates/gafira-time/src/flicks.rs`), which `gafira-doc` may not depend on under 02 §2.1 (line 92), 02 §2.2 (line 138) and the CI layer table (`.github/workflows/ci.yml:83`, `gafira-doc gafira-geom gafira-color`). The literal half of the row is still unimplementable in the domain.

The rule the row protects does hold in the delivered code, and the deviation is declared in `apply-progress.md`; the finding is a gap in the product row, not a defect of the code. **Status: open.** The coordinator's recorded decision routes it to `specifier` (section `### Coordinator decisions on verify round 1` of `apply-progress.md`); it is not closed and the archive must not consume it.

### W2 — WARNING — class: implementation — **CLOSED** by `fe6349e`

The new test is `visit_contract::arq_136_the_measure_context_carries_no_time` (`crates/gafira-doc/tests/visit_contract.rs:201-245`). It reads the sources of the crate with `include_str!` and checks three things: the fields `MeasureCx` declares are exactly `page` and `dpi`; the block of the struct names no time type; and no file of `crates/gafira-doc/src/` names one of `Flicks`, `FrameRate`, `Instant`, `Duration`, `SystemTime`.

The test was probed on the scratch export (`/tmp/gafira-verify-dc1/w2probe`, `git archive a17fe2e`, files touched so the build is not stale) with the private target dir. Three probes, each applied and then reverted; the worktree was never modified.

| Probe | Wrong shape applied | Result | Killed |
|---|---|---|---|
| P1 | `MeasureCx` gains the field `t: std::time::Duration` (and `new` sets it, so the crate still compiles) | `arq_136_the_measure_context_carries_no_time` FAILED at `visit_contract.rs:206:5`: `left: ["page", "dpi", "t"]`, `right: ["page", "dpi"]`; the other 4 tests of the file kept passing | yes |
| P2 | `visit.rs` names a time type (`use std::time::Instant;`), the struct untouched | the same test FAILED at `visit_contract.rs:239:13`: `the domain names the time type Instant in src/visit.rs` | yes |
| P3 | a new module `timeline` enters `lib.rs` and the list of sources does not cover it | the same test FAILED at `visit_contract.rs:228:13`: `src/lib.rs declares the module timeline, which the list of sources does not cover: add it` | yes |

3 probes, 3 killed, 0 survivors. Every probe failed with an **assertion**, never with a compile error: the crate built with each mutant in place, which is what makes the rule observable. The three messages reproduce the RED evidence and the negative controls recorded in `apply-progress.md` exactly, at the same line numbers.

**The completeness guard fails closed**: P3 shows that a module added to the crate later makes the test fail with the name of the module to add, so the list of 23 sources cannot silently go stale.

After reverting every probe, `cargo test -p gafira-doc --test visit_contract --locked` is `ok. 5 passed; 0 failed`, and the export is byte-identical to `a17fe2e` (`diff` empty). The worktree was never touched: `git status --short` is empty and `git diff a17fe2e -- crates/` is empty.

### W3 — WARNING — class: implementation — **CLOSED** by the coordinator's recorded decision

The decision is registered in `apply-progress.md`, section `### Coordinator decisions on verify round 1`: *"**W3 (720 lines of code, over the ~700 ceiling): accepted.** The delivery is the visitor contract with its tests and the shape checks, and the size is 3 % over the ceiling of a `full` run. The owner gave the coordinator the decision in YOLO mode on 2026-11-03, and the coordinator accepts it here."*

The decision is present, it states why, and it was taken by the coordinator with the owner's delegation, which is what the finding asked for.

**The measurement it accepted has grown.** The 720 lines were the whole delivery at `df8ab59`; the fix round that closed W2 added 139 more net lines to the test file. The real size of DC1 at `a17fe2e` is below. The number the decision quotes (720, "3 % over the ceiling") is stale; the delivery is now 859 code lines, 22.7 % over the ~700 ceiling. The character of the delivery is the one the decision describes (the visitor contract, its tests and its shape checks), so the decision stands; the coordinator may want to refresh the figure before the archive (S4).

## Size

Measured on the delivery branch against its integration base `65392f0` (the tip of `develop` merged at `a17fe2e`), with `git diff --numstat`:

| Scope | Result |
|---|---|
| Whole delivery | 6 files changed, 1 241 insertions(+) |
| Code only (`crates/`) | 3 files changed, 859 insertions(+) |
| `crates/gafira-doc/src/visit.rs` | 551 |
| `crates/gafira-doc/tests/visit_contract.rs` | 307 |
| `crates/gafira-doc/src/lib.rs` | 1 |
| `specs/changes/layer-3-visitor/apply-progress.md` | 181 (19 of the card, 162 of notes) |
| `specs/changes/layer-3-visitor/verify-report.md` | 180 |
| `specs/changes/layer-3-visitor/verification.json` | 21 |

The forecast was 430 (x1.4 = 600). The implementation commit `df8ab59` was 720 code lines (the figure of round 1) and the fix round `fe6349e` added 141 insertions and 2 deletions to the test file alone, for 859 net code lines now.

## Findings

### W1 — WARNING — class: ambiguity — **open** — `specs/02-arquitectura-software.md` §4.1, ARQ-136 (line 495)

The row still says the instant travels apart as `t: Flicks`, and `Flicks` is unreachable from `gafira-doc` under 02 §2.1, 02 §2.2 and the V5 layer table. No commit of the branch touched the file. The rule the row protects holds in the code (the instant is a separate argument; `MeasureCx` carries only the borrowed page and the DPI, now pinned by `arq_136_the_measure_context_carries_no_time`). `specifier` still has to reconcile the row; until it does, the product spec and the delivery disagree about the literal type.

### W2 — WARNING — class: implementation — **closed** — `crates/gafira-doc/tests/visit_contract.rs:201-245`

Closed by `fe6349e`. The probe above proves the test bites on all three of its halves and that the completeness guard fails closed. Verified in this round.

### W3 — WARNING — class: implementation — **closed** — `AGENTS.md` "Delivery rules"; `specs/changes/layer-3-visitor/apply-progress.md` (section `### Coordinator decisions on verify round 1`)

Closed by the coordinator's recorded decision. The measurement it quotes (720 code lines) is now 859 (see S4).

## Suggestions

- **S1** (round 1, still open) — AC3 of the card reads "declares three `visit_*` methods"; the trait declares four names that start with `visit_` (`visit_node` plus the three kind-specific ones), which is what 02 §4.1 sketches. Reword the card so the count excludes the mandatory `visit_node`, or a later reader will read the delivery as failing AC3. The card is frozen; changing it needs a `slice card amend` commit by the coordinator that states why.
- **S2** (round 1, still open) — the Scope of the card enumerates `NodeVisitor`, `walk`, `WalkOrder`, `PageView` and `MeasureCx`; the delivery also adds two public traits, `NodeBehavior` and `NodeExtension`. Both are demanded by the rows the card cites (ARQ-136 names `local_bounds` and `hit_test`, ARQ-137 names `NodeExtension`), so the code is right and the enumeration is incomplete. Reword the card.
- **S3** (round 1, still open in a new form) — the gate evidence in `apply-progress.md` comes from the pre-`SC1` tree, and the notes of the fix round state that the step `Testkit ports feature compiles (ADR 0013)` is not in the branch's `ci.yml`. That was true at `6672880` and is **no longer true at `a17fe2e`**: the merge brought `develop`'s `ci.yml` and the step now exists and passes (exit 0 locally, success in the remote run). Refresh the notes before the archive so the record matches the merged tree.
- **S4** (new) — the decision on W3 quotes 720 code lines ("3 % over the ceiling"); the delivery is now 859 code lines (22.7 % over). The decision stands on the character of the delivery it describes, but its figure is stale; refresh the measurement in the notes before the archive.

## next

archive
