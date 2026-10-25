# Spike: domain-doc CI-shell and lint claims

Date 2026-10-24, main = 75f7e78. Scratch work under the session scratchpad (`.../scratchpad/v13`, `v12w`, `v5w`, `cl`); the repo was not modified except this file.
Tools: bash (GitHub's `bash --noprofile --norc -eo pipefail`), git 2.34.1, GNU awk 5.1.0 and mawk 1.3.4, cargo/clippy 0.1.93 on toolchain 1.93.1.
The V13 and V12 bodies were extracted verbatim from `design.md:149-172` and `design.md:129-143` and run in throwaway git repos (clean tree = `crates/gafira-doc/src/edit.rs` with `pub struct EditToken {`, an app, a color crate).

## 1. V13 (edit-capability "V13 CI step", E0-E15)

Command shape: `cd <repo> && bash --noprofile --norc -eo pipefail step.sh` (the step starts with its own `set -euo pipefail`, so both invocations are equivalent).

| Scenario | Result | Trimmed output | Verdict |
|---|---|---|---|
| E0 clean | rc=0 | (silent) | CONFIRMED |
| E1 `// EditToken` in apps/gafira-cli | rc=1 | `::error::EditToken outside allowed crates:` `apps/gafira-cli/src/main.rs:2:// EditToken` | CONFIRMED |
| E2 constructor name in gafira-color test | rc=1 | `crates/gafira-color/tests/t.rs:1:...` | CONFIRMED |
| E3 definition renamed `EditTok` | rc=1 | `positive control failed` | CONFIRMED |
| E4a broken pattern `'EditToken('` | rc=1 | `fatal: ... ( desemparejados` `::error::git grep failed (rc=128)` | CONFIRMED outcome; see R1 (rc is 128, not 2) |
| E4b broken `allowed` `'^crates/(gafira-doc'` | rc=1 | `grep: ( ... desemparejados` `::error::filter grep failed (rc=2)` | CONFIRMED |
| E5 run from `crates/` | rc=1 | `positive control failed` | CONFIRMED |
| E5 non-repository copy | rc=1 | `fatal: not a git repository` `git grep failed (rc=128)` | CONFIRMED |
| E6 `pub use gafira_doc::EditToken as Cap;` (untracked) | rc=1 | `gafira-edit must not publicly re-export EditToken:` | CONFIRMED |
| E7 one-line group `{EditToken, other}` | rc=1 | re-export message | CONFIRMED |
| E7 multi-line group (`;` record split) | rc=1 | record printed as `pub use gafira_doc::{` / `EditToken,` / `}` | CONFIRMED |
| E8 `pub type Cap = gafira_doc::EditToken;` | rc=1 | re-export message | CONFIRMED |
| E9 `pub use gafira_doc::*;` | rc=1 | re-export message | CONFIRMED |
| E10 no `crates/gafira-edit/` | rc=0, stderr empty | `git ls-files ... -- crates/gafira-edit \| grep '\.rs$' \|\| true` prints nothing; `-- 'crates/gafira-edit/*.rs'` prints `warning: could not open directory` and exits 0 | CONFIRMED (both halves of the git 2.34.1 claim) |
| E11 alias + `pub use self::T;` | rc=0 | silent | CONFIRMED (limit 1 is really missed) |
| E12 `pub(crate) use ...EditToken;` | rc=0 | silent | CONFIRMED |
| E13 `pub fn lend(&EditToken)` | rc=0 | silent | CONFIRMED (limit 2) |
| E15 `pub use gafira_doc as d;` and `pub use gafira_doc::edit;` | rc=0 both | silent | CONFIRMED (limit 6) |
| E14 mentions in gafira-doc/tests, gafira-testkit, gafira-edit without re-export | rc=0 | silent | CONFIRMED |
| Limit 4: `;` in a comment hides a match (`pub use gafira_doc::{ // a;b` newline `EditToken,` newline `};`) | rc=0 | silent | CONFIRMED (limit 4 is real) |
| Tracked (committed) `gafira-edit/src/lib.rs` with re-export | rc=1 | re-export message | CONFIRMED (`--cached` works) |

mawk 1.3.4 (placed first in PATH): E0 rc=0; E6, E7 multi-line, E8, E9 rc=1; E12 rc=0. Same results as gawk. CONFIRMED.

Extra observations (not spec claims; unlisted limits, no fix required but worth a line in the known-limits list):
- A `gafira-edit` file excluded by `.gitignore` is neither listed by `git ls-files --exclude-standard` nor searched by `git grep --untracked`: re-export in it rc=0.
- `pub(super) use` / `pub (crate) use` are not matched (same as the accepted `pub(crate)` case).
- The glob check is a false positive on any `pub use` record that contains `*` (for example the comment `/* pub use a::b * c */` rc=1). It is inside the "stricter than needed, accepted" clause.

### R1 REFUTED (wording only): a broken pattern gives git rc 128, not 2
`git grep -E 'EditToken('` exits 128 on git 2.34.1 (`fatal: ... unmatched`); only the second grep (`filter grep`) gives rc 2. The step outcome (exit 1, "git grep failed") is right, so no code change.
Fix: `openspec/changes/domain-doc/specs/edit-capability/spec.md:124` says "so that exit 2 (a broken pattern or missing path) or 128 (no repository) fails the step". Change to "so that any exit above 1 (git grep: 128 for a broken pattern or no repository; the filter grep: 2 for a broken pattern) fails the step". Same wording at `design.md:174` ("a broken pattern (rc 2) or a missing repository (rc 128)").

## 2. D1 tool claims

### V12 hardened body (`design.md:129-143`), copies of the real geom clippy.toml plus the two ban lines
| Case | rc | Message | Verdict |
|---|---|---|---|
| clean, 3 copies | 0 | - | CONFIRMED |
| H-N3 ban removed everywhere | 1 | `lacks the std::collections::HashMap ban under disallowed-types` | CONFIRMED |
| H-N5 HashMap line commented, all copies | 1 | same | CONFIRMED |
| H-N6 ban moved to `disallowed-methods` | 1 | same | CONFIRMED |
| H-N2 extra line in doc copy | 1 | `crates/gafira-geom/clippy.toml y crates/gafira-doc/clippy.toml son distintos` | CONFIRMED |
| H-N4 only canonical left | 1 | `expected >= 3 domain clippy.toml copies, found 1` | CONFIRMED |
| canonical file missing | 2 | awk cannot open the file | CONFIRMED (fails, step stops) |
| mawk: clean / H-N5 | 0 / 1 | as above | CONFIRMED |

### V5 against main's `.github/workflows/ci.yml` (step body extracted from ci.yml, run in a copy of the repo with a mock `gafira-doc` crate: geom, color, serde, serde_json)
- Main as is: rc=0, xtrace shows `cargo tree -p gafira-geom` and `-p gafira-color`. CONFIRMED (table-driven body exists on main and matches the spec text).
- Doc crate with `clippy.toml` and no row: rc=1 `gafira-doc has clippy.toml but no V5 row`. CONFIRMED (spec: the row must land with the clippy.toml).
- Row `gafira-doc gafira-geom gafira-color`: rc=0. CONFIRMED. Row shrunk to `gafira-doc gafira-geom`: rc=1 `gafira-doc must not depend on gafira-color`. CONFIRMED (D-N1).
- Prefix pattern `^(egui|eframe|winit|wgpu|tokio|mio|socket2|reqwest|ureq)`: matches `wgpu-core v0.1.0`, `tokio-util v0.7`, `winit-x v1`; does not match `foo-wgpu v1`, `serde v1`. CONFIRMED (N5).
- `cargo tree -p gafira-doc ...` lists gafira-doc, gafira-color, gafira-geom in that order (geom after color). Spec `workspace-tooling/spec.md:88` says "exactly `gafira-doc`, `gafira-geom`, `gafira-color`": true as a set, not as an ordered list. Not a refutation if the check sorts; flag only if the verify test compares order.

### R2 REFUTED (stale wording): "pre-change step matched these names anywhere in a line"
Main's V5 (`ci.yml:98`) is already `grep -E "^($forbidden)"` (since d3e524b). The anywhere-in-line form existed only before `domain-color` (`d64532b`, `ci.yml:78` then). The sentence is inherited from `domain-color/specs/workspace-tooling/spec.md:21`.
Fix: `openspec/changes/domain-doc/specs/workspace-tooling/spec.md:44`: replace "pre-change step, which matched these names anywhere in a line" by "the step on main (`ci.yml` V5), whose pattern is already a prefix match; it MUST NOT be weakened".

### `clippy.toml` ban (clippy 0.1.93, toolchain 1.93.1, `rust-toolchain.toml` copied, crate with geom's `clippy.toml` plus the two entries)
`cargo clippy --all-targets -- -D warnings`:
- lib code `std::collections::HashMap<u8,u8>` and `HashSet<u8>` (full paths): `error: use of a disallowed type std::collections::HashMap` / `HashSet`, note `ARQ-10: unstable iteration order; use BTreeMap` / `BTreeSet`. CONFIRMED.
- `#[cfg(test)] mod tests` inside src: fires (`src/lib.rs:14`). CONFIRMED.
- integration test `tests/it.rs` with `use std::collections::HashSet;` and `HashSet::new()`: fires on the `use` and on both occurrences. CONFIRMED.
- `src/main.rs` bin: fires. CONFIRMED. Without `--all-targets` only lib code fires (tests are skipped), which is why the spec's `--all-targets` wording matters. CONFIRMED.
- `BTreeMap` use: no diagnostic (lib compiles clean). CONFIRMED.
- No "does not refer to an existing" configuration warning appeared. CONFIRMED.
- Not run: the same in the real `gafira-geom`/`gafira-color`/`gafira-doc` crates (doc does not exist); grep of the repo shows no HashMap/HashSet in geom and color per design claim, not re-verified here.

## 3. Delivery cross-check (design.md vs proposal.md vs spec tags)

- Sizes D1..D6 = 595 / 335 / 465 / 690 / 640 / 465 / 645 in `design.md:350-366` and `proposal.md:72-78`: identical. Sum 3 835 correct. Row sums (91+504=595, 335, 466, 689, 638, 463, 646) agree within rounding with the totals. `clippy.toml` 45 + 2 = 47 lines matches "47 new". V12 "+15 -3" matches the new body (15 lines).
- Order D1 <- D2a <- D2b <- D3 <- D4 <- D5 <- D6 and "seven deliveries" are consistent in design (`:342`, `:378`), proposal (`:68`) and the "seven deliveries" statements in `document-model/spec.md:7` and `document-serde/spec.md:7`.
- Requirement tags: model D1 (purity, ids, Dpi, PageSize, BlendMode), D2a (Placement, ElementBase), D2b (id counter, NodeKind, Node/Page, Document, sharing), D4 (`from_parts`), D6 (properties); serde D3/D4/D6; validation D4 (P1-P4, all 18 variants), D5 (P5-P6, rows 15-18), D6; edit-capability D5; workspace-tooling D1 (V5, V12, ban) and D5 (V13). All match the design's file map and commit table. No mismatch found.

## Summary
Confirmed: all V13 scenarios E0-E15 and known limits (also with mawk), V12 negatives, V5 behaviour on main, clippy ban in lib/test/integration/bin targets, delivery numbers. Refuted: 2 (wording only, no step logic change): R1, R2. Not verified: the real `gafira-doc` crate (does not exist yet), CI on ubuntu-24.04 git (only 2.34.1 locally).
Next: spec fix (R1, R2 wording), then tdd-apply D1.
