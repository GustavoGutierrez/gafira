# CLI Entrypoint Specification

## Purpose

Minimal `gafira-cli` binary (`apps/gafira-cli`) proving the workspace builds an executable without UI (ARQ-07). New capability; no existing spec.

## Requirements

### Requirement: Version flag

`gafira-cli --version` MUST print `gafira-cli <workspace version>` and a newline to stdout, nothing to stderr, and exit 0. The version MUST come from the workspace package version.

#### Scenario: Version printed (integration)
- GIVEN the built binary
- WHEN run with `--version`
- THEN stdout is `gafira-cli ` plus the workspace version, stderr is empty, exit code 0

#### Scenario: Version tracks workspace (integration)
- GIVEN the workspace version in `Cargo.toml`
- WHEN the output is compared
- THEN they are equal

### Requirement: Unknown and missing arguments

Unknown flags or subcommands MUST exit non-zero (2), write a message naming the argument and a usage hint to stderr, and write nothing to stdout. Invocation with no arguments SHALL behave the same (assumption: no default action). `--help` MUST print usage to stdout and exit 0. C1 MUST NOT define `validate` or other C2 commands.

#### Scenario: Unknown flag (integration)
- GIVEN the binary
- WHEN run with `--bogus`
- THEN exit code is 2, stderr mentions `--bogus`, stdout is empty

#### Scenario: No arguments (integration)
- GIVEN the binary
- WHEN run with no arguments
- THEN exit code is 2 and stderr contains usage

#### Scenario: Help (integration)
- GIVEN the binary
- WHEN run with `--help`
- THEN exit code is 0 and stdout contains usage and `--version`

### Requirement: No UI dependencies

The CLI MUST NOT depend on `gafira-ui`, `eframe`, `winit` or `wgpu` in its minimal build.

#### Scenario: Dependency tree (command)
- GIVEN the workspace
- WHEN `cargo tree -p gafira-cli --no-default-features` runs
- THEN none of those crates appear
