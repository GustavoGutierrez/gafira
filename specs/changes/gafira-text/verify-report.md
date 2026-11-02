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
