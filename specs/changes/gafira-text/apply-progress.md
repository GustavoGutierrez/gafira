## Slice card
Flow: full | level: L3 | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: owner | reason: layer-2 crate in the owner's parallel full route; one writer per worktree
Product IDs: TX1 FD-41 (a, d), ARQ-115, ARQ-116, ARQ-75 (partial file), ARQ-110, ARQ-111; TX2 FD-41 (c, e to j); TX3 FD-45 (a, c, d, e), FD-46 (a, b, c, e, f), ARQ-14 (FitBox); TX4 FD-44 (a, c, d, e), FD-43 (a), ARQ-114 (text) (source: specs/02-arquitectura-software.md, specs/01-producto-y-calidad.md, specs/04-funcionalidad-diseno-imagen-exportacion.md, specs/05-funcionalidad-video-y-distribucion.md)
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
