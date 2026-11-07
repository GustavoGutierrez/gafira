# XP2 — the 8-bit preview against the f32 export

**Claim (`specs/changes/gafira-render-cpu/design.md`, "Needs explorer").** The 8-bit preview and the
f32 export agree within 1/255 on 99.9 % of the pixels for scenes with up to three nested
semi-transparent layers. The risk it names: each `blend_u8` rounds to nearest, and the roundings
accumulate layer over layer.

**Verdict: CONTRADICTED.** The 99.9 % is not reached with nested semi-transparent groups. The
tolerance holds only for the shallowest scenes, and even there the margin is thin.

## What was measured

A throwaway project under `/tmp/gafira-xp2/` (path dependencies on `gafira-raster`, `gafira-color`,
`gafira-doc` and `gafira-testkit`; no file of the repo was touched). It composes every scene twice:

- **the preview path**, `gafira_raster::blend_u8` over an sRGB 8-bit premultiplied surface;
- **the export path**, `gafira_raster::blend_f32` over a linear f32 premultiplied surface.

A group with opacity `α` is applied the way `CB2` reads it (design line 469: *the opacity applied
once, not to each child*): the group's buffer is scaled by `α` and composited into its parent with
`blend_u8` / `blend_f32`. The export is converted with the rules of `D9`: unpremultiply (`a = 0` →
`(0, 0, 0, 0)`), encode with the sRGB curve (`LinearRgba::to_srgb`), quantise to 8 bits
(`round(c × 255)`) and premultiply with `(c × a + 127) / 255`.

The comparison is the one `CB3` will use: `gafira_testkit::golden::compare(Mode::CrossPlatform,
preview, encoded_export)`, which allows `pixels / 1000` pixels differing by more than 1/255 on any
channel, that is 99.9 % at ≤ 1/255.

**The grid.** 64 × 64 = 4096 pixels per scene. The page background, the colour of each fill and the
opacity of each group vary from pixel to pixel: the opacity sweeps the 0.5–0.9 range of the family
and the colours come from a deterministic hash, so every scene is the same in every run. The shape on
top is opaque with an anti-aliased edge of eight columns (coverage 255 inside, a gradient at the edge,
0 outside). The run is a **release** build (`cargo run --release`); the rounding is arithmetic and
does not depend on the optimisation, but the number is a release number.

## The scenes and the numbers

Allowed = 4096 / 1000 = 4 pixels. "Out %" is the share of pixels that differ by 2 or more on any
channel, that is, the share that breaks the 99.9 %.

| Scene | Differing pixels | Allowed | Worst channel difference | Out of tolerance | Inside the tolerance |
|---|---:|---:|---:|---:|:---:|
| 1 group, opacity swept 0.5–0.9, no shape on top | 10 | 4 | 2 | **0.244 %** | no |
| 1 group, opacity swept 0.5–0.9, shape on top | 10 | 4 | 2 | **0.244 %** | no |
| 1 group, every opacity at 0.5 | 0 | 4 | 2 | 0.000 % | yes |
| 1 group, every opacity at 0.9 | 10 | 4 | 2 | **0.244 %** | no |
| 2 groups, opacity swept 0.5–0.9, no shape on top | 41 | 4 | 2 | **1.001 %** | no |
| 2 groups, opacity swept 0.5–0.9, shape on top | 41 | 4 | 2 | **1.001 %** | no |
| 2 groups, every opacity at 0.5 | 128 | 4 | 2 | **3.125 %** | no |
| 2 groups, every opacity at 0.9 | 72 | 4 | 2 | **1.758 %** | no |
| 3 groups, opacity swept 0.5–0.9, no shape on top | 138 | 4 | 3 | **3.369 %** | no |
| 3 groups, opacity swept 0.5–0.9, shape on top | 138 | 4 | 3 | **3.369 %** | no |
| 3 groups, every opacity at 0.5 | 527 | 4 | 3 | **12.866 %** | no |
| 3 groups, every opacity at 0.9 | 206 | 4 | 3 | **5.029 %** | no |

The differences are small but they are not rare: most pixels agree exactly or differ by 1, and the
ones that break the tolerance differ by 2 (and by 3 in the three-group scenes). For the worst scene,
the histogram is 2147 pixels at 0, 1422 at 1, 453 at 2, 74 at 3.

**Worst case: three nested groups with every opacity at 0.5, at 12.866 % of the pixels out of
tolerance, with a worst channel difference of 3/255.** The second worst is the same scene with the
opacities at 0.9 (5.029 %).

**The shape on top changes nothing** (the same numbers with and without it): an opaque shape is the
same colour in both paths, and the pixels of its anti-aliased edge stay inside the tolerance. The
difference comes from the nested groups, not from the shape.

## Why the difference is what it is

The preview path quantises the *premultiplied* value of every group buffer, in sRGB, before the
unpremultiply that `blend_u8` does. One level of 8 bits on a premultiplied value is lost and comes
back amplified: unpremultiplying divides the error by `α`, and the sRGB curve flattens the result
near the black end. A single level of rounding therefore ends up as 2 to 3 levels of the final 8-bit
sRGB value, and every nested group adds one more. The f32 path has none of that.

The measurements show the two effects separately: the error grows with the depth (0.244 % → 1.001 % →
3.369 % with the opacities swept), and within a depth the more transparent the groups are, the worse
it gets at two and three levels (12.866 % at α = 0.5 against 5.029 % at α = 0.9 with three groups),
because the unpremultiply divides by `α`.

## What this means for `CB3` and `D3`

`CB3` cannot promise ARQ-42's 99.9 % with the 8-bit preview path as `D3` describes it. The golden it
builds would fail on any scene with two nested semi-transparent groups, and the failure would be
real, not a bug of the test.

The options, in the order I would take them to the owner:

1. **The preview composites in f32 and quantises once, at the end.** The layer buffers of the preview
   path become f32 (the final surface stays sRGB 8-bit, which is what the port hands back). The two
   paths then agree by construction — the preview's surface is the export's `D9` conversion — and the
   tolerance is met with room. The cost is memory: the layer pool is `depth × 1 MiB` instead of
   `depth × 256 KiB`, a budget the design already carries for the f32 path. The CPU cost does not
   grow: `blend_u8` decodes and encodes the sRGB curve on every pixel, `blend_f32` does not, so the
   f32 buffers are the cheaper ones. The price is that the golden stops measuring the quantisation
   and keeps measuring the rasteriser.
2. **Keep the 8-bit preview path and change the acceptance.** ARQ-42 could ask for 99 % at ≤ 1/255
   with three nested groups, or 99.9 % at ≤ 2/255. It is a change of the requirement, and it is the
   owner's, not the coordinator's. The measured numbers above are what such a change would have to
   be written against.
3. **Move the comparison to another space** (`SP6` asks exactly this: in which space is the tolerance
   measured?). In linear light the same differences are far below 1/255, because the steep part of
   the sRGB curve near black is what magnifies them. This is also a change of the requirement.
4. **Reduce the precision loss in the preview path**: keeping the group's alpha outside the
   premultiplied value, or keeping a straight-alpha buffer, would remove the unpremultiply
   amplification. Both are larger changes to the layer stack than (1) and neither is needed if (1)
   is taken.

## Reproducing it

```
cd /tmp/gafira-xp2
CARGO_TARGET_DIR=/tmp/gafira-xp2-target cargo run --release --offline
```

The project is disposable; it was not committed and nothing in the repo depends on it.

## Notes

- The measurement is deterministic: the same grid, the same hash, the same numbers in every run.
- The comparator used is the one of `CB3`, not a bespoke loop: `gafira_testkit::golden::compare` in
  the `CrossPlatform` mode.
- `XP2` was measured on the *compositing* primitives only. The rasteriser (tiny-skia) is not in the
  loop: the coverage is 8-bit in both qualities by `D3`, so it does not add a difference of its own.
  The numbers above are about the quantisation, which is what the claim names.
