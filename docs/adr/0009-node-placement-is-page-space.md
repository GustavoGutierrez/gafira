# 0009. Every node's `Placement` is in page space, group children included

- Status: proposed
- Date: 2026-10-30
- Related: FD-05, FD-06, FD-09, FD-11, FD-13, D-28, 02 §3.2, change `layout-core` (spike item 4)

## Context

Layout needs page-space boxes. Nothing states whether a group child's `Placement` is page-space or relative to its group. The code leans one way:

- `Placement` documents `x` and `y` as page-space, for every node (`crates/gafira-doc/src/placement.rs`).
- `Reparent` moves nodes without touching their placement (`crates/gafira-edit/src/reparent.rs`).
- Nothing composes a child transform with its parent (`spike.md` item 4).

`Placement` cannot hold a general affine: `w` and `h` size the box and never scale it. Rewriting a child relative to a rotated or skewed group does not, in general, give a `Placement`.

This is a format decision. After the first saving build, changing it means a migration (ADR 0006).

## Decision

1. Every node's `Placement` is in page space, group children included. No transform is composed through the tree.
2. A group's own `Placement` never moves its children.
3. A group's bounds are the union of its children's bounds, computed. Only an empty group uses its own box.
4. Moving a group moves each descendant. Rotating or resizing a group is a command that rewrites each descendant's `Placement`.

## Consequences

- `gafira-layout::node_bounds` is one line for a leaf (D-28) plus a union for a group. There is no tree transform and no inverse to fail.
- FD-13(a), grouping keeps absolute positions, holds with no extra code, as `Reparent` already does.
- Group commands are O(descendants). 02 §6.7 already accepts this: core commands are O(nodes touched).
- A group's stored box can go stale. Rule 3 makes that harmless: nothing reads it unless the group is empty.
- A group skew has no representation in its children. The Inspector must not offer it on groups.
- Animation is the real cost. Keyframing a group cannot move children by itself. A preset on a group must compile to tracks on each child, or a later group-delta field must be added. That choice waits for `Animatable<Transform2D>`, and this ADR must be revisited before the first saving build if the video design needs group keyframes.

## Alternatives considered

- **Relative to the parent group.** The usual scene-graph model, and it gives animated groups for free. But it needs a general `Transform2D` on nodes instead of `Placement`, a tree walk and matrix inverse in every bounds query, and a conversion in `Reparent` for FD-13(a). Layout would need to handle singular group transforms. All of this for a benefit with no consumer in the MVP slice.
- **Page space, with a group's box kept in sync by every command.** Adds an invariant to every command for a value that the union already gives.
