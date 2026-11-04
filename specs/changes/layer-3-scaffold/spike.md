# Spike: layer-3 scaffold (XP8, XP9, XP10)

Date: 2026-11-04. Branch `feat/layer-3-scaffold` at `17a93bc` (SC1 committed).
Tools: cargo 1.93.1, cargo-deny 0.20.2, cargo-mutants 27.1.0, rustc 1.93.1, rustup
(1.93.1 toolchain), gcc (host). Host: Linux x86_64, 20 cores.

Every experiment ran outside the repo tree: the workspace was copied to
`/tmp/gafira-xp` (patched copies) and `/tmp/gafira-sc1` (pristine SC1), and a
throwaway crate for the hash benchmark lives in `/tmp/xp8-bench`. The repo tree
was not modified (only this file was written).

## Summary

| Claim | Verdict |
|---|---|
| XP8 | **Partly confirmed, partly not verified.** `cargo deny` on the three targets: confirmed. The three-target list of `deny.toml` demonstrably drives the graph: confirmed. `blake3` and `kurbo` as direct dependencies of `gafira-render`: confirmed. The blake3 feature mechanism (default assembles SIMD with `cc`, `pure` drops it): confirmed, with a measured ~30 % throughput cost. The **native Windows and macOS compile: not verified** (this machine has no MSVC toolchain and no Apple clang/SDK). The **`workflow_dispatch` on the `SC1` branch: not verified** (the branch exists only locally and pushing was out of scope). |
| XP9 | **Confirmed**, measured by proxy, with two coordination findings. The `ports` feature does not make the mutants recompile `gafira-ports`: `gafira-ports` is compiled once per `cargo-mutants` run (in the baseline build) and never per mutant. Caveats: `gafira-render` has 0 mutants today, so the literal run the claim asks for is impossible, and the premise (the dev-dependency) is not in the tree yet. |
| XP10 | **Confirmed.** The impossibility holds: the repo has no codec, no encoder and no muxer, only panicking test doubles. |

Counts over the three claims: confirmed 2 (XP9, XP10), refuted 0, not verified 1
(XP8, with two unverified sub-items: the native Windows/macOS compile and the CI
dispatch on the `SC1` branch). Nothing is refuted; no requirement or design
decision is broken by these measurements.

---

## XP8 — `cargo deny`, the three targets, the compile, `blake3` and `kurbo`

Claim: the two new crates pass `cargo deny` on the three targets and compile on
all three, with `blake3` as a new direct dependency and `kurbo` as another. The
feature set of `blake3` is open: the default assembles the SIMD kernels with
`cc` on x86_64, and `pure` drops that at a speed cost.

### XP8.1 `cargo deny check` on the three targets — CONFIRMED

Command (repo root, the CI command):

```
cargo deny check
```

Output (abridged):

```
warning[duplicate]: found 2 duplicate entries for crate 'miniz_oxide'
   ... miniz_oxide 0.8.9 registry+... (png v0.18.1 -> gafira-testkit ...)
   ... miniz_oxide 0.9.1 registry+... (flate2 v1.1.10 -> png v0.18.1 -> gafira-testkit ...)
advisories ok, bans ok, licenses ok, sources ok
```

Exit 0. The duplicate warning is pre-existing (`multiple-versions = "warn"` in
`deny.toml`) and unrelated to the two new crates. The inclusion graph of that
warning lists `gafira-render` and `gafira-media`, so the new crates are inside
the checked graph.

Does `cargo deny check` really use the target list of `deny.toml`? Two
measurements, both in `/tmp/gafira-xp` (the same tree):

1. `[graph].targets` is a key cargo-deny knows, and the config is parsed
   strictly. Adding a bogus key under `[graph]` fails the run:

```
error[unexpected-keys]: found 1 unexpected keys, expected: ["targets", "exclude",
"features", "all-features", "no-default-features", "exclude-dev", "exclude-unpublished"]
  --> /tmp/gafira-xp/deny.toml:5:1
```

2. The list changes the graph. With the repo's three targets, `cargo deny list`
   prints `cpufeatures@0.3.1` among the Apache-2.0 crates (66) and the MIT ones
   (70). Reducing `targets` to `["aarch64-apple-darwin"]` drops it (65 and 69),
   because `cpufeatures` is `blake3`'s x86-only dependency:

```
targets = [x86_64-pc-windows-msvc, x86_64-unknown-linux-gnu, aarch64-apple-darwin]
  Apache-2.0 (66): ... blake3@1.8.7 ... cc@1.10.0 ... cpufeatures@0.3.1 ...
targets = [aarch64-apple-darwin]
  Apache-2.0 (65): ... blake3@1.8.7 ... cc@1.10.0 ... (no cpufeatures)
```

The licences of the new subtree are all in the allowlist: `blake3@1.8.7` is
`CC0-1.0`, `Apache-2.0` and `Apache-2.0 WITH LLVM-exception`;
`arrayvec@0.7.8`, `cc@1.10.0`, `cfg-if@1.0.5`, `constant_time_eq@0.4.2` and
`cpufeatures@0.3.1` are Apache-2.0/MIT/CC0-1.0/MIT-0; `kurbo@0.13.1` is
Apache-2.0/MIT.

### XP8.2 Compiling on the three targets — Linux CONFIRMED, Windows and macOS NOT VERIFIED

The pinned toolchain had only the host target:

```
$ rustup target list --installed --toolchain 1.93.1
x86_64-unknown-linux-gnu
```

Both other targets were then installed with
`rustup target add --toolchain 1.93.1 x86_64-pc-windows-msvc aarch64-apple-darwin`,
so the standard library of each target is available; what is missing is the
**native** toolchain of each platform.

In `/tmp/gafira-sc1` (pristine SC1):

```
$ cargo build -p gafira-render -p gafira-media --target x86_64-unknown-linux-gnu --locked
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.13s      exit 0

$ cargo build -p gafira-render -p gafira-media --target x86_64-pc-windows-msvc --locked
  error occurred in cc-rs: failed to find tool "ml64.exe": No such file or directory (os error 2)
  warning: build failed, waiting for other jobs to finish...                 exit 101

$ cargo build -p gafira-render -p gafira-media --target aarch64-apple-darwin --locked
  cargo:warning=cc: error: unrecognized command-line option '-arch'
  cargo:warning=cc: error: unrecognized command-line option '-mmacosx-version-min=11.0'
  error occurred in cc-rs: command did not execute successfully (status code exit status: 1):
    "cc" "-O2" ... "-arch" "arm64" "-mmacosx-version-min=11.0" ... "-c" "c/blake3_neon.c"
  warning: build failed, waiting for other jobs to finish...                 exit 101
```

Both failures come from `blake3`'s build script, which compiles its SIMD/NEON
kernels with `cc` for the **target**: `ml64.exe` is the MSVC assembler and the
host gcc cannot emit Apple arm64 objects. The CI's own step fails the same way
when cross-run here, and it fails on another crate too once `blake3` is out of
the way, so the blocker is the machine and not the two new crates:

```
$ cargo clippy --workspace --all-targets --locked --target x86_64-pc-windows-msvc -- -D warnings
  warning: alloca@0.4.0: GNU compiler is not supported for this target
  error: failed to run custom build command for `alloca v0.4.0`               exit 101
$ cargo clippy --workspace --all-targets --locked --target aarch64-apple-darwin -- -D warnings
  warning: alloca@0.4.0: cc: error: unrecognized command-line option '-arch'
  error: failed to run custom build command for `alloca v0.4.0`               exit 101
```

`alloca` is a dependency of `criterion` (the bench harness), so it enters the
`--all-targets` builds only, and it compiles `alloca.c` with `cc` for every
target: no configuration of `blake3` makes a cross build of the whole workspace
from Linux possible. The library-only build of the two new crates does not pull
`criterion`, which is why it cross-builds with `blake3/pure` (below). On the
native runners (`windows-2022` with MSVC, `macos-14` with Xcode) those tools
exist, which is exactly what layer 2 proved in XP2 with the dispatch of run
`41393718000`.

With `blake3/pure` the two new crates themselves cross-build for both targets:

```
$ cargo build -p gafira-render -p gafira-media --target x86_64-pc-windows-msvc --features blake3/pure --locked
   Finished `dev` profile ... in 3.02s                                        exit 0
$ cargo build -p gafira-render -p gafira-media --target aarch64-apple-darwin --features blake3/pure --locked
   Finished `dev` profile ... in 3.03s                                        exit 0
```

**Verdict for XP8.2: not verified for Windows and macOS.** What is missing is
`ml64.exe`/`cl.exe` (MSVC) and an Apple clang with the macOS SDK; a native
`cargo build`/`cargo clippy` on the CI runners is the only proof that closes
this. The Linux target is confirmed.

### XP8.3 `workflow_dispatch` on the `SC1` branch — NOT VERIFIED

The branch does not exist on the remote:

```
$ git ls-remote --heads origin feat/layer-3-scaffold
(empty)
```

`workflow_dispatch` needs a ref that exists in the repository, and pushing was
outside the scope of this task, so the dispatch on the `SC1` branch could not
be run. The exact blocker and the exact fix:

```
git push -u origin feat/layer-3-scaffold
gh workflow run CI --ref feat/layer-3-scaffold
```

Supporting run, on the only ref available (`develop`, which does **not**
contain the two new crates, so it does not measure the claim):

```
$ gh workflow run CI --ref develop
$ gh run view 41834477166
* develop CI #69 · 41834477166
  Triggered via workflow_dispatch
  ✓ policy in 1m10s
  ✓ test (aarch64-apple-darwin) in 4m16s
  * test (x86_64-unknown-linux-gnu)
  * test (x86_64-pc-windows-msvc)
```

The workflow and the three-target matrix are healthy; the `SC1` branch itself
is still unproven by CI. (`*` marks a job still running when this was written.)

### XP8.4 `blake3` and `kurbo` as direct dependencies — CONFIRMED (with a nuance)

`crates/gafira-render/Cargo.toml` declares `blake3.workspace = true` and
`kurbo.workspace = true`; `gafira-media` declares neither.

```
$ git show ed1dc7d:Cargo.lock | grep -E '^name = "(blake3|kurbo|cc|arrayvec)"'
name = "arrayvec"
name = "cc"
name = "kurbo"                 # already in the tree before SC1
```

So `blake3` is a new package in the workspace (SC1 added it with `arrayvec`,
`cc`, `cfg-if`, `constant_time_eq` and `cpufeatures`), while `kurbo` was
already there through `gafira-geom` and `gafira-ports`: it is new as a
**direct** dependency of `gafira-render`, not as a package of the lockfile.

Observation, not part of the claim: the design's shared-files table names
`arrayref` in the `blake3` tree; `blake3 1.8.7` uses `arrayvec` and has no
`arrayref`.

### XP8.5 The feature set of `blake3` — CONFIRMED (mechanism), cost measured

`blake3 1.8.7`'s `build.rs` chooses the code path from `CARGO_FEATURE_PURE`:
with `pure` it emits the Rust intrinsics and skips AVX-512; without it, it
assembles the SSE4.1/AVX2 and AVX-512 kernels with `cc`. The build script
output in the target directory is the direct evidence:

```
default:  cargo::rustc-cfg=blake3_sse2_ffi
          cargo::rustc-cfg=blake3_sse41_ffi
          cargo::rustc-cfg=blake3_avx2_ffi
          cargo::rustc-cfg=blake3_avx512_ffi
pure:     cargo::rustc-cfg=blake3_sse2_rust
          cargo::rustc-cfg=blake3_sse41_rust
          cargo::rustc-cfg=blake3_avx2_rust
```

Clean build of `gafira-render` in a fresh target directory: 8.26 s (default)
against 8.79 s (pure) — no meaningful build-time cost.

Throughput, `/tmp/xp8-bench` (1 MiB blocks, 64 MiB per run, best of 10 pinned
to one core, `taskset -c 2`):

```
default (SIMD via cc): 4785.91 MiB/s
pure (rust intrinsics): 3353.66 MiB/s      -> pure costs ~30 % of the throughput
```

Two notes for the open decision (design P7): the claim's wording ("on x86_64")
is incomplete, because the default also compiles the NEON C kernel
(`c/blake3_neon.c`) on `aarch64-apple-darwin`, which `pure` drops as well; and
the `pure` feature is also what removes the need for the target's native C
toolchain (the two cross builds above pass with it and fail without it). The
number that XP2 still has to weigh is the cost inside the 2 ms frame budget,
not this micro-benchmark.

---

## XP9 — the `ports` feature does not make the mutants recompile `gafira-ports` — CONFIRMED (by proxy)

Claim: enabling the `ports` feature of `gafira-testkit` in the
dev-dependencies of `gafira-render` and `gafira-media` does not make the
mutants of those crates recompile `gafira-ports` on every mutant. Layer 2
already saw load timeouts with `--jobs 8`.

Two facts about the tree come first:

1. **The premise is not in the tree.** Neither `crates/gafira-render/Cargo.toml`
   nor `crates/gafira-media/Cargo.toml` has a `[dev-dependencies]` section, so
   the `ports` feature is not enabled anywhere yet. The claim describes a state
   that `SC1` did not create.
2. **`gafira-render` has no mutants**, so the literal run the claim asks for
   cannot happen:

```
$ cargo mutants -p gafira-render --jobs 8
Found 0 mutants to test
 WARN No mutants found under the active filters
```

What was measured instead, in `/tmp/gafira-xp` (the workspace with the
hypothetical dev-dependency added):

1. **Shape of the graph.** `gafira-ports` is one package in the graph, reached
   by two edges:

```
$ cargo tree -p gafira-render --edges normal,dev -i gafira-ports
gafira-ports v0.1.0-beta.1
├── gafira-render v0.1.0-beta.1
└── gafira-testkit v0.1.0-beta.1
    [dev-dependencies]
    └── gafira-render v0.1.0-beta.1
```

2. **Cargo level.** After an edit of the crate under test, only that crate is
   rebuilt; `gafira-ports` is not recompiled:

```
$ touch crates/gafira-render/src/lib.rs && cargo build -p gafira-render --tests
   Compiling gafira-render v0.1.0-beta.1
    Finished `dev` profile ... in 0.08s
```

The same for `gafira-media` (0.10 s to 0.08 s).

3. **`cargo mutants` level, with a proxy that has the same shape.**
   `gafira-raster` was used because there `gafira-ports` is reachable *only*
   through the dev-dependency, which is the worst case for the claim. Five
   mutants of `crates/gafira-raster/src/tile.rs`, `--jobs 1`:

| | baseline | per-mutant build | total |
|---|---|---|---|
| `ports` feature ON | 21 s build + 3 s test | 1 s, 0 s, 0 s, 0 s | 5 mutants in 40 s |
| `ports` feature OFF (control) | 22 s build + 3 s test | 0-1 s | 5 mutants in 43 s |

   With `--leak-dirs`, the mutant build directory shows when `gafira-ports` was
   compiled: in the run with the feature it has **one** artifact, written during
   the baseline build, and nothing afterwards; in the control run there is no
   `gafira-ports` artifact at all.

```
ports ON :  2026-11-04 08:08:47  libgafira_ports-04bb898b300f2540.rmeta
            2026-11-04 08:08:47  libgafira_ports-04bb898b300f2540.rlib
            (the 5 mutants ran after this timestamp)
ports OFF:  0 artifacts for gafira_ports
```

   The mechanism, from the debug log of the same run: `cargo mutants` copies
   the whole workspace (221 MB, 833 files) into one temporary directory per
   **run**, builds there with `--profile=mutants`, and each mutant only
   recompiles the mutated crate:

```
DEBUG Copied source tree total_bytes=221451729 total_files=833 temp_dir="/tmp/cargo-mutants-gafira-xp-UuSAce.tmp"
DEBUG run{phase=Build}: quoted_argv=... cargo test --no-run --profile=mutants --verbose --package=gafira-raster@0.1.0-beta.1
      process_status=Success elapsed=903.740211ms
```

4. **A larger sample.** 32 mutants of `crates/gafira-raster/src/view.rs`,
   `--jobs 8`: without the feature 3 min (1 missed, 24 caught, 7 unviable),
   with the feature 3 min (same result). The one sampled mutant showed 1 s of
   build without the feature and 4 s with it, which is contention from eight
   parallel jobs on a loaded machine, not a recompilation: the `--jobs 1` runs
   above show 0-1 s per mutant in both configurations.

**Verdict: confirmed.** The `ports` feature adds `gafira-ports` to the mutant
build graph, where it is compiled once per `cargo-mutants` run (in the baseline
build), and it is never recompiled per mutant. The claim's assertion holds.

Two findings for the coordinator (neither refutes XP9):

- **The dev-dependency changes `Cargo.lock`.** Adding
  `gafira-testkit = { features = ["ports"] }` to `gafira-render` and
  `gafira-media` adds two lines to the lockfile (`"gafira-testkit"` in the
  dependency list of each crate); no new package is needed, since `gafira-ports`,
  `gafira-raster` and `gafira-testkit` were already in it. The design gives
  `Cargo.lock` to `SC1` alone and says that a later dependency is a stop
  ("mark that delivery serial and merge it alone"), so the delivery that adds
  the dev-dependency must be serial and must regenerate the lockfile.
- **The literal run on `gafira-render` stays impossible until `RE1`/`ME1` land**,
  because the crate has no mutable code. The T4 rounds of both crates are the
  first runs that can produce mutants.

---

## XP10 — the proxy generation time cannot be measured here — CONFIRMED

Claim: "≥1× real time of the proxy generation (PROPUESTA) cannot be measured
here: the MJPEG codec is layer 4." The task was to confirm or refute that
impossibility.

The ports exist and are complete as traits, in
`crates/gafira-ports/src/media.rs`: `Container`, `Demuxer`, `Muxer`, `Codec`,
`Decoder`, `Encoder`, `CodecRegistry`, and `CodecId::Mjpeg` (line 113).

The **only** implementations in the repo are the test doubles of
`gafira-testkit` and one test-local fake:

```
crates/gafira-testkit/src/ports/codec.rs:23:impl Codec for FakeCodec
crates/gafira-testkit/src/ports/decoder.rs:9:impl Decoder for FakeDecoder
crates/gafira-testkit/src/ports/encoder.rs:9:impl Encoder for FakeEncoder
crates/gafira-testkit/src/ports/muxer.rs:9:impl Muxer for FakeMuxer
crates/gafira-testkit/src/ports/container.rs:23:impl Container for FakeContainer
crates/gafira-ports/tests/media.rs:27:impl Codec for Fake
```

and the doubles panic:

```rust
fn codec_id(&self) -> CodecId {
    panic!("stub: the codec double is not filled yet")
}
```

No codec, encoder or muxer exists:

- `ls crates/` has no `gafira-codec-free`, no `gafira-codec-os` and no
  `gafira-mux`. `specs/02-arquitectura-software.md:111` puts
  `gafira-codec-free` in **layer 4** and names it as the owner of "MJPEG intra
  para proxies y para AVI (Fase 2)".
- The lockfile has 160 packages and no image or video codec at all: the only
  codec-ish one is `png`. An MJPEG encoder is a JPEG encoder, and there is none
  in the tree.
- `deny.toml` bans `ffmpeg-sidecar`, `ffmpeg-next`, `ffmpeg-sys-next`,
  `symphonia`, `gstreamer`, `openh264`, `x264` and `mp3lame-encoder`, so an
  MJPEG encoder has to be a permissive-licence pure-Rust crate, and no such
  crate is in the tree.
- `specs/05-funcionalidad-video-y-distribucion.md:354` names the technique of
  FV-22 as `gafira-media` (proxies, cache) plus the "`Codec` MJPEG de
  `gafira-codec-free`", and line 355 states the budget "generar a ≥ 1× el tiempo
  real en 1080p en gama media (PROPUESTA)".

**Verdict: confirmed.** The number cannot be measured in this cut. What is
missing, precisely:

1. `gafira-codec-free` with an MJPEG intra encoder implementing
   `ports::Codec` and `ports::Encoder` (layer 4).
2. `gafira-mux` with a container able to hold an MJPEG stream (the MP4/ISO-BMFF
   writer of layer 4; AVI is Fase 2).
3. A source to re-encode: without a decoder the pipeline cannot read the
   original either.
4. The measurement rig: a real 1080p source and mid-range hardware, neither of
   which the repo has.

Who measures it: the layer-4 change that creates `gafira-codec-free` (and
`gafira-mux`). `ME3` of this cut can only exercise the proxy pipeline against
the testkit double, so the coordinator should keep the FV-22 budget marked
`PROPUESTA` and measure it in the layer-4 change, where the codec exists.

---

## Next

No claim is refuted, so no spec fix is required. The open item is XP8's
unverified half, which needs the coordinator (it owns the push):

1. Push the branch and dispatch CI, then paste the run id into
   `specs/changes/layer-3-scaffold/apply-progress.md` as the AC8 evidence:
   `git push -u origin feat/layer-3-scaffold` and
   `gh workflow run CI --ref feat/layer-3-scaffold`.
2. Record the XP8 feature-set evidence in the design's P7 (`specs/changes/layer-3/design.md:196`):
   the default assembles the SIMD kernels with `cc` on x86_64 **and** the NEON
   kernel with `cc` on aarch64, `pure` drops both, the build-time cost is
   negligible (8.26 s against 8.79 s) and the measured throughput cost on this
   host is about 30 % (4786 against 3354 MiB/s). XP2 still owes the number
   inside the 2 ms budget.
3. Record the two XP9 findings before the delivery that adds the testkit
   dev-dependency: it touches `Cargo.lock` (two lines), so it must be serial,
   and the T4 rounds of `gafira-render` and `gafira-media` are the first runs
   that can produce mutants.
4. Keep the FV-22 budget marked `PROPUESTA`; the number belongs to the layer-4
   change that owns the MJPEG codec.

`next`: tdd-apply (SC1 continues), with the XP8 CI dispatch and the two
coordination items above as the coordinator's to close. No spec change is
needed.
