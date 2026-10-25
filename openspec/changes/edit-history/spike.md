# Spike: edit-history (C3), risky claims R1 to R15

Date 2026-10-24. Host Linux x86_64, cargo 1.93.1 (pinned by `rust-toolchain.toml`), cargo-deny 0.20.2, cargo-mutants 27.1.0, git 2.34.1, GNU awk 5.1.0 and mawk.
Scratch repo: a copy of `main` (f198790) plus a `gafira-edit` crate, a `gafira-testkit` crate (generator moved with `git mv`) and probe tests, under the session scratchpad `c3-spike/repo` (outside this repo). The repo's `clippy.toml`, `deny.toml`, `rust-toolchain.toml`, `.cargo/mutants.toml`, workspace lints (`unsafe_code = "deny"`) and CI step bodies (extracted from `.github/workflows/ci.yml`) were used unchanged. Network for the first fetch, then `CARGO_NET_OFFLINE=true`.

## Summary

| # | Verdict |
|---|---|
| R1 | CONFIRMED |
| R2 | CONFIRMED (tree differs from the proposal's list; the "Lock delta" scenario is REFUTED, see R2) |
| R3 | NOT VERIFIABLE LOCALLY (Linux part done) |
| R4 | CONFIRMED (one extra known limit) |
| R5 | CONFIRMED |
| R6 | CONFIRMED |
| R7 | CONFIRMED |
| R8 | CONFIRMED (plus a D2 finding: `mod guards` needs `#[cfg(test)]`) |
| R9 | CONFIRMED (estimate is a lower bound, 1.2x to 2.5x below real heap) |
| R10 | CONFIRMED |
| R11 | REFUTED in part (upcast and downcast work; "return it unchanged otherwise" does not work by value) |
| R12 | CONFIRMED |
| R13 | REFUTED in part (rows pass and gafira edges are constrained; V5 does not constrain `trybuild`/`proptest`) |
| R14 | CONFIRMED on Linux (Windows and macOS timing not measured) |
| R15 | CONFIRMED for the values; one label inconsistency in a backlog document |

Counts: 11 CONFIRMED, 2 REFUTED (in part), 1 NOT VERIFIABLE LOCALLY, plus 1 refuted spec scenario found under R2 and 2 extra findings (R8, R4).

---

## R1. trybuild builds and runs on 1.93.1 inside `cargo test --workspace --locked`, offline

`trybuild` release choice: crates.io API lists 1.0.122 (2026-10-07, 17 days old, rust_version 1.88) and 1.0.121 (2026-09-08). Used `trybuild = "1"` resolving to 1.0.122.

```
$ cargo test -p gafira-edit          # first run: network, creates the .stderr snapshot
  test ui ... FAILED   "successfully created new stderr files for 1 test cases"
$ CARGO_NET_OFFLINE=true cargo test --workspace --locked      # after Cargo.lock was updated and the snapshot moved to tests/ui/
  ... all crates ok
$ CARGO_NET_OFFLINE=true cargo test --locked -p gafira-edit --test ui -- --nocapture
test tests/ui/fail_edit_no_token.rs [should fail to compile] ... ok
test tests/ui/pass_ctor.rs [should pass] ... ok
test result: ok. 1 passed
```

trybuild 1.0.122 source: `cargo.rs:49` passes `--offline` to its nested cargo; `cargo.rs:80-82` copies the workspace `Cargo.lock` into its project dir. Both match the claim.

Operational notes for D5 (not refutations):
- A new fixture makes the first run FAIL and writes the snapshot to `crates/gafira-edit/wip/<name>.stderr` (the dir has its own `.gitignore` with `*`); `TRYBUILD=overwrite` writes into `tests/ui/` directly. The author moves/overwrites once, then commits `.stderr`.
- The `.stderr` for `doc.edit()` contains `--> $WORKSPACE/crates/gafira-doc/src/edit.rs` and a copy of the source line `pub fn edit(&mut self, token: &EditToken) -> DocMut<'_> {`. A later edit of that signature in `gafira-doc` makes the snapshot stale. Keep fixtures to errors whose notes do not quote other crates, or accept re-blessing.

**CONFIRMED.**

## R2. trybuild's tree passes the current `deny.toml` unchanged

```
$ cargo deny check            ->  advisories ok, bans ok, licenses ok, sources ok
$ cargo deny check licenses bans sources  ->  bans ok, licenses ok, sources ok
$ cargo tree -d --target all  ->  nothing to print   (no multiple-versions warning)
$ cargo tree -p trybuild --target all --prefix none -f '{p}' | awk '{print $1}' | sort -u
glob itoa memchr proc-macro2 quote serde serde_core serde_derive serde_json serde_spanned syn
target-tuple termcolor toml toml_datetime toml_parser toml_writer trybuild unicode-ident
winapi-util windows-link windows-sys winnow zmij
```

Licenses seen: MIT OR Apache-2.0 for most; `memchr`, `termcolor` Unlicense OR MIT; `winnow`, `zmij` MIT; `unicode-ident` (MIT OR Apache-2.0) AND Unicode-3.0. No allowlist change needed.

Corrections to the proposal's expected list (`proposal.md:102`): the crate is `target-tuple` (not `target-triple`); the tree also has `itoa`, `memchr`, `zmij`, `serde_spanned`, `toml_datetime`, `toml_parser`, `toml_writer`, `syn` (v3), `winapi-util`, `windows-link` (`windows-sys` is only in the `--target all` tree).

### Refuted spec scenario found here: "Lock delta is bounded"

```
$ diff <(names of Cargo.lock at main) <(names after adding gafira-edit + trybuild)
> equivalent  gafira-edit  glob  hashbrown  indexmap  serde_spanned  target-tuple  termcolor  toml
> toml_datetime  toml_parser  toml_writer  trybuild  winapi-util  windows-link  windows-sys  winnow
```

`equivalent`, `hashbrown` and `indexmap` enter the lock (through `toml`'s optional `indexmap`, which `cargo metadata` resolves) but are NOT in `cargo tree -p trybuild` (also with `-e all --target all`). They pass `cargo deny` (MIT OR Apache-2.0).
- Spec to fix: `specs/workspace-tooling/spec.md:208` ("each is `trybuild` or a package of `cargo tree -p trybuild`"). Suggested wording: "each is a `registry` package (no git source) that appears in `cargo metadata` as a transitive dependency of `trybuild`, and `cargo deny check` passes". Compare against `cargo metadata --format-version 1` resolve nodes, not `cargo tree`.

**CONFIRMED for the claim; spec scenario at `workspace-tooling/spec.md:208` REFUTED as written (fix above).**

## R3. `.stderr` identical on Windows, Linux, macOS

Cannot run Windows/macOS here. What was checked on Linux:
- Snapshot generated and replayed on Linux 1.93.1: stable (R1).
- trybuild 1.0.122 `src/normalize.rs`: line 85 `output.replace("\r\n", "\n")` (CRLF), and path normalization to `$DIR/`, `$WORKSPACE`, `$CRATE` (lines 187, 249, 263, 463 to 468). Backslash paths are normalized to those markers by design.
- The recorded error shape depends on the pinned rustc (`rust-toolchain.toml` = 1.93.1), which is the same on the three CI hosts.

**NOT VERIFIABLE LOCALLY.** Needs a CI throwaway branch: commit D5's fixtures with Linux-generated `.stderr` and look at the `test (x86_64-pc-windows-msvc)` and `test (aarch64-apple-darwin)` jobs. Until then the Linux-only fallback in the risk table stays.

## R4. Fixtures under `crates/gafira-edit/tests/ui/*.rs` pass V13

V13 body extracted from `.github/workflows/ci.yml` and run from the scratch repo (git repo, fixtures tracked or untracked):

```
clean tree with src/lib.rs (private static TOKEN), tests/ui/{fail_*,pass_*}.rs, tests/*.rs  -> rc=0
fixture "pub use gafira_doc::EditToken;"                  -> rc=1  "gafira-edit must not publicly re-export EditToken"
fixture "pub use gafira_doc::*;"                          -> rc=1
fixture multi-line "pub use gafira_doc::{\n Document,\n EditToken,\n};"  -> rc=1
fixture "pub type T = gafira_doc::EditToken;"             -> rc=1
EditToken in crates/gafira-ui/src/lib.rs                   -> rc=1  "EditToken outside allowed crates"
benign fixture: "use gafira_doc::{Document, EditToken};", "pub use gafira_doc::Document as D;",
                "let x = 2 * 3;" , "// comment mentioning EditToken and a glob *"   -> rc=0
with mawk and gawk (awk step substituted): benign rc=0, multi-line violation rc=1
```

Extra known limit (add to `specs/edit-capability/spec.md:48`, limit (4)): a `;` ends a record, so a fixture comment such as `// pub use gafira_doc::*;` is a FALSE POSITIVE (rc=1, output `...pass_benign.rs: // pub use gafira_doc::*`). Fixtures must not contain the text `pub use ...*;` or `pub use ... EditToken ...;` even in comments.

**CONFIRMED.**

## R5. The hidden constructor compiles from a trybuild fixture

```
// tests/ui/pass_ctor.rs
use gafira_doc::{Document, EditToken};
fn main() { let token = EditToken::__new_for_gafira_edit_only(); let mut doc = Document::default(); let _m = doc.edit(&token); }
test tests/ui/pass_ctor.rs [should pass] ... ok
```

So no fixture may claim that building a token fails (Q13 holds). **CONFIRMED.**

## R6. `cargo mutants -p gafira-edit` works with trybuild tests in the crate

Crate with `trybuild` ui test, the testkit cycle (R7), `touch` and `add` functions:

```
$ cargo mutants -p gafira-edit --list
crates/gafira-edit/src/lib.rs:5:34: replace touch with ()
crates/gafira-edit/src/lib.rs:6:37: replace add -> u32 with 0 / with 1
crates/gafira-edit/src/lib.rs:6:39: replace + with - in add / with * in add
$ CARGO_NET_OFFLINE=true cargo mutants -p gafira-edit --jobs 2 -o <scratchpad>
ok       Unmutated baseline in 9s build + 7s test
MISSED   ...lib.rs:5:34: replace touch with () in 0s build + 2s test
5 mutants tested in 28s: 1 missed, 4 caught
```

The trybuild test runs in the baseline and in every mutant (copied tree, nested build, `$WORKSPACE` normalization holds), about 2 s per mutant after the first build. No exclusion is needed and `#[mutants::skip]` is not used. `--locked` was not passed on the command line (`.cargo/mutants.toml` supplies it). Caveat: every mutant reruns the whole crate test set, including `ui`; a mutant in `gafira-doc` is out of `-p gafira-edit` scope. **CONFIRMED.**

## R7. Dev-dependency cycle `gafira-testkit` <-> `gafira-edit` / `gafira-doc`

`gafira-testkit` (normal deps: geom, color, doc, edit, proptest, serde_json); `gafira-doc` and `gafira-edit` dev-depend on `gafira-testkit`.

```
integration test (tests/) in gafira-edit using gafira_testkit::document() and gafira_edit::touch  -> compiles, ok
integration test: let _p: gafira_edit::Probe = gafira_testkit::make_probe();                     -> ok (one copy)
unit test in src/lib.rs:  let _p: crate::Probe = gafira_testkit::make_probe();
  error[E0308]: mismatched types ... expected `Probe`, found `gafira_edit::Probe`   (two copies)
V5 (-e normal,build) with rows for gafira-edit and gafira-testkit, cycle present            -> rc=0
cargo clippy --workspace --all-targets --locked -- -D warnings                              -> clean
```

The harness may be used only from `tests/`, as the claim says. **CONFIRMED.**

## R8. `git mv` of `crates/gafira-doc/tests/common/mod.rs`

Move to `crates/gafira-testkit/src/lib.rs` with a 4-line header edit, `Cargo.toml` of the new crate, 1 line in each dev-dependency list, and 2 changed lines in `properties.rs`:

```
$ git diff --shortstat <prev>...HEAD -- . ':!Cargo.lock' ':!openspec'
 6 files changed, 26 insertions(+), 4 deletions(-)          # rename detected: ".../tests/common/mod.rs => gafira-testkit/src/lib.rs | 4 ++--"
```

Runtime (Linux, debug profile): `gafira-doc` properties before 6.54 s for 8 tests (the two generator guards ran inside this binary); after 5.59 s for its 6 tests, plus the 2 guards now in `cargo test -p gafira-testkit` at 6.31 s. Same CPU work; wall time of the workspace test run grows by about the guard time (about 6 s) because the guards now run in a separate test binary.

**Finding for D2** (not in the proposal): `mod guards` of the moved file is not `#[cfg(test)]`; in a library it fails the gate:

```
error: constant `DRAWS` is never used --> crates/gafira-testkit/src/lib.rs:732:11   (also draws, value_depth,
payload_in_bounds, tree_depth, note_text, note_payload, note_f64)
```

Fix: put `#[cfg(test)]` on `mod guards` in the same D2 commit (then clippy and tests pass). Add it to `proposal.md:83` (D2 row) and `design.md:140` (Q10 option A).

**CONFIRMED.**

## R9. Q7 estimate against real heap; counting allocator choices

Allocators: `dhat` 0.3.3 (MIT OR Apache-2.0) and `stats_alloc` 0.1.10 (MIT), added as dev-dependencies of the scratch `gafira-edit`.

```
$ cargo deny check   ->  advisories ok, bans ok, licenses ok, sources ok       (both, with trybuild)
$ cargo tree -d      ->  nothing to print
extra lock packages: addr2line adler2 backtrace gimli lazy_static lock_api miniz_oxide mintex object
                     parking_lot parking_lot_core redox_syscall rustc-demangle rustc-hash scopeguard thousands (dhat tree)
```

Unsafe in our code: none. Both register with a plain `#[global_allocator] static ...` (`dhat::Alloc`, `&INSTRUMENTED_SYSTEM`); `grep -c unsafe` on the probe tests = 0, and they compile under `unsafe_code = "deny"` and the domain `clippy.toml` (`cargo clippy --workspace --all-targets --locked -- -D warnings` clean). The libraries use `unsafe` internally (not our code). Use constraints: one `#[test]` per integration-test binary (the allocator is process-wide and tests run on threads); `dhat` needs `Profiler::builder().testing()` (no file written).

Estimate (design Q7 method A: `size_of` of the records, `Arc` handles, string lengths, serialized extension payload length; no map or allocator overhead) against the net live heap after building the document:

| shape | real (stats_alloc) | real (dhat) | estimate | real/est (stats_alloc / dhat) |
|---|---|---|---|---|
| 1 page x 100 nodes | 54 822 | 48 286 | 22 164 | 2.47 / 2.18 |
| 20 pages x 5 nodes | 51 280 | 43 696 | 24 490 | 2.09 / 1.78 |
| 20 x 5, payload 64 B | 52 560 | 44 976 | 25 770 | 2.04 / 1.75 |
| 20 x 5, payload 1 KiB | 71 760 | 64 176 | 44 970 | 1.60 / 1.43 |
| 5 x 20, payload 4 KiB | 177 956 | 170 924 | 145 505 | 1.22 / 1.17 |

The estimate never over-counts; it under-counts real heap by a factor of 1.2 to 2.5 (largest with many small nodes: `BTreeMap` node and malloc overhead). Suggested sentence for `specs/edit-history/spec.md:113`: the estimate is a lower bound within about 2.5x of the real heap (measured 100 nodes / 20 pages), so 512 MiB is a nominal budget, not an RSS limit. A per-entry constant (for example 48 B per map entry) would bring it near 1.3x if the owner wants it tighter.

**CONFIRMED** (factor recorded: at most 2.5).

## R10. Domain `clippy.toml` does not block the plan

Scratch `gafira-edit` with the canonical copy (byte-identical, V12 rc=0 with four copies):

```
tests/clippy_probe.rs:  static TOKEN: EditToken = EditToken::__new_for_gafira_edit_only();
                        const _: () = assert_send_sync::<EditToken | Document | Arc<Document>>();   (const fn, no thread)
                        Duration::from_millis(500) as caller time
$ cargo clippy --workspace --all-targets --locked -- -D warnings   -> Finished, no diagnostics
$ git grep -nE 'static mut|lazy_static!|inventory::' -- crates/gafira-edit   -> no match (rc=1)
controls (must fail):  Instant::now / SystemTime::now / thread::spawn in a test
  error: use of a disallowed method `std::time::Instant::now`   = note: ARQ-02   (same for the other two)
```

`EditToken: Sync` holds (its only field is `()`), the `const fn` constructor works in a `static`. The ARQ-31 search is not a CI step yet; the equivalent grep finds 0 hits. The trybuild test file itself (`TestCases::new()`) is clean under the ban list. **CONFIRMED.**

## R11. Trait upcasting `Box<dyn Command>` -> `Box<dyn Any + Send + Sync>` on 1.93.1

```
trait Command: Any + Send + Sync + Debug { fn merge(&mut self, next: Box<dyn Command>) -> ... }
let any: Box<dyn Any + Send + Sync> = next;  any.downcast::<A>()      -> compiles, same-type merge ok
```

Upcast and `downcast::<T>()` work. But the second half of the claim ("return it unchanged otherwise") fails by value:

```
Err(back) => Err(back),   // back: Box<dyn Any + Send + Sync>
error[E0308]: mismatched types   expected struct `Box<dyn Command>`  found struct `Box<dyn Any + Send + Sync>`
```

After a failed by-value `downcast`, the original `Box<dyn Command>` cannot be recovered. What works (tested, 2 tests ok, clippy clean): probe by reference first, then consume.

```rust
let probe: &dyn Any = &*next;            // reference upcast
if !probe.is::<A>() { return Some(next); }   // (spec: Err(next)), still owned here
let any: Box<dyn Any + Send + Sync> = next;  // box upcast
match any.downcast::<A>() { Ok(a) => { /* merge a */ } Err(_) => unreachable!("probed") }
```

**REFUTED in part.** Fix:
- `specs/edit-commands/spec.md:46-48` scenario "Merge can recover the concrete type": say the test command first checks `(&*next as &dyn Any).is::<Own>()` and returns `Err(next)` on mismatch, and only then upcasts the box and downcasts; a downcast of the box alone cannot give the command back.
- `proposal.md:111` (R11) and `proposal.md:124`: keep "upcast and downcast work", drop "return it unchanged otherwise" from the by-value path.

## R12. A 300-event merged drag keeps one entry and copies only the edited path

Simulation of the design (`work = Arc::clone(&current)`; `Arc::make_mut(&mut work)`; `edit().node_mut(page, id)`; one snapshot at gesture start), 1 page x 100 nodes, 300 events on one node:

```
entries.len() = 1
untouched nodes sharing Arc with the origin: 99 of 99     (Arc::ptr_eq)
strong_count(untouched node) = 2                          (origin page + current page; not growing)
page Arc is NOT shared (the page's BTreeMap of Arcs is rebuilt per event); nodes are not copied
live bytes added after N events (stats_alloc), origin kept: [(1, 4777), (2, 4841), (10, 4841), (100, 4841), (300, 4841)]
```

Memory is flat (about 4.8 KB for the one copied page map and node). Per event the cost is O(nodes in the page) `Arc` clones for the page map, not a document copy. **CONFIRMED** (state the O(page) cost if the spec promises O(touched path)).

## R13. V5 rows for `gafira-edit` and `gafira-testkit`

V5 body extracted from `ci.yml`, rows added:

```
gafira-edit    gafira-geom gafira-color gafira-doc
gafira-testkit gafira-geom gafira-color gafira-doc gafira-edit
with rows                                      -> rc=0
original table, crates/gafira-edit/clippy.toml -> "::error::gafira-edit has clippy.toml but no V5 row", rc=1
row for gafira-edit without gafira-color       -> "::error::gafira-edit must not depend on gafira-color", rc=1
gafira-edit -> gafira-testkit as normal dep    -> cargo "cyclic package dependency" (V5 aborts, rc=101)
cargo tree -p gafira-edit -e normal,build --target all   -> no trybuild, no proptest (count 0)
gafira-edit -> trybuild as a NORMAL dependency, V5 with rows -> rc=0   (NOT detected)
```

So rows pass, gafira-* edges are constrained, and `trybuild`/`proptest` are absent today, but V5 does not fail when they become normal dependencies (it checks only `gafira-*` names and the forbidden prefix list). The dev-only guarantee rests on the `workspace-tooling` scenario "Dev-only (D5, command)": `cargo tree -e normal,build -i trybuild --workspace` prints `warning: nothing to print.` with rc=0 when clean, and prints the dependent chain (rc=0) when violated, so that scenario MUST judge the output text, not the exit code.

**REFUTED in part** (the "constrain" half). Fix:
- `proposal.md:113` (R13): "V5 constrains only `gafira-*` edges and forbidden prefixes; dev-only `trybuild`/`proptest` is checked by the D5 `cargo tree -i` scenario".
- `specs/workspace-tooling/spec.md:202-203`: say the check is on the command output ("nothing to print" or no dependent listed), not on the exit code. If CI enforcement is wanted, add a step failing when `cargo tree -e normal,build -i trybuild --workspace` lists a package (owner decision).

## R14. `debug_assert!(validate())` and 1 000-case property runs

Probe law (apply; revert; apply on the first node, 3 `validate()` calls, `assert_eq!` against the original) with the testkit generator (up to 3 pages x 12 nodes), debug profile, Linux:

```
1 000 cases with validate  : 0.29 s, 0.28 s, 0.29 s
1 000 cases without validate: 0.22 s x3          (validate adds about 0.07 s per 1 000 cases)
10 000 cases with validate : 2.96 s
```

Eight to ten commands at 1 000 cases stay under 5 s of test time on this host. Windows and macOS times were not measured (CI matrix); the extrapolation is linear. **CONFIRMED on Linux.**

## R15. Cross-document numbers

```
$ git grep -nE '512 ?(MB|MiB)|536 ?870|512 000' -- docs   (and the plain \b512\b search)
docs/specs/02-arquitectura-software.md:767           "por defecto 512 MB, FIJADA"
docs/specs/backlog/01-producto-y-calidad.md:326      CAL-29 "Presupuesto de 512 MB (FIJADA)"
docs/specs/backlog/04-funcionalidad-diseno-imagen-exportacion.md:614   "presupuesto de 512 MB (PROPUESTA)"
$ git grep -nE '500 ?ms' -- docs     -> the merge window is only in 02:769 ("menos de 500 ms"); the other hits are unrelated budgets
ARQ-32 02:846 (100 capas, 1 000 deshacer, p99 < 16 ms) = CAL-28 01:325 (< 16 ms p99, 100 capas) = 04:618 (1 000 deshacer < 16 ms p99)
ARQ-33 02:847 (300 eventos, len() + 1) = 04:128 (arrastre de 300 eventos, una sola entrada)
```

Values agree everywhere (512 with the unit "MB" in the docs, MiB in the change by Q7; 500 ms; 300 events; 100 layers; 1 000 undos; 16 ms). One label inconsistency: `docs/specs/backlog/04-funcionalidad-diseno-imagen-exportacion.md:614` says "(PROPUESTA)" where 02 §5.1 and CAL-29 say FIJADA. It lies outside this change (backlog 04); the holder fixes it when 04 is promoted. D-03 itself (`00-decisiones.md:34`) only says the other figures in 02 become fixed; none of the documents names MiB. **CONFIRMED** (values), label noted.

---

## Fix list

| Claim | File:line | Fix |
|---|---|---|
| R11 | `openspec/changes/edit-history/specs/edit-commands/spec.md:46-48` | The merge scenario must probe with `(&*next as &dyn Any).is::<Own>()` before consuming; a by-value `downcast` failure cannot return the `Box<dyn Command>`. |
| R11 | `openspec/changes/edit-history/proposal.md:111` | Drop "return it unchanged otherwise" for the by-value path (see above). |
| R13 | `openspec/changes/edit-history/proposal.md:113` | V5 does not constrain `trybuild`/`proptest`; the dev-only check is the D5 `cargo tree -i` scenario. |
| R13 | `openspec/changes/edit-history/specs/workspace-tooling/spec.md:202-203` | Judge the command output (rc is 0 in both cases). |
| R2 | `openspec/changes/edit-history/specs/workspace-tooling/spec.md:208` | Lock delta includes `equivalent`, `hashbrown`, `indexmap` (optional dependency of `toml`) that `cargo tree -p trybuild` does not list; compare with `cargo metadata` instead. |
| R2 | `openspec/changes/edit-history/proposal.md:102` | `target-tuple`, not `target-triple`; list the other crates seen. |
| R8 | `openspec/changes/edit-history/proposal.md:83`, `design.md:140` | D2 must add `#[cfg(test)]` to `mod guards` (dead-code errors in a library otherwise). |
| R4 | `openspec/changes/edit-history/specs/edit-capability/spec.md:48` | Add to known limit (4): a comment such as `// pub use x::*;` in a fixture is a false positive. |
| R9 | `openspec/changes/edit-history/specs/edit-history/spec.md:113` | State that the estimate is a lower bound, 1.2x to 2.5x below real heap. |
| R15 | `docs/specs/backlog/04-funcionalidad-diseno-imagen-exportacion.md:614` | "(PROPUESTA)" should read "(FIJADA)" like 02:767 and CAL-29 (outside this change). |
