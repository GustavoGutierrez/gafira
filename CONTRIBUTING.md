# Contributing to Gafira

Thanks for helping. Gafira is a free and open source graphic design app for Windows, macOS and Linux, written in Rust. Website: <https://gafira.com> (Spanish) and <https://gafira.com/en/> (English).

## Ways to help

| You want to | Do this |
|---|---|
| Report a bug | Open an issue with the version, your system, the steps to reproduce it, what you expected and what happened. A screenshot or the `.gaf` file helps. |
| Suggest a feature | Open an issue that explains the problem it solves before proposing a solution. |
| Improve a translation | Gafira ships in Spanish and English; text lives in the `gafira-i18n` catalogs. Open a pull request with both languages. |
| Fix or build something | Read the rest of this guide, then open a pull request. For anything larger than a small fix, open an issue first so we agree on the approach. |
| Report a security problem | Do not open a public issue. Use GitHub's private vulnerability reporting (Security → Report a vulnerability). |

## Set up

You need Rust (the exact toolchain is pinned in `rust-toolchain.toml`; `rustup` installs it on first build).

```sh
git clone https://github.com/GustavoGutierrez/gafira.git
cd gafira
cargo build --workspace --locked
```

The `README.md` covers running the app, release builds and the project layout. The website lives in `website/` and has its own `README.md`.

## How work is done

The full methodology is in `AGENTS.md`; this is the short version.

1. **Specs first.** Features are described in `specs/` (in Spanish), indexed in `specs/README.md`. A change that adds or alters behaviour starts from a spec or updates one.
2. **Branch from `develop`.** Name it `feat/<spec>-<topic>`, `fix/<topic>`, `docs/<topic>` or `chore/<topic>`. `main` holds released versions only.
3. **Test first for critical logic.** Document model, file formats, edit history, colour, geometry, timing and size limits are written test-first (red, green, refactor). A bug fix in that logic starts with a regression test. Tests name the requirement they prove (`RF-12`, `ARQ-40`…).
4. **Pass the gates** before opening the pull request:

   ```sh
   cargo fmt --check
   cargo clippy --workspace --all-targets --locked -- -D warnings
   cargo test --workspace --locked
   cargo deny check          # when dependencies changed
   python3 tools/sync_agents.py --check
   ```

5. **Open the pull request against `develop`.** Describe what changed and why, link the issue or spec, and include screenshots for anything visible.

## Rules the code follows

- Code, comments and commit messages in English; user-facing text in Spanish and English.
- `#![forbid(unsafe_code)]` in every crate, and no `#[allow]` to silence lints.
- Domain crates use `BTreeMap`/`BTreeSet`, never `HashMap`/`HashSet`; the crate layering is enforced in CI.
- Third-party dependencies must have permissive licenses (the allowlist is in `deny.toml`).
- Gafira runs fully local: no network calls without the user's action, and no external binaries.
- No absolute or personal paths in tracked files.

## Commits

Use [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `docs:`, `chore:`…), one reviewable unit of work per commit.

Sign off every commit to certify the [Developer Certificate of Origin](https://developercertificate.org/), that is, that you wrote the change or have the right to submit it under the project's license:

```sh
git commit -s -m "fix(editor): keep the selection after undo"
```

## License of contributions

Gafira is licensed under the GNU General Public License, version 3 or later (`LICENSE`). By submitting a contribution you agree that it is licensed under the same terms. The Gafira name and logo are not part of that license; see `TRADEMARKS.md`.

## Be kind

Be respectful and assume good intent. Critique ideas and code, not people. Harassment of any kind is not accepted.
