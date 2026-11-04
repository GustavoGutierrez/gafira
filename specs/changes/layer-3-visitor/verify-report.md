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
