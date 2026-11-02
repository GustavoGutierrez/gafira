# Verify report: gafira-text, TX2, round 3 (closure)

Commit 3c7b184 on `feat/gafira-text`, base `develop`. Fix commits d284e4e, 413e9d9, f15b184; spec commits ed09718, 1f5fdf2. Depth V2, mode full, tier T4 at close, `mutation: archive`: no `mutants` report, and that is not a finding. Round 2 of TX2 follows this section.

**Verdict: FAIL.** 5 CRITICAL, 4 WARNING, 1 SUGGESTION. Classes: implementation 8 (5 CRITICAL), ambiguity 1 (follow-up). All gates are 0. Fix size: 411 lines added, 42 deleted in `crates/`, 112 added in `src/`.

Closed: C7, C8, C15, C17, and the one-word defect as stated. Still open: C16 (narrowed), W13, W26, W27. New: C18, C19, C20, C21, W28, S9.

The rewrite of the alignment holds in its main path: 44 608 layouts with no break of an invariant, and every difference from round 2 I sampled was intended. All five CRITICAL sit at its edges. Each has a small, local fix. This was the last authorized round and the third FAIL in a row with class `implementation`, so the coordinator stops and asks the owner.

next: tdd-apply

## Gates

`df -h /`: 25 GB free at the start, 31 GB at the end, after freeing 5.1 GB. Each gate ran alone on a clean export of HEAD, without `CARGO_TARGET_DIR`, and each exit code was read alone.

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (1 445 passed, 0 failed, 118 binaries) |
| `cargo deny --all-features check` | 0 |
| CI policy: ARQ-07, unsafe, V5, V12, ARQ-107 | 0 each (export) |
| CI policy: V13, V14 | 0 each (worktree, read-only) |

## Integrity (step 3c)

- `docs(gafira-text): slice card amend` (1f5fdf2) touches only `apply-progress.md`, changes one line of the card, and states why.
- ed09718 touches only `specs/04-...`. Both commits come before the fix commits, which touch only `crates/gafira-text/src/` and `tests/`.
- 3c7b184 touches only `apply-progress.md`, and its hunks start at line 195, so the card is unchanged.
- No change since round 2 to root `Cargo.toml`, `Cargo.lock`, `deny.toml`, `.github/` or `.cargo/`.
- FD-41 (c), (e) to (k) and (m) have tests that name them. (l) has none: see the ruling.

## Criteria

| Criterion | State | Proof |
|---|---|---|
| FD-41 (c) | covered | unchanged |
| FD-41 (e) | covered | formula holds. The Saltos rule beside it fails in the slack band, C18 |
| FD-41 (f), (g), (h) | covered | unchanged |
| FD-41 (i) | **failing** in wide boxes and with leading spaces | C18, C20, C21. Holds in the sweep: 4 605 justified lines, 0 bad |
| FD-41 (j) | **failing** in one DPI window | C16. Center, Right and Justify closed |
| FD-41 (k) | **failing** in a right-to-left paragraph, line as wide as the box | C19. The literal cases of (k) hold |
| FD-41 (l) | covered by construction, no test names it | see the ruling, W26 |
| FD-41 (m) | covered | 3 tests with the exact figures; my probe reads 4.50, 41.56, 38.19 |
| Fuente sin métricas, decorations | covered | unchanged |
| ARQ-111, ARQ-110 | covered | no infinity in `Ok` over the DPI sweep to 1e42 |
| ARQ-116 | covered | logical concatenation holds in all 44 608 layouts |
| XP12, TX1 W10, W12 | covered | unchanged |

17 criteria: 14 covered, 3 failing. Left out on purpose: FD-41 (b), per the card.

## Rulings asked by the coordinator

**The alignment deviation (no parley `align`).** The main path holds. No regression in TX1 or in Left, Center and Right of left-to-right text. Evidence:

- 44 608 layouts: 17 texts (RTL, mixed bidi, numbers, tabs, NBSP, explicit breaks, empty paragraphs, leading and trailing spaces), 2 font orders, 4 spacings, 82 widths, 4 alignments. Clusters touch with no gap, the concatenation gives the input, sums match, and overflow and line count do not change with the alignment.
- Of those, with lines that open on a space or end in an other-level space left out, 4 605 justified lines reach the edge. Right, Center and Left of the same lines have 0 bad.
- Empty texts, `"\n\n"`, `"\u{2028}a\u{2028}"`, `"\r\n"`, in every alignment: as ARQ-116 states, no panic.
- Differential against 2fcf998, 24 192 layouts: about 20 750 equal. Left in left-to-right text and all of Center and Right are unchanged. Left differs only in right-to-left paragraphs (C15). Justify differs in right-to-left text (C8, C15), on opposite-level one-word lines (the fix), and with U+2003: it now hangs at the end of a line and the ink reaches the edge. I sampled these groups by type; I did not check each of the roughly 3 400 differences alone.
- The forced-line test is safe. `is_hard_line_break` is `Whitespace::Newline`, the same condition that gives parley's `BreakReason::Explicit`, so U+2028 and U+2029 are forced too.

The edges that fail are C19, C20 and C21.

**The 4e-6 slack: closes C7, breaks something else (C18).** The exact fit holds: 1 620 layouts with lines up to 400 clusters, 2 scripts, 10 sizes, 9 spacings, 0 splits, 0 false overflows. But the overflow test has no slack, so inside the band a word stays on its line and reports overflow. The notes place that on "a cluster wider than the box"; it happens to any line (W28). The comment's "far under the 0.01 px of the spec" holds only under a box of about 2 500 px.

**The `f32::MAX` probe: closes C16 as stated, not fully.** Center and Right keep the room and the line count of Left, and the hang is in proportion to the DPI up to the limit. The note "a text that does not fit gives `NonFinite`" is false in one window, see C16.

**FD-41 (l): met by construction today, testable another way, not a real gap.** Class `implementation`, follow-up (W26 stays).

- By construction: `layout_box` has no hinting input, the only path goes through `ranged_builder(.., false)`, and FD-41 and 02 §6.2 give hinting only to the preview and the outlines. Mutant M1 (advances quantised) is killed by 8 tests.
- Testable now, against the unhinted metric rather than against a mode: an advance is `hmtx × px / upem`. My probe over 1 363 fractional sizes of `H` in Inter (1522 of 2048 units) stays within 5.6e-8 relative. A `fd_41_l_` test of this kind is about 15 lines and needs no new input. Inter has no `fpgm`, so it proves "no quantising", not "instructions ignored".
- The literal bit-equal check "with any mode" needs an API with a mode. FD-41 puts that on the outlines, so it belongs to TX4. TX4 should pin it.
- The note "(l) cannot be tested" is too strong; the card can stay as it is.

**C17.** Closed by acceptance and not implemented: no L1 code was added. The `fd_41_m_*` tests assert every figure of (m) within 0.01. Line 919 of 02 stays as the known doc follow-up, W27, not routed.

**The deviation on Center and Right with an other-level trailing space.** Consistent with (m). The 9 pt line whose content passes the box stays on the left, and the 12 pt line, which fits, is aligned on its content. See S9.

## Closure

| ID | Status | Evidence |
|---|---|---|
| C7 | closed | See the ruling. `Hello` 16 pt t=500 in a box of its own advance gives 1 line. C18 comes from this fix |
| C8 | closed | Arabic 12 pt, box 100, t=100: lines 0.00 to 100.00. 4 605 lines in the sweep |
| C15 | closed | `ابج دهو  ` in 200: ink 0 to 47.02, as without the spaces. Wrapped at 100: 0 to 98.21, no overflow |
| C16 | **open, narrowed** | Center, Right, Justify: same line counts as Left, text starts at the indent plus the hang. DPI 1e10 is proportional. The window is in C16 below |
| C17 | closed | Accepted limit, (m) pinned with its figures |
| One-word line | closed as stated | `abc ابج دهو def`, 12 pt, box 40: the second line equals Left, the space keeps 4.50, advance 22.996. The leading-space variant is C21 |
| W13 | open | Recorded, history cannot repair it |
| W26 | open, follow-up | See the ruling |
| W27 | open, follow-up | Card closed by 1f5fdf2. Line 919 of 02 remains, known and not routed |

## CRITICAL

**C16. class: implementation.** `list.rs:152`, FD-41 Listas. Narrowed. At DPI 1.7e39 to 2e39 the probe `1. x` wraps inside a box of `f32::MAX`, and `text_start` reads only `lines.first()`, so it returns the `x` of the space. `layout_list` gives `Ok` with a hang of 1.97e38 where 3.3e38 is due, and the text sits on the marker. At 1.5e39, 1.6e39 and from 2.1e39 it gives `NonFinite`, as the note says for all such cases. The window is far outside any real DPI, but TX1 set no DPI range and C9 and C16 were raised at such values. Giving `Err` when the probe has more than one line closes it.

**C18. class: implementation.** `layout.rs:209`, `layout.rs:216`, FD-41 Saltos, (i), ARQ-116. New, from d284e4e. The box is widened by 4e-6 for the break and not for `overflow`. In that band a word wider than the box is not broken and the line reports overflow, though no cluster is wider than the box. Round 1's C7 had the same shape.

| Text | Box short of the advance by | Lines | `overflow` | Widest cluster |
|---|---|---|---|---|
| `Hello`, 16 pt | 0.000201 px | 1 | true | 15.85 of 51.42 |
| `Hello`, 100 pt | 0.001253 px | 1 | true | 99.09 of 321.35 |
| `Hamburgefonstiv`, 1000 pt | 0.042280 px | 1 | true | 1167.97 of 10841.11 |

FD-45's "no hay desborde de clúster" reads this flag. In wide boxes the same band breaks (i): box 4 975.449, justified line ends 0.019 past the edge, 96 of 200 boxes in the band over 0.01; box 19 915.298, 0.077. `style.rs` tests 0.02 px short only, outside the band at that size.

**C19. class: implementation.** `layout.rs:514`, FD-41 Espacio que cuelga, (k). Center and Right return early when `room <= 0.0`, before any hang, so in a right-to-left paragraph the paragraph-level space stays inside the line. `ابج دهو ابج دهو ابج`, 12 pt, t=100, Right, box 56.6240044: ink 4.16 to 60.78, 4.16 px past the box, `overflow = false`. That box is the line's own ink, the C7 case. Left gives 0 to 56.62. A cluster wider than the box does the same: `ج ابج`, box 3, ink at 4.16 with Center and Right, at 0 with Left. Tests of this path use `W` in left-to-right text only. Round 2 had the same return, so this is not new; 413e9d9 fixed the hang for Left and left this path out.

**C20. class: implementation.** `layout.rs:472`, FD-41 (i). Regression from 413e9d9. The justified line is summed again in `f32`, so the error of each small advance on a large `x` adds up. Long lines at 12 pt, t=100:

| Box | Round 2 | HEAD |
|---|---|---|
| 2 000 | 0.0005 | 0.0035 |
| 4 000 | 0.0051 | **0.0161** |
| 8 000 | 0.0132 | **0.0420** |
| 16 000 | 0.0566 | **0.2441** |

At t=0 both give the same error, 0.0117 at 8 000. A scratch patch that sums in `f64` in that loop gives 0.00000 at every size, spacing 0 and 100, to 32 000 px. No justify test goes past 333.3 px.

**C21. class: implementation.** `layout.rs:462`, FD-41 Justificado, (i). Leading spaces count as places to stretch, so a line of one word is stretched. `  abcdefgh ijklmnop qrs`, 12 pt, box 90, Justify: first line ink 18.23 to 90, spaces of 9.11, against 9 to 80.77 with Left. Same on a line after a forced break. In a multi-word line the leading spaces take a share of the room, which FD-41 gives to the gaps between words. Inherited from parley, so not a regression, but the code is now this crate's and it excludes only trailing space. In a right-to-left paragraph the leading spaces sit on the right, so the ink stops short of the right edge.

## WARNING

| ID | Class | Ref | Finding |
|---|---|---|---|
| W13 | implementation | `list.rs`, f120b7e, Strict TDD | TDD breach kept on record |
| W26 | implementation, follow-up | FD-41 (l) | No test names (l). Met by construction; a linear-scaling test is possible now, the mode check belongs to TX4 |
| W27 | ambiguity, follow-up | 02 line 919 | Card closed; the stream row still stops at (j). Known, not routed |
| W28 | implementation | apply-progress TX2 fix round 3, items 2, 3; `layout.rs:121` | Three statements do not hold: the slack case is any line, not a lone cluster; a text that does not fit `f32::MAX` can give `Ok`; "far under the 0.01 px" fails above about 2 500 px |

## SUGGESTION

- S9: "esa línea no se centra ni se alinea a la derecha" in FD-41 can be read as every line with an other-level trailing space. The code keeps it only for a line whose content passes the box, which is the better behaviour. One clause would settle it.

S5, S7 and S8 stay open as they were.

## Mutants

No independent budget spent. M1 of FD-41 (l), advances quantised, reapplied on the export and restored: killed by 8 tests. 1 run, 1 killed. The scratch `f64` patch of C20 is a probe, not a mutant, and was restored; `layout.rs` compared equal to the worktree.

## Independent probes

On scratch exports only, in throwaway test files: exact fit, slack band, right-to-left exact fit, invariants, wide boxes, closure figures, empties, lists, DPI sweep, linear scaling, and the differential against 2fcf998. The worktree is clean at 3c7b184.

## Result

FAIL. CRITICAL 5, WARNING 4, SUGGESTION 1. Per class: implementation 8 (5 CRITICAL), ambiguity 1 (follow-up).

next: tdd-apply

---

# Verify report: gafira-text, TX2, round 2 (closure)

Commit 2fcf998c on `feat/gafira-text`. Fix commits c986a36, b007072, 689a84c, 9dbc3e8, 282db5c; specs merged in 16204dd. Depth V2, mode full, tier T4 at close, `mutation: archive`: no `mutants` report, and that is not a finding. Round 1 of TX2 follows this section and is in commit 084c45e.

**Verdict: FAIL.** 5 CRITICAL, 3 WARNING, 1 SUGGESTION. Classes: implementation 7 (5 CRITICAL), ambiguity 1 (follow-up). All gates are 0. Fix size: 845 lines added, 137 deleted, 242 added in `src/`.

Closed: C9 to C14, W14 to W25. Still open: C7 (narrowed), C8 (right-to-left only), W13 (stays recorded). New: C15, C16, C17, W26, W27, S8.

Second FAIL in a row with class `implementation` on TX2: the coordinator stops and asks the owner.

next: tdd-apply

## Gates

`df -h /`: 36 GB free at the start, 32 GB at the lowest. Each gate ran alone on a clean export with its own target dir, and each exit code was read alone.

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (1 434 passed, 0 failed, 118 binaries) |
| `cargo deny --all-features check` | 0 |
| CI policy: ARQ-07, unsafe, V5, V12, ARQ-107 | 0 each (export) |
| CI policy: V13, V14 | 0 each (worktree, read-only) |

## Integrity (step 3c)

- `docs(gafira-text): slice card` (2c96bde) touches only `apply-progress.md`. No line of it changed at HEAD.
- The fix commits touch only `crates/gafira-text/src/`, `crates/gafira-text/tests/` and `apply-progress.md`. Every commit since 084c45e that touches `specs/0*` is a `docs(specs)` commit merged from `develop`.
- No change to root `Cargo.toml`, `Cargo.lock`, `deny.toml`, `.github/` or `.cargo/`.
- FD-41 (c) to (k) have tests that name them. (l) has none (W26). Neither the card nor the stream row of 02 lists (k) and (l) (W27).

## Criteria

| Criterion | State | Proof |
|---|---|---|
| FD-41 (c) | covered | identity test; M1 killed |
| FD-41 (e) | covered | formula, `"ab  "` at t=500 ends on the edge; ligature exception stated |
| FD-41 (f) | covered | 61.743, 121.289, 241.992; the CSS model worked by hand from `hhea` matches at lh 0.5, 0.75, 1 and 3 |
| FD-41 (g), D-19 | covered | unchanged |
| FD-41 (h), ARQ-107 | covered | unchanged, ranges amended |
| FD-41 (i) | **failing** in right-to-left text with spacing | C8 |
| FD-41 (j) | **failing** with Center and Right | C16; holds with Left and Justify |
| FD-41 (k) | **failing** in a right-to-left paragraph | C15; the literal cases of (k) hold |
| FD-41 (l) | covered, no test names it | M1 killed by 8 tests; W26 |
| Fuente sin métricas | covered | zero-metrics probe: `"abc"` and `""` give `InvalidFont`; behind Inter, `Ok` |
| Subrayado y tachado | covered | fonts without `post` or `OS/2`: thickness 5.0 at 100 px |
| ARQ-111 | covered | unchanged |
| ARQ-110 | covered | 704 list layouts around the limit, 0 infinities in `Ok` |
| ARQ-116 ranges | covered | `straße`: `4..6, 4..6, 6..7` |
| XP12 | covered | unchanged |
| TX1 W10, W12 | closed | unchanged |

16 criteria: 13 covered, 3 failing. C17 also touches FD-41 (a), a TX1 row: its stated example passes, a wrapped line does not.

## Rulings asked by the coordinator

**The fit in a box wider by the spacing: sound, not exact.** The principle holds: 14 640 layouts over 6 texts (both paragraph directions, mixed runs) and t in -50, 0, 100, 300 match a greedy break oracle with no difference. It is not exact: float rounding leaves a box equal to the line's own advance 1 ulp short. See C7.

**Hanging space in an opposite-level run (coordinator's deviation 3, item 4 of the notes): the stated effect is not shown; a different harm is.** No early break was found. The 14 640 layouts above include lines that end in such a space. At t=0, 7 235 by-cluster layouts over 5 texts gave no early break. At t≠0 my by-cluster check counted false hits, so that case rests on the word-level oracle and on the code: `line_break.rs:1219` appends an atom when it fits without its hanging white space, and it does not look at the bidi level. Parley's level test sits in `hanging_whitespace`, which runs after the line is built. The note should be corrected. The real harm is that the space stays inside the line (C17).

**List spacing and kerning in the hang: accepted, no finding.** Spacing: FD-41 puts it after every cluster but the last of a line, and marker and space are followed by text. At t=100 the hang is 31.70, that is 25.30 + 4 × 1.6. (j) holds because all items share the hang. Kerning inside a marker is part of its width. Across the space the hang read the same for `x`, `T`, `V`, `a` and `y` in Inter, so the test letter adds nothing here. See S8.

**W13: stays recorded, open.** History cannot repair it. The RED condition is met as far as it can be: I replayed each fix commit's tests on its parent's source.

| Tests | Parent | Result |
|---|---|---|
| `extremes.rs` of c986a36 (C9, C10) | 16204dd | 4 of 5 fail on assertions; C9: left `None`, right `Some(NonFinite { field: "output" })` |
| `list.rs` of 9dbc3e8 | 689a84c | 7 of 16 fail on assertions; `["1.", "2.", "•", "1."]` against `"3."`; left `OutOfRange`, right `NoFont` |
| `align.rs`, `style.rs` of b007072 | c986a36 | 4 of 11 and 4 of 17 fail on assertions |

C13 could not go RED: the check already existed, so its test is test-only and its proof is M6, killed. The notes say so.

## Closure

| ID | Status | Evidence |
|---|---|---|
| C7 | **open, narrowed** | Round 1 cases pass: `Hello` t=500 in 94.10 gives 1 line; t=-100 in 41.82 gives 2 lines, no overflow. Exact fit fails, see below |
| C8 | **open, narrowed** | Left-to-right closed: right ends at 200.00002 for t=100, 500, -100; justified sweep passes. Right-to-left justify fails, see below |
| C9 | closed | Round 1 inputs give `NonFinite`. 704 list layouts, dpi 1e34 to 1e39: 500 `Ok`, 204 `Err`, 0 infinities. RED replayed |
| C10 | closed | Independent fonts without `post` and without `OS/2`: 5.0 at 100 px |
| C11 | closed | M3 killed |
| C12 | closed | M2 killed by 5 tests |
| C13 | closed | M6 killed by 2 tests |
| C14 | closed | FD-41 amended to the CSS model; numbers match; I also worked the model by hand from the tables at 4 line heights |
| W13 | open | Recorded, see the ruling |
| W14 | closed | M4 killed; the doc, the code and the test agree |
| W15 | closed | M5 killed by 2 tests that tie `height` to a measure |
| W16 | closed | Note corrected; probe matches |
| W17 | closed | Row amended; `Ok` behind Inter |
| W18 | closed | ARQ-116 amended; `straße` as stated |
| W19 | closed | Origin stated; test |
| W20 | closed | Hanging text stated; test (Left and Justify, see C16) |
| W21 | closed | Bullets stated; test |
| W22 | closed | Rule stated. The left-to-right code meets it; the right-to-left gap is C15 |
| W23 | closed | Rule stated; test |
| W24 | closed | Exception stated in (e) |
| W25 | closed | FD-41 now cites 02 §6.2, whose line on preview hinting is there |

## CRITICAL

**C7. class: implementation.** `layout.rs:203`, FD-41 Saltos, (e). A box as wide as the line's own output advance can still split it. `Hello` at 16 pt, t=500, advance 94.08333587646484: a box of exactly that gives 2 lines. Over 4 texts, 9 sizes and 7 spacings other than 0, 42 of 252 fail, with a margin of up to 0.000064 px needed; at t=0, 0 of 36. Same defect as round 1, a line split though it fits, now from rounding: the box sum and parley's sum disagree by an ulp. A box sized from a measured line wraps its text. The closure test cannot see it: `style.rs:403` adds 0.001 to the box.

**C8. class: implementation.** `layout.rs:215`, FD-41 (i). Right-to-left justified lines are off by one spacing. Arabic, 12 pt, box 100, t=100: three lines span -1.60 to 98.40, not 0 to 100. Parley's right-to-left offset does not allow for the spacing the output removes. The last line of `ابج دهو  ` in a box of 200 sits at 139.84 to 196.80, against 143.04 to 200.00 without the trailing spaces. `tests/align.rs` uses Latin text only.

**C15. class: implementation.** `layout.rs:213`, FD-41 Espacio que cuelga. "Cuelga en toda alineación" and "no cuenta para alinear" fail for `Left` in a right-to-left paragraph. `ابج دهو  ` in a box of 200 has ink at 9.0 to 56.36, not 0 to 47.36 as without the spaces. Wrapped at 100 it runs 4.5 to 103.73, 3.73 px past the box with `overflow = false`. A space before a forced break does the same. Notes item 2 records this as a choice ("Left alignment moves nothing"), but the amended rule leaves no such choice.

**C16. class: implementation.** `list.rs:141`, `list.rs:143`, FD-41 Listas, (j). `text_start` passes the list's style, alignment included, to a probe in a box of 1e9, so it reads the `x` of an aligned letter. With `Center`, item text starts at x = 500 000 030; with `Right`, at 1 000 000 000. The room falls to 0: an item that takes 3 lines takes 41 with `overflow = true`. All list tests use Left. The same fixed box breaks the probe at very high DPI: at 1e10 the hang is 0, against 1.6e9 needed, so text sits on the marker. Introduced by 9dbc3e8, the fix round.

**C17. class: implementation.** `layout.rs:400`, FD-41 Espacio que cuelga, (a). Parley does not apply UAX #9 rule L1 (its own comment, `line_break.rs:1898`), so a line's trailing space in an opposite-level run stays at its bidi position. Rule L1 puts it at the paragraph level, at the end of the line. Two effects. `abc ابج دهو def`, 12 pt, box 40, Left: the second line starts with the space at x=0 and its ink starts at 4.5. At 9 pt, box 39.3, the first line's ink ends at 41.56 with `overflow = false`, since `content_advance` drops a space that is still in the middle of the line. The output could move those clusters. If the owner prefers to accept parley's limit, FD-41 must say so, and that is a `specifier` decision.

## WARNING

| ID | Class | Ref | Finding |
|---|---|---|---|
| W13 | implementation | `list.rs`, f120b7e, Strict TDD | TDD breach for lists, kept on record; see the ruling |
| W26 | implementation, follow-up | FD-41 (l) | No test names (l), and `layout_box` has no hinting input, so it holds by construction. M1 pins it, killed by 8 tests. New test gap after round 1 |
| W27 | ambiguity, follow-up | Slice card, 02 line 919 | Both still say FD-41 up to (j). (k) and (l) need a `slice card amend` and a row change |

## SUGGESTION

- S8: `text_start` measures with the letter `x`, so a font that kerns the space with `x` would shift every hang by that pair. FD-41 could state the measure in one line: marker and space with the list's spacing.

S5 and S7 stay open as they were. The notes close S6 with a sweep test; I did not re-examine it.

## Mutants reapplied

No independent budget spent. Scratch export of HEAD, restored after each, compared equal to the worktree at the end.

| # | Rule and wrong behaviour | Result |
|---|---|---|
| M1 | FD-41 (l): advances quantised | killed, 8 tests |
| M2 | forced break is the last cluster | killed, 5 tests |
| M3 | last cluster in visual order | killed, `fd_41_e_the_last_cluster_in_logical_order_...` |
| M4 | list levels before fonts | killed, `arq_110_a_list_checks_...` |
| M5 | `height` doubled | killed, 2 tests |
| M6 | no `NoFont` check in a list | killed, 2 tests |

6 run, 6 killed.

## Result

FAIL. CRITICAL 5, WARNING 3, SUGGESTION 1. Per class: implementation 7, ambiguity 1 (follow-up).

next: tdd-apply

---

# Verify report: gafira-text, TX2, round 1

Commit 2b8c276f on `feat/gafira-text`, base 0600d68. Depth V2, mode full, tier T4 at close, `mutation: archive`: no `mutants` report, and that is not a finding. Steps 1c and 4b ran in full, with probes written before reading the implementer's tests.

**Verdict: FAIL.** 8 CRITICAL, 13 WARNING, 3 SUGGESTION. Classes: implementation 11 (7 CRITICAL), ambiguity 10 (1 CRITICAL). All gates are 0. Size: 1 417 lines, 281 source (forecast 615); noted, no finding.

next: specifier, then tdd-apply

## Gates

Each gate ran alone on a clean export with its own target dir, and each exit code was read alone.

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (1 407 passed, 0 failed, 117 binaries) |
| `cargo deny --all-features check` | 0 |
| CI policy: ARQ-07, unsafe, V5, V12, ARQ-107 | 0 each (export) |
| CI policy: V13, V14 | 0 each (worktree, read-only) |

## Integrity (step 3c)

- `docs(gafira-text): slice card` (2c96bde) touches only `apply-progress.md`. Its lines are unchanged at HEAD: TX2 commits only add to the file.
- No TX2 commit touches `specs/0*`, the layer-2 design, root `Cargo.toml`, `Cargo.lock`, `deny.toml`, `.github/` or `.cargo/`.
- Every TX2 letter of FD-41 has tests that name it (`fd_41_c` to `fd_41_j`). No TX2 ID was left out. D-19, XP12 and ARQ-107 are named in test headers; no test names ARQ-42 (W25).

## Criteria

| Criterion | State | Proof |
|---|---|---|
| FD-41 (c) | covered | `style.rs` identity; M1 killed |
| FD-41 (e) | covered, with survivors | formula and ligatures hold (p11). C11, C12 are test gaps, C7 is the fit rule |
| FD-41 (f) | **failing** on a mixed line | C14; passes on one font and on nested metrics |
| FD-41 (g), D-19 | covered | `fallback.rs`; P1 killed; cmap premises checked with fonttools |
| FD-41 (h), ARQ-107 | covered | `case.rs`, p12; `Locale` 0 matches, policy step 0 |
| FD-41 (i) | **failing** with spacing | C8; passes at spacing 0 |
| FD-41 (j) | covered | `list.rs`, p08; spot-check killed |
| Fuente sin métricas (W6) | covered | `style.rs`, p09; W16, W17 |
| Subrayado y tachado | **failing** on an absent table | C10 |
| ARQ-42 (hinting) | covered | layout not quantised; M1 killed by 5 tests |
| ARQ-111 | covered | `styled.rs`. `parley` has no `system` feature, so no machine font enters a layout (p03, p04) |
| ARQ-110 | **failing** | C9 |
| XP12, parley half | covered | `fallback.rs`, P1 |
| XP12, `skrifa` half | covered by this verify | Not run in TX2. P14 entry on a scratch copy: `Cargo.lock` 155 packages before and after, one line added (the dependency of `gafira-text`), `skrifa` 0.44.0; `cargo deny` 0, V5 0, ARQ-107 0. The design still lists XP12 as open |
| TX1 W10, W12 | closed | `layout.rs` tests; `content_advance` linear by reading |

15 criteria: 11 covered, 4 failing. TX1 W7, W8, W9 and W11 stay open as they were; W9's row still says invalid font while the code gives `NoFont`.

## CRITICAL

**C7. class: implementation.** `crates/gafira-text/src/layout.rs:200`, FD-41 Saltos, Interletrado, ARQ-116. Line breaking uses parley's width, which holds one spacing after the last cluster; the output removes it afterwards, so the two measures disagree. At 16 pt and 96 dpi, `Hello` t=500 has advance 94.08 and in a box of 94.10 it is split into 2 lines, though it fits. `Hello` t=-100 has advance 42.88 and in a box of 41.82 stays on 1 line with `overflow = true`, though no cluster is wider than the box and a word must break by cluster.

**C8. class: implementation.** `layout.rs:214`, FD-41 (i) and alignment. `align` runs on parley's advance, which holds that same spacing. FD-41 (i) fails: Justify t=100, width 90, a line ending in `-` ends at 88.4, not 90 ±0.01 (t=500, width 150: 142). Right, one line of `Hello` in a box of 200: t=100 ends at 198.40, t=500 at 192, t=-100 at 201.60, past the box. Center drifts by half of that. `align.rs` tests spacing 0 only.

**C9. class: implementation.** `layout.rs:334`, `list.rs:85`, `list.rs:101`, ARQ-110. `Ok` with an infinity in the output. `check_output` skips `height`: `layout_box("ab\ncd")` at 1000 pt, dpi 1.02e37, gives `height = inf` with finite baselines. `layout_list` has no output check after its own sums: a level 4 bullet with empty text at dpi 5.5e36 gives `indent = inf` and cluster `x = inf`; three items at dpi 6e36, t=500, give `top = inf`. `styled.rs` tests dpi 96 only and never calls `layout_list` at the extremes.

**C10. class: implementation.** `layout.rs:300`, FD-41 Subrayado y tachado. A font with no `post` table gives thickness 5.556 at 100 px (parley's 1/18 em), not 5.0 (0.05 em). `decoration` sees only a positive number, so its comment "0 or none" is false. Parley's `shaped_text.rs:107` uses the same fallback for a missing strikeout. Apply-progress item 8 of the deviations names the gap but leaves it open.

**C11. class: implementation.** `layout.rs:279`, FD-41 Interletrado. Hand mutant M3 survived: take the last cluster in visual order, not logical. Right-to-left text then loses the spacing on its first cluster. The line total is unchanged, and the per-cluster test uses `abc` only.

**C12. class: implementation.** `layout.rs:278`, FD-41 Interletrado. Hand mutant M2 survived: a forced break counts as the last cluster. It then gets a negative advance and the last visible cluster keeps its spacing. The tests read line totals only, and those match.

**C13. class: implementation.** `list.rs:66`, Lista de fuentes vacía. Hand mutant M6 survived: remove the `NoFont` check. `layout_list(&[], &[], ..)` would give `Ok`; only that check stops it, and no test calls it with both lists empty.

**C14. class: ambiguity.** FD-41 Interlineado, (c), (f). `Inter` + `NotoEmoji` on each line, 100 px; their ascents and descents do not nest. Formula gives 120.996 x lh. Measured distance:

| lh | Measured | Formula |
|---|---|---|
| 0.5 | 61.743 | 60.498 |
| 1.0 | 121.289 | 120.996 |
| 2.0 | 241.992 | 241.992 |

So (f) fails on that text: 241.992 is not 2 x 121.289, off by 0.59 px. FD-41 contradicts itself here: the formula says the largest of the sums, while "referencia de `line-height: normal`" and the identity of (c) both give parley's union of ascent and descent (ascent of Inter + descent of Noto Emoji). Both cannot hold, so `specifier` decides before any code changes. `style.rs` uses a taller font that nests, so it cannot see this.

## WARNING

| ID | Class | Ref | Finding |
|---|---|---|---|
| W13 | implementation | `list.rs`, f120b7e, Strict TDD | **TDD breach for lists.** Implementer's own record: list code and tests were written together and RED was shown afterwards on a stub, which the owner forbids. Not raised to CRITICAL: no rule sets its severity, and history cannot repair it. The tests do bite on the spot-check. The owner can raise it. The fix round for C9 and C13 must show real RED first |
| W14 | implementation | `list.rs:50`, `list.rs:61`, `tests/list.rs:162` | Doc comment and test name say fonts are checked before levels; the code checks levels first (p08). M4 swaps them and survives. No spec fixes the order, so the doc or the code must change |
| W15 | implementation | `layout.rs:219`, `tests/list.rs:135` | M5 doubles `height` and survives: `top == height` is a tautology and `height > 0` is the only other check. No test ties `height` to a measure, and TX3 will build on it |
| W16 | implementation | apply-progress TX2, deviation 6 | Note says a zero-metrics font with empty text is `Ok`. Measured: `Err(InvalidFont { index: 0 })` for `""` alone, which fits the row. Note is wrong, no test covers it |
| W17 | ambiguity | Fuente sin métricas | A zero-metrics font nothing uses gives `Ok`, pinned by a test. The row has no such limit, while Lista de fuentes vacía does apply to empty text. Whether unused fonts are checked is not stated |
| W18 | ambiguity | ARQ-116, FD-41 Mayúsculas | Ranges index `TextLayout::text` after upper case; ARQ-116 says concatenation gives the input text, and `straße` gives 7 clusters over `STRASSE`. No map to the source text, which editing and FV-07 may need |
| W19 | ambiguity | FD-41 Listas, (j) | Step of 1.5 em is stated; the origin is not. `(level - 1) x 1.5 em` and `level x 1.5 em` both meet (j) |
| W20 | ambiguity | FD-41 Listas | One space after the marker is stated nowhere, and wrapped lines return under the marker (p08). A designer expects hanging text, with `9.` and `10.` aligned. Neither is specified |
| W21 | ambiguity | FD-41 Listas | A bullet resets the count of its level (`1. 2. • 1.`). Not stated; `3.` is as defensible |
| W22 | ambiguity | FD-41, ARQ-116 | Right alignment with trailing spaces: FD-41 is silent, so the implementer broke no rule. `"ab  "` ends 12 px short of the edge in a box of 100, yet a wrapped line hangs its space and ends on the edge, and ARQ-116 hangs that space for overflow. A designer expects text on the edge in both cases. Recommended: hang it |
| W23 | ambiguity | FD-41 Interletrado | "El último de la línea" on a line that ends in a hanging space: the space loses the spacing and the last letter keeps it, so its ink stops one spacing short (t=500 at 16 pt, 96 dpi: 10.7 px). The rule must say which cluster counts |
| W24 | ambiguity | FD-41 (e) | "Mide lo mismo que sin él" cannot hold where a ligature forms at 0: Manrope `fifl`, t=100, 0.344 px off. The code follows CSS; the criterion needs that exception |
| W25 | ambiguity | FD-41 Hinting | Cites ARQ-42, whose row in 02 is about preview and export sharing a graph, not hinting. No test names ARQ-42. The behaviour holds |

## SUGGESTION

- S5: `◦` and `▪` are `.notdef` in the Inter subset (no cmap entry), so levels 2 and 3 of every list test render a missing glyph. Check markers with a stack that covers them.
- S6: The recorded survivor "overflow read after the alignment" may not be equivalent: justified lines reach 200.00002 at t≠0 (p01). The code is right; a spacing case would pin it. Also, "3 601 widths" in the notes is not in the repository, whose test has 4.
- S7: `fallback.rs` takes coverage from comments, not from `cmap`. All premises are true (checked with fonttools); reading the table as `style.rs` does would keep them true.

## Classification asked by the owner

| Item | Class | Where |
|---|---|---|
| API additions | derivable: FD-41 needs each input and (j) needs a list call; names are free; `height` follows from FD-46 | no finding; W15 for its test |
| Ranges after upper case | ambiguity | W18 |
| List indent `(level - 1) x 1.5 em` | step stated, origin ambiguous | W19 |
| Space after a marker | ambiguity | W20 |
| A bullet restarting the count | ambiguity | W21 |
| Spacing on a forced break | derivable: (e) counts clusters of a line, and a break is not visible | no spec finding; C12 for its test |
| Zero-metrics font, empty text | stated by the row; the code meets it, the note is wrong | W16; W17 for unused fonts |
| Right alignment, trailing spaces | FD-41 silent, so ambiguity; against a designer's expectation it is wrong | W22 |

**Mutants "inside parley".** The claim holds for `style.rs`: `natural_height` reads `head`, `OS/2` and `hhea` by offset (offsets checked), and also runs on patched copies. Applied live on a scratch copy of parley, both are killed: P1 (first font covering any point) by 2 tests of `fallback.rs`, P2 (first run only) by `fd_41_f_the_natural_height_is_the_largest_...`. `fallback.rs` expects literals, not table reads (S7). "Descent sign" and `USE_TYPO_METRICS` were not applied live; they live in `skrifa`, and the table tests cover them by reading.

## Hand mutants

| # | Rule and wrong behaviour | Result |
|---|---|---|
| spot | recorded item 9: indent 1.4 em | killed, `fd_41_j_a_list_of_three_levels_...` |
| M1 | ARQ-42: advances quantised | killed, 5 tests |
| M2 | forced break is the last cluster | **survived**, C12 |
| M3 | last cluster taken in visual order | **survived**, C11 |
| M4 | fonts checked before levels in a list | **survived**, W14 |
| M5 | `height` doubled | **survived**, W15 |
| M6 | no `NoFont` check in a list | **survived**, C13 |
| P1 | parley: first font covering any point | killed, 2 tests |
| P2 | parley: first run only | killed, 1 test |

9 run, 4 killed.

## Independent probes

Run on a scratch export only: p01 to p13, q1 to q4. Findings above quote them. Checked and sound: bidi and marks under spacing (N-1 formula exact on 7 of 8 texts, W24 is the eighth), upper case on 7 texts, justified Arabic and Hebrew, one-word and forced-break lines, list level bounds, and no system font in any of 10 scripts and symbol sets tried.

---

# Verify report: gafira-text, TX1, round 2 (closure)

Commit d44eadc5 on `feat/gafira-text`. Fix commits 63e751a and 17eafac, specs merged in a37dd52. Depth V2, mode full, tier T4 at close, `mutation: archive`: no `mutants` report, and that is not a finding. Round 1 is in commit 8014656 and in `verification.json`.

**Verdict: PASS for delivery TX1.** 0 CRITICAL, 6 WARNING (all new), 0 SUGGESTION. Classes: implementation 3, ambiguity 3. All of C1 to C6 and W1 to W6 are closed. TX2 to TX4 remain, so this is not a change-level PASS. W7 needs the owner and W9 needs `specifier`; neither blocks.

## Gates

Each gate run alone, exit code read alone, on a clean export with its own target dir.

| Gate | d44eadc (HEAD) | 63e751a alone |
|---|---|---|
| `cargo fmt --check` | 0 | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | 0 |
| `cargo test --workspace --locked` | 0 (1 318 passed, 110 binaries) | 0 (1 313 passed, 110 binaries) |
| `cargo deny --all-features check` | 0 | 0 |
| CI policy: ARQ-07, unsafe, V5, V12, ARQ-107 | 0 each | 0 each |
| CI policy: V13, V14 | 0 each (worktree, read-only) | 0 each (export after `git init`, files untracked) |

17eafac has no export of its own: HEAD differs from it only in `apply-progress.md`, so the HEAD column is its tree. The counts add up: 1 309 in round 1, +4 tests in 63e751a, +5 in 17eafac.

## Integrity (step 3c)

- `docs(gafira-text): slice card` (2c96bde) touches only `apply-progress.md` and precedes the first implementation commit.
- The card's 9 lines are byte-identical at HEAD. 03b9172 and d44eadc only add to the file.
- No implementer commit touches `specs/0*` or the layer-2 design. The spec changes came through `develop` (464e8d3, de4fdcd, bd98d79, 639f94f).
- No change to root `Cargo.toml`, `Cargo.lock`, `deny.toml`, `.github/` or `.cargo/` since the card.
- Every TX1 ID has tests that name it. Left out on purpose: all TX2 to TX4 IDs of the card.
- Fix size: 253 lines added, 10 deleted.

## Closure

| ID | Status | Evidence |
|---|---|---|
| C1 | closed | `content_advance` strips the logical end of each line. Mutants M1 to M3 killed. Independent sweep: 10 more texts (bidi, NBSP, tab, U+3000, U+2003, emoji, forced breaks), 1/16 px steps to 200 px, 32 010 layouts, 0 wrong against "a non-white-space cluster is wider than the box". See W12 for its cost |
| C2 | closed | I1 killed by `arq_116_overflow_looks_at_every_line` |
| C3 | closed | I4 killed by `arq_116_a_cluster_with_a_later_glyph_without_cover_is_notdef` |
| C4 | closed | I3 and I5 killed by `arq_110_the_numbers_are_checked_in_order_...` |
| C5 | closed | `index` is `Int`; the Text mutant is killed by `arq_102_every_text_variant_gives_its_key_and_arguments` |
| C6 | closed | Provenance says item 6 (b) is VERIFICAR, not done. Restoring the old sentence is killed |
| W1 | closed | Row amended in 464e8d3. Probes: `" "`, `"   "`, `"\t"`, `"\n"`, `"\r\n"` no overflow; `"a"`, `" a"` overflow. Test gap in W10, one corner in W11 |
| W2 | closed | Row amended in de4fdcd. Probes: `""` 1 line, `"abc\n"` 2, `"abc\r\n"` 2, `"\n"` 2, `"a\n\nb"` 3, each as stated. Test gap in W10 |
| W3 | closed | Row amended in bd98d79. `"abc"` and `""` give `Err` with an empty list, and NaN DPI gives `NonFinite` first. Three mutants killed. Variant in W9 |
| W4 | closed | Field list fixed in 639f94f. All 5 entries have the 7 fields, fonts add repository, commit and subset command, the splash adds the 7 of D-09 and `Propia`. Four removal mutants killed |
| W5 | closed | Name ID 0 read with fonttools: Inter 2016, Manrope 2019, Arabic 2022, Emoji 2013, all equal to `autoría`. The OFL line is in `copyright_ofl_txt`. Mutant killed |
| W6 | closed | Still not built, as agreed, and now recorded for TX2 in `apply-progress.md` |

## BLAKE3 values

Recomputed with a pure-Python BLAKE3 written from the specification, not with the `blake3` crate the implementer used. It passes the empty-input and `abc` vectors. All 5 values match their files at HEAD.

| File | Bytes | Result |
|---|---|---|
| `assets/fonts/test/Inter-test.ttf` | 265 896 | match |
| `assets/fonts/test/Manrope-test.ttf` | 88 376 | match |
| `assets/fonts/test/NotoSansArabic-test.ttf` | 323 992 | match |
| `assets/fonts/test/NotoEmoji-test.ttf` | 5 992 | match |
| `assets/splash_screen.jpg` | 296 847 | match |

## Rulings on the splash, against D-09

| Field | Value | Ruling |
|---|---|---|
| `exige atribución` | `false` | **Holds.** D-09 and item 6 of `00`: the OpenAI terms allow commercial use "sin atribución ni regalías". The "generada con IA (OpenAI)" credit is a separate condition, not to pass it off as human work, and `ai_notice` carries it |
| `modificado` | `false` | **Not derivable from D-09.** D-09 records no edit, but it does not say there was none. It says the repository copy has no C2PA metadata, and item 6 (b) speaks of an original download with C2PA, so the file in the repository is not those bytes. See W7 |

File facts checked: JPEG, 1672×941, 296 847 bytes, as in D-09. It has one JFIF APP0 segment, no EXIF and no JUMBF, which agrees with "no C2PA". It cannot show whether the pixels changed.

## Findings

### WARNING

**W7. Splash `modificado = "false"` is not established.** class: ambiguity. Ref: `assets/LICENSES.toml`, D-09, FV-38.
See the ruling above. FV-38 does not say whether removing metadata counts as modified, and no test pins the value (mutant survives). The header comment already asks for the owner's confirmation, so this is not a false statement like C6. Owner: say what happened to the file, and whether a metadata strip counts.

**W8. No test recomputes BLAKE3.** class: implementation, follow-up. Ref: `crates/gafira-testkit/tests/licences.rs`.
One digit changed in a hash passes every test, so a replaced asset leaves a stale hash unseen. No stated rule forbids the absence: FV-38 and ARQ-75 ask for the field, and ARQ-75 places the 100 % check in `xtask`. `blake3` is listed in 02 §9, and P2 makes a later dependency a serial merge, not a ban. It needs a root `Cargo.toml` and `Cargo.lock` change, out of this round.

**W9. `NoFont` against "`Err` de fuente inválida".** class: ambiguity. Ref: 02 §4.17.3 "Lista de fuentes vacía", `crates/gafira-text/src/error.rs`.
The row names the invalid-font error, "Fuente sin métricas" uses the same words, and the code returns a separate variant and key, `error.text.no_font`. `InvalidFont` needs an `index`, which an empty list does not have, so the variant is reasonable, but it was not recorded as a deviation. Keys are a stable contract. `specifier`: either word the row as its own error, or state how it maps to `InvalidFont`.

**W10. The amended acceptance rows are only partly tested.** class: implementation, follow-up. Ref: ARQ-116, 02 §4.17.3 "Caja de tamaño 0".
No test has `" "`, `"   "`, `"\t"`, `"\n"`, `"\r\n"` or `" a"` in a box of 0, or `"abc\r\n"` for the line count. Mutant: only U+0020 and newline count as white space; **survived**. All these cases pass by probe, so the behaviour is right today. Step 3c alone would make a missing proof CRITICAL; the closure rule for a new test gap after round 1 is the more specific one, so this is a WARNING.

**W11. Non-white-space clusters with no advance, in a box of 0.** class: ambiguity, follow-up. Ref: 02 §4.17.3 "Caja de tamaño 0", ARQ-116.
U+200B, U+200C, U+200D, U+00AD, U+FEFF, U+2060, U+200E and U+200F alone give `overflow = false`. The row, read literally, asks for overflow, since each is a cluster outside `White_Space`. ARQ-116 compares advances, and 0 is not greater than 0, so the two rules disagree here. Nothing visible is lost, so the code is not at fault. Suggested wording: "un clúster con avance mayor que 0".

**W12. `content_advance` is quadratic in trailing white space.** class: implementation, follow-up. Ref: `crates/gafira-text/src/layout.rs`, `trailing.contains(i)`, added by the C1 fix.
100 000 spaces on one line take 27 s in a debug build, and 0.19 s with a set in place of the slice lookup (scratch patch, reverted). Text of the same length with words takes 0.14 s. No spec row sets a budget and I did not measure a release build, so this is not CRITICAL. Worth fixing before FD-45 calls layout repeatedly.

S1 to S4 of round 1 were not re-examined; they stay open in that report.

## Criteria

| ID | Status | Proof |
|---|---|---|
| FD-41 (a) | covered | `fd_41_a_*`, unchanged |
| FD-41 (d) | covered | `fd_41_d_*`, unchanged |
| FD-41 empty font list | covered | `fd_41_an_empty_font_list_...`; variant in W9 |
| ARQ-115 | covered | `arq_115_*` |
| ARQ-111 | covered | `arq_111_*` |
| ARQ-75 (partial file) | covered | `arq_75_*`; hash values by this report, W8 |
| ARQ-116 | covered | `arq_116_*`; amended rows partly by probe, W10 |
| ARQ-110 | covered | `arq_110_*` |
| ARQ-102 | covered | `tests/message.rs`. The catalogue does not exist yet |

## Mutants reapplied

21 on a scratch export of HEAD, restored after each, and the export compared equal to the worktree at the end. 18 killed, 3 survived. No independent budget was spent: these reapply round 1 and check closure.

| # | Mutant | Result |
|---|---|---|
| M1 | C1: `metrics.hanging_advance` back | killed, sweep test |
| M2 | C1: no white space excluded | killed, sweep and hanging-space tests |
| M3 | C1: trailing clusters taken in visual order | killed, sweep test |
| I1 | Only the last line decides | killed |
| I4 | `.notdef` read from the first glyph | killed |
| I3 | Ranges before non-finite checks | killed |
| I5 | Numbers after fonts | killed |
| M4 | `index` as Text | killed |
| M5 | No empty-list check | killed |
| M6 | Empty-list check before the numbers | killed |
| M7 | Empty-list check only for non-empty text | killed |
| M8 | Provenance says the original "is kept" | killed |
| M9 | Splash licence not `Propia` | killed |
| M10 | `modificado` removed, Inter | killed |
| M11 | `exige atribución` removed, splash | killed |
| M12 | `hash` emptied, splash | killed |
| M13 | `origin` removed, Emoji | killed |
| M14 | Inter `autoría` back to 2020 | killed |
| M15 | Only U+0020 and newline are white space | **survived** (W10) |
| M16 | One hash digit changed | **survived**, expected (W8) |
| M17 | Splash `modificado` flipped | **survived**, expected (W7) |

## Result

PASS for TX1. CRITICAL 0, WARNING 6, SUGGESTION 0. Per class: implementation 3, ambiguity 3.

next: tdd-apply (TX2)
