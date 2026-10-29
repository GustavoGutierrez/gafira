# 0003. Develop integration branch, PR to main on request

- Status: accepted
- Date: 2026-10-29 (decided by the owner)
- Related: `AGENTS.md` (delivery rules), `.agents/agents/coordinator.md` (route, integration), ADR 0001 and ADR 0002 (flow; neither fixed the integration model), skill `git-workflow-and-versioning`

## Context

Until now integration was trunk-based against `main`: after each delivery's verify PASS the coordinator pushed the branch and opened a PR against `main` (or the previous unmerged delivery branch), and the owner merged every PR in order, bottom-up, retargeting stacked PRs whose base was already merged. One change with several deliveries meant several stacked PRs and several owner merges, and the owner stopped the flow at each one.

## Decision

`main` holds released, stable code; `develop`, created from `main`, is the integration branch. They are the only long-lived branches, local and remote.

- Every change, and every delivery of it, is a short-lived branch cut from `develop` (`feat/<change>-<delivery>`, `docs/...`, `chore/...`).
- After verify PASS the coordinator merges that branch into `develop` locally with `git merge --no-ff`, runs the gates once on `develop` (`cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`; skipped when no Rust or Cargo files changed), pushes `develop`, then deletes the branch locally (and remotely if it was pushed). No PR per delivery or per change. A FAIL never reaches `develop`.
- Deliveries of one change are merged into `develop` one after another, in order. Stacked PRs are gone.
- Normal work does not open PRs: after the merge and push the coordinator keeps working.
- The only PR is `develop` -> `main`, opened only when the owner asks or at a release-worthy point (a release or tag, a completed product document, a milestone the owner names); the coordinator may suggest one in one line at those points and opens it only with the owner's OK. The owner reviews and merges it with a merge commit.
- Releases and tags are cut from `main` after that PR is merged. Then the coordinator syncs: `git switch develop && git pull && git merge --ff-only origin/main` (a plain merge of `origin/main` if fast-forward fails), and pushes `develop`.

## Consequences

- The owner merges only at points they choose instead of once per delivery; the coordinator no longer waits on merges between deliveries or changes.
- The coordinator gains authority to merge change branches into `develop`; the `develop` -> `main` merge stays with the owner.
- The `develop` -> `main` PR can be large; the owner accepts that. The ~700-line delivery signal stays: it bounds review and verify size, not PRs.
- Agent and skill definition changes are reviewed by the owner in the `develop` -> `main` PR, after they are already on `develop`.
- Risk: `develop` runs well ahead of `main` between PRs. Mitigation: the coordinator suggests a PR at release-worthy points, and syncs `develop` after every owner merge.

## Alternatives considered

- Keep trunk-based PRs per delivery against `main`: rejected, one owner merge per delivery and retargeting of stacked PRs slow the flow.
- One PR per change against `develop`: rejected, it keeps an owner stop per change with little added review value over the `develop` -> `main` PR.
- Coordinator merges straight into `main`: rejected, the owner keeps a review point before code reaches the stable branch.
