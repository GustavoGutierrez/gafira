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
