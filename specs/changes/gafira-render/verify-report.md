# Verify report: gafira-render, RE1, round 1

**Verdict: FAIL.** CRITICAL 2, WARNING 2, SUGGESTION 5. Classes: implementation 3, ambiguity 1, architecture 0, claim 0.

Verified: `feat/layer-3-render` at `6d66219`, worktree `/tmp/gafira-wt/re`, target dir `/tmp/gafira-verify-re`. Depth V2, mode full, tier T4, mutation `archive` (no run: the owner decision O4 sets the close run to `mutation: off`, and per the contract a missing report is not a finding). Round 1 is the first verify of `RE1`, so steps 1c (independent probes) and 4b (hand mutants) ran.

The implementation is sound on the substance: the canonical description matches §6.1 byte for byte, the graph holds the operations of the page and none of the extension, D-35 is honoured and tied by a test, cancellation and progress meet ARQ-139, and the pool seam makes the graph independent of the thread count. The two CRITICAL items are a real defect in the key derivation and a hole in the suite around the bytes §6.1 fixes by name.

## Gates

| Gate | Command | Exit |
|---|---|---|
| Format | `cargo fmt --check` | 0 |
| Clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| Test | `cargo test --workspace --locked` | 0 |
| Deny | `cargo deny check` | 0 |

The workspace suite is green: 2308 passed, 0 failed, 0 ignored. The delivery adds 20 tests (5 in `key.rs`, 12 in `graph.rs`, 1 in `pool.rs`, 2 in `tests/ports_doubles.rs`).

`df -h /` before the build: 77 GB free. One `CARGO_TARGET_DIR` per tree (`/tmp/gafira-verify-re` for the worktree, `/tmp/gafira-verify-re-export-target` for the scratch export), `CARGO_INCREMENTAL=0` everywhere.

## CI run (policy steps, extracted from the `run:` blocks of `.github/workflows/ci.yml`)

| Step | Exit |
|---|---|
| Format | 0 |
| CLI has no UI dependencies (ARQ-07) | 0 |
| Domain crates forbid unsafe code | 0 |
| Domain crates respect the layer table (V5) | 0 |
| Domain clippy.toml copies are in sync (V12) | 0 |
| Layers 0-2 know no language (ARQ-107) | 0 |
| EditToken only in the edit crates (V13) | 0 |
| Every core command has a property test (V14) | 0 |
| Testkit ports feature compiles (ADR 0013) | 0 |

The tenth step, "Check host matches matrix target", is a GitHub Actions matrix guard whose body is a `${{ matrix.target }}` expression; it does not run outside the matrix and is not a check of this tree.

## Acceptance criteria

Every criterion of the Slice card was checked against the code and against an independent probe, all of them outside the repo (`git archive 6d66219` into `/tmp/gafira-verify-re-export`, own target dir, the probe file `crates/gafira-render/tests/probe_verify.rs`).

| AC | Proof in the delivery | Probe | Verdict |
|---|---|---|---|
| AC1 | `graph::tests::arq_141_two_nodes_with_the_same_data_give_the_same_key`, `key::tests::arq_141_the_same_data_gives_the_same_key` | `probe_ac1_the_same_data_gives_the_same_key` | covered |
| AC2 | `key::tests::arq_141_a_different_input_gives_another_key` (changes the value of the same parameter only) | `probe_ac2_a_different_parameter_gives_another_key` **failed** | **failing** (C1) |
| AC3 | `graph::tests::arq_141_the_keys_do_not_depend_on_the_order_of_the_walk` | `probe_ac3_the_keys_do_not_depend_on_the_walk_order` (also compares the graph and the ops) | covered |
| AC4 | `key::tests::arq_141_the_key_is_the_hash_of_the_description`, `..._the_description_holds_the_bytes_of_the_section`, `..._the_non_finite_pattern_of_the_section_is_used` | `probe_ac4_*` (3 probes) | covered, but the suite does not pin the literal bytes (C2) |
| AC5 | `graph::tests::arq_141_the_graph_holds_the_operations_of_the_page_and_none_of_the_extension` | `probe_ac5_the_graph_holds_the_page_and_none_of_the_extension` (exact op sequence) | covered |
| AC6 | `graph::tests::arq_141_the_preview_and_the_export_hold_the_same_operations` (compares the op kinds only) | `probe_ac6_the_preview_and_the_export_hold_the_same_operations` (also origin, transform, blend, opacity) | covered |
| AC7 | `graph::tests::arq_110_a_non_finite_number_is_an_error` (opacity NaN, placement ∞) | `probe_ac7_a_non_finite_number_is_an_error_and_nothing_is_delivered` (adds the fill NaN case) | covered |
| AC8 | command: the canonical `clippy.toml` lint gives 0 warnings on the crate | whole-tree grep of the crate, every `use std::` import, every file | covered |
| AC9 | `graph::tests::arq_139_the_build_reports_the_progress_and_honours_the_cancel` on the testkit's `check_progress_contract(0.02, ...)` | `probe_ac9_*` (2 probes: the ratio, the 0,02 bound, the cancel at report 3) | covered |
| AC10 | `graph::tests::arq_140_the_same_input_gives_the_same_graph`, `..._the_graph_does_not_depend_on_the_number_of_threads`, `key::tests::arq_140_the_results_keep_the_order_of_the_indices` | `probe_ac10_*` (2 probes: same input, pools of 1, 2 and 4) | covered |
| AC11 | command: `forbid(unsafe_code)`, the V5 row, and `git grep -E 'RenderGraphBuilder\|gafira_render' -- crates/gafira-doc` finds nothing | re-run all three | covered |

11 criteria, 10 covered, 1 failing.

### The canonical description, byte by byte

§6.1 fixes the description as: one domain byte, one node type byte, one LOD byte, a big-endian `u32` with the number of input keys, that many 16-byte big-endian keys, and last the sampled parameters, each a big-endian `u32` with the length of the value and the bytes of the value. `key.rs` writes exactly that order, and the probe `probe_ac4_the_description_is_the_one_of_the_section_built_by_hand` rebuilds the 23-byte description of the "one input key, no parameters" case from the section alone and gets the same `ContentKey` as the crate. The truncation is the first 16 bytes of the BLAKE3 hash read as a big-endian `u128` (the probe also checks that the little-endian reading is not the key). The non-finite pattern is the quiet NaN with a zero payload (`0x7FC0_0000` for `f32`, `0x7FF8_0000_0000_0000` for `f64`), `+∞` and `−∞` keep theirs and give different keys. The order of the fields, the endianness of the count, of the keys and of the length prefix, and the framing of the parameters (exactly `u32` length + bytes, nothing between the blocks and nothing left over) all match §6.1.

The gap is not in the description but in the two bytes §6.1 fixes *by name* and that no test pins: the domain byte `R` and the node type `1` = Group, `2` = Shape, `3` = Extension. See C2.

## Product IDs (step 3c)

| ID | Test that names it | Status |
|---|---|---|
| ARQ-110 | `graph::tests::arq_110_a_non_finite_number_is_an_error` | covered |
| ARQ-111 | `graph::tests::arq_140_the_same_input_gives_the_same_graph` (names ARQ-111 in the comment) | covered |
| ARQ-117 | none — only the note "ARQ-117 has nothing to check in this cut" | **left out** (W2) |
| ARQ-138 | the command of AC8; the lint of the canonical `clippy.toml` gives 0 warnings on the crate | covered |
| ARQ-139 | `graph::tests::arq_139_the_build_reports_the_progress_and_honours_the_cancel` on the testkit's `check_progress_contract` | covered |
| ARQ-140 | `graph::tests::arq_140_the_same_input_gives_the_same_graph`, `..._the_graph_does_not_depend_on_the_number_of_threads` | covered |
| ARQ-141 | 12 tests in `key.rs` and `graph.rs` | covered |

ARQ-117 is the only one left out. Its row (02 §4.17.x, `gafira-raster`) is about pixel buffers: the premultiplication of the 65 536 `(c, a)` pairs, the unpremultiplication, and the length error of a buffer. The graph of `RE1` holds no buffer — no effect, no mask and no image source reaches it yet — so the row has no testable criterion in this cut. The design's scope table (`02 §6.1, §4.4, §4.13; ARQ-110, ARQ-111, ARQ-117`) and the Slice card both list it for `RE1` anyway, and §4.17.7's "primer corte" table for `gafira-render` does not list ARQ-117 at all. The delivery disclosed the reason in its notes. This is a card-and-design scope question, not a code question: W2.

## Artifact integrity (step 3c)

- `c7e6548 docs(gafira-render): slice card` touches only `specs/changes/gafira-render/apply-progress.md` (32 insertions, 0 deletions). It precedes the first implementation commit (`b8e9ce1`).
- No implementer commit changed the card. `b8e9ce1`, `52e196a` and `44b3269` do not touch `apply-progress.md` at all; `c13aae4` (the notes) only appends the `## RE1 (tdd-apply)` section below it. The card block (lines 1 to 19) is byte-identical between `c7e6548` and `c13aae4`.
- The amend `6d66219 docs(gafira-render): slice card amend` touches only `apply-progress.md` (+2/−1) and states why: "The card now covers the whole render stream instead of RE1 only, and lists its five deliveries with their order. The acceptance criteria, the Flow line and the owner decisions are unchanged." The diff confirms it: the `Scope` line and a new `Deliveries` line change; AC1 to AC11, the `Flow` line and the owner decisions do not.

## The merge of `develop` (7e3a23c)

`7e3a23c Merge branch 'develop' into feat/layer-3-render`, parents `c7e6548` (the card) and `65392f0` (develop after `SC1`).

- **Necessary and clean.** The branch was cut before `SC1` landed, so it had no `crates/gafira-render` at all; the merge brought the scaffold (the crate's `Cargo.toml` and `clippy.toml`, the testkit module list, the ports stubs, the V5 and V12 rows) and nothing else. There are no conflict markers in the tree and the commit records no conflict resolution: `git show --cc 7e3a23c` shows the files each parent contributes and no manual fix-up.
- **No foreign work.** The diff of the merge against its second parent is exactly `develop@65392f0`'s content.
- **No duplicates of `SC1` or `DC1`.** `crates/gafira-doc/src/visit.rs` exists once, with one `pub fn walk` and one `pub trait NodeVisitor`; `crates/gafira-render` exists once.
- **The branch is behind `develop` now, and that is clean too.** `develop` has since taken `DC1` (via `feat/layer-3-visitor`, commit `c710d13`) and the owner decisions O4 and O5. `git diff HEAD develop -- crates/gafira-doc/` shows only `crates/gafira-doc/tests/visit_contract.rs` differing (develop's DC1 fix round), so re-merging `develop` before merging the branch is conflict-free; the branch's `gafira-doc` *source* is byte-identical to develop's.

## Size

`git diff --shortstat 7e3a23c 44b3269` (the code of the delivery): **1571 insertions, 31 deletions, 9 files**. With the notes and the amend (`7e3a23c..6d66219`): 1612 insertions, 32 deletions, 10 files. The forecast of the design was 480 lines, at most 672 with the x1.4 factor, and the guide marks ~700 as the ceiling. The measurement is 2.34x the planning figure and 3.27x the raw forecast.

| File | Added | Of which tests |
|---|---|---|
| `crates/gafira-render/src/graph.rs` | 1018 | 444 |
| `crates/gafira-render/src/key.rs` | 298 | 118 |
| `crates/gafira-render/src/pool.rs` | 46 | 12 |
| `crates/gafira-render/src/lib.rs` | 19 | 0 |
| `crates/gafira-render/tests/ports_doubles.rs` | 126 | 126 |
| `crates/gafira-testkit/src/ports/render_backend.rs` | +47 / −23 | 0 |
| `crates/gafira-testkit/src/ports/page_renderer.rs` | +13 / −6 | 0 |
| `crates/gafira-render/Cargo.toml`, `Cargo.lock` | 4 | 0 |
| `specs/changes/gafira-render/apply-progress.md` (notes) | +41 / −1 | 0 |

Production code in the crate: 809 lines. Test code: 700 (574 inside `src`, 126 in the integration test). Testkit doubles: 60 added, 29 removed.

The split explains part of it and not all of it. The notes are right that the forecast of 480 did not count the pool seam and its thread test, the cancellation and the progress of ARQ-139, nor the doubles of the testkit and their test — together roughly 300 to 400 lines. But the graph module alone is 1018 lines against a forecast that covered the graph, the builder, the content key and the emission together, and 574 of those lines are the production part of `graph.rs`, which is more than the whole forecast.

Whether this needs a chained PR is the coordinator's decision, and the mechanism is not the one the `chained-pr` skill describes: ADR 0003 merges each delivery into `develop` locally without a PR. The levers are to split `RE1` further, or to accept the size with a recorded owner OK. W2 of the layer-2-scaffold precedent treats "above the ceiling by the rule's measure, with no recorded OK" as a WARNING; this is the same case.

## Mutants and probes (steps 1c, 4b)

### Probes (step 1c)

16 probes derived from AC1 to AC11 and from the rows of §6.1 and §4.17.7, written in a scratch export outside the repo, before reading the implementer's tests. 15 pass. The one that fails is `probe_ac2_a_different_parameter_gives_another_key`, and it is a real defect of the code, not a weakness of the probe: see C1.

Comparing with the implementer's tests, the only criterion proved by a weak test is AC2: `arq_141_a_different_input_gives_another_key` changes the *value* of `Opacity` and never the *kind* of the parameter, so it cannot see the collision. Everything else the delivery proves, it proves with a test that stands on its own; my probes added three facts the delivery does not assert (the fill case of AC7, the node fields of AC6 beyond the op kinds, and the graph of AC3 beyond the keys), and all three hold.

### Hand mutants (step 4b)

Applied on a scratch export, one at a time, against the implementer's suite only (the probe file was moved aside, so a survivor means the *delivered* suite does not catch it). 8 mutants, 6 killed, 2 survived.

| # | Mutant | Result | Test that kills it |
|---|---|---|---|
| S1 | spot-check of the recorded "the hash is read little-endian": `u128::from_be_bytes` → `from_le_bytes` | killed | `key::tests::arq_141_the_key_is_the_hash_of_the_description` |
| S2 | spot-check of the recorded "the placement of the group is composed into its children": `Affine::translate((100,100)) * placement.to_transform()` | killed | `graph::tests::arq_141_the_transform_is_the_placement_of_the_node` (left `[1,0,0,1,105,105]`, right `[1,0,0,1,5,5]`) |
| H1 | **the domain byte is not `R`**: `DOMAIN = b'R'` → `b'X'` | **survived** | — |
| H2 | **the node type byte of a shape is not `2`**: `NodeType::Shape = 2` → `= 5` | **survived** | — |
| H3 | an input key is written little-endian | killed | `key::tests::arq_141_the_description_holds_the_bytes_of_the_section` |
| H4 | the big-endian `u32` count of input keys is not written | killed | `key::tests::arq_141_a_node_without_inputs_is_valid`, `..._the_description_holds_the_bytes_of_the_section` |
| H5 | the layer is popped with `1.0 - opacity` | killed | `graph::tests::arq_141_the_blend_and_the_opacity_are_emitted` |
| H6 | the extension records the node and draws a layer | killed | `graph::tests::arq_141_the_graph_holds_the_operations_of_the_page_and_none_of_the_extension` |

H1 and H2 are the two independent mutants the contract asks for; H3 to H6 are controls that show the suite does pin the neighbouring facts, so the two survivors are a hole and not a blind spot of the method. The two spot-checks confirm the recorded list of `tdd-apply`: both of its entries I tried behave exactly as the notes record.

D-35 is honoured by the final code and tied by a test: the transform of a node is its own placement (`graph.rs:354`), the child's placement is not composed with the group's, and S2 above is the forbidden bug, killed by `arq_141_the_transform_is_the_placement_of_the_node`.

## Findings

### CRITICAL

**C1. Two different parameters give the same content key.** class: implementation. Ref: `crates/gafira-render/src/key.rs:101-129` (`KeyParam::write`), `crates/gafira-render/src/lib.rs:20` (public `KeyParam`, `content_key`, `canonical_description`); AC2; ARQ-141; 02 §6.1.

The encoding of a parameter carries no tag of its kind, and four kinds share the single byte `0x00`: `Quality(Preview)`, `Geometry(Rect)`, `Fill(None)` and `Blend(Normal)`. `content_key(NodeType::Shape, 0, &[], &[KeyParam::Quality(RenderQuality::Preview)])` and `content_key(NodeType::Shape, 0, &[], &[KeyParam::Geometry(ShapeGeometry::Rect)])` return the same `ContentKey(83267603764345867193730438760013285531)`. The probe `probe_ac2_a_different_parameter_gives_another_key` fails on exactly that.

AC2 says "Given a node with a **different parameter** ... the key differs", and ARQ-141 says "cambiar cualquiera de ellos da otra clave"; §6.1 explains the canonical pattern so that "dos valores iguales den la misma clave", whose converse is that two different values give different keys. The implementation is right about the description of §6.1 but the encoding of the parameter is not injective over `KeyParam`, so the public key API can alias two different contents.

The collision is not reachable through `RenderGraphBuilder`: the builder fixes the parameter slots per node kind (a shape is quality, geometry, fill, transform, opacity and blend; a group is quality, transform, opacity and blend), so within the graph the kind of a slot never changes and the keys stay distinct. The exposure is the public API (`content_key`, `KeyParam` and `canonical_description` are exported from `lib.rs`), and the risk lands on `RE3`, whose cache is addressed by (`ContentKey`, level, column, row) with no quality and no other discriminator (ARQ-142): a collision there serves the wrong tile.

Root cause is the encoding, not the description: the fix belongs in `KeyParam::write`, which needs a tag of the kind per parameter, or in a distinct value encoding per variant.

**C2. The bytes §6.1 fixes by name are not pinned by any test.** class: implementation. Ref: `crates/gafira-render/src/key.rs:16` (`DOMAIN`), `crates/gafira-render/src/key.rs:25-30` (`NodeType`), `crates/gafira-render/src/key.rs:244` (the assertion that uses the constant); 02 §6.1.

§6.1 names the domain byte `R` and the node types `1` = Group, `2` = Shape, `3` = Extension. The test that checks the bytes of the description asserts them against the crate's own constants (`&description[0..3] == &[DOMAIN, NodeType::Shape as u8, 2]`), so the literals are never checked against the section. The mutants H1 (`DOMAIN = b'X'`) and H2 (`NodeType::Shape = 5`) both survive the whole delivered suite.

The content-key derivation is critical code for `RE1` (design, "Critical code"), and these two bytes are the ones another component has to agree on: a key computed with a different domain byte or a different type numbering does not match anything else that computes keys over the same description, and §6.1 reserves the numbering ("Un tipo nuevo toma el siguiente número libre y no reutiliza ninguno"). The fix is to assert the literals in the test — `b'R'`, `1`, `2`, `3` — instead of the constants.

### WARNING

**W1. The delivery is above the size ceiling with no recorded OK.** class: implementation. Ref: `apply-progress.md` ("The size"), `git diff --shortstat 7e3a23c 44b3269`; AGENTS.md "Delivery rules".

1571 lines against a ~700 ceiling and a 480 forecast (672 with the x1.4 factor): 2.34x the planning figure. The split is 809 lines of production code and 700 of tests; the forecast missed the pool seam and its thread test, the cancellation and the progress of ARQ-139, and the testkit doubles and their test, which account for some 300 to 400 lines, but the graph module alone is more than the whole forecast. The notes flag it and leave the decision to the coordinator; the decision still needs to be recorded as an OK. See "Size" above.

**W2. ARQ-117 is listed for `RE1` but has no testable criterion in this cut.** class: ambiguity. Ref: the Slice card "Product IDs", `specs/changes/layer-3/design.md` scope table ("ARQ-110, ARQ-111, ARQ-117"), 02 §4.17.7 "primer corte".

ARQ-117 is about pixel buffers (premultiplication of the 65 536 `(c, a)` pairs, unpremultiplication, the length error of a buffer); the graph of `RE1` holds no buffer, so no test can prove it here, and §4.17.7's first-cut table for `gafira-render` does not list ARQ-117. The delivery disclosed the reason in its notes, and no acceptance criterion of `RE1` depends on ARQ-117, so I read this as a WARNING and not as the CRITICAL that step 3c would make of a missing name literally, following the judgement the layer-2-scaffold precedent recorded in its W1. The artifact to reconcile is the card and the design: either ARQ-117 leaves the `RE1` list or the card says why it is claimed. The coordinator or the owner can overrule this reading.

### SUGGESTION

**S1.** The quality inside the content key is spec-consistent, but the coupling is written down nowhere. §6.1 puts "los parámetros muestreados en `t`" in the description and says that a parameter that does not depend on time "se incluye igual"; §6.2 makes the quality a parameter of the graph and ARQ-142 addresses the cache by (`ContentKey`, level, column, row) with no quality in it. Putting the quality in the key is therefore both allowed and necessary — without it the cache could serve a preview tile as an export one. The suggestion is to write that coupling in the design, because `RE3` works on the cache address and a reader who does not know it may "simplify" the key. My probe confirms AC6: the preview and the export hold the same operations and differ only in the quality, and the keys differ in every node, as that coupling implies.

**S2.** `GraphParams::t` is carried and never consumed, and that is right for this cut. §6.1 does not put `t` in the description; it puts "the parameters sampled at `t`", and the document model has no parameter that depends on time, so the key of a node is the same at every `t` today. The suggestion is to note in the design that a time-dependent parameter needs a sampling seam in `KeyParam`, so that `RE2` or `RE4` does not add `t` to the description by hand and break the canonical description of §6.1.

**S3.** The command recorded for AC8 cannot see an aliased import. The pattern `std::thread::spawn` and `std::time::(Instant|SystemTime)::now` miss `use std::thread;` followed by `thread::spawn(...)` and `use std::time::Instant;` followed by `Instant::now()`. The conclusion holds — I grepped the whole tree of the crate, every `use std::` import and every file, and the only `std::` uses are `collections`, `fmt`, `sync` and `thread::scope` inside the test module — and the criterion of the spec is the lint of the canonical `clippy.toml`, which does resolve through the imports and gives 0 warnings. The recorded command could be tightened so that the next verifier does not have to redo the work.

**S4.** `RenderGraph` does not implement `Debug`, so `Result<RenderGraph, GraphError>` cannot be unwrapped with `expect_err` in a test. It cost the probe a rewrite. A `Debug` derive or a manual implementation on the public types would remove the friction.

**S5.** The branch is behind `develop` by 15 commits, and the coordinator has to re-merge before merging the branch into `develop`. The re-merge is conflict-free (`crates/gafira-doc/tests/visit_contract.rs` is the only file that differs, and the branch does not touch it), but the branch also carries the copy of `DC1` that `develop` took by another route, so the merge will bring the visitor's history along with it.

## Notes

- The probe file (`crates/gafira-render/tests/probe_verify.rs`, 16 probes) and the mutant script live only in the scratch export `/tmp/gafira-verify-re-export`, outside the repo. They are not part of the delivery and were not committed.
- The scratch export needed one line the delivery does not have: `serde_json` as a dev-dependency of `gafira-render`, to build an `ExtensionNode` for the AC5 probe. The export's `Cargo.lock` was regenerated for it; the worktree's was not touched.
- The `cargo-mutants` run did not happen (`mutation: archive`, no report, owner decision O4). Per the contract that is not a finding; `mutation` is `null` in `verification.json`.
- The worktree was left as it was found: no commit, no branch change, no push.

next: specifier
