# Verify report: gafira-text, TX1, round 1

Commit 03b91728 on `feat/gafira-text` (base 9124700). Depth V2, mode full, tier T4 at close, `mutation: archive`: no `mutants` report, and that is not a finding.

**Verdict: FAIL.** 6 CRITICAL, 6 WARNING, 4 SUGGESTION. Classes: implementation 8, ambiguity 4.

## Gates

Each run alone on a clean export of HEAD, with its own target dir.

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (1 309 passed, 110 binaries) |
| `cargo deny --all-features check` | 0 |
| CI policy: ARQ-07, unsafe, V5, V12, ARQ-107 (`Locale`, ICU) | 0 each |
| CI policy: V13, V14 (need git, run in the worktree, read-only) | 0 each |

## Integrity (step 3c)

- `docs(gafira-text): slice card` (2c96bde) touches only `apply-progress.md` and precedes the first implementation commit.
- 03b9172 only adds to that file (59 insertions, 0 deletions), so the card is unchanged.
- No change to root `Cargo.toml`, `Cargo.lock`, `ci.yml` or `deny.toml` (P2). `crates/gafira-testkit/tests/licences.rs` is the file P7 calls for.
- Every TX1 ID has tests that name it. Left out on purpose: all TX2 to TX4 IDs of the card.
- Size: 1 258 authored lines, 380 source, confirmed from the numstat. The owner said this does not concern him. Noted, no finding.

## Findings

### CRITICAL

**C1. `overflow` is true when the text fits, in mixed-direction text.** class: implementation. Ref: `crates/gafira-text/src/layout.rs:138`.
parley hangs trailing white space only when its bidi level equals the paragraph's (`hanging_whitespace` in `parley/src/layout/line_break.rs:1901`, "We don't currently apply UAX #9 L1"). A trailing space inside an RTL run of an LTR paragraph is therefore not excluded. `"a ابج دهو"`, Inter + Arabic, width 32: line `a␠جبا␠` has advance 36.48, content 31.98 ≤ 32, and `overflow` is true. The same text at width 20 is also wrong (line advance 22.996, content 18.496). Sweep at 1/16 px steps up to 250 px: `"hello world ابج دهو"` wrong at 322 of 3 999 widths, up to 110 px; `"مرحبا بالعالم hello world"` at 262; pure LTR at 0. It contradicts FD-41 Saltos and the doc comment of `layout_box`, and FD-46 "Cabe" would inherit it in TX3. `metrics.hanging_advance` is not the right input: the exclusion has to come from the logically trailing white-space clusters. The only test of the rule is LTR (`arq_116_trailing_space_that_hangs_past_the_box_is_not_overflow`).

**C2. Survivor I1: only the last line decides `overflow`.** class: implementation. Ref: `layout.rs:139`, ARQ-116.
Mutant `|=` to `=`: all tests pass. No test has a too-wide cluster on a line other than the last (`"W\ni"` in 10 px is the case).

**C3. Survivor I4: `.notdef` seen only in the first glyph.** class: implementation. Ref: `layout.rs:130`, ARQ-116, FD-41.
Mutant `glyphs.contains(&0)` to `glyphs.first() == Some(&0)`: all tests pass. `q` + U+0301 in Inter gives glyphs `[233, 0]` and is the uncovered case. No test has a cluster where a later glyph is the `.notdef`.

**C4. Survivors I3 and I5: check order.** class: implementation. Ref: `layout.rs:83`, `layout.rs:188`, ARQ-110.
I3 moves the range checks before the non-finite checks of `dpi` and `bounds`: size 0.5 with NaN `dpi` gives `OutOfRange` and all tests pass. I5 moves `check_input` after font registration: NaN `dpi` with junk font bytes gives `InvalidFont` and all tests pass. The code is right today (`NonFinite { field: "dpi" }` in both cases), and ARQ-110 says numbers are checked before computing, but every test sets one bad input at a time.

**C5. ADR 0012 item 3: a number in `Text`.** class: implementation. Ref: `crates/gafira-text/src/error.rs:50`, `crates/gafira-text/tests/message.rs:34`.
`index = Text(index.to_string())`. The rule is that numbers go in `Int`, `Count`, `Decimal` or `Quantity`; `gafira-pathops` uses `Int` for its index. The test pins the wrong form. Keys are a stable contract, so this is cheapest to fix now.

**C6. The splash entry states a pending item as done.** class: implementation. Ref: `assets/LICENSES.toml`, `provenance` of `assets/splash_screen.jpg`.
It says "The original download with its C2PA metadata is kept outside the repository as proof." D-09 does not say that, and pending item 6 (b) of `00-decisiones.md` keeps it as VERIFICAR for the owner. The `similarity_check` field covers item (a) only. A licence record must not assert what the source leaves open. Item (b) should read VERIFICAR, as (a) does, and the test should pin it.

### WARNING

**W1. Box of size 0 against hanging white space.** class: ambiguity. Ref: 02 §4.17.3 "Caja de tamaño 0", FD-41 Saltos.
The row says there is overflow if the text is not empty. `" "`, `"   "`, `"\t"`, `"\n"` and `"\r\n"` in a box of 0 give `overflow = false`. The row is tagged FD-45 (TX3), but it is in the `gafira-text` table and decision (b) of TX1 already conflicts with it. `specifier` should say whether white space alone counts.

**W2. Final newline and empty text.** class: ambiguity. Ref: ARQ-116, `apply-progress.md` deviation 2 (a).
Neither line count is stated, and tests pin both. See the ruling below.

**W3. An empty font stack loses the text.** class: ambiguity. Ref: `layout.rs:76`, FD-41 Fuentes de reserva, ARQ-116.
`layout_box("abc", &[], ...)` is `Ok` with one line, no clusters, `overflow = false` and no `.notdef`. The doc comment says an uncovered cluster keeps a `.notdef`. No spec row covers this input and no test or progress note records it. Recommended: `Err(InvalidFont)` or a stated rule, then a test.

**W4. ARQ-75 register fields disagree across documents.** class: ambiguity. Ref: 05 "Registro" (line 495), P7.
05 asks each entry for path, hash, licence, authorship, origin, attribution required and modified. P7 sets other fields. Font entries have no hash, attribution or modified field, and the subsets are modified. The splash entry has no `license` field. It follows P7 and D-09, so TX1 is not at fault, but `xtask check-licenses` will need one field list.

**W5. Copyright year differs from two fonts.** class: implementation. Ref: `assets/LICENSES.toml`.
The register copies upstream's `OFL.txt`, which is correct as a licence text, but the font's own `name` ID 0 differs: Inter 2020 against 2016, Manrope 2018 against 2019. Noto Sans Arabic and Noto Emoji match. Record both lines in the entry.

**W6. "Fuente sin métricas" is neither built nor recorded.** class: implementation. Ref: 02 §4.17.3, FD-41.
A copy of Inter with zero metrics gives `Ok` with baseline 0, where the row asks for `Err` of invalid font. The row belongs with FD-41 (f) in TX2, so it does not count against TX1's IDs, but the progress notes omit it. Carry it to TX2 explicitly.

### SUGGESTION

- **S1.** XP3 says "Sizes ... hold", but against E4's table Inter is +5.5 %, Manrope +52 %, Arabic +21 % and Emoji -13 %. No test depends on it. Correct the design row.
- **S2.** An empty line has `advance = -0.0`, from `f32` `sum()` over nothing. Harmless, but visible in bit comparisons.
- **S3.** The doc of `Cluster::glyphs` says none "for a forced break". A ligature continuation is empty too (Manrope `fi`, second cluster).
- **S4.** TX4 needs `skrifa` as a workspace dependency. 02 §9 lists it, but P2 makes it a stop: plan a serial merge.

## Rulings the owner asked for

| Item | Ruling |
|---|---|
| FD-41 (a) "real outline" by a test-only `glyf` reader | **Accepted for TX1, no finding.** Layout picks font 2 and glyph 5 for `😀`; fonttools independently gives 7 contours, 124 points and a non-empty box for that glyph. The reader follows `loca`, and its self-test tells a letter from a space. It proves the outline exists for the chosen glyph, not that `gafira-text` can extract it; extraction is TX4's. `n != 0` also accepts a composite, which does not apply to this font |
| Final newline adds an empty line | **Ambiguity** (W2). No row states it. Parley's behaviour, and reasonable for an editor, but it changes line counts (FV-08) and height (FD-45, FD-46) |
| Empty text is one empty line | **Ambiguity** in the count (W2). That it is valid with no overflow is derivable from "Caja de tamaño 0" |
| Word wider than the box breaks by cluster | **As stated**: FD-41 Saltos. Probe: no line of two or more clusters exceeds the box, in 17 589 layouts |
| Hanging white space excluded from overflow | **Derivable** from FD-41 Saltos and FD-46 "desborde de clúster". A literal reading of ARQ-116 would flag every wrapped line. Two limits: the code does not do it in bidi (C1), and white space alone conflicts with the size-0 row (W1) |
| Negative width acts as 0 | **Derivable**: Rangos leaves it unspecified on purpose, and no test pins the value. The surviving hand mutant is correct |
| `letter_spacing`, `line_height` validated, unused | **As stated**: FD-41 (d) and ARQ-110 check inputs that do not take part in the result. (c, e, f) are TX2. Probe: same bytes with 500 and 4.0 |
| Vertical fit not in `overflow` | **Derivable** from ARQ-116's width comparison and FD-46 "Cabe". A taller text gives `false` today. The ARQ-116 wording "excede una caja `Fixed`" still has to hold in TX3, FD-45 (c) |
| `dpi`, width, height without a range | **As stated**: Rangos. The `dpi > 0` row names other functions |
| ARQ-111 with parley's real output | **Holds.** Same bytes twice in one process, in 8 threads, in 4 processes with different environments (`env -i`, other locale, no fontconfig). No system fonts: `default-features = false`. `FontContext` is fresh per call. No `rayon` in the `gafira-text` tree, so the 1, 2, 4 pools do not apply |
| Licence entries and OFL texts | The 4 OFL files are byte-identical to `google/fonts` at the pinned commit, with no Reserved Font Name. Commit and source paths are real. The 7 D-09 fields are present. Problems: C6, W5, W4 |
| Subsets against XP3 | **Hold.** The P7 recipe with fonttools 4.51.0 on the pinned files **reproduces all 4 files byte for byte**. Advances and bearings match upstream (0 differences). Arabic joining: 4 distinct forms. Manrope `liga` forms; Inter has none. parley loads all four. Code points absent from the subsets are absent upstream (Inter 2, Manrope 19). Sizes: S1 |

## Criteria

| ID | Status | Proof |
|---|---|---|
| FD-41 (a) | covered | `fd_41_a_*`; probes on 7 more bidi texts and 14 grapheme cases (flag, modifier, keycap, Hangul, CRLF) all correct |
| FD-41 (d) | covered | `fd_41_d_*` |
| ARQ-115 | covered | `arq_115_*`; walks the repository |
| ARQ-111 | covered | `arq_111_*`; probes above |
| ARQ-75 (partial file) | failing | `arq_75_*` pass; C6 |
| ARQ-116 | failing | The 4 literal criteria pass; C1, C2, C3 |
| ARQ-110 | failing | `arq_110_*` pass and probes found no panic or non-finite output; C4 |
| ARQ-102 | failing | `tests/message.rs` passes and pins the wrong form; C5. The catalogue does not exist yet |

Also checked: ARQ-107 and XP10 (no `Locale`, CI step 0). P7 family rule: two fonts of one family stay separate in the stack, and `FontFamilyName` has no id form.

## Hand mutants

Run on a scratch export. Spot-check plus 5 independent ones: 2 killed, 4 survived.

| # | Mutant | Result |
|---|---|---|
| S1 | Spot-check of item 3: `>=` against the width | killed by `arq_116_overflow_compares_with_greater_than` and 2 more, as recorded |
| I1 | `overflow` taken from the last line only | **survived** (C2) |
| I2 | Baseline left out of the output check | killed by `arq_110_a_sum_that_overflows_in_the_output_...` |
| I3 | Range checks before the non-finite checks | **survived** (C4) |
| I4 | `.notdef` read from the first glyph only | **survived** (C3) |
| I5 | Numbers checked after the fonts | **survived** (C4) |

## Result

FAIL. CRITICAL 6, WARNING 6, SUGGESTION 4. Per class: implementation 8, ambiguity 4.

next: specifier, then tdd-apply (classes: ambiguity W1 to W4, implementation C1 to C6, W5, W6)
