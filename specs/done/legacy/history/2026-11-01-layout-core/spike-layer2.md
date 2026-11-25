# Layer-2 gaps: gafira-text, gafira-raster, gafira-audio, gafira-segment

Read-only survey, 2026-10-30. Paths are relative to the repo. "02" = docs/specs/02-arquitectura-software.md, "04" = docs/specs/04-...md, "05"/"01" = docs/specs/backlog/. Nothing in 00-decisiones.md is reopened.

Verified by running: `rustc 1.93.1` rejects `f16` (E0658, unstable, issue #116909). A scratch `cargo` probe (parley features, licenses) was denied by the sandbox, so every claim about third-party crate internals is marked NOT VERIFIED.

## 0. Cross-cutting facts (apply to all four crates)

| # | Fact | Evidence | Class |
|---|---|---|---|
| X1 | None of the four crates has a Cargo dir, V5 row or clippy.toml yet. Each needs: V5 row in .github/workflows/ci.yml:79-88, byte-identical clippy.toml (V12, ci.yml:118-133), `#![forbid(unsafe_code)]` (ci.yml:68-73), workspace entry in Cargo.toml:13-23. | ci.yml, Cargo.toml | mechanical |
| X2 | The shared contracts have no home. `Describe`/`Descriptor` (02:420), `Registry` (02:413), `CancelToken` (02:969), `Progress` (02:553/593), `ParamSchema`/`ParamValues`/`Cost`/`EffectCx`/`EffectError`/`UniformBlock` (02:513-521), `ImageView(Mut)`/`IRect` (02:517), `AudioBlock`/`ParamSnapshot` (02:652-653), `AnimatableParams` (02:736) are used by traits that live in layer 2, but 02 §2.1 gives `Registry` to gafira-ports (layer 3, 02:100) and names no crate for the rest. Layer 2 cannot depend on layer 3 (ARQ-01). gafira-geom has `Rect` only (kurbo, crates/gafira-geom/src/lib.rs:11), no `IRect`. gafira-doc has `NodeKind` = Group/Shape/Extension only (crates/gafira-doc/src/node.rs:64-72): no Text, Image, Mask, EffectInstance, TransitionInstance, Clip. | 02 §2.1, grep of crates/*/src | specifier (design decision, `architect`): options (a) module in gafira-doc next to `msg` (no new crate, no new V5 row beyond doc edges), (b) new tiny layer-0 crate (needs a new 02 §2.1 row and V5 row, ADR-01 criterion 1). Recommend (a). |
| X3 | Labels must be keys: `Descriptor` and `ParamSchema` labels are extracted by `xtask check-i18n` (ARQ-98, 02:1807), so they are `MessageKey` from gafira-doc::msg (02:1695). So raster, audio and segment need an edge to gafira-doc. 02 §2.2 states raster -> color, geom only (02:136) and "geom/color/time/doc según necesidad" generically. | 02:136, 1695, 1807 | specifier: say it in 02 §2.2 and in the V5 rows |
| X4 | `AudioCrossfade` is returned by `Transition::audio()` (02:731), a raster trait, but the gains belong to gafira-audio. Raster -> audio and audio -> raster are both unstated edges. | 02:731 | specifier: enum lives with the shared contracts (X2) or in raster; audio exposes pure gain functions; gafira-media (layer 3) glues them. No new edge. |
| X5 | Allocation guard vs unsafe policy. 02:664 and 05 FV-20(b)/ARQ-23 require "el asignador de prueba de gafira-testkit, que hace panic si hay asignaciones en el hilo de audio". A `#[global_allocator]` needs `unsafe impl GlobalAlloc`. AGENTS.md:97: "`unsafe_code` denied (`#![forbid(unsafe_code)]` per crate), no `#[allow]` to silence lints". gafira-testkit has `#![forbid(unsafe_code)]` (crates/gafira-testkit/src/lib.rs:8) and CI checks it (ci.yml:68-73). No crate in 02 §9.2 provides an allocation guard. | above | OWNER (O1) |
| X6 | Golden harness absent. 02 §10 names `gafira-testkit::golden`; the testkit has only `harness` + generators (crates/gafira-testkit/src/lib.rs, harness.rs). Every golden criterion (ARQ-25, ARQ-14, FD-33, FD-38) needs it, or analytic expected values instead. Tolerances exist: 02 §10 (CPU same platform byte-identical; cross-platform <= 1/255 on 99.9 % px). Baseline platform, file format and regeneration command (`gafira-cli`, not built) are unspecified. If the harness takes `ImageView`, testkit gains an edge to raster: V5 testkit row (ci.yml:85) must be amended. | 02:1410, ci.yml:85 | specifier |
| X7 | Perf budgets for text have no §6.7 row. FD-41 (type < 16 ms, 5 000 chars), FD-44 (< 16 ms/event, 100 chars), FD-45 (relayout 5 000 chars, 4 cols < 16 ms), FD-46 (< 16 ms, 500 chars) cite "CAL-01" (FD-41), none, none; CAL-01 is "Selección de un objeto" (01:298). ARQ-40 requires a bench per §6.7 figure. | 04:422,444,452,460; 01:298 | specifier: add §6.7 rows/CAL ids |
| X8 | Determinism recipe. ARQ-10 bans HashMap in domain crates (clippy.toml `disallowed-types`); rayon pool is passed by parameter, never global (02:970); ARQ-02 bans `Instant::now`/threads in tests too (`--all-targets`). So: no wall-clock assertions in unit tests (cancel <= 100 ms and ETA must be tested as poll-count/iteration bounds), parallel reductions need fixed order for byte-identical goldens. | 02:970, crates/gafira-pathops/clippy.toml:7-45 | specifier |

## 1. gafira-text

**Rows:** 02:95 (MVP; shaping, bidi, line breaks, box layout, tracking, ligatures, text on curve, outlines; fonts arrive as bytes; must not enumerate system fonts). ADR-01 criterion 4 (02:170) says layout also uses gafira-text, but 02 §2.2 (02:136) says `layout -> geom, doc` and `text -> pathops`: contradiction (layout-core change is already scoped; just keep consistent).

### 1.1 Requirement IDs
Pure-function testable (layout geometry, no pixels):
- FD-41 (04:418-424): (c) tracking 0 and line_height 1.0 are identity; size 1-1000 pt, tracking -100..500 milli-em, line_height 0.5-4.0, kerning/ligature toggles, lists with indent per level, justify, vertical anchor. (a) tildes/bidi/emoji is "golden": testable at glyph-run/cluster level without pixels.
- FD-45 (04:448-454): (a) 3 columns, 3 000 chars, heights differ <= 1 line when balanced; (c) overflow indicator; count 1-8, gap 0-200.
- FD-46 (04:456-462): (a) grows until touching edge within 1 line; (b) empty text unchanged; (c) respects min/max; default 6-400 pt.
- FD-44 (04:440-446): (a) Curvar 0 equals straight text (diff 0); (c) rebuild on text change. Needs gafira-pathops for outlines (exists).
- FD-43 (04:432-438): (a) 4 px outline does not change layout.
- ARQ-14 (02:400): FitBox identical on 3 platforms with embedded fonts (golden; geometry-level is enough); type_lock half is a doc/edit-level property test, not gafira-text.
- FD-48 (04:472-478): (a) substitute font moves boxes <= 2 % (needs metrics API; the choice algorithm has no owner crate, see 1.4).
- FV-08 (05:111-115): warnings for > 2 lines / 42 chars per line, so layout must expose line count; FV-07: "escribir letra a letra" needs per-grapheme-cluster addressing.
UI/export/E2E only: FD-42 (colour picker, eyedropper, contrast warning), FD-43(b)(c) (SVG/PDF export), FD-47 (commands), FD-48(b)(c), FD-49 (Command/Visitor), FD-41(b) (one Command), FV-08(a)(b).

### 1.2 MVP vs later
All of RF-24/25/26 are MVP (01:171-173). Fase 2 inside FD-44: text on a vector shape and vertical text (PROPUESTA, 04:442). Manual column break: PROPUESTA (04:448) yet FD-45(b) tests it. Effects glow/echo/neon are paint-level (render), not this crate.

### 1.3 First slice and order
- T1: input model + `shape` + line breaking + `layout_box` single column, align L/C/R/justify, tracking/line_height identity laws, overflow flag, per-run kerning/liga flags, line and cluster geometry output. T2: columns + balance. T3: FitBox bisection. T4: glyph outlines + Curvar (uses pathops).
- Prerequisites: (1) decision X2; (2) input types: either TextNode in gafira-doc first (02:233-244 gives fields, but `RichText`, `TextAlign`, `VAnchor`, `TextSizing`, `TextPath`, `StyleRef` fields are undefined) or plain input structs owned by gafira-text with the doc-to-text mapping in layer 3. Recommend the second: keeps "fonts are bytes" and avoids a doc change before text exists. (3) font fixtures + licence entries (assets/ has no fonts and no LICENSES.toml; ARQ-75, 02:1396). (4) a spike on parley/skrifa/harfrust (see 1.5).
- Order: independent of raster/audio/segment. Needs gafira-pathops (done) and gafira-geom (done). Does NOT need to wait for any other layer-2 crate.
- V5 row candidate: `gafira-text gafira-geom gafira-pathops` (+ `gafira-doc` if TextNode types are used; + `gafira-color` only if style types carry colour). Edges text -> pathops stated (02:136); text -> doc only generic.

### 1.4 Open items
| # | Item | Evidence | Class |
|---|---|---|---|
| T-a | `line_height: f32` (02:254) multiplies what? FD-41 says 1.0 is identity (04:424) but not relative to em or to font ascent+descent+lineGap. Blocks every height assertion. | 02:254, 04:420,424 | specifier (reference: CSS `normal` = hhea/OS2 natural line height x factor) |
| T-b | `tracking` unit: Animatable<f32> in 02:254, "milésimas de em" in 04:420. Identity 0 agreed. Ligatures with tracking != 0 not stated. | | specifier (CSS rule: disable ligatures when tracking != 0) |
| T-c | FitBox: bisection tolerance and result quantization are unspecified, so "same size on 3 platforms" (ARQ-14) cannot be asserted; "fits" is not monotone in size under word wrap, so "mayor tamaño" is not what bisection returns; per-run sizes (mixed sizes in one TextNode) unspecified; behaviour with AutoWidth/AutoHeight (columns say Fixed/AutoHeight only, 02:259); min > max, min <= 0, NaN/inf; "tolerancia de 1 línea" (04:462) is not a number. Default 6-400 (04:456) vs size range 1-1000 (04:420). | 02:260, 04:456-462 | specifier (define step e.g. fixed N bisection steps on a size grid, scale all runs proportionally, invalid range -> error) |
| T-d | `type_lock.size` + FitBox: PROPUESTA (04:456). | | OWNER (O8) |
| T-e | Manual column break: PROPUESTA (04:448) with acceptance (b) (04:454); `TextColumns` has no field for it (02:250). | | OWNER (O7) |
| T-f | Columns: `count` outside 1-8 (0, 9, u8 up to 255), `gap` unit and negative, `balance` with Fixed sizing and overflow. | 02:250,259, 04:448 | specifier |
| T-g | Curvar -100..100 -> arc angle mapping is not defined (04:442). | | OWNER (O9) |
| T-h | Emoji and non-Latin: embedded Inter/Manrope cover only latin/latin-ext (02:1183, ARQ-105); the system fallback is supplied by gafira-platform as bytes (02:1183). FD-41(a) requires "tildes, bidi y emojis" goldens. Nothing says which fonts the tests use, whether colour emoji (COLR/CBDT/sbix) are in the MVP, or who decides font fallback order. | 04:424, 02:1183 | OWNER (O2) |
| T-i | Preview uses "hinting de pantalla" (02:951), export does not: hinting mode must be an input; golden = unhinted only. | 02:951 | specifier |
| T-j | List numbering styles, bullet glyphs, indent amount per level, max levels; justify of last line; case mapping ("mayúsculas") locale-independent vs ARQ-107 (no `Locale` in layers 0-2, 02:1816). Underline/strike geometry from font post/OS2 tables. | 04:420 | specifier (root-locale full case mapping; font-table metrics) |
| T-k | Font substitution ("misma clase y métricas similares", 04:472) has no owner crate: gafira-text only gives metrics; gafira-platform only enumerates (02:116). | 04:472 | specifier (assign to engine; text exposes metrics/class) |
| T-l | Stale marks: 04:412 and R-B (04:730) say "VERIFICAR licencias" of the text crates; 02:1311 says parley+skrifa+harfrust VERIFICADA (cargo-deny). No bidi crate is named in 02 §9.2. | 04:412,730; 02:1311 | specifier |
| T-m | (NOT VERIFIED, recollection only) parley's default features may pull a system font collector (fontique) and file access; that would clash with "fonts arrive as bytes" (02:95) and ARQ-02. Check `cargo tree -p gafira-text -e features` in the spike. | | explorer |

### 1.5 Third parties vs 02 §9.2 and deny.toml
Named: parley, skrifa, harfrust (02:1311), icu_segmenter (02:1304, explicitly allowed in gafira-text by ARQ-107). Not named: a bidi crate; `kurbo` types shared via gafira-geom. All licences look inside deny.toml:17-31 (Apache-2.0/MIT/Unicode-3.0) but NOT VERIFIED here. No deny.toml edit expected. icu_segmenter compiled data size vs the 40 MB binary / 2 MB i18n budgets (ARQ-106) not verified.

## 2. gafira-raster

**Row:** 02:97 (MVP; buffers u8/f16/f32 premultiplied, tiles, mipmaps, `Effect` + core effects, `Transition` + core transitions, scopes Fase 2). 04/05 also assign to raster: blend modes (04:142), dominant-colour palette (04:272), perceptual hash (04:288), tracking and stabilisation motion estimation (05:265,273, Fase 2). The 02 raster row lists none of these.

### 2.1 Requirement IDs
Pure-function testable:
- Transitions: ARQ-25 (02:798) and 02 §4.13 step 3 (02:748): output == `from` at progress 0, == `to` at 1, goldens at 0 / 0.5 / 1; FV-06(a) (05:96). RF-39.
- FD-33 (04:344-348): (a) every slider at default is identity (diff 0); (d) Ajuste automático idempotent within 1/255. FD-34(b) intensity 0 == no filter; (a) thumbnail == final within 1/255. FD-35(a) shadow follows alpha; presupuesto < 100 ms radius <= 40 px 1080p. FD-12(c) 16 blend modes (CPU half). FD-28(b) resized image flagged similar (dHash); hash 12 MP < 100 ms. FD-26(a) 5 dominant colours, dE2000 < 15. ARQ-42 / CAL-34 (<= 1/255 on 99.9 %). Benches: CAL-04 (filter 4000 px < 300 ms), CAL-05.
UI/GPU/E2E only: FD-33(b)(c) and FD-35(c) (GPU, UI), FD-32 (Zona de edición, engine), FD-37 (Comparar/Restablecer), FD-35(b) (SVG export), FV-06(b) and FD-12(c) GPU half (render-gpu).

### 2.2 MVP vs later
MVP (RF-20/21/39): lighting/colour/white-balance/texture adjustments, blur, sharpen, vignette, shadows (Paralela, Brillo, Contorno), 6 transitions, 16 blend modes (PROPUESTA, 04:142), tiles 256x256 and mips (FIJADA, 02:943), palette, dHash. Shadow "Curva": PROPUESTA (04:354). Fase 2: scopes, chroma key (FV-16), LUT 3D, tracking, stabilise (FV-17/18), curves/wheels (FV-19).

### 2.3 First slice and order
- R1: pixel buffer types (u8 premultiplied, f32; f16 deferred or `half`, see 2.5), straight<->premultiplied conversion, `ImageView(Mut)`, then `Transition` trait + Cut/Fade/Dissolve/Slide/Zoom/Wipe with the ARQ-25 laws and analytic expected values. R2: blend modes + composite. R3: adjustments (identity law), gaussian blur, shadow. R4: mips/tiles, palette, dHash.
- Prerequisites: X2 (shared contracts: `Describe`, `ParamSchema`, `ParamValues`, `EffectCx`, `EffectError`) blocks even R1 because `Transition: Describe` and `params()` return `ParamSchema`; X6 for goldens from R3 on; gafira-color (has `ColorSpaceId`, `PixelLayout::{Rgb,Rgba}` only, no gray/alpha-only layout, crates/gafira-color/src/space.rs:10-20, transform.rs:10), gafira-geom (done).
- Order: independent of text and audio. Gate for gafira-segment.
- V5 row candidate: `gafira-raster gafira-color gafira-geom` + `gafira-doc` (X3). Fase 2 `KeyTrack` output of tracking would add `gafira-anim`: NOT in 02 §2.2 (new edge, only when Fase 2 starts).

### 2.4 Open items
| # | Item | Evidence | Class |
|---|---|---|---|
| R-a | `progress` outside [0,1] or NaN: easing can overshoot (CubicBezier y in [-2,3], ARQ-17) so the builder can hand progress < 0 or > 1. Not defined. Same for NaN/inf pixels in `from`/`to` (lerp at t=1 with inf gives NaN). | 02:728, 02:403 | specifier (clamp, as Q-31 does for alpha; inputs must be finite) |
| R-b | Per-transition edge definitions: `Cut` at 0 < p < 1 (threshold?), `Wipe` softness must not leak `to` at p = 0 or `from` at p = 1 (range must be padded by softness), circular wipe radius must cover the corners, `Zoom` start scale, `Slide` direction convention. FV-06 "diferencia 0" at 0 and 1 needs an exact-copy fast path in u8. | 05:93,96; 02:748 | specifier |
| R-c | Buffer rules: u8 premultiplied rounding (round-half-up `(x*a+127)/255`?), unpremultiply at alpha 0, f16 conversion, pixel layout of masks (AlphaMask is 1-channel), who owns the 256x256 tile type: raster ("tiles", 02:97) vs doc `Mask::Painted(TileSetRef)` (02:265). Colour transforms on premultiplied data (D-11 `apply_f32` is layout-only). | 02:97, crates/gafira-color/src/transform.rs | specifier |
| R-d | Slider maths: FD-33 gives ranges (-100..100, 0-100, tone -180..180) but no formula for Brillo, Contraste (S-curve), Luces, Sombras, Blancos, Negros, Vitalidad, Claridad, Viñeta, Temperatura/Matiz, auto-adjust (histogram + gray world), detected-colour tolerance (up to 8 samples). Without formulas, goldens only pin the implementer's choice. No Blur row exists in the FD-33 table, shadow parameter ranges are missing, blur "radius" vs sigma mapping is unstated (SVG/CSS convention matters for FD-35(b)). | 04:326-340,360 | specifier (propose formulas as PROPUESTA, cite W3C/SVG definitions); look approval folded into O12 |
| R-e | Blend and compositing formulas: W3C Compositing and Blending Level 1 (separable + non-separable, premultiplied). 04:142 puts blend in raster, 02:362 says "composición premultiplicada de gafira-raster", but 02:113 puts the f32 compositor in gafira-render-cpu and the raster row (02:97) omits blend. | 04:142, 02:362, 02:113 | specifier |
| R-f | Mip filter and colour space of averaging (gamma vs linear), tile edge handling, `roi` for blur at tile borders, edge policy (clamp/transparent). | 02:943, 02:515 | specifier |
| R-g | FD-34(a) "thumbnail == final within 1/255": true only if the effect is resolution-independent (radius in px scales with size) - needs a `scale` in `EffectCx`. | 04:350 | specifier |
| R-h | Palette algorithm ("5 colores dominantes"), "dE2000 < 15" (PROPUESTA) needs Lab/dE2000; gafira-color has no Lab or dE (grep of crates/gafira-color/src found none, though 02:104 lists Lab). 02:104 says palette samples are in gafira-color, 04:272 says in raster. dHash is PROPUESTA (04:284). | 04:272,284,290 | specifier |
| R-i | 16 blend modes in MVP is PROPUESTA (04:142); `BlendMode::ALL` already has 16 (crates/gafira-doc/src/blend.rs). | | OWNER (O10), recommend confirm |
| R-j | Shadow "Curva" PROPUESTA (04:354). | | OWNER (O10), recommend Fase 2 |

### 2.5 Third parties
Named in 02 §9.2 and usable: rayon (02:1323). NOT named but implied: `half` for f16 (VERIFIED: f16 is unstable on 1.93.1, so buffers need `half` or a hand-written u16 converter; licence MIT/Apache, NOT VERIFIED). tiny-skia is for gafira-render-cpu, not raster.

## 3. gafira-audio

**Row:** 02:98 (MVP: mix graph, `AudioProcessor`, gain, fades, 3-band EQ, loudness levelling EBU R128, resampling; Fase 2: gate, compressor, limiter, noise reducer, time-stretch). Must not open audio devices.

### 3.1 Requirement IDs
Pure-function testable:
- FV-10 (05:127-139): (a) 2 s fade goes 0 to 1 continuously; (b) Nivelar lands within +-0.5 LU of target (-14 default, -16, -23); (d) 1 ms offset applied with 1-sample precision (media/engine glue, the arithmetic is time); targets in 05:136.
- FV-20 (05:310-337): (c) EQ sweep matches theoretical curve +-0.5 dB (100 Hz shelf, 1 kHz bell, 8 kHz shelf, +-15 dB); (d) levelling +-0.5 LU; (e) limiter ceiling +0.1 dB with a 0 dBFS tone (Fase 2). 02 §4.9 steps (02:664-666): impulse/sweep response tests; no allocation in `process`.
- FV-04(b) (05:81): 2.0x speed keeps A/V sync (resampler output length).
- ARQ-23 (02:796) and CAL-15 (01:312, FIJADA): 16 tracks, EQ, < 25 % of one core at 48 kHz (bench, `bench_mix_16`).
UI/E2E only: FV-10(c) Separar audio (media/doc), waveform < 5 s (media), FV-11(c), FV-12(b)(f) (A/V < 40 ms, WAV length +-1 sample: mux/export), FV-09 (assets), monitoring latency < 20 ms (05:333, PROPUESTA; cpal side).

### 3.2 MVP vs later
Contradiction: 02 §4.9 (02:661) and §11.2 (02:1492) put Eq3Band (and Pan in §4.9 only) in the MVP crate, while 01 RF-51 (01:218) and FV-20 (Part B, "Fase 2") put the Mixer, 3-band EQ and 16-track mixer in Fase 2; RF-42 (MVP, 01:204) lists only volume, fades, mute, separate, levelling. So EQ has no MVP consumer. §11.2 lists "ganancia, fundidos, EQ de 3 bandas, nivelación" without Pan; §4.9 lists Pan. FV-20 also lists the loudness meter (momentary, short-term, integrated, LRA, dBTP) in Part B; Nivelar (MVP) needs only integrated loudness (and a peak policy). Fase 2: gate, compressor, limiter, NoiseReducer, TimeStretch (declared latency), meters. OWNER O5.

### 3.3 First slice and order
- A1: `AudioBlock`, `AudioProcessor` + `ProcessorChain`, `Gain` (linear/dB, fader -inf..+12 dB, 05:312), `Fade` (shape: see A-b), sample-rate conversion, BS.1770 K-weighted integrated loudness + `LoudnessNormalizer`. A2: `Eq3Band`, `Pan`, param ring buffer (rtrb), allocation-guard test, `bench_mix_16`. A3 (Fase 2): rest.
- Prerequisites: X2 (`Describe`, `ParamSnapshot`), X5 (alloc guard), gafira-time (done: `SampleRate` accepts only rates dividing 705.6 M flicks, 44.1/48/96 kHz etc., crates/gafira-time/src/rate.rs:141,180).
- Order: independent of the other three. V5 row candidate: `gafira-audio gafira-time` (+ `gafira-doc` for keys, X3). If volume automation consumes `Animatable<f32>`, `gafira-anim` is also needed: not stated for audio in 02 §2.2 (generic "según necesidad"). Recommend audio takes plain sampled values and gafira-media samples the track.

### 3.4 Open items
| # | Item | Evidence | Class |
|---|---|---|---|
| A-a | Loudness: gating (absolute -70 LUFS, relative -10 LU), K-weighting coefficients are specified for 48 kHz only (other rates must be derived), channel weights/layouts (mono, stereo, surround; `channels: u16`), silence (-inf LUFS), the test signal for "+-0.5 LU". 05:134 says "VERIFICAR parámetros exactos". | 05:134,335 | specifier (ITU-R BS.1770-4 + EBU Tech 3341/3342 cases, e.g. -23 dBFS stereo 1 kHz sine gives -23.0 LUFS; synthesise, do not ship the EBU wavs) |
| A-b | Fade curve shape (linear, equal-power, log); FV-10(a) only needs monotone 0 to 1. Transition crossfade is `EqualPower` by default (02:731). | 05:136 | specifier (propose linear for clip fades, equal-power for crossfades) |
| A-c | Pan law (-3 dB constant power vs -6/-4.5 dB), mono vs stereo behaviour. | 02:660 | specifier |
| A-d | EQ: shelf slope and bell Q not given; "curva teórica" (analog prototype vs bilinear-transformed) not defined; at 8 kHz / 44.1 kHz warping matters. | 05:322,336 | specifier (RBJ cookbook, define target curve = digital response of the design) |
| A-e | Resampler: quality (kernel length, stop-band), variable ratio for speed 0.1-10x (05:77), 44.1<->48 exactness, latency reporting, output-length rule for FV-04(b) and "sync" tolerance. | 05:77,81 | specifier (propose numeric stop-band floor) |
| A-f | Nivelar can push samples above 0 dBFS and the limiter is Fase 2. Behaviour (clip, cap gain by peak, refuse) is unspecified; clip volume goes to 200 % (+6.02 dB, 05:131) and fader to +12 dB (05:312). | 05:131,312 | OWNER (O4) |
| A-g | 1 ms offset at 44.1 kHz is 44.1 samples: rounding rule for "precisión de 1 muestra". 1 ms = 705 600 flicks, 1 sample at 44.1 kHz = 16 000 flicks. | 05:139 | specifier |
| A-h | Track count limit (16 "visibles", 05:312) vs unlimited; block size 512 fixed (05:334) vs `max_block`; denormals/NaN in input. | | specifier |
| A-i | Allocation guard (X5). | | OWNER (O1) |
| A-j | Time-stretch "WSOLA/vocoder o crate permisivo (VERIFICAR)" (02:663): Fase 2. | | defer |

### 3.5 Third parties
Named: rtrb (02:1324). Not named: a resampling crate (own implementation or e.g. `rubato`), an FFT crate for NoiseReducer/TimeStretch (Fase 2), an allocation-guard crate (X5). 05:334 says the EBU R128 meter is "propio" (own), so no `ebur128` crate. No deny.toml change expected; NOT VERIFIED.

## 4. gafira-segment

**Row:** 02:99 (MVP; `Segmenter` trait; `ClassicSegmenter` with GrabCut, GMM, max-flow on SLIC superpixels, matting; `onnx` feature Fase 3; must not ship weights). 02 §2.2 (02:136): segment -> raster.

### 4.1 Requirement IDs
Pure/integration testable: FD-38 (04:384-390): (a) IoU >= 0.90 on the testkit reference set, (b) `bench_segment_4000` < 5 s CPU (CAL-06), (c) cancel returns <= 100 ms and leaves the document intact (ARQ-43, CAL-27); reduction to 1024 px long side (02:596). 02 §4.7 step 2 (02:602): IoU >= 0.90 for any new segmenter. FD-32 (04:329-335) uses it for Elemento/Primer plano/Fondo.
UI/E2E/other crates only: FD-38 (d), FD-39 (brush tool and `PaintMask`, not this crate), FD-40 (installer has no `.onnx`, ARQ-76), CAL-07 GPU < 2 s (VERIFICAR, R-04), P-04 (01:404).

### 4.2 MVP vs later
Classic segmenter MVP (RF-22, 01:164). `OnnxSegmenter` Fase 3 (RF-23). GPU guided-filter variant: VERIFICAR (R-04, C-12).

### 4.3 First slice and order
- S1: `Segmenter` trait, `SegReq`, `SegHints`, `AlphaMask`, `SegError`, progress/cancel contract, size reduction to 1024 px, SLIC superpixels (deterministic). S2: GMM + max-flow GrabCut. S3: guided-filter upscale + edge matting. S4: IoU gate and bench.
- Prerequisites: raster R1 (`ImageView`, `AlphaMask` buffer) and X2 (`Progress`, `CancelToken`, `Describe`). The IoU reference set (X6, O3). 
- Order: LAST of the four; cannot start before raster R1. V5 row candidate: `gafira-segment gafira-raster gafira-geom` (+ `gafira-doc` X3). rayon with a pool passed in (02:970).

### 4.4 Open items
| # | Item | Evidence | Class |
|---|---|---|---|
| S-a | IoU >= 0.90: FIJADA in 02:602, PROPUESTA in 04:390 and 01:404. The reference set does not exist, its content, size and licence (ARQ-75) are unspecified, and "IoU" averaged or per image is unstated. | 02:602, 04:390 | OWNER (O3) |
| S-b | Algorithm parameters absent: SLIC superpixel count/compactness, GMM component count (5/5 in the original GrabCut), iterations, guided-filter radius/eps, matting method (02:596 names both a guided filter and "matting de bordes"; 04:384 says guided filter upscales and matting refines), trimap rules. No crate for max-flow, GMM or SLIC appears in 02 §9.2, so they are "propio". | 02:596, 04:384 | specifier, with an explorer spike for feasibility against the 5 s budget |
| S-c | Determinism: GMM init (k-means) needs a seeded PRNG; `rand` is not in 02 §9.2; rayon reduction order affects bytes. | 02:970 | specifier (own xorshift, fixed chunking) |
| S-d | `SegHints` fields ("todo, primer plano, trazo del usuario", 02:592) and `SegReq` fields (CPU/GPU, RAM, max working size) undefined; images <= 1024 px (no upscale?); "4000 px" is long side or area. | 02:590-593, 04:388 | specifier |
| S-e | "Estimación de tiempo" (04:384, PRD) cannot use `Instant::now` (ARQ-02): ETA comes from a cost model (`SegReq`) or from the caller timing the progress fractions. | 04:384, clippy.toml | specifier |
| S-f | Cancel <= 100 ms is not unit-testable by clock (X8): define poll granularity (per superpixel pass / per max-flow augment batch) and test by counting polls. | 04:390 | specifier |
| S-g | FD-32 options "Elemento" and "Texto" (04:327): "Elemento" has no hint model (default hints?), "Texto" (detect raster text regions) is assigned to no crate and no algorithm, and the dimming rule ("sin resultado fiable") is given for the first three only. | 04:327 | OWNER (O6) |
| S-h | Output `AlphaMask` (8-bit? f32?) vs doc `Mask::Alpha(AssetRef)` / tiles (02:265): the conversion is outside this crate. | | specifier |

### 4.5 Third parties
rayon (02:1323) only. `ort` is Fase 3 behind `onnx`, with `load-dynamic` and `std` (02:1344); out of scope now (ARQ-08).

## 5. Layering and 02 §9.2 floors

| Floor | Detail | Where to change |
|---|---|---|
| F1 | Four new V5 rows + four clippy.toml copies (X1). Expected, not new policy. | ci.yml:79-88 |
| F2 | Edges not spelled out in 02 §2.2: raster/audio/segment -> doc (keys, X3), possibly raster -> doc (`AnimatableParams`/`ParamValue`) ; raster -> anim (tracking, Fase 2); testkit -> raster (golden helper, X6); text -> doc (only if doc gets TextNode). | 02:127-139 and ci.yml:79-88 |
| F3 | Shared contracts home (X2) and `AudioCrossfade` (X4): either a doc module (no new row) or a new crate (new V5 row + 02 §2.1 row). | 02:100,420 |
| F4 | Third parties not in 02 §9.2: `half` (f16, VERIFIED unstable), a resampler (or own), a bidi crate (or confirm via parley), FFT crates (Fase 2), allocation-guard crate (X5), `rand`-like PRNG (S-c, avoid). All look licence-compatible with deny.toml but NOT VERIFIED. No deny.toml `[bans]` conflict (the ban list is ffmpeg*, symphonia, imbl, im, rpds, gstreamer, openssl-sys, openh264, x264, mp3lame-encoder, gettext*). | 02:1286-1362 |
| F5 | Unsafe policy (X5). | AGENTS.md:97 vs 02:664 |
| F6 | Order inside layer 2: audio, raster R1, text are mutually independent; segment waits for raster R1. No layer-2 crate needs layout. | 02:136 |

## 6. Specifier-resolvable items worth doing first (no owner)
1. X2/X3/X4 shared contracts and V5 edges (blocks raster, audio, segment).
2. X6 golden harness contract and baseline platform/format.
3. T-a, T-c (line-height base, FitBox bisection contract).
4. A-a (BS.1770 gating and test vectors), A-d, A-e.
5. R-a, R-b (progress and transition edge definitions).
6. S-b..S-f (algorithm parameters, determinism, ETA, cancel by poll count).

## 7. Owner questions, most blocking first

| # | Question | Fork / options | Recommendation (one line) |
|---|---|---|---|
| O1 | Allocation guard (ARQ-23, FV-20(b)) conflicts with `forbid(unsafe_code)` and "no #[allow]" (AGENTS.md:97, testkit lib.rs:8). | (a) allocation-guard crate as dev-dependency (unsafe stays inside the dependency; needs licence check, not in 02 §9.2); (b) one test-only exception to the unsafe policy; (c) drop the runtime guard, keep structural review + bench. | (a): keeps the repo policy intact and keeps ARQ-23 mechanically testable; amend 02:664 (guard is not in the testkit). |
| O2 | Text test fonts and fallback (FD-41(a) "tildes, bidi y emojis"): which OFL fonts go in the repo for fixtures (Inter/Manrope only cover latin, 02:1183), and are colour emoji in the MVP? | (a) Inter+Manrope subsets + one OFL Arabic/Hebrew subset for bidi; emoji only as outline/monochrome via caller-provided fallback bytes, colour emoji Fase 2; (b) colour emoji now (COLR/CBDT/sbix in render). | (a): smallest set that makes bidi and cluster goldens deterministic, with licences registered under ARQ-75. |
| O3 | Segment reference set and gate: IoU >= 0.90 is FIJADA in 02:602 but PROPUESTA in 04:390/01:404; no dataset exists. | (a) gate on procedural synthetic scenes now, real CC0/public-domain photos curated by the owner later; (b) wait for a real set before any gate. | (a), and treat the real-photo IoU as PROPUESTA (warning, not CI break) until the owner supplies a licensed set. |
| O4 | Nivelar when the target would clip (no limiter in MVP; clip volume up to 200 %, 05:131). | (a) cap gain so true peak <= -1 dBTP (the limiter default ceiling, 05:318) and report "target not reached"; (b) allow clipping; (c) refuse. | (a): consistent with 05:318 and never clips silently. |
| O5 | Audio scope: 02 puts Eq3Band/Pan in the MVP crate; 01 RF-51 and FV-20 put mixer and EQ in Fase 2 (no MVP UI consumer). | (a) keep 02 (crate-level MVP, UI later); (b) move EQ/Pan to Fase 2 and edit 02 §2.1, §4.9, §11.2. | (a): CAL-15 is FIJADA and the cost is small; first slice A1 excludes them either way. Fix 02 §11.2 to list Pan. |
| O6 | Zona de edición (FD-32): "Texto" (detect raster text regions) has no crate or algorithm; "Elemento" has no hint model. | (a) MVP ships Todo, Mano alzada, Primer plano, Fondo, Elemento (ClassicSegmenter, centre-prior hints); "Texto" is Fase 2 and shown dimmed with a reason; (b) keep all six in the MVP. | (a). |
| O7 | Manual column break (FD-45, PROPUESTA) yet acceptance (b) tests it; `TextColumns` has no field. | (a) Fase 2, drop acceptance (b) from the MVP; (b) MVP, add a break character to the content model. | (a): needs a content-model decision and has no consumer in RF-26. |
| O8 | `type_lock.size` with FitBox (PROPUESTA, 04:456): lock wins (mode does not change size)? | (a) confirm: lock wins, toggle dimmed with reason; (b) FitBox wins. | (a): protects the lock's promise (FD-47). |
| O9 | Curvar slider -100..100 -> arc sweep (04:442). | (a) +-180 degrees at +-100; (b) +-360. | (a). |
| O10 | 16 blend modes in the MVP (PROPUESTA, 04:142) and shadow "Curva" (PROPUESTA, 04:354). | blend: confirm 16 (enum already has 16 in crates/gafira-doc/src/blend.rs) ; Curva: MVP or Fase 2. | Confirm 16; move "Curva" to Fase 2. |
| O11 | (Folded) approval of the look of the image sliders once the specifier proposes formulas (R-d). | | Ask only after the specifier proposal exists. |

## 8. Stray file
A placeholder was written by mistake to `layer2-gaps.md` in a scratch directory with the wrong name; the sandbox denied `rm`. It is outside the repo and can be ignored.
