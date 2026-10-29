# OpenSpec change skeleton

Only for the full OpenSpec derivation (owner request or full mode). The default fast lane uses a Slice card in `apply-progress.md` instead (`SKILL.md` section 4).

Layout: `openspec/changes/<change>/{proposal.md, design.md (optional), specs/<capability>/spec.md}`. `config.yaml` has the per-phase rules.

## proposal.md

```markdown
# Proposal: <Title> (C<n>, <one-line subject>)

## Intent
Why this slice now; which product IDs it implements (ARQ-.., RF-..).

## Change Chain
| Order | Change | Content |
|---|---|---|
| C<n-1> | `<previous>` (merged) | ... |
| C<n> | `<change>` (this) | ... |
| C<n+1> | `<next>` | ... |

## Scope
### In Scope
### Out of Scope

## Capabilities
### New Capabilities
- `<capability>`: ...
### Modified Capabilities
- `<capability>`: ...

## Approach
Strict TDD. New dependencies (permissive licenses only). Amendments to product docs (listed, applied in their own commit).

## Delivery plan
| # | Delivery | Forecast | Depends on |
|---|---|---|---|
| D1 | ... | ~NNN | main |

Each delivery is at most ~700 changed lines. Measured size has run x1.4 to x1.9 over forecast (mostly tests); measure with `git diff --shortstat` and re-cut if above x1.3.

## Risks
| Risk | Likelihood | Mitigation |

## Rollback Plan
Revert the change's commits per delivery, newest first; what to delete.

## Success Criteria
- [ ] Gates pass after every delivery (fmt, clippy, test, deny).
- [ ] <observable outcome per product ID>
```

## specs/<capability>/spec.md

```markdown
# <Capability> Specification

## Purpose
One paragraph; what is out of scope; where related requirements live.

## Requirements

### Requirement: <Name> (<ARQ-nn, RF-nn>) (D1, D2a)
The system MUST ... / SHALL ... / SHOULD ... / MAY ...

#### Scenario: <Name> (D1, unit)
- GIVEN <state>
- WHEN <action>
- THEN <observable result>
```

Delta headers in a change: `## ADDED Requirements`, `## MODIFIED Requirements` (same requirement name; replaces it), `## REMOVED Requirements` (add **Reason** and **Migration**), `## RENAMED Requirements`. Scenario types: unit, property, integration, command, manual.
