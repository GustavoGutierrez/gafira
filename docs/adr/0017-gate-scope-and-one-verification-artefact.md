# 0017. Gates scoped to the crate, and no second verification artefact

- Status: accepted, amended by 0018
- Date: 2026-11-05
- Related: `AGENTS.md` (Strict TDD), ADR 0001 (superseded in part: the `verification.json` artefact and the per-commit `--workspace` gate), ADR 0002, ADR 0003, ADR 0004, change `layer-3b`

## Context

Ten deliveries of the layer-3 and layer-3b cuts were built on 2026-11-04 and 2026-11-05. They closed with real defects found, so the controls are not the problem: the V2 round of `ME1` found a keyframe seek that ignored the cancel token, the first round of `RE1` found two CRITICALs in the content key, and the combined round of layer-3b found a proxy budget that only counted its own session. The cost of the process is a separate question from its value.

Measured on 2026-11-05, warm, on this workspace:

```
cargo test --workspace --locked        148.15 s
cargo test -p gafira-render --locked     2.00 s
cargo test -p gafira-media  --locked     1.25 s
```

`AGENTS.md` required every commit to pass `cargo test --workspace --locked`. A delivery of three to five commits therefore cost 7 to 12 minutes of test time, its eight RED runs cost about twenty more, and six hand mutants — each one applying a wrong behaviour and running a test to see it fail — cost about fifteen. That is around 45 minutes of waiting per delivery, and roughly four hours over the ten deliveries, for changes that in nine cases out of ten touched a single crate.

The same applies to a second artefact. ADR 0001 introduced `verification.json` beside `verify-report.md` "because the coordinator today reads a report of 111 to 1910 lines to find `class:` and `next:`". In practice the coordinator read the report anyway: the verdict, the findings table and the `next:` line sit at the top of it, and the second file had to be kept consistent with the first by hand. It was written for every round of both cuts and read for routing in none of them.

## Decision

**Gates are scoped to what the commit can break.** Every commit still passes alone: `cargo fmt --check`, `cargo clippy -p <crate> --all-targets --locked -- -D warnings` and `cargo test -p <crate> --locked`, with `--workspace` when the commit touches a crate that others depend on, a `Cargo.toml`, a `Cargo.lock` or `ci.yml`. A leaf crate with no dependents — today `gafira-render` and `gafira-media` — only needs its own scope, because an unchanged crate's tests cannot regress. The full workspace suite runs once at the delivery HEAD and once before the `develop` -> `main` PR. The same scope applies to the RED run of each acceptance criterion and to the test that kills each hand mutant.

**One verification artefact.** `verify-report.md` is the only file `verify` writes. `verification.json` is retired; the verdict, the findings table and the `next:` line are read from the report.

**No re-gate after a merge.** The branch already passed its gates at HEAD and a merge adds no code of its own. The coordinator pushes `develop` and keeps working; the workspace suite runs before the PR. The coordinator had also been running the full suite after merges that touched only documentation, which the previous text already exempted.

## Consequences

- The controls that found the defects stay untouched: hand mutants on critical code, one independent verification round, the RED evidence, the Slice card, the per-commit build and lint, the nine CI policy steps and the CI run on the PR. What changes is how much of the workspace each check runs.
- A regression in a leaf crate's own tests is caught by the same scope that proves its change. A regression in a shared crate is still caught by `--workspace`, because the rule keys off the crate's dependents rather than off the size of the diff.
- A crate that gains a dependent must leave the leaf set in `AGENTS.md`. If that is forgotten, its per-commit gate is narrower than it should be; the delivery HEAD and the PR still run the workspace.
- Routing now depends on reading the report. If a report ever buries its `next:` line, the fix is the report's shape, not a second file.
- The measurement is a point in time on one machine. The ratio, not the absolute seconds, is what the decision rests on.

## Alternatives rejected

- **Keep the workspace gate per commit and accept the cost.** It is four hours of waiting across two cuts for checks that, in the nine single-crate deliveries, could not have failed anywhere but in that crate.
- **Drop the hand mutants and the verification round to match the pace of a lighter tool.** Both found defects that the gates and the implementer's own tests did not. Speed bought by removing them would be speed bought with defects.
- **Keep `verification.json` and make the coordinator read it.** It was the stated design of ADR 0001 and it did not happen in ten deliveries; a file nobody reads is a file somebody has to maintain.
