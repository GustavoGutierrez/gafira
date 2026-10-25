# Changelog

All notable changes to Gafira are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Upcoming: edit history deliveries D2 to D10 (the `edit-history` OpenSpec change).

## [0.1.0-beta.1] - 2026-10-24

### Added

- Cargo workspace and CI, with the layering policy steps (V5, V11, V12, V13) and `cargo deny` license checks.
- `gafira-geom`: geometry primitives.
- `gafira-color`: color model, including `ColorTransform`.
- `gafira-doc`: document model, canonical JSON with validation on load, and `EditToken` / `DocMut` for controlled mutation.
- `gafira-edit`: skeleton with the `Command` contract and `Transaction`, an all-or-nothing composite command.
- `gafira-cli --version`.
- Product specs and the owner decisions D-01 to D-13, including the D-10 audio exception and the D-12 licensing model.
- OpenSpec changes archived: `workspace-scaffold`, `domain-color`, `domain-doc`. The `edit-history` change is in progress (D1 done).

### Notes

- Pre-release: there is no GUI, no installer and no binaries are attached.
- The API is unstable and may change without notice.
- Gafira is proprietary software; see `LICENSE`.
