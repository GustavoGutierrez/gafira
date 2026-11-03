## Slice card
Flow: full | level: L3 | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: owner | reason: layer-2 crate in the owner's parallel full route; one writer per worktree
Flow-change: T4 skipped at close (trigger: owner order; decided-by: owner; G5 waived for this change).
Product IDs: TX1 FD-41 (a, d), ARQ-115, ARQ-116, ARQ-75 (partial file), ARQ-110, ARQ-111; TX2 FD-41 (c, e to n; (a) for bidi paragraphs); TX3 FD-45 (a, c, d, e), FD-46 (a, b, c, e, f), ARQ-14 (FitBox); TX4 FD-44 (a, c to g), FD-43 (a), ARQ-114 (text) (source: specs/02-arquitectura-software.md, specs/01-producto-y-calidad.md, specs/04-funcionalidad-diseno-imagen-exportacion.md, specs/05-funcionalidad-video-y-distribucion.md)
Scope: TX1 to TX4 of specs/changes/layer-2/design.md, with P7 and the XP3 and XP10 records; TX3 needs one workflow_dispatch during its verify (ARQ-14).      Non-goals: other streams; FD-41 (b), FD-47, FD-49, FD-42, FD-46 (d), FD-43 (b, c), FD-44 (b), FD-48; `StyleProperty::Locale` (ARQ-107).
Deliveries: TX1, TX2, TX3, TX4, serial; each verified and merged before the next.
Plan: specs/changes/layer-2/design.md, table and decisions; `crates/gafira-text/` plus the files P2 assigns.
Owner decisions touched: D-40 to D-44 as recorded; none reopened.

# Apply progress: gafira-text

## TX1

Branch `feat/gafira-text`, from `develop` 9124700. Commits: b7f291f (fonts, register, ARQ-115 test), ab43d34 (numbers, `TextError`, ranges), 8dce187 (`layout_box`: clusters, lines, overflow, `.notdef`).

Gates per commit, each run alone: `cargo fmt --check` 0, `cargo clippy --workspace --all-targets --locked -- -D warnings` 0, `cargo test --workspace --locked` 0.

Measured: 1 258 authored lines (2 deleted) excluding the font binaries and the OFL texts: 380 production (`layout.rs` 204, `error.rs` 54, `fonts.rs` 42, `LICENSES.toml` 69, `lib.rs` 11) and 878 tests. Over the ~700 mark; the forecast was 465 (650).

### Done

| ID | Proof |
|---|---|
| ARQ-115 | `crates/gafira-testkit/tests/licences.rs`: walks the repository for font files and fails if one has no entry in `assets/LICENSES.toml`; font entries carry OFL-1.1, upstream, commit, subset command, licence file |
| ARQ-75 (partial file) | `assets/LICENSES.toml` with the four fonts and `assets/splash_screen.jpg` (D-09 fields; similarity check stays VERIFICAR); `arq_75_*` tests |
| ARQ-116 | `crates/gafira-text/tests/layout.rs`: 3 forced lines, logical concatenation (LTR, RTL, emoji, ligature font), `e` + U+0301 is 1 cluster, overflow keeps the text, `.notdef` flag |
| FD-41 (a) | bidi `abc ابج` visual order, ZWJ `👩‍💻` is 1 cluster, `😀` from the emoji font with a real glyf outline (test-only reader of `glyf`), see Deviations |
| FD-41 (d) | `crates/gafira-text/tests/validation.rs`: bounds and the nearest values outside for size, letter spacing, line height |
| ARQ-110 | NaN, +inf, -inf in each of 6 numbers give `NonFinite` with the field; computed `size_px` not finite in f64 or f32; non-finite output; property test on finite input; unknown-range numbers never panic |
| ARQ-111 | `arq_111_the_same_input_gives_the_same_bytes_twice` (bit fingerprint, 3 texts, 3 widths) |
| ARQ-102 | `TextError` implements `ToMessage`; `tests/message.rs` |

### RED evidence

| Criterion | Failing test and assertion (stub) |
|---|---|
| ARQ-115, ARQ-75 | `arq_115_every_font_file_in_the_repository_has_a_licence_entry`: fonts without a licence entry `[Inter, Manrope, NotoEmoji, NotoSansArabic]`; `arq_115_font_entries_record_...`: no entry for `Inter-test.ttf`; `arq_75_splash_image_has_an_entry_with_the_seven_d09_fields`: no entry for the splash |
| ARQ-110 | `arq_110_nan_and_infinities_in_any_number_give_err_naming_the_field`: left `None`, right `Some(NonFinite { field: "style.size_pt" })`; `arq_110_a_computed_size_that_is_not_finite_gives_err`: left `None`, right `Some(NonFinite { field: "size_px" })`; `arq_110_bytes_that_are_not_a_font_...`: left `None`, right `Some(InvalidFont { index: 0 })` |
| FD-41 (d) | `fd_41_d_values_just_outside_their_bounds_give_out_of_range`: left `None`, right `Some(OutOfRange { field: "style.size_pt" })` |
| ARQ-102 | `arq_102_every_text_variant_gives_its_key_and_arguments`: key `error.text.todo`, expected `error.text.non_finite` |
| ARQ-116 | `arq_116_three_forced_lines_give_three_lines`: left 0, right 3; `arq_116_clusters_in_logical_order_concatenate_to_the_input`: left `("", false)`, right `("Hello world", true)`; `arq_116_a_trailing_newline_starts_one_more_empty_line`: left 0, right 1; overflow, `.notdef` and e-acute tests: assertion failed or no cluster to read |
| FD-41 (a) | `fd_41_a_bidi_text_is_ordered_visually_by_the_unicode_algorithm`: left `[]`, right `[a, b, c, " ", ج, ب, ا]`; the ZWJ and emoji tests: no cluster to read |

Not RED, passed or vacuous on the stub, proved by mutant: `fd_41_d_values_at_their_bounds_are_accepted`, `arq_110_numbers_without_a_range_row_never_panic`, `arq_111_*`, `arq_110_finite_input_in_the_domain_gives_finite_output`, `arq_110_a_sum_that_overflows_in_the_output_...`, `arq_115_the_check_names_a_font_that_has_no_entry`.

### Hand mutants

| # | Wrong behaviour | Killed by |
|---|---|---|
| 1 | `e` + U+0301 split into 2 clusters (split by char) | `arq_116_a_composed_e_acute_is_one_cluster`, `fd_41_a_a_zwj_sequence_is_one_cluster_and_the_emoji_font_covers_it` |
| 2 | Break cluster dropped; RTL clusters lose their range | `arq_116_clusters_in_logical_order_concatenate_to_the_input`, `arq_116_wrapped_text_keeps_every_cluster_in_logical_order`, `fd_41_a_bidi_*`, `fd_41_a_a_right_to_left_paragraph_*` |
| 3 | `>=` against the width; overflow drops a cluster | `arq_116_overflow_compares_with_greater_than`, `arq_116_trailing_space_that_hangs_past_the_box_is_not_overflow`, `arq_116_overflow_in_a_box_of_size_zero_...`; `arq_116_a_text_wider_than_a_fixed_box_flags_overflow_and_keeps_all_text` |
| 4 | Last line popped at a final newline; empty lines skipped | `arq_116_a_trailing_newline_starts_one_more_empty_line` |
| 5 | `.notdef` never reported | `arq_116_clusters_without_cover_are_flagged_notdef`, `fd_41_a_the_emoji_subset_gives_a_smile_a_real_non_empty_outline` |
| 6 | NaN size or width not an `Err`; upper bound exclusive; lower bound exclusive; f32 overflow of `size_px` unchecked | `arq_110_nan_and_infinities_in_any_number_give_err_naming_the_field`, `fd_41_d_values_at_their_bounds_are_accepted`, `arq_110_a_computed_size_that_is_not_finite_gives_err` |
| 7 | A counter added to the baseline | `arq_111_the_same_input_gives_the_same_bytes_twice` |
| 8 | A font file without entry; a checker that reports nothing | `arq_115_every_font_file_in_the_repository_has_a_licence_entry` and `arq_115_embedded_fonts_are_the_files_on_disk`; `arq_115_the_check_names_a_font_that_has_no_entry` |

Extra mutants killed: pt to px with 96, hanging white space counted, no break by cluster, unchecked output, font index always 0, wrong x origin, logical instead of visual order. One survivor, by design: a negative box width is clamped to 0, and no row of 02 4.17.3 fixes that value.

### Deviations and open points

1. **FD-41 (a), outline.** The spec asks for a real, non-empty outline of `😀`. Extracting outlines in production needs `skrifa`, which is in the lock through `parley_engine` but is not a workspace dependency, and P2 makes a new dependency a stop. TX1 proves the clause with the cluster's font index, glyph id (not 0) and a test-only reader of `loca`/`glyf` (contours != 0). `TX4` (curve, contours) will need `skrifa` as a workspace dependency, which is a root `Cargo.toml` and `Cargo.lock` change.
2. **Spec gaps, decided and recorded, no stop:** (a) a final newline gives one more, empty line, and an empty text is one empty line; (b) overflow is a line wider than the box once hanging white space is removed, and a word wider than the box breaks by cluster (`OverflowWrap::Anywhere`), so only one cluster wider than the box overflows; (c) vertical fit is not part of `overflow` (TX3); (d) `dpi`, width and height get no range check, per the Rangos rule; (e) a width below 0 acts as 0.
3. **`letter_spacing` and `line_height`** are validated (FD-41 d) but not applied yet: TX2 applies them (FD-41 c, e, f). The doc comment says so.
4. **Error fields** use dotted paths (`style.size_pt`, `bounds.width`) plus the computed `size_px` and `output`. Message keys `error.text.*` (ARQ-102) are new; the catalogue does not exist yet.
5. **API shape** (not fixed by the spec): `layout_box(text, fonts: &[&[u8]], &TextStyle, dpi, Size) -> Result<TextLayout, TextError>`; fonts are copied and registered on every call, so TX3/TX4 benches may need a cached stack. `shape` of FD-41 Técnica is not built; no ID of TX1 needs it.
6. **Font subsets** follow the P7 recipe and match XP3's bytes only for Arabic; Inter, Manrope and Emoji differ in bytes from the XP3 run (the design says no comparison by bytes). The source files had the same SHA-256 as XP3's downloads.
7. The Inter subset has no U+0301, so `e` + U+0301 is one cluster whose glyph comes from normalisation, not flagged `.notdef`; the test pins the cluster count only.

### TX1 fix round 1

Commits: 63e751a (register), 17eafac (text). Gates ran on the combined tree: `cargo fmt --check` 0, `cargo clippy --workspace --all-targets --locked -- -D warnings` 0, `cargo test --workspace --locked` 0. Each commit is independent of the other. Measured: 253 lines added, 10 deleted in the two commits, before this note.

| Finding | Fix | RED or proof |
|---|---|---|
| C1 | `content_advance` strips the white space at the logical end of each line; `hanging_advance` unused | RED `arq_116_overflow_in_mixed_direction_text_depends_only_on_the_widest_cluster` (left true, right false); mutant with `hanging_advance` back is killed |
| C2 | test only | `arq_116_overflow_looks_at_every_line`; mutant I1 killed (not RED) |
| C3 | test only (code already read every glyph) | `arq_116_a_cluster_with_a_later_glyph_without_cover_is_notdef`; mutant I4 killed (not RED) |
| C4 | test only | `arq_110_the_numbers_are_checked_in_order_...`; mutants I3 and I5 killed (not RED) |
| C5 | `index` as `Int` | RED `arq_102_every_text_variant_gives_its_key_and_arguments` |
| C6 | splash `provenance` says item 6 (b) is VERIFICAR | RED `arq_75_the_splash_licence_is_propia_...`; pinned: contains VERIFICAR, not "is kept" |
| W3 | `TextError::NoFont`, key `error.text.no_font`, after numbers | RED `fd_41_an_empty_font_list_is_an_error_also_for_an_empty_text` |
| W4 | FV-38 fields in every entry, splash `license = "Propia"` | RED `arq_75_every_entry_has_the_fv_38_fields_and_a_blake3_hash`; `arq_75_the_check_fails_when_a_field_is_removed_or_empty` is not RED, proved by removing a field |
| W5 | `autoría` is name ID 0 of the subset (read with fonttools): Inter 2016, Manrope 2019; OFL.txt line kept as `copyright_ofl_txt` | `arq_75_test_fonts_are_modified_and_record_the_year_...` |
| W6 | not built; carried to TX2 | see below |

Deviations and open points:
1. **BLAKE3 is not recomputed by any test.** `blake3` is not a direct dev-dependency of the workspace (it is in the registry only through other crates), so per the instruction I stopped short of adding it. The hashes were computed with `blake3` 1.8.7 in a scratch project outside the repository; the test checks only 64 lowercase hex digits. A test that hashes the file needs `blake3` as a workspace dev-dependency of `gafira-testkit`: a root `Cargo.toml` and `Cargo.lock` change for the coordinator.
2. Splash `modificado = "false"` and `exige atribución = "false"` are my reading of D-09 (no edit recorded; OpenAI terms need no attribution; the AI notice is in `ai_notice`). The owner should confirm. Field keys with accents are quoted TOML keys, as the coordinator wrote them.
3. **W6 for TX2:** "Fuente sin métricas" (natural height <= 0 gives `Err` of invalid font) is not built. It goes with FD-41 (f), which needs the natural height. Needs a font with zero metrics as fixture.
4. W1 (white space alone in a box of size 0) is unchanged: a box of 0 with `" "` gives no overflow. It waits for `specifier`.
5. S2 (`-0.0` advance of an empty line), S3 (doc of `Cluster::glyphs` for ligature continuations) not touched.

## TX2

Branch `feat/gafira-text`, from `develop` 0600d68. Commits: f165a85 (W12, W10), fec397f (spacing, line height, W6), dae3fcd (fallback, XP12), 755b909 (upper case), 4903a1d (align, justify), 55da274 (decorations), f120b7e (lists), 220bc04 (ARQ-111, ARQ-110 over the styles).

Gates per commit, each run alone: `cargo fmt --check` 0, `cargo clippy --workspace --all-targets --locked -- -D warnings` 0, `cargo test --workspace --locked` 0. `Locale` has 0 matches in `crates/gafira-text`. No dependency, `Cargo.toml` or `Cargo.lock` change.

Measured: 1 417 lines added, 28 deleted (`git diff --numstat 0600d68 HEAD`): 281 in `src/`, 1 136 in `tests/`. Over the ~700 mark; the forecast was 440 (615).

### Done

| ID | Proof |
|---|---|
| FD-41 (c) | `style.rs`: spacing 0 (and -0.0) with line height 1.0 equal a raw parley layout bit for bit, 5 texts incl. bidi, ligatures, wrap |
| FD-41 (e) | `style.rs`: line advance grows by `(N-1) t/1000 size` +-0.01 for t in -100..500, 4 texts; spacing goes after each cluster, never after the last; optional ligatures off for any t but 0 (also 1e-6), required Arabic forms kept |
| FD-41 (f) | `style.rs`: baseline distance is `lh x natural` for lh 0.5..4, 2.0 is twice 1.0, natural height from the fonts' tables: largest of the line, `OS/2` with `USE_TYPO_METRICS` else `hhea`, the line gap counts |
| FD-41 (g), D-19, XP12 | `fallback.rs`: each cluster takes the first font that covers all its points; else the font with most; a tie goes to the first. Fixtures: space + U+0651 (Arabic whole), U+0131 + U+0651 (tie), + U+064B (Arabic 2 of 3), both list orders. No `skrifa` |
| FD-41 (h) | `case.rs`: `straße` gives `STRASSE`, `i` gives `I`, `ﬁ` gives `FI`, final sigma, no language |
| FD-41 (i) | `align.rs`: every justified line but the last ends at the edge +-0.01 and starts at 0; last line, forced-break line and one-word line unchanged; no overflow from stretching (3 601 widths) |
| FD-41 (j) | `list.rs`: indent 1.5 em per level +-0.01, 1, 2, 3 at one level, restart on a new descent, bullets cycle, 4 levels, level 0 or 5 is `OutOfRange` |
| W6 | `style.rs`: a font whose natural height is 0 gives `InvalidFont` with its position |
| W10 | `layout.rs`: CRLF/LF line counts, box of 0 with `" "`, `"\t"`, `"\r\n"`, NBSP, U+2003, U+3000 and `"a"`, `" a"` |
| W12 | `content_advance` linear: 100 000 trailing spaces 29.08 s before, 0.16 s after (debug) |
| Underline, strikethrough | `decoration.rs`: `post` and `OS/2`, thickness 0 is 0.05 em, per font |
| ARQ-111, ARQ-110 | `styled.rs`: same Debug bytes twice for 16 style mixes and a list; finite output at the extremes |

### RED evidence

| Criterion | Failing test and assertion |
|---|---|
| FD-41 (e) | `fd_41_e_a_line_grows_by_n_minus_1_times_the_spacing`: `"Hello" t=-100: 0 vs -12.8`; `..._spacing_goes_after_each_cluster_...`: cluster 1 moves 0; `..._optional_ligatures_...`: `t=-1: [[237], []]` and, after parley's own switch, `t=0.000001` still formed |
| FD-41 (f) | `..._distance_between_baselines_is_line_height_times_the_natural_height`: `lh=0.5: [31.0, 69.71875, 108.4375]`; `fd_41_f_2_is_twice_1_...`: assertion failed |
| W6 | `fd_41_a_font_without_metrics_...`: left `None`, right `Some(InvalidFont { index: 0 })` |
| FD-41 (h) | left `"straße"`, right `"STRASSE"` (3 tests) |
| FD-41 (i) | `..._every_line_but_the_last_ends_at_the_edge_of_the_box`: `ends at 199.859375`; the forced-break test and `fd_41_left_center_and_right_place_one_line_in_the_box` failed |
| Decorations | 5 tests of `decoration.rs`: `underline` expected on the first cluster |
| FD-41 (j) | by stub: the list tests were written with the code in one step, so RED is shown on a `layout_list` returning no entries (6 of 9 failed, index out of bounds or `left != right`); see Deviations |

Not RED, passing on the code that existed, proved by mutant or kept as negative rules: FD-41 (c) identity and the OS/2, line gap and largest-font tests of (f) (parley already does it); all of `fallback.rs` (XP12); the one-word and no-overflow tests of (i); W10; W12 (no clock under ARQ-02, timed by hand); `styled.rs`.

### Hand mutants

| # | Wrong behaviour | Killed by |
|---|---|---|
| 1 | spacing after the last cluster; `/100`; one cluster too many | `fd_41_e_a_line_grows_by_n_minus_1_times_the_spacing`, `fd_41_e_spacing_goes_after_each_cluster_...` |
| 2 | optional ligatures left on (also for a tiny t); required forms turned off | `fd_41_e_optional_ligatures_...`; `fd_41_e_required_ligatures_of_a_script_stay_with_spacing`, `fd_41_e_a_line_grows_...` |
| 3 | features pushed at t = 0; `FontSizeRelative` | `fd_41_c_*` (identity), `fd_41_e_optional_ligatures_...` |
| 4 | `FontSizeRelative`; `Absolute(1.2 em)` instead of the natural height | `fd_41_f_*` (6 tests). "First run only", "`USE_TYPO_METRICS` ignored" and "descent sign" live in parley and cannot be applied to this crate: the tests compute the expected height from the `OS/2` and `hhea` tables (also from patched copies) and parley meets them |
| 5 | 2.0 off by 1 % | `fd_41_f_2_is_twice_1_for_the_same_text`, `..._times_the_natural_height` |
| 6 | last line stretched (forced break too); justify as Left; edge off by 0.05 px | `fd_41_i_a_line_with_a_forced_break_is_not_stretched`, `..._every_line_but_the_last_...`; all 4 for the edge. Survivor: overflow read after the alignment (no width in 3 601 changes it) |
| 7 | font list reversed (last font on a tie); only the first font | 4 and 3 tests of `fallback.rs`. "First font covering any point" lives in parley: the space + U+0651 case expects font 1 while Inter covers one of the two points |
| 8 | ASCII-only upper case; first char of each expansion; a Turkish dotted I | `case.rs` (3, 3 and 1 tests) |
| 9 | bullets not cycling (2 forms); 5th level; indent 1.4 em; indent from level 1; no restart; count by 2; bullet keeps the count; items not stacked | each killed by one or more of the 9 tests of `list.rs` |
| 10 | thickness not replaced at 0; 0.1 em; `post` and `OS/2` swapped (both ways); `>= 0.0` | `fd_41_a_thickness_of_zero_is_five_hundredths_of_an_em`, `fd_41_underline_comes_from_post_and_strikethrough_from_os2`, `fd_41_underline_and_strikethrough_do_not_read_each_others_table` |
| 11 | no break by cluster (a word is not split) | `arq_116_a_word_wider_than_the_box_breaks_by_cluster_and_does_not_overflow`, `arq_116_overflow_in_mixed_direction_text_...` |
| other | whitespace limited to U+0020 and newline; counter in a decoration (ARQ-111) | `arq_116_in_a_box_of_size_zero_...`; both `arq_111_*` of `styled.rs` |

### Deviations and open points

1. **API (not fixed by the spec).** `TextStyle` gains `uppercase`, `align`, `underline`, `strikethrough` and a `Default`; `TextLayout` gains `text` (the laid out text, so ranges index it when upper case changes the length) and `height`; `Cluster` gains `underline` and `strikethrough` (`Decoration { offset, thickness }` in px, offset as in the font tables); new `layout_list`, `ListItem`, `ListKind`, `ListEntry`, `TextAlign`. Existing tests build styles from `TextStyle::default()`.
2. **Lists, decisions for `specifier`.** The level is 1 to 4, the indent is `(level - 1) x 1.5 em` (first level at the edge; "1,5 em por nivel" does not say which), the marker is laid out with one space before the text, and the item gets the box minus its indent. A bullet resets the count of its level. Numbered markers have no number format other than `n.`.
3. **List RED is by stub.** I wrote `layout_list` and its tests together; the RED run was done afterwards on a stub. The mutants above show the tests bite.
4. **Letter spacing and a hard break.** A forced break is not a cluster for "N clusters" (parley gives it no spacing); the last visible cluster in logical order loses the spacing, which gives the same total for right-to-left text.
5. **Optional ligatures.** Parley turns them off itself with spacing but ignores a tiny one, so `"liga" 0, "clig" 0, "dlig" 0, "hlig" 0` is pushed for every t other than 0.
6. **"Fuente sin métricas".** Only fonts that shape a cluster are checked, with parley's `ascent + descent + leading`; a text with no cluster never reads a font. An empty text with a font of zero metrics is `Ok`.
7. **Right alignment and trailing white space.** Parley counts trailing spaces in the line width for right alignment (`"ab  "` ends 9 px short of the edge). Nothing in FD-41 says otherwise, so no test and no change.
8. **Not built, outside the TX2 IDs:** bold and italic selection, vertical anchor, kerning and ligature switches, a font with no `post` table (thickness absent is covered only by 0), `shape`. Hinting (ARQ-42) has no input in `layout_box`: the layout is not quantised (`ranged_builder(.., false)`).
9. W1, W2, W3 of TX1 were amended in the specs and are tested; W11 (zero-advance clusters in a box of 0) is unchanged: they give no overflow.

### TX2 fix round 1

Commits: c986a36 (C9, C10), b007072 (C7, C8, C11, C12, hang), 689a84c (W18 ranges), 9dbc3e8 (lists: C13, W14, W15, amended rules), 282db5c (W16, W17, mixed line height). Gates per commit, each run alone: `cargo fmt --check` 0, `cargo clippy --workspace --all-targets --locked -- -D warnings` 0, `cargo test --workspace --locked` 0. Every RED below was run before the code it needed, with real assertions and no stub. Measured since the merge 16204dd: 845 lines added, 137 deleted (242 added in `src/`).

| Finding | Fix | RED |
|---|---|---|
| C7 | parley adds the spacing after every cluster; the fit uses a box wider by that spacing, so a line fits when its output advance does | `fd_41_e_a_word_fits_a_box_as_wide_as_its_line_with_any_spacing`: 2 lines at t=500; `..._breaks_by_cluster_with_any_spacing`: `t=-50: 1 lines` |
| C8, (i), (k) | Center and Right are mine, on the ink of the line (the hanging white space is outside it); Justify stays with parley on the same wider box | `fd_41_i_a_justified_line_ends_at_the_edge_with_any_spacing` (t -100..500, widths 40..200); `fd_41_right_and_centre_use_the_spacing_the_output_keeps`: `t=-100: right ends at 201.6`; `fd_41_k_*`: `b ends at 91` |
| C9 | `check_output` reads `height` and the decorations; `layout_list` checks indent, shifted `x` and `top` | `arq_110_an_infinite_height_with_finite_lines_is_an_error` (5.2e36 dpi), `..._a_list_indent_that_overflows_...`, `..._a_list_top_that_overflows_...`: left `None`, right `Some(NonFinite { field: "output" })` |
| C10 | a font with no `post` (or `OS/2`) table gives 0.05 em, read from the table directory | `fd_41_a_font_without_post_or_os2_...`: `underline 5.5555553` |
| C11, C12 | tests only | not RED (the code already did it): `fd_41_e_the_last_cluster_in_logical_order_...` (M3), `fd_41_e_a_forced_break_is_not_the_last_cluster_...` (M2) |
| C13 | test only | not RED: `fd_41_an_empty_font_list_is_an_error_for_a_list_with_no_items_too` (M6) |
| W14 | list checks numbers, fonts, levels, as documented | `arq_110_a_list_checks_its_numbers_then_its_fonts_then_its_levels`: left `OutOfRange`, right `NoFont` |
| W15 | height pinned to `n x lh x natural` and the stacked tops | `fd_41_f_the_height_of_n_lines_is_...`, `fd_41_j_items_stack_by_the_height_...` (not RED); M5 killed by both |
| W16, W17 | note corrected below; `""` with a zero-metrics font tested | not RED: `fd_41_a_zero_metrics_font_is_checked_with_an_empty_text_only_when_it_is_the_first` |
| W18, ARQ-116 | ranges index the input after upper case through a map; `straße` gives `4..6, 4..6, 6..7` | `arq_116_ranges_after_upper_case_...`: left `[..., 4..5, 5..6, 6..7]`, right `[..., 4..6, 4..6, 6..7]` |
| Lists, amended | the marker is not part of the laid out text; the item text hangs under the widest marker of its level in the list plus a space (measured as the `x` of a letter after "marker space"); a bullet neither adds nor restarts; a shallower item restarts the deeper levels; `TextLayout::text` is removed | `fd_41_j_nine_and_ten_start_...`: x is 0; `..._the_text_hangs_under_the_widest_marker_...`: `0 vs 19.375`; `..._a_bullet_neither_adds_...`: left `["1.", "2.", "•", "1."]`, right `[..., "3."]`; `..._the_ranges_of_an_entry_index_its_own_text`: byte index 5 is not a char boundary |
| (f) mixed | test only | not RED: `fd_41_f_inter_and_noto_emoji_on_each_line_give_61_74_121_29_and_241_99_at_100_px` |
| S6 | the sweep over spacings and widths now pins overflow before the alignment | a mutant that reads it after justify is killed |

Hand mutants reapplied or added, all restored:

| # | Wrong behaviour | Killed by |
|---|---|---|
| M2 | a forced break is the last cluster | `fd_41_e_a_forced_break_is_not_the_last_cluster_...`, `fd_41_e_a_hanging_space_...`, `fd_41_e_a_line_grows_...`, two of `align.rs` |
| M3 | the last cluster in visual order | `fd_41_e_the_last_cluster_in_logical_order_is_the_one_without_spacing` |
| M4 | list levels checked before fonts | `arq_110_a_list_checks_its_numbers_then_its_fonts_then_its_levels` |
| M5 | `height` doubled | `fd_41_f_the_height_of_n_lines_is_...`, `fd_41_j_items_stack_by_the_height_...` |
| M6 | no `NoFont` check in a list | `fd_41_an_empty_font_list_is_an_error_for_a_list_with_no_items_too`, the order test |
| more | hanging space carries spacing; fit without or with the wrong sign; right ignores the hang; centre as right; a wider line is moved; overflow after justify; ranges unmapped or from the first character only; bullet counts or restarts; no restart of deeper levels; last marker instead of widest (new test); each item under its own marker; no hang; 1.4 em; 5th level; bullets not cycling; height left out of the finite check; metrics check removed; `FontSizeRelative` | each by at least one test |

Deviations and open points:
1. **W16 correction.** My earlier note said a zero-metrics font with an empty text is `Ok`. It is `Err(InvalidFont { index: 0 })` when that font is the first of the list; it is `Ok` behind a healthy font. Now tested.
2. **Left alignment moves nothing.** For a right-to-left paragraph the hanging space of a line sits on its left; with Left it stays where parley puts it. Only Center and Right use the ink of the line. *Corrected in fix round 3: wrong (C15). With Left the paragraph-level space hangs on the left of a right-to-left paragraph, at negative x.*
3. *(Superseded in fix round 3: Justify is now placed by this crate, see below.)* **Justify** is parley's. It reaches the edge within 0.01 px for every spacing and width swept (40 to 200 px, five spacings). A line of one word is not stretched, and the test skips it.
4. **Hanging space in a right-to-left run.** *(Corrected in fix round 3; the earlier text claimed that a right-to-left line that ends in a space inside an opposite-level run may break one cluster early. The verify of round 2 found no early break: parley's line breaker does not look at the bidi level, only its hang does. The real effect of that space is in its position, not in the break; it is the limit accepted in FD-41 and pinned by (m).)* Only white space with the bidi level of the paragraph hangs at the edge of the line. The space at the end of a run of the other level stays where bidi puts it, inside the line; break, letter spacing, overflow and alignment of Center and Right treat it as hanging space, and Left keeps it in place.
5. **Decoration of a missing table.** Only the thickness is 0.05 em; the offset of a missing table is parley's, which no rule fixes.
6. **List marker measure.** The hang is the `x` of a letter after "marker space", measured with the style of the list, so letter spacing and kerning count; markers `◦` and `▪` are not in the Inter subset (S5), they only change the width of that level.
7. **Not raised:** C14 resolved by the amended FD-41 (f), whose numbers parley already gives (mixed-line test). W19 to W25 and S7 need no change in code; `fallback.rs` still takes coverage from comments (S7).

### TX2 fix round 3

Commits: d284e4e (C7), 413e9d9 (C8, C15, one-word line, FD-41 m), f15b184 (C16). Gates per commit, each run alone, exit codes read separately: `cargo fmt --check` 0, `cargo clippy --workspace --all-targets --locked -- -D warnings` 0, `cargo test --workspace --locked` 0 (all three commits). Every RED below was run on the existing code, with real assertions and no stub, before the production change. Measured: 411 lines added, 42 deleted (`git diff --shortstat d284e4e^ HEAD`): 112 added in `src/` (layout.rs 102, list.rs 10), 299 in `tests/`. This note adds more.

| Finding | Fix | RED |
|---|---|---|
| C7 | the fit box gets a relative slack of 4e-6 (0.0004 px at 100 px); the test uses the exact advance, no 0.001 | `fd_41_e_a_box_as_wide_as_its_line_with_any_spacing_does_not_split_it` (4 texts x 9 sizes x 7 spacings): `"Hello" 8 pt t=500: box 47.04166793823242`, left 2 lines, right 1. `fd_41_e_a_box_narrower_than_its_word_breaks_it_by_cluster` (0.02 px short splits) is not RED: it pins the slack as small |
| C8 | Center, Right and Justify are placed by this crate (no parley `align`); Justify shares the room among the spaces that are not trailing white space; the start of a line is computed without the spacing the output removed | `fd_41_i_a_right_to_left_justified_line_ends_at_the_edge_with_any_spacing`: `t=-100 w=40: starts at 1.5999999046325684` |
| C15 | the trailing white space with the level of the paragraph hangs at negative x on the left of a right-to-left paragraph, with Left too | `fd_41_hanging_space_of_a_right_to_left_paragraph_is_outside_the_ink_with_left`: `t=0 "  ": ink starts at 8.32`; `fd_41_a_wrapped_right_to_left_paragraph_with_left_keeps_its_ink_in_the_box`: `t=0 w=30: starts at 4.16` |
| One-word line, opposite level | the trailing space of a run of another level is not a place to stretch; a line without one is not stretched | `fd_41_i_a_one_word_line_behind_an_opposite_level_space_is_not_stretched`: the justified line has advance 40.000156, the left aligned one 22.996 (the space went from 4.50 to 21.50) |
| C16 | the probe is left aligned, in a box of `f32::MAX` px | `fd_41_j_the_alignment_of_the_list_does_not_change_where_the_text_starts_or_its_room`: `Center: the room of the item` left 36 lines, right 2; `fd_41_j_the_hang_is_in_proportion_to_the_dpi_at_any_dpi`: `dpi 10000000000: hang 0, expected 3253580729.17` |
| C17 | closed by acceptance, not implemented (no UAX #9 L1) | `fd_41_m_*` (3 tests) pin the limit with its figures: 12 pt box 40, the space at x 0 and the content at 4.50; 9 pt box 39.3, content ends at 41.56, 2.26 outside, no overflow; `abc ابج def` ends at 38.19 with the space behind. Not RED: they pass on the code of round 2 and on the new one, because they describe behaviour that stays |
| W26, (l) | **open item, not built** | `layout_box` has no hinting input, so (l) cannot be tested without inventing one. Hand mutant M1 of the verify (advances quantised) is killed by 8 tests that do not name (l). No `fd_41_l_` test exists |
| W27, (k) | tests named | `fd_41_k_*` (two tests in `align.rs`) already carry the letter; nothing was renamed. The card amend is the coordinator's |

Another test pins the discriminator by level: `fd_41_a_space_of_the_other_level_in_a_right_to_left_paragraph_does_not_hang_with_left` (Latin word and its space at the end of a line of an Arabic paragraph stay at x >= 0 with Left; the sweep checks that some line does end in such a space). It is not RED: the code never hung it, it kills the mutant "hang any level".

Hand mutants (all restored; the tree equals the committed one):

| # | Wrong behaviour | Killed by |
|---|---|---|
| 1 | fit slack removed (`FIT_TOLERANCE` 0, the RED); sign reversed (box shrunk); slack 1e-3 | `fd_41_e_a_box_as_wide_as_its_line_...` (first two), `fd_41_e_a_box_narrower_than_its_word_...` (third) |
| 2 | spacing count of Justify off by one (+1) | `fd_41_i_every_line_but_the_last_...`, `..._with_any_spacing` (both directions), `fd_41_i_a_line_with_a_forced_break_...` |
| 3 | the same off by one only on right-to-left lines | `fd_41_i_a_right_to_left_justified_line_ends_at_the_edge_with_any_spacing` |
| 4 | the hang applied to the logical end of the line whatever the direction of the paragraph (negative shift in left-to-right text too) | 5 tests: `fd_41_i_*` x4, `fd_41_m_a_trailing_space_of_the_paragraph_level_stays_behind_the_content` |
| 5 | white space of any level counts as hanging | `fd_41_a_space_of_the_other_level_in_a_right_to_left_paragraph_does_not_hang_with_left` |
| 6 | no hang in a right-to-left paragraph | `fd_41_hanging_space_...`, `fd_41_a_wrapped_right_to_left_...`, the right-to-left justify test |
| 7 | one-word line justified (the trailing space is a place to stretch) | 6 tests incl. `fd_41_i_a_one_word_line_behind_an_opposite_level_space_is_not_stretched` |
| 8 | list measures with the alignment of the list | `fd_41_j_the_alignment_of_the_list_does_not_change_...` |
| 9 | list hang from a fixed box (1e9) | `fd_41_j_the_hang_is_in_proportion_to_the_dpi_at_any_dpi` |

Deviations and open points:
1. **Alignment is now this crate's.** `layout_box` no longer calls parley's `align`. The only parley input is the line break at the box plus the spacing. Justify matches parley's old rule (spaces and NBSP are the places; the last line, a line with a forced break and a line with no place are not stretched; such a line starts at the left, or at the right in a right-to-left paragraph, as parley did). It differs on purpose in one thing: the trailing space of a run of the other bidi level is not a place to stretch (the accepted limit treats it as hanging).
2. **Slack.** 4e-6 relative is mine; the spec's tolerance is 0.01 px and the measured gap was 7e-7. The overflow test has no slack: a cluster wider than the box by under 4e-6 relative can sit alone on a line that parley accepted, and then `overflow` is true. No test needs it. *Corrected in fix round 4 (W28): wrong. Break and overflow now share one limit (`width x (1 + 4e-6)`); a line inside it neither breaks nor overflows, and the old statement that only a lone cluster was affected was false: it was any line.*
3. **List probe.** `f32::MAX` is a fixed box too, but a text that does not fit it gives `NonFinite`. A hang that cannot be measured is an error, not 0. *Corrected in fix round 4 (W28): wrong in one window: a text that does not fit `f32::MAX` can give `Ok` (C16, accepted limit of FD-41 (j): the hang is in proportion to the DPI within 0.01 % up to 1.3e39 DPI, and the test `fd_41_j_the_hang_of_1_dot_at_12_pt_is_in_proportion_to_the_dpi_up_to_1_3e39` pins that, not "any dpi").*
4. **Open for the coordinator:** (l) has no test (no input to vary); the Slice card and the 02 stream row still need the amend for (k) and (l) (W27).
5. **Center and Right with a trailing space of the other level** keep the old rule: the ink without it is aligned and the space stays beside it. FD-41's accepted limit says such a line is not centred or right aligned; I did not widen or narrow that.

### TX2 fix round 4

Placement is one function, `align_line`: for every alignment and direction it applies the hang, then computes the room, then the offsets, in f64. A stretched line is summed in f64 from the line origin; other lines keep the layout position plus the offset (FD-41 (c) identity bit for bit). The fit has one limit (`width x (1 + 4e-6)`) for the break and the overflow. Leading white space (logical start) is not a place to stretch.

| Finding | RED (failing test and assertion, run on the code of round 3) |
|---|---|
| C18 | `fd_41_c_a_box_within_the_fit_slack_neither_breaks_nor_overflows`: `Hello k=0.0000005: kept whole yet overflow` |
| C19 | `fd_41_hang_applies_when_there_is_no_room_in_a_right_to_left_line`: `Center: content starts at 9` |
| C20 | `fd_41_i_a_justified_line_ends_at_the_edge_in_a_very_wide_box`: `ends at 15999.90625` (box 16 000, 2 600 words) |
| C21 | `fd_41_i_leading_white_space_takes_no_extra_spacing`: left 5.6140623, right 4.5 (advance of a leading space) |
| W26 (l) | `fd_41_l_advances_are_the_linear_scaling_of_hmtx`: not RED, 2 000 single-glyph clusters at 400 fractional sizes match `hmtx x size / unitsPerEm` within 1e-6; it proves (l) by construction (no hinting input). |
| FD-41 (j) DPI clause | renamed `fd_41_j_the_hang_of_1_dot_at_12_pt_is_in_proportion_to_the_dpi_up_to_1_3e39`, now at 12 pt and up to 1.3e39 DPI: not RED (passes on the existing code). |
| W28 | the three statements corrected in place above and in the comment of `FIT_TOLERANCE` |

Hand mutants (all restored): slack on the break only, killed by `fd_41_c_a_box_within_the_fit_slack_...`; slack on the overflow only (no slack in the fit), killed by `fd_41_e_a_box_as_wide_as_its_line_with_any_spacing_does_not_split_it`; hang skipped when room <= 0, killed by `fd_41_hang_applies_when_there_is_no_room_...`; incremental f32 sum, killed by `fd_41_i_a_justified_line_ends_at_the_edge_in_a_very_wide_box`; leading spaces stretched, killed by `fd_41_i_leading_white_space_takes_no_extra_spacing`.

Adversarial sweep (throwaway, not committed): 5 120 layouts (8 texts incl. LTR, RTL, mixed, one word and explicit breaks, 5 leading/trailing space combinations, spacing 0, -100, 100, 500, 8 boxes: exact advance, +-1 ulp, +-0.0002, 90, 4 000, 16 000, 4 alignments). Checks: ink inside the box unless a cluster is wider, justified lines end at the edge (+-0.01), overflow only with a cluster wider than the box, limit (m) excepted. Result: 0 violations after the fix. It first flagged 26 justified RTL lines with a leading space; they were the leading space at the right edge, not a defect of the placement (see open point).

Open point: in a right-to-left paragraph a leading space sits at the right edge, so the ink of a justified line ends one space before it (the line, spaces included, ends at the edge). FD-41 says nothing about leading white space, so which edge the ink or the space takes is not decided here; returned to the coordinator.

### TX2 fix round 5

Verify round 4 findings C18 (wide boxes) and C22, with the fix proven on scratch: `overflow` reads the widest content cluster (`widest_content(..) > limit`), Justify stretches when `room > -width * 1e-4`, `Line.advance` of an unstretched line is summed in f64, and the `FIT_TOLERANCE` comment no longer states a limit.

| Finding | RED (run on the code of round 4) |
|---|---|
| C18, overflow | `fd_41_i_a_wide_box_reports_overflow_only_for_a_cluster_wider_than_it`: `w=3073.86: overflow with widest cluster 14.4156` (Left, 16 pt, 72 DPI, t=25; 7 boxes found by a sweep) |
| C18, edge | `fd_41_i_a_wide_justified_line_ends_at_the_edge_when_parley_keeps_it_over_the_box`: `w=3073.86 t=25: ends at 3073.8701171875` |
| C22 | `fd_41_e_the_advance_of_a_wide_unstretched_line_is_the_sum_of_its_clusters`: `pt=9 t=-100: advance 15986.401 vs 15986.892516374588` |

Hand mutants (all restored): overflow from the running sum of the line, killed by `fd_41_i_a_wide_box_reports_overflow_only_...`; `Line.advance` summed in f32, killed by `fd_41_e_the_advance_of_a_wide_unstretched_line_...`.

Throwaway sweep (not committed, release): 4 320 layouts, 3 texts (Latin, mixed bidi, Arabic), 4 size/DPI pairs, t -100 to 500, Left/Right/Justify, boxes 3 008, 4 012, 16 000, 3 073.86, 3 705.08, 3 753.55, 5 000, 9 000 each +-0.0002: 77 103 lines, 0 false overflow, 0 advance error over 0.01, 0 justified lines off the edge over 0.01, worst 0.00046 px.

### TX2 bidi paragraphs (FD-41, spec commit ff1d0e1)

Parley 0.12 has no paragraph split: `analyze_text` runs one bidi resolution over the whole text, and U+2028 and the like only break lines. Its `BaseDirection::Auto` already applies P2 and P3 (skips isolates, LTR with no strong character), so nothing of that is duplicated: `bidi_paragraphs` splits the text, each paragraph gets one parley layout with its direction from `Layout::is_rtl`, and `layout_box` concatenates lines, offsetting baselines (f64 sum of `line_height`) and ranges. The break stays in the paragraph it closes, so its cluster and line metrics do not change; the empty line parley adds after a final break is dropped, because the next paragraph is that line, and the text after the last break is always a paragraph, also when empty. Its height is the one of an empty text, as parley gave. `layout_list` needs no change: it calls `layout_box` per item.

Commit: e520633 (573 lines added, 60 deleted; `layout.rs` and tests). Gates on it, each run alone: `cargo fmt --check` 0, `cargo clippy --workspace --all-targets --locked -- -D warnings` 0, `cargo test --workspace --locked` 0.

| Criterion | RED (run on the code before the commit; the test or its row came first) |
|---|---|
| (n) paragraphs, order and x, `\n`, `\r`, `\r\n`, U+2028, U+2029 | `fd_41_n_each_forced_break_opens_a_paragraph_with_its_own_direction`: left `[d, e, f, " ", ج, ب, ا, " ", a, b, c]`, right `[a, b, c, " ", ج, ب, ا, " ", d, e, f]` |
| (n) Justify | `fd_41_n_justify_starts_a_line_where_its_paragraph_starts`: assertion `a` at x = 0 failed |
| (n) Left, hang | `fd_41_n_white_space_hangs_where_its_own_paragraph_ends`: first space not at the end of `b` |
| (m), last sentence | `fd_41_m_a_trailing_space_of_the_other_level_starts_the_line_at_the_edge`: `the space is at -4.5`; `fd_41_m_content_of_the_other_level_can_end_past_the_box_without_overflow`: `the content ends at 38.188`; `fd_41_m_a_trailing_space_of_the_paragraph_level_stays_behind_the_content`: left `c`, right `" "` |
| Dirección del párrafo | `fd_41_a_paragraph_takes_its_direction_from_its_first_strong_character`: `"abc": starts at 72.08`; `fd_41_an_open_isolate_ends_with_its_paragraph`; `fd_41_an_empty_paragraph_is_left_to_right_and_does_not_inherit`: `"\n": the empty paragraph` |
| Justificado inside each paragraph | `fd_41_i_justify_stretches_inside_each_paragraph_and_not_its_last_line`: `0 and -4.5` |
| List callers | `fd_41_a_list_item_has_bidi_paragraphs_too`: left `[d, e, f, ...]`, right `[a, b, c, ...]` |

Not RED, they hold on the old code too and are proved by the mutants below: `fd_41_n_u_0085_keeps_one_right_to_left_paragraph` (its first RED was a fault of the test, which found the wrong alef; rewritten before GREEN), `fd_41_controls_that_are_not_forced_breaks_do_not_open_a_paragraph`, `fd_41_a_crlf_is_one_break_and_lone_cr_and_lf_are_one_each`, `fd_41_a_final_break_gives_one_more_empty_line_of_the_same_height`, `fd_41_line_height_goes_on_across_paragraphs`, `fd_41_overflow_looks_at_every_paragraph`, `arq_116_ranges_index_the_whole_text_across_paragraphs`, `arq_116_upper_case_keeps_the_ranges_across_paragraphs`.

(n) holds as written: `abc ابج def` fits 100 px on one line at 12 pt; `ابج<U+0085>abc ابج def` wraps there and stays one right-to-left paragraph, so its test checks that its last line ends at the right edge, and the order on one line in a box of 1 000 px.

| Hand mutant (all reverted) | Killed by |
|---|---|
| Direction from the first paragraph | `fd_41_n_each_forced_break_...`, `fd_41_m_*`, 7 more |
| Direction inherited from the previous paragraph (no strong character) | `fd_41_a_paragraph_takes_its_direction_...`, `fd_41_n_justify_...`, `fd_41_an_empty_paragraph_...` |
| Empty paragraph not LTR | `fd_41_an_empty_paragraph_is_left_to_right_and_does_not_inherit` |
| CRLF as two breaks | `fd_41_a_crlf_is_one_break_...`, `fd_41_n_each_forced_break_...`, `arq_116_a_final_crlf_or_lf_opens_one_more_empty_line` |
| U+2028 does not open a paragraph | `fd_41_n_each_forced_break_...` |
| U+0085 opens a paragraph | `fd_41_n_u_0085_...`, `fd_41_controls_that_are_not_forced_breaks_...` |
| Hang at the side of the first paragraph | `fd_41_n_white_space_hangs_...`, `fd_41_m_a_trailing_space_of_the_other_level_...` |
| Justify start edge by text, not by paragraph | `fd_41_n_justify_...`, `fd_41_a_paragraph_takes_...`, `fd_41_i_justify_stretches_...` |
| Byte ranges not offset | `arq_116_ranges_index_the_whole_text_...`, `arq_116_upper_case_keeps_the_ranges_...`, 14 more |
| Extra: overflow of the last paragraph only; baselines not offset; parley's empty line kept; U+000D or U+2029 not a break; no final paragraph | `fd_41_overflow_looks_at_every_paragraph`; `fd_41_line_height_goes_on_across_paragraphs`; `fd_41_a_final_break_...`; `fd_41_n_each_forced_break_...`; `fd_41_a_final_break_...` |

Throwaway sweep (not committed, release): 138 240 layouts, 12 pieces (Latin, Arabic, mixed, the (m) text, leading and trailing spaces, empty, spaces only, digits) in triples, 5 breaks, 4 alignments, spacing 0 and 100, boxes 40, 100, 4 012 at 12 pt and, beyond the request, 5 and 25 at 12 pt and 36 at 9 pt. 1 301 760 lines: line count equal to the single-paragraph layouts in all, every cluster at the x of its single-paragraph layout (worst 0 px), baselines strictly growing, ink outside the box only in 345 600 lines of a layout with a cluster wider than the box (`overflow`) and 1 440 lines of limit (m), 0 unexplained. The sweep found nothing to fix.

Deviations and open points:
1. A blank paragraph after an Arabic one now takes its line metrics alone. The final empty line and the middle break already used Inter, so no measured height changed; heights are not fixed by (n).
2. U+000B, U+000C, U+0085 and U+001C to U+001E keep what parley does with them (one cluster with the font's glyph, not a break).


## TX2 follow-ups (W31, W32, S13)

Commits: 533e6e6 (test only), d386830 (refactor). Gates per commit, each run alone: `cargo fmt --check` 0, `cargo clippy --workspace --all-targets --locked -- -D warnings` 0, `cargo test --workspace --locked` 0.

| Item | Change | Proof |
|---|---|---|
| W31 | `("123 \u{627}", true)` in the direction table of `fd_41_a_a_paragraph_takes_its_direction_from_its_first_strong_character` | Not RED (HEAD is correct). Mutant: direction from the first logical cluster instead of `laid.is_rtl()` (`layout.rs` `let rtl`): killed by that test (`"123 ا"` laid left to right); the table without the new row passes the mutant (16 of 16), so the row is what kills it |
| W32 | `ابج  \nabc` under Justify, added to `fd_41_n_justify_starts_a_line_where_its_paragraph_starts`: the ink ends at the edge (100 ±0.01) | Not RED. Mutant: `offset = width - total + hung_width` (the hang left out of the Justify RTL offset): killed by that test |
| S13 | `read_lines` no longer returns the per-line "forced break" flag; `stretch = i != last` | Refactor, tests green. Only the last line of a paragraph can end in a forced break, because the break closes its paragraph |

## TX3

Branch `feat/gafira-text`, from `develop` da288d6. Commits: 17fc089 (columns, `Line::height`), 590cacd (font stack registered once, refactor), f69c486 (fit to the box, `ARQ-14` golden). Gates per commit, each run alone: `cargo fmt --check` 0, `cargo clippy --workspace --all-targets --locked -- -D warnings` 0, `cargo test --workspace --locked` 0. No dependency, `Cargo.toml` or `Cargo.lock` change. `Locale` has 0 matches in `crates/gafira-text`.

Measured (`git diff --shortstat d386830 HEAD`): 2 097 lines added, 34 deleted; 614 in `src/` (`columns.rs` 248, `fit.rs` 200, `layout.rs` 162, `lib.rs` 4) and 1 483 in `tests/` (`columns.rs` 687, `fit.rs` 644, `golden_fit.rs` 152). Over the ~700 mark by a factor of three; the forecast was 400 (560). The tests carry it: two brute-force oracles (all splits of the lines, all 128 sequences of 7 lines) and the rustfmt expansion of tables.

### Done

| ID | Proof |
|---|---|
| FD-45 (a) | `columns.rs`: 3 columns of 3 000 characters at 40 widths (all three residues of the line count mod 3) differ by at most one line; balance equals the best split found by trying every split (all 128 sequences of 7 lines, 2 to 4 columns, in a roomy box, in the least box that holds the text, and with auto height, plus 3 sequences of 9 where the best split is taller than the least box) |
| FD-45 (c) | overflow shows when a column is taller than a fixed box (equality fits), when a cluster is wider than its column, and not before; auto height never overflows by height; no text lost (ranges cover the input once, in reading order) |
| FD-45 (d) | `count` 0, 9, `u32::MAX`; `gap` -0.01, 200.01 (also with one column); column width 1 px is in, 22.99 px box for 3 columns and gap 10 is out; NaN and infinities first; order count, gap, width, then the font stack |
| FD-45 (e) | one column equals `layout_box` bit for bit (Debug text of the lines, `to_bits` of the heights), for gaps 0, 0.5, 37, 200, balance on and off, both sizings, 4 texts incl. bidi and paragraphs, widths 0, 30, 300, Left and Justify |
| FD-45 Reparto | columns of equal width `(w - (n-1) gap) / n`, `x = j (width + gap)`, wraps equal to `layout_box` at that width; fill by height; leftover to the last column; auto height balances always and takes its height from the columns; Justify works inside a column |
| `Line::height` | new field, the height of each line; the heights add up to `TextLayout::height` and, for one font, to the baseline distance |
| FD-46 (a) | 12 cases (bidi, paragraphs, odd ends, wrap by cluster, always and never fits): the result fits, the next size of the grid does not, unless it is `max_size`; `overflow` equals "does not fit"; the layout is the one `layout_box` gives at that size; equality of the height fits, a hair less does not; a cluster wider than the box does not fit |
| FD-46 (b) | empty text or no run: sizes unchanged, no overflow, whatever the box and the range |
| FD-46 (c) | the result is inside `[min, max]` and on the grid (multiples of 0.25, `min`, `max`); always fits gives `max`; never fits gives `min` with overflow; ranges with no multiple inside; the last multiple below a `max` that is not one |
| FD-46 (e) | `min = max` gives that size (1, 6.1, 13, 1000); `min > max`, outside 1 to 1000, NaN, infinities give `Err` naming the field; the size of a run is checked too |
| FD-46 (f) | runs of 10 and 20 pt give exactly 1:2 (also 5, 20, 10 give `r/4, r, r/2`) and their glyphs are laid out at those sizes; letter spacing is in em of each run; upper case and forced breaks keep the runs on their text |
| ARQ-14 (`FitBox`) | `golden_fit.rs`: 7 cases with literal sizes (18.5, 62.5, 24.75, 30.75, 10.0, 12.0 with overflow, 48.0), each also checked for "fits and the next does not". **Needs the `workflow_dispatch` run on 3 platforms during verify** |
| ARQ-110 | a size whose layout is not finite is too big for the search; if the minimum has none, `Err(NonFinite)` |

### RED evidence

Every RED was run on the code that existed, with a stub that compiles and does not do the behaviour (`layout_columns` laid the text out in one column at the full width; `fit_box` laid the text out at the size of `style`), before the code of the criterion. Real assertions, no compile error.

| Criterion | Failing test and assertion |
|---|---|
| `Line::height` | `fd_45_a_the_height_of_a_line_is_exposed_and_adds_up`: `heights [0.0, 0.0, 0.0, 0.0, 0.0, 0.0]` (the field was 0.0) |
| FD-45 (a) | `fd_45_a_three_columns_of_3000_characters_...`: left 1, right 3; `fd_45_a_balance_minimises_...`: `k=2 [...] AutoHeight`, left `[9]`, right `[5, 4]` |
| FD-45 Reparto | `fd_45_a_text_split_by_height_fills_each_column_to_the_box`: left `[14]`, right `[5, 5, 4]`; `fd_45_a_line_taller_than_the_box_...`: left `[3]`, right `[1, 2]`; `fd_45_columns_are_equal_in_width_...`: `(col.width - cw).abs() <= 0.01`; `fd_45_columns_work_with_justify`: `column 0 line 0 ends at 210` |
| FD-45 (c) | `fd_45_c_overflow_shows_when_...`: `fits(five - 1e-6, 2, false)` |
| FD-45 (d) | `fd_45_d_the_numbers_are_checked`: left `None`, right `Some(OutOfRange { field: "columns.count" })` |
| FD-46 (a) | `fd_46_a_the_result_fits_and_the_next_size_of_the_grid_does_not`: `"gafira fit dolor ...": 12.25 fits`; `fd_46_a_a_height_equal_to_the_box_fits`: left 12.0, right 20.0 (`equality fits`); `fd_46_a_a_cluster_wider_than_the_box_does_not_fit`: `the next size overflows` |
| FD-46 (b) | `fd_46_b_empty_text_does_not_change_...`: `the size of the run stays`, left 12.0, right 16.0 |
| FD-46 (c) | `fd_46_c_the_result_stays_between_...`: left `(12.0, false)`, right `(40.0, false)` |
| FD-46 (e) | `fd_46_e_equal_minimum_and_maximum_...`: left 12.0, right 1.0; `fd_46_e_a_minimum_over_the_maximum_...`: left `None`, right `Some(OutOfRange { field: "fit.min_size" })` |
| FD-46 (f) | `fd_46_f_runs_of_10_and_20_...`: `the largest run decides`, left 12.0, right 20.0; `fd_46_f_the_largest_run_decides_...`: left `[5.0, 20.0, 10.0]`, right `[3.0, 12.0, 6.0]` |

Not RED, proved by mutant (they pass on the code that existed, or were written with it): `fd_45_e_*` (the stub is one column too), `fd_45_auto_height_*` (vacuous on the stub, strengthened with `columns.len() == 3` before the code), `fd_45_a_with_fewer_lines_than_columns_...`, `fd_46_a_the_same_input_gives_the_same_result` (determinism has no natural RED), the six `fd_46_c_*`/`fd_46_f_*` tests added in the mutant round (`last multiple below max`, `no multiple inside`, `letter spacing in em of each run`, `upper case and forced breaks`, `non-finite layout`), and `golden_fit.rs` (the literals were measured on the code, then checked by the "fits, next does not" oracle).

### Hand mutants

| # | Wrong behaviour | Killed by |
|---|---|---|
| 1 | column width with `count` gaps instead of `count - 1`; gap left out of `x`; last column off by one | `fd_45_columns_are_equal_in_width_...`, `fd_45_d_*`, `fd_45_e_*`, `fd_45_columns_work_with_justify` |
| 2 | fill with `<` instead of `<=`; a column that may stay empty with a taller line | `fd_45_a_text_split_by_height_fills_...`, `fd_45_c_overflow_...`, `fd_45_a_line_taller_than_the_box_...` |
| 3 | vertical overflow with `>=`; cluster overflow dropped; auto height does not balance; balance only with `balance` | `fd_45_c_*`, `fd_45_e_*`, `fd_45_a_balance_minimises_...`, `fd_45_auto_height_*` |
| 4 | count range `1..8`, `0..=8`; gap bounds 199.99, 0.001; width `<= 1` | `fd_45_d_the_numbers_are_checked` (and `fd_45_e_*` for the gap lower bound) |
| 5 | objective is the tallest column, not the difference; tie goes to the smaller ends; `d < best + TIE`; `h < limit` in the search (equality binds); last column not checked against the box (`if true`); mean pruning with the wrong divisor; `bound - 0.5` on the spread | `fd_45_a_balance_minimises_the_difference_of_heights_when_lines_differ` (the first four after the 7-line and 9-line sequences were added; the last two only by the 3 sequences of 9 where the best split is taller than the least box) |
| 6 | `top += height` replaced by `top = height`; box height ignored (`INFINITY`); baselines not reset; `x: 0.0`; `width` of the box in `Column::width`; tallest column as the minimum | `fd_45_a_text_split_...`, `fd_45_a_line_taller_...`, `fd_45_c_*`, `fd_45_columns_are_equal_...`, `fd_45_e_*` |
| 7 | grid step 0.5 and 0.125; grid without `min`; `max - 0.01`; `ceil` to `floor` at the top; `last > 1`; `>= ` for min=max; scale `r / 2`, inverted ratio; ranges 999.99 and 1.01; size of `grid[last]` | `fd_46_a_*`, `fd_46_c_*`, `fd_46_e_*`, `fd_46_f_*` |
| 8 | `<` for `<=` in "fits"; "fits" without the cluster overflow; "fits" without the height; bisection side swapped; `hi - lo > 2`; overflow forced to false and to `index == last`; empty text threshold `< 2` | `fd_46_a_a_height_equal_to_the_box_fits`, `fd_46_a_*`, `fd_46_c_*`, `fd_46_e_*`, `fd_46_b_*` |
| 9 | the non-finite layout of a candidate size propagated as `Err`; the fallback for the minimum returns an empty layout | `arq_110_a_size_whose_layout_is_not_finite_is_too_big_and_the_minimum_is_an_error` |
| 10 | runs: no conversion to the upper case text; last run ends at 0; ranges of a run not relative to its paragraph; spacing of the base size for every run; `font_size()` fixed at 16 | `fd_46_f_runs_follow_the_upper_case_expansion_and_the_forced_breaks`, `fd_46_f_letter_spacing_is_in_em_of_each_run` |

Survivors, each with its proof, none needs a test:

| Mutant | Why it is equivalent |
|---|---|
| `TIE` removed or the order of the candidates reversed (`columns.rs`), `bound - 0.5` on the mean (`off <= bound - 0.5`) | The search keeps every state whose spread can tie the best, so the result does not depend on the order. Heights are sums of f32 values in f64, exact, so ties are exact. The mean bound `off <= D` is looser than the true one by `D / count`, which is at least 3.6 px here; a perturbation under that slack discards nothing |
| `from <= to` for `from < to` (`layout.rs`) | Pushing a style for an empty range changes nothing |
| `floor + 1` to `ceil` at the bottom of the grid (`fit.rs`) | Differs only when `min` is a multiple of 0.25: `min` appears twice. The bisection keeps `fits(lo)` and `!fits(hi)`, so a duplicate of `min` is never the bracket |
| `if !fits && last > 0` to `if !fits` | With `last == 0` the minimum is probed again and gives the same answer |
| `(lo + hi + 1) / 2` for the midpoint | For a predicate that is monotone in the size the result does not depend on the probes; the same 798 boxes gave the same sizes with the other midpoint, and 65 100 layouts (5 texts, 60 widths, 6 to 60 pt in steps of 0.25) gave no case of a height that falls as the size grows. FD-46 says any local maximum is valid |
| `run.size_pt == largest` guard | Removed from the code: `r * (s / s)` is `r` exactly |

### Adversarial sweep (throwaway, release, not committed)

Columns: 86 400 layouts (9 texts: empty, white space, one cluster, a word wider than any column, 1 500 characters of Latin, Arabic, Latin + Arabic + emoji, paragraphs with `\n`, `\r\n`, U+2029; boxes 0 to 16 000 px in width and height; counts 1, 2, 3, 5, 8; gaps 0, 0.5, 200; balance on and off; both sizings; Left, Justify with spacing 100, Right at 1 pt upper case spacing -100, Center at 1 000 pt spacing 500; underline). Checks: no panic; the only `Err` is `columns.width`, and only when the column is under 1 px; finite output; the right number of columns; clusters cover the input once, in reading order; the height is the tallest column; columns left to right; no overflow flag with a column taller than the box; baselines increase; one column equals `layout_box`; deterministic. 0 problems.

Fit: 15 120 calls (8 texts as one run, a 10/20 pair, a 1/500 (empty)/1000/3 mix, all-empty runs; boxes 0 to 16 000; ranges (1, 1000), (6, 400), (1, 1), (1000, 1000), (6.1, 6.3), (999.9, 1000), (12, 12.25); DPI 1, 96, 300; Left and Justify with spacing 250 and upper case). Checks: `Ok`, result inside the range and on the grid, run proportions, the layout finite, `overflow` equal to the re-fit at that size, the next size of the grid does not fit, deterministic. 0 problems in 22.6 s.

### Deviations and open points

1. **API shape (not fixed by the spec; `TextColumns` and `Autofit` do not exist in `gafira-doc`).** `layout_columns(text, fonts, style, dpi, bounds, &TextColumns, ColumnSizing) -> ColumnsLayout { columns: Vec<Column { x, width, height, lines }>, height, overflow }`; `fit_box(runs: &[FitRun { text, size_pt }], fonts, style, dpi, bounds, &FitBox { min_size, max_size }) -> FitResult { sizes_pt, size_pt, layout, overflow }` (`FitBox::default()` is 6 to 400). `ColumnSizing::{Fixed, AutoHeight}` carries the "alto automático" rule; "ancho automático" (count counts as 1) is the caller's, so it is not built.
2. **Mixed sizes.** FD-46 (f) needs runs of different size, and `layout_box` took one style. The core now takes crate-private `SizeRun`s (byte range of the input, size in px); `layout_box` passes none and is bit for bit what it was (every existing test passes). Letter spacing is in em of each run; the break uses the one of the largest size (parley takes one width), so with spacing and mixed sizes a line whose last cluster is in a smaller run is allowed up to the difference. No FD-41 or FD-46 clause asks more.
3. **Spec gaps, decided and recorded, no stop:** (a) `gap` is validated also with `count = 1` ("se ignora" is read as "no effect on the layout"); (b) the width check of a column applies with `count > 1`, a single column has no such limit (a box of size 0 is not an error, 02 §4.17.3); (c) a column fills while the heights add up to at most the box (equality fits, as FD-46), a column always takes one line, the leftover goes to the last column and flags overflow; (d) balance minimises tallest minus shortest over every split into contiguous runs of lines, empty columns allowed, ties to the split with more lines in the earlier columns, found by branch and bound from the filled split; a fixed box that cannot hold the text is filled and flagged; (e) `overflow` is the OR of a cluster wider than its column and, in a fixed box, a taller column; (f) for empty text FD-46 (b) "no cambia" returns the sizes of the runs (the largest, or `style.size_pt` with no run), not clamped to `[min, max]`; (g) `min > max` names `fit.min_size`, and the two ranges are checked first; (h) a candidate whose layout is not finite is too big; (i) FD-46 "Cabe" is the height of the layout (the sum of the line heights) against the box and `!overflow` of FD-41.
4. **Not built, outside the TX3 IDs:** columns with lists (`layout_list` lays each item with `layout_box`), the manual column break (D-24), FD-46 (d) and the type lock, FD-46 with columns (the two are independent here).
5. **Performance.** The font stack is registered once per search (`Stack`), not once per probe. The benchmarks `bench_text_columns_5000` and `bench_text_fit_500` belong to ARQ-114 (TX4).
6. **ARQ-14.** The literals were measured on Linux x86_64 with the embedded fonts. The three platforms are proved only by the `workflow_dispatch` run of the coordinator on this branch; until then the test passes on one.

## TX4

Branch `feat/gafira-text`, after TX3. Commits: 3f495ed (`skrifa` as a direct dependency, P14), 7ac695d (curve, FD-44), 04f9066 (outlines, FD-43), f2d13e8 (benchmarks, ARQ-114), da3901d (perf of the balanced split). Gates per commit, each run alone: `cargo fmt --check` 0, `cargo clippy --workspace --all-targets --locked -- -D warnings` 0, `cargo test --workspace --locked` 0. The first commit ran the gates on a tree that held an untracked throwaway sweep file, so its fmt and clippy failed; the commit was made anyway by a chained command. The gates were re-run on the clean tree afterwards: 0, 0, 0 (the commit has only manifests).

**P14, first commit.** Root `Cargo.toml`: `skrifa = { version = "0.44", default-features = false, features = ["std"] }`; `crates/gafira-text/Cargo.toml`: `skrifa.workspace = true`; `Cargo.lock` regenerated with `cargo check --workspace --all-targets` (not edited by hand): **one line added** (`"skrifa"` in the dependencies of `gafira-text`), no new package. `cargo deny check`: advisories, bans, licenses, sources ok. The V5 step and the ARQ-107 step of `ci.yml` run locally (the `run:` blocks extracted): exit 0, 0 (`Locale` has 0 matches in the crate). `cargo tree -p gafira-text -i skrifa`: 0.44.0, the copy `parley_engine` already uses.

Measured (`git diff --shortstat 3f495ed^ HEAD`): 1 402 lines added, 8 deleted. 332 in `src/` (`curve.rs` 177, `outline.rs` 122, `layout.rs` 13, `columns.rs` 16 net of the perf commit, `lib.rs` 4), 928 in `tests/` (`curve.rs` 458, `outline.rs` 448, `benches.rs` 22), 135 in `benches/text.rs`, 7 in manifests. Over the ~700 mark; the forecast was 350 (490).

### Done

| ID | Proof |
|---|---|
| FD-44 (a) | `curve.rs`: bend 0 and -0.0 leave every cluster at `x + advance / 2`, the baseline, rotation 0, bit for bit, and no `Arc` (no radius computed), for 8 texts incl. bidi, justify, empty and `"\n"` |
| FD-44 (c) | `fd_44_c_a_new_text_makes_a_new_curve`: the curve of `layout_curved` is the curve of the new layout; the radius follows the new advance |
| FD-44 (d) | at ±100 the chord of the arc is `2L/pi` ±0.01 px and the radius `L/pi`, for 5 texts and 3 widths and alignments; the chord and radius follow `2 (L/t) sin(t/2)` for 8 bends; every cluster turns `(s/L - 0.5) theta` ±1e-6 rad for 8 bends, 5 texts and 3 alignments, each line with its own `L`; +50 and -50 turn in opposite ways, open down and up, and mirror in y |
| FD-44 (e) | ±100.01, ±1e300, NaN, infinities give `Err` naming `curve.bend` (ends included); 0.01 and -0.01 keep every position finite and within 0.01 px of the straight line centred in the box; an empty line, a line that is only a forced break and an empty first line do not change |
| FD-44 geometry | every cluster is at `|R|` from the centre, at the angle of its centre; the arc is centred in the width of the box; lines are concentric with one centre, each apex on its baseline |
| FD-43 (a) | `outline.rs`: the outline of a cluster is the glyph of its font (`Cluster::font`, 3 fonts in one text) at its size, x and baseline: its box equals the box of the glyph in the font header (read with `skrifa` metrics, not drawing), ±0.05 px; `H`, `A`, `B` have 1, 2, 3 contours; a 4 px miter stroke from `gafira-pathops` is 2 px outside the fill on every side while the layout, built twice, is equal and the outlines are equal; spaces and breaks have an empty path; a beh with a fatha is drawn with each glyph at its own offset, the fatha 4 px over the body and centred on it; the runs of a `fit_box` layout are drawn at their own sizes (1:2); a font that cannot be read, a position outside the stack and a glyph that cannot be drawn give `Err(InvalidFont)`; a non-finite outline gives `Err(NonFinite)` |
| `Cluster` | two fields for the outline: `glyph_offsets` (x from the left edge of the cluster, y down from the baseline) and `size_px` |
| ARQ-114 (text) | `benches/text.rs`: `bench_text_type_5000`, `bench_text_curve_100`, `bench_text_columns_5000`, `bench_text_fit_500`; `tests/benches.rs` checks each name is registered once |

### RED evidence

Run before the code of the criterion, on a stub that compiles and does not do the behaviour (`curve_layout` returned the straight positions; `outline_text` returned an empty path per cluster, `glyph_offsets` empty, `size_px` 0). Real assertions, no compile error.

| Criterion | Failing test and assertion |
|---|---|
| FD-44 (d) | `fd_44_d_a_cluster_at_s_turns_...`: `"Hello curved world" bend 100: 0 instead of -1.3844700849289602`; `fd_44_d_the_chord_at_100_is_two_l_over_pi`, `fd_44_d_the_chord_and_radius_follow_the_bend`, `fd_44_d_plus_and_minus_...`: `an arc` (no arc) |
| FD-44 geometry | `fd_44_each_cluster_sits_on_the_circle_...`, `fd_44_lines_use_concentric_arcs_...`: `arc`; `fd_44_one_cluster_sits_at_the_top_of_the_arc_without_turning`: `100: CurvedCluster { x: 7.8828125, y: 15.5, rotation: 0.0 }` (x should be 50) |
| FD-44 (c) | `fd_44_c_a_new_text_makes_a_new_curve`: `arc` |
| FD-44 (e) | `fd_44_e_the_bend_is_checked_and_its_ends_are_in`: `100.01`, left `None`, right `Some(OutOfRange { field: "curve.bend" })`; `fd_44_e_a_line_with_no_advance_does_not_change`: `curved.arc.is_some()` |
| FD-43 (a) | `fd_43_a_each_cluster_has_the_outline_of_its_glyph_...`: `"H" has an outline`; `fd_43_a_the_counters_of_a_letter_...`: left `[0, 0, 0]`, right `[1, 2, 3]`; `fd_43_a_a_4_px_stroke_...`: `a letter`; `fd_43_a_every_size_of_a_layout_with_runs_...`: `an H`; `fd_43_a_a_font_that_cannot_be_read_is_an_error`: left `None`, right `Some(InvalidFont { index: 0 })` |
| FD-43 (a), marks | `fd_43_a_a_cluster_with_a_mark_...`: the first run failed on my miscount of glyphs (a beh with a fatha has 3 glyphs, not 2), fixed before the code; the stub leaves `glyph_offsets` empty, which `glyph_offsets.len() == glyphs.len()` rejects (0 and 3), not observed separately |
| ARQ-114 | `arq_114_each_text_figure_has_a_benchmark_named_after_its_row`: `bench_text_type_5000 must be registered once with criterion`, left 0, right 1 |

Not RED, passing on the stub or written with the code, proved by mutant: `fd_44_a_bend_zero_...`, `fd_44_e_a_tiny_bend_...`, `arq_110_a_bend_too_small_for_a_radius_is_never_a_nan`, `fd_44_a_negative_width_acts_as_zero`, `fd_43_a_the_same_layout_gives_the_same_outlines`, `fd_43_a_a_glyph_that_cannot_be_drawn_is_an_error`, `arq_110_an_outline_that_is_not_finite_is_an_error`, `fd_43_a_a_glyph_without_cover_...` (the test subsets have an empty glyph 0, so the test accepts a path that is empty exactly when the font header has no box).

### Hand mutants

| # | Wrong behaviour | Killed by |
|---|---|---|
| 1 | 1.0 or 0.9 degrees per unit; radius `L / theta` signed or halved | `fd_44_d_the_chord_*`, `fd_44_d_a_cluster_at_s_*`, `fd_44_each_cluster_sits_on_the_circle_*`, `fd_44_e_*` |
| 2 | angle without the -0.5; with `abs(theta)`; `s` from the start of the cluster, or not relative to the line | `fd_44_d_a_cluster_at_s_*`, `fd_44_each_cluster_*`, `fd_44_d_plus_and_minus_*`, `fd_44_one_cluster_sits_at_the_top_*` |
| 3 | direction swapped (no sign in x, no sign in y, sign flipped in y); sigma always 1 | `fd_44_d_plus_and_minus_*`, `fd_44_each_cluster_sits_on_the_circle_*`, `fd_44_e_a_tiny_bend_*` |
| 4 | centre at `width / 3`, or `width` without the clamp at 0; centre below instead of above | `fd_44_each_cluster_*`, `fd_44_a_negative_width_acts_as_zero`, `fd_44_d_the_chord_at_100_*` |
| 5 | concentric radius with the wrong sign or none; line apex off its baseline | `fd_44_lines_use_concentric_arcs_with_the_same_theta` |
| 6 | an arc at bend 0 or on a line with no advance; a straight line that gets an arc; empty line counted | `fd_44_a_bend_zero_*`, `fd_44_e_a_line_with_no_advance_*`, `arq_110_a_bend_too_small_*` |
| 7 | range check removed, or its upper end 100.01; NaN width unchecked; bend not checked before the font stack | `fd_44_e_the_bend_is_checked_and_its_ends_are_in` |
| 8 | the radius from the shortest line, or from the last of equals; ends at `theta` instead of `theta / 2` | `fd_44_d_the_chord_at_100_*`, `fd_44_e_a_line_with_no_advance_*`, `fd_44_e_a_tiny_bend_*` |
| 9 | glyph y flipped (down for up), baseline or x of the cluster ignored, offsets ignored (dx, dy), font index always 0, only the first glyph of a cluster, even-odd fill, size fixed at 16 px, draw error ignored, finite check removed | `fd_43_a_each_cluster_has_the_outline_*`, `fd_43_a_a_cluster_with_a_mark_*`, `fd_43_a_every_size_of_a_layout_with_runs_*`, `fd_43_a_a_glyph_that_cannot_be_drawn_*`, `arq_110_an_outline_that_is_not_finite_*` |
| 10 | in `layout.rs`: glyph offset y negated, x subtracted, x zero, y zero, size fixed at 16 px | `fd_43_a_a_cluster_with_a_mark_draws_the_glyphs_where_the_font_puts_them` (the check that kills them reads the drawing: the fatha is 0 to 15 px over the body and centred on it within 10 px, not the offsets, which the implementation also reads) |
| 11 | columns search: interval bounds off by one or removed, objective, tie-break, strict limit, mean divisor (re-run after the perf commit) | `fd_45_a_balance_minimises_...`, `fd_45_a_three_columns_*`, `fd_45_a_with_fewer_lines_*`, `fd_45_auto_height_*` |

Survivors, none needs a test:

| Mutant | Why |
|---|---|
| `offsets.push((g.x, g.y))`, `(pen, g.y)` and `pen += 0.0` (the pen accumulation of `glyph_offsets`) | Equivalent with the embedded fonts: every cluster with more than one glyph in them is a base with zero-advance marks, and in visual order the base is last, so the pen is 0 for every glyph that has a position. Open point: a font with a multi-glyph cluster whose first glyph advances (a Latin base with a combining mark, in Inter) is not available. The rule is parley's `Glyph { x, y, advance }` as documented in `parley_engine` |
| `glyphs[0] != 0` (skip a cluster of `.notdef`) | Equivalent with the embedded fonts: glyph 0 has no outline in the three subsets (the font header has no box for it). The test checks the box when the header has one |
| Pruning guards of the balanced split: `.min(low + bound)`, `max(high - bound)`, `if high - low > bound`, strict or `-1.0` variants | They only discard states that `offer` would reject by itself, so the result is the same; they bound the time (20 000 lines, 5 columns: 606 ms before the interval search, 55 ms after, almost all of it the layout) |
| `floor + 1` to `ceil` at the bottom of the fit grid; `last > 0` guard; midpoint rounding | See TX3 |

### Benchmarks (release, `cargo bench -p gafira-text --bench text -- --warm-up-time 1 --measurement-time 5`, i7-12700H under load, not D-03 hardware)

| Name | Mean | p50 | p99 over the 100 samples | Budget |
|---|---|---|---|---|
| `bench_text_type_5000` | 2.45 ms | 2.30 | 4.77 ms | p99 < 16 ms |
| `bench_text_curve_100` | 0.094 ms | 0.091 | 0.136 ms | p99 < 16 ms |
| `bench_text_columns_5000` | 2.77 ms | 2.39 | 4.98 ms | p99 < 16 ms |
| `bench_text_fit_500` | 5.85 ms | 6.31 | 7.92 ms | p99 < 16 ms |

Each sample is the mean of many iterations, so these p99 are a lower bound of the p99 of one call; the nightly job on the reference machine decides. `bench_text_curve_100` measures the whole `layout_curved` (shaping and curve), not only the curve. The columns bench was measured before the perf commit; the search it changed is under 1 ms at 5 000 characters.

### Adversarial sweep (throwaway, release, not committed)

Curve and outlines: 2 475 checks (9 texts: empty, `"\n"`, a space, one letter, Latin, 12 repeats of a sentence, Arabic with fatha, Latin + Arabic + emoji + an uncovered CJK, five lines with an empty one and different lengths; widths 0, 1, 60, 300, 16 000; five styles from 1 pt to 1 000 pt, 1 to 300 DPI, spacing -100 to 500, upper case, Justify, Right, Center, underline; ten bends: ±100, 99.99, 50, ±0.01, 1e-9, 0, 5e-324, 37.3). Checks: lines and clusters agree; all positions finite; radius equal to the longest line's `L/|theta|` less the baseline distance; every cluster on its circle; rotations per formula; chord of the reference line `2 R sin(theta/2)` ±0.01 where the radius is under 1e7; no arc on a straight line; outlines: one for each cluster, finite, inside two em of the cluster, deterministic, and a round 4 px stroke from `gafira-pathops` succeeds (sizes under 5 000 px). 0 problems. Fit and outlines: 180 `fit_box` calls with runs of 1, 500, 1 000 and 3 pt, boxes 0 to 16 000 px, ranges (1, 1000), (6, 400), (12, 12), DPI 1 to 300, underline, Justify, spacing 150: `Ok`, and the outlines of the result finite and within two em of the baseline. 0 problems.

Performance of the columns search (found by the sweep, fixed in da3901d): 5 000 to 20 000 forced lines, uniform, random and periodic heights, 2, 5, 8 columns, auto, roomy and tight: at 20 000 uniform lines and 5 columns the split took 606 ms (it scanned every end at every node); it now takes ~0 ms. Same results (all 12 `columns.rs` tests, including the exhaustive comparison with every split).

### Deviations and open points

1. **API shape (not fixed by the spec).** `curve_layout(&TextLayout, width, bend) -> Vec<CurvedLine>` and `layout_curved(text, fonts, style, dpi, bounds, bend) -> CurvedText { layout, lines }`; `CurvedLine { arc: Option<Arc { radius, center, start, end }>, clusters: Vec<CurvedCluster { x, y, rotation }> }`, one `CurvedLine` per line and one `CurvedCluster` per cluster. `x, y` is the centre of the advance on the arc, `rotation` is in radians, clockwise positive with y down. `outline_text(&TextLayout, fonts) -> Vec<ClusterOutline { line, cluster, path: gafira_pathops::Path }>`, filled with the non-zero rule, in px from the top left of the box. A text on a curve is not outlined here (FD-44 (b) is out of the cut).
2. **Spec gap, multi-line curves (FD-44 "Varias líneas ... concéntricos con el mismo θ", PROPUESTA). Decided, not blocked, for `specifier`.** "radio = L / |θ|" fixes the radius of one line; with several lines it cannot hold for each and also keep them concentric one baseline distance apart (two justified lines of the same `L` would lie on the same arc). Chosen: one centre; the longest line (the first of equals) has `R = L / |theta|`; every other line is one baseline distance nearer to the centre in an arch and further in a cup (`R_i = R - sigma (b_i - b)`); each line spans the same angle `theta`, so its glyphs are spaced by `R_i |theta| / L_i` of their natural advance (a shorter line is spread, an inner line of the same length is packed), and each cluster turns by its own line's `(s / L_i - 0.5) theta`, which is what (d) says. The longest was chosen over the first line so that no line is packed tighter than the reference when the first line is short. A line deep inside an arch can reach `R <= 0`, which mirrors it through the centre; the result is finite and I did not clamp it. The chord criterion (d) is exact for the reference line. Recommendation: state in 04 FD-44 which line sets the radius, and what happens when `R <= 0`.
3. **The arc is centred in the width of the box**, as FD-44 says, so a curved line ignores the alignment of its layout (a Left line is centred on the arc).
4. **A bend that is not 0 but gives `theta` = 0** (below about 1e-322) is straight, with no radius; a radius that overflows (bends of 1e-306 or less on long lines) is `Err(NonFinite { field: "output" })`. FD-44 (e) asks only for 0.01.
5. **FD-43 (a).** "Un contorno de 4 px no cambia el layout" is proved as: the contours come from the layout and its fonts (`&TextLayout` in, no style or effect in), a 4 px stroke from `gafira-pathops` grows the ink by 2 px on every side, and the layout (and the cluster positions and advances) is equal before and after. The layout has no outline input to vary, so nothing else can be tested. Glyphs use the first face of a font file; a glyph with no outline gives no contour. `Cluster` gained `glyph_offsets` and `size_px` for this.
6. **The `.notdef` of the three test subsets has no outline**, so the `.notdef` cluster has an empty path. A font whose glyph 0 is a box would draw it.
7. **Subsets and bidi in the sweep** found no defect in TX3 or TX4 other than the performance of the balanced split.

## Fix round 1 of verify (TX3 and TX4), `tdd-apply`

Commits on `feat/gafira-text`: `3d9d9d1` (test gaps C25, C26, C27, W36, S17), `84cea9f` (C23), `6240b05` (W35, S14, S15), `b91594e` (C24, W34, FD-44 (d) to (g), FD-45 one column). Spec read: `39df3a9`. The gates ran on a clean tree before each commit: `cargo fmt --check` 0, `cargo clippy --workspace --all-targets --locked -- -D warnings` 0, `cargo test --workspace --locked` 0, each exit code read alone.

### RED evidence

Items whose code was right (C25, C26, C27, W36, S17) had no failing test on HEAD by nature: the test is the new proof and its RED is the hand mutant that it kills (below). The others failed first on the code of HEAD:

| Item | Test | Failing line on the code before the fix |
|---|---|---|
| C23 | `fd_43_a_letter_spacing_opens_every_gap_between_inked_clusters_in_both_directions` | `"داراز": gap 1 grew by 0 instead of 10` |
| W35 | `fd_46_f_no_line_passes_the_box_when_the_sizes_and_the_spacing_differ` | `(20.0, 10.0) t 500: a line of 63.12499761581421 px in a box of 60`; for a negative spacing, with the first fix only: `(20.0, 10.0) t -100: a line of 85.40624952316284 px in a box of 84.66` |
| S14 | `fd_46_f_a_run_with_no_text_does_not_set_the_largest_size` | `left: 7.425000000000001 right: 74.25` |
| flag | `fd_44_the_trailing_white_space_of_a_line_is_flagged_in_either_direction` | `"ab cd"  left: [[], []]  right: [[2], []]` |
| C24, W34, FD-44 (d) | `fd_44_f_no_line_of_an_ordinary_box_has_a_radius_of_zero_or_less` | `bend 100: an arc of 236.42291722191004 px for a line of 292.7421875` |
| W34, FD-44 (g) | `fd_44_g_a_space_at_the_end_does_not_move_the_glyphs_before_it`, `fd_44_d_a_wrapped_line_ignores_the_space_that_hangs_at_its_end`, `fd_44_d_a_space_inside_the_line_that_hangs_leaves_no_gap_in_the_arc` | `assertion failed: (ca.radius - cb.radius).abs() <= 0.01`; the others at their `assert!` on positions and rotations |
| FD-44 (e) | `fd_44_e_a_line_with_no_advance_does_not_change` (rows `" "`, `"a\n \nb"`, `"a\n   \nb"`) | `left: Some(Arc { radius: 2.04627783975294, ... })  right: None` |
| FD-44 multi-line | `fd_44_lines_use_concentric_arcs_with_the_same_theta` (rewritten to the `R_ref` rule, three more texts) | failed on HEAD at the radius of its lines |

Old curve tests (`fd_44_d_*`, `fd_44_e_a_tiny_bend_*`, the circle test) now read `L` and `s` without the white space that hangs; they changed with the rule, and failed on HEAD with it.

### Hand mutants (made in `src`, seen to fail, restored; none committed)

| Mutant | Killed by |
|---|---|
| C25: `Pen::quad_to` draws a `line_to` to the end point | `fd_43_a_a_round_glyph_keeps_the_curves_of_its_font`: `LineTo((21.65625, 93.34375)) instead of QuadTo((28.953125, 97.859375), (21.65625, 93.34375))` |
| S17: `Pen::curve_to` draws a `line_to` | `outline::tests::fd_43_a_a_cubic_curve_of_a_font_stays_a_cubic_curve_moved_and_flipped`: `left: LineTo((15.0, 94.0))  right: CurveTo(...)` (in-crate, since no embedded font has cubics) |
| C26: `ColumnsLayout::height` is `out[0].height` | `fd_45_auto_height_is_the_tallest_column_when_the_lines_differ_in_height`: `left: 53.151375 right: 58.078125` (the fixture asserts first that column 0 is shorter) |
| C27: `text.trim().is_empty()` in `fit_box` | `fd_46_a_white_space_is_a_text_with_a_height_and_grows_to_the_box`: `grew from 16 pt, got 16` |
| W36: `bench_text_fit_500` out of `criterion_group!` | `arq_114_each_benchmark_function_is_in_the_criterion_group`: `bench_text_fit_500 is defined but criterion_group! does not run it` |
| C23: `-=` for `+=`; `/ 2.0`; no `c.rtl` guard (left-to-right moves too) | the C23 test: `gap 1 grew by -10 instead of 10`; `grew by 5 instead of 10`; `"HIHIH": gap 4 grew by 0 instead of 10` |
| W35: the spacing of the largest run (HEAD); the smallest for a negative spacing; the largest for a positive one | the W35 test: `63.12` px in 60; `t -100 ... 85.41 px in 84.66`; `t 500 ... 63.12 px in 60` |
| S14: `.filter(\|_\| true)` for runs with text | `fd_46_f_a_run_with_no_text_does_not_set_the_largest_size`: `left: 7.425 right: 74.25` |
| one column: `overflow` not set for `count = 1` | `fd_45_a_a_single_column_flags_overflow_when_the_text_is_taller_than_a_fixed_box`: `taller than the box` |
| curve: no `sigma` term in `R_ref` | `fd_44_lines_use_concentric_arcs_with_the_same_theta`, `fd_44_d_the_chord_at_100_*`, `fd_44_f_*` |
| curve: `L` is `line.advance` | `fd_44_d_a_cluster_at_s_*`, `fd_44_g_*`, `fd_44_d_a_wrapped_line_*`, `fd_44_d_a_space_inside_*`, the concentric test |
| curve: `s` counts the advance of the hanging space | `fd_44_d_a_space_inside_the_line_that_hangs_leaves_no_gap_in_the_arc`, `fd_44_d_a_wrapped_line_*`, `fd_44_g_*` |
| curve: lines with `L = 0` enter the maximum; `length < 0.0` curves them; `min` for `max`; radius `+` for `-` | the concentric test (fixture `"ab\n \n \n "`); `fd_44_e_a_line_with_no_advance_does_not_change`; `fd_44_d_the_chord_at_100_*`, `fd_44_f_*`; the concentric test |
| flag: `c.trailing = !*h` | `fd_44_the_trailing_white_space_of_a_line_*` and six more |

### Adversarial sweep (throwaway, release, deleted)

Curves: 30 720 checks (10 texts: Latin, Arabic, Latin + Arabic both ways, white space only, leading and trailing spaces, empty lines, Arabic + emoji; boxes 1 to 5 000 px; 6, 12, 40, 200 pt; spacing 0, 100, -50, 500; Left, Center, Right, Justify; bends ±100, 99.9, -50, 17, ±0.01, 1e-5). For every line: `Ok`; radius > 0; `R_i |theta| >= L_i` within 0.01 px; the lines with content share the centre; one has the equality; no line with content is straight. 0 problems. Ink gaps: 858 checks on the outlines (Latin, Arabic, joined Arabic, a phrase, 12 and 75 pt, spacing 100, 300, -50, four alignments; Justify skipped where a space moves) plus 108 with two runs of 10 and 20 pt in both orders and both directions through `fit_box`: the gain of each gap over the unspaced layout is the spacing of the cluster that owns it (the logical previous one), within 0.05 px. 0 problems.

### Measured

`git diff --shortstat 2e8d7a8..HEAD` before this note: 12 files changed, 738 insertions, 97 deletions (mostly tests: `tests/curve.rs` +319 -86).

### Deviations and open points

1. **`Cluster::trailing`** is a new public field (white space at the logical end of its line, any direction). The curve takes only a `TextLayout`, with no text, and FD-44 now needs the hanging space (also the opposite-level one, inside the line). It is the `hanging_mask` that `read_lines` already computed for the spacing. `Cluster` is built only in `read_lines`.
2. **A hanging space has a point on the curve, no place**: it sits where the content before it ends (`s` does not grow by its advance), so one position per cluster stays and the cluster count of a line is unchanged. The spec says it "has no place on the arc" and fixes nothing else. Tested only for finite values and for not moving the glyphs.
3. **W35 and negative spacing.** The break widens the box by the spacing of the smallest run for a positive spacing and of the largest for a negative one (the smallest value the last cluster can have), found by the test above. A line that ends in a large run may break earlier, as the report said.
4. **S14.** An empty run keeps its proportion to the largest run with text, so its entry in `sizes_pt` can pass 1 000 pt (an empty run of 1 000 pt beside text of 1 pt, at the top of the range); it is only a number, since it lays out nothing. S15 is a sentence in the doc of `fit_box`.
5. **Interior opposite-level space.** No embedded text I found makes parley put a trailing space of the other level inside a line (probed 10 texts, with RLM and LRM), so that case is proved on a layout whose cluster is flagged by hand (`fd_44_d_a_space_inside_the_line_that_hangs_leaves_no_gap_in_the_arc`). The flag itself is proved in both directions in `tests/layout.rs`.
6. **W37 stays open** (FD-45 with lists: `layout_list` and `layout_columns` do not meet). S16 and S18 are settled by `39df3a9`; S18 got a test (`fd_45_a_a_single_column_flags_overflow_*`), S16 was already tested in `fd_45_d_the_numbers_are_checked`.
7. The `Slice card` was not touched.
