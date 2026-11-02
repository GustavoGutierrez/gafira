## Slice card
Flow: full | level: L3 | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: owner | reason: runs in parallel beside engine-contracts under the owner's parallel full route for layer 2
Product IDs: ARQ-122 (Lab, ΔE2000 and ΔE76 parts only; palette and dHash arrive with `RS6`) (source: specs/02-arquitectura-software.md, row ARQ-122)
Scope: a new `gafira-color` module with the value type `Lab`, conversion from linear sRGB through XYZ D50 with Bradford, `delta_e76` and `delta_e2000` in `f64`, as stated in ARQ-122 and design P6.      Non-goals: a `Color` or `ColorSpaceId` variant, any format change, validation of non-finite input, palette, dHash, reliability (ARQ-131).
Acceptance:
  AC1 Given the 34 pairs of Sharma, Wu and Dalal 2005 table I, When `delta_e2000` runs, Then each matches within ±1e-4 [unit]
  AC2 Given linear sRGB white and black, When converted to Lab, Then they give L 100, a 0, b 0 and 0, 0, 0 within ±1e-3 [unit]
  AC3 Given linear sRGB blue and yellow, When converted to Lab, Then they match the ARQ-122 values within its stated tolerance [unit]
  AC4 Given (50, 0, 0) and (53, 4, 0), When `delta_e76` runs, Then it gives 5 within ±1e-12 [unit]
Plan: design `specs/changes/layer-2/design.md` P6, `crates/gafira-color/src/` only; test first for each AC.
Owner decisions touched: none (checked D-44, which reads the averaging order only, for ARQ-131).

# Apply progress: color-lab
