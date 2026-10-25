# Archive report: domain-time

Date: 2026-10-25. Change: `domain-time` (crate `gafira-time`, layer 0, deliveries D0 to D4). Flow: full, tier T3, verify V3 (round 1 and 2), V1 (round 3 pre-archive check).

## Verdict chain

| Round | Commit | Verdict | Report |
|---|---|---|---|
| 1 | `3b4e58f` | FAIL (2 CRITICAL, 2 WARNING, 4 SUGGESTION) | `verify-report.md`, round 1 |
| 2 | `dbcd799` | PASS (0 CRITICAL, 2 WARNING, 3 SUGGESTION) | `verify-report.md`, round 2 |
| 3 (pre-archive check) | `1c869e0` | PASS (0 CRITICAL, 2 WARNING, 2 SUGGESTION) | `verify-report.md`, pre-archive check |

The independent evidence for the whole crate is in `verification.json` (rounds 1 to 3, including the round 3 mutation record).

## Commits of this archive

| Commit | Content |
|---|---|
| `2edcebd` | docs(domain-time): record round 3 pre-archive check in verify report |
| `17fc5ff` | chore(openspec): merge domain-time delta specs |
| `e7d648f` | docs(domain-time): update design D-10 for the removed guard and canonical text |
| `c190304` | docs(specs): record domain-time as archived in the status table |
| (this commit) | chore(openspec): archive domain-time (folder move and this report) |

Branch `docs/domain-time-delta-fix` on `main` `70b94dc`. Nothing pushed.

## Capabilities merged into `openspec/specs/`

| Capability | Action | Requirements |
|---|---|---|
| `time-model` | new main spec (`cp`, `## ADDED Requirements` renamed to `## Requirements`) | 11 |
| `timecode` | new main spec (`cp`, same rename) | 5 |
| `workspace-tooling` | ADDED requirement "gafira-time domain crate wiring (V5 row, clippy.toml copy)" appended | 11 becomes 12 |

The V5 row (`gafira-time  -`) and the fifth `clippy.toml` copy were already in the main `workspace-tooling` spec from the `edit-history` archive and were not duplicated. No `Owner decisions` or `Closed decisions` record was merged as a requirement; the open-question sections are copied as section text only. No requirement name collides across capabilities (the only duplicate name, `Serde round trip`, belongs to `geometry-primitives` and `color-model` and predates this change).

## Destructive changes

None. There is no REMOVED and no MODIFIED delta. The only edit to an existing main spec is the append above. The `verification.json` round 1 entry changed only its closing brace to add the round 3 entry.

## Owner decisions and ratifications

- C2 waiver (verify round 1, owner waiver): commit `25afa83` (D2) and `f80d182` (D3) fail `cargo test --workspace --locked` on their own (`25afa83` fails for `num = 350, den = 3`; `f80d182` fails on some seeds). History was not rewritten; the final tree is correct, and the fix is `2a57705`. No PR may be cut at `25afa83` or `f80d182`. The change shipped as one PR (#54), so the condition is met.
- C1 ratification (specifier, `dfbf8ae`): the example "1,234 ms" in `time-model` "Seconds and milliseconds are exact" is 870,710,400 flicks (`3d5ace9`). It corrects a wrong example; it is not a behaviour change.
- Q1 = A and Q2 = A (owner, 2026-10-25): canonical timecode text only, and no negative zero. Recorded in `timecode` "Timecode text is canonical" and implemented in `67ed2a6`. `design.md` D-10 now records the rule (`e7d648f`).
- Cross-platform acceptance: Windows and macOS were not observed (GitHub CI is disabled). ARQ-03 rests on Linux evidence only. The same acceptance applies as for `edit-history`. The three-target criterion (Windows, Linux x86_64, macOS 12+ aarch64) is NOT verified. The trybuild criterion is not applicable to this change.

## Mutation (T4 equivalent, HEAD `1c869e0`)

Per-file T3 runs over every source file of `crates/gafira-time/src` (`lib.rs` has no mutants) together form the whole-crate T4 for this change:

| Result | Count |
|---|---|
| Found | 233 |
| Caught | 217 |
| Missed | 0 |
| Timeout | 0 |
| Unviable | 16 |

`timecode.rs` was rerun after the fix round: 143 found, 137 caught, 0 missed, 6 unviable. The pre-fix figure (229 found, 214 caught) is not cited because it includes the earlier `timecode.rs` run.

## Other archive notes

- `design.md` D-10: the stale non-empty check was removed, and the canonical text rule is recorded (`e7d648f`).
- `docs/specs/README.md`: the `domain-time` entry is recorded as archived at `openspec/changes/archive/2026-10-25-domain-time/` (`c190304`).
- `docs/specs/02-arquitectura-software.md` was not edited. Its `gafira-time` row (§2.1 and the crate table) is consistent with the specs. It names the standard frame rates as integer rates; the NTSC rates (23.976, 29.97, 59.94) are also exact but are not listed there. That is incomplete, not false.

## Gates (after the archive, tree at this commit)

| Gate | Result |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (610 passed, 0 failed) |
| `cargo deny check` | 0 (advisories, bans, licenses, sources ok) |
| CI policy ARQ-07, V11, V5, V12, V13, V14 (step bodies from `ci.yml`) | 0 each |

## Open items

- Windows and macOS builds and tests were not observed (see cross-platform acceptance).
- Optional suggestions not done (not archive blockers): a `trybuild` compile-fail test pinning "`Timecode` MUST NOT be serialized" (round 1 S1, round 2 R2-S2); an associativity and commutativity property for checked arithmetic (round 2 R2-S2); a property biased towards zero labels (apply-progress archive note 9).
- Carried waiver: `25afa83` and `f80d182` fail the test gate alone.

next: none
