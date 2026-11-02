## Slice card
Flow: full | level: L3 | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: owner | reason: layer-2 crate in the owner's parallel full route; one writer per worktree
Product IDs: TX1 FD-41 (a, d), ARQ-115, ARQ-116, ARQ-75 (partial file), ARQ-110, ARQ-111; TX2 FD-41 (c, e to m); TX3 FD-45 (a, c, d, e), FD-46 (a, b, c, e, f), ARQ-14 (FitBox); TX4 FD-44 (a, c, d, e), FD-43 (a), ARQ-114 (text) (source: specs/02-arquitectura-software.md, specs/01-producto-y-calidad.md, specs/04-funcionalidad-diseno-imagen-exportacion.md, specs/05-funcionalidad-video-y-distribucion.md)
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
2. **Left alignment moves nothing.** For a right-to-left paragraph the hanging space of a line sits on its left; with Left it stays where parley puts it. Only Center and Right use the ink of the line.
3. **Justify** is parley's. It reaches the edge within 0.01 px for every spacing and width swept (40 to 200 px, five spacings). A line of one word is not stretched, and the test skips it.
4. **Hanging space in a right-to-left run.** Parley hangs the space of the line end only at the paragraph's bidi level (C1 of TX1). The output rule (no spacing, outside the ink) is mine and holds in any direction; the fit is parley's, so a right-to-left line that ends in a space inside an opposite-level run may break one cluster early.
5. **Decoration of a missing table.** Only the thickness is 0.05 em; the offset of a missing table is parley's, which no rule fixes.
6. **List marker measure.** The hang is the `x` of a letter after "marker space", measured with the style of the list, so letter spacing and kerning count; markers `◦` and `▪` are not in the Inter subset (S5), they only change the width of that level.
7. **Not raised:** C14 resolved by the amended FD-41 (f), whose numbers parley already gives (mixed-line test). W19 to W25 and S7 need no change in code; `fallback.rs` still takes coverage from comments (S7).
