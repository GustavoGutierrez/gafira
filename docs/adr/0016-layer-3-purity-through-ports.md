# 0016. Layer-3 crates take their I/O, their clock and their threads through ports

- Status: proposed
- Date: 2026-11-03
- Related: 02 §2.1, §2.3, §4.14, §6.3, §6.5; ARQ-01, ARQ-02, ARQ-44; ADR 0005, ADR 0007, ADR 0013; change `layer-3`

## Context

ARQ-02 states that the domain and the engines do no I/O: no `std::fs`, no `std::thread::spawn`, no `std::time::SystemTime::now`, no `std::env`. Layer 3 is "Aplicación", not domain and not engine, so ARQ-02's text does not name it.

The mechanism reaches layer 3 anyway. The V5 step of CI ties a `clippy.toml` copy to every crate in its table, and the V12 step keeps every copy byte-identical to the canonical one in `crates/gafira-geom/`. The canonical copy is the ARQ-02 ban. A layer-3 crate therefore cannot be in the V5 table without carrying the ban, and V5 fails if a crate has a `clippy.toml` and no row.

Layer 3 also has real needs that ARQ-02's wording seems to forbid: 02 §6.3 gives the render work to a dedicated `rayon::ThreadPool` of n−1 cores and says the pool is received by the pure engines rather than taken from the global one; §6.5 needs decoding threads and a background job for the proxies; FV-22 needs the user cache on disk; the frame budget needs a monotonic clock.

`gafira-ports` already settled the precedent. ADR 0013 decision 2 gives the first layer-3 crate a byte-identical `clippy.toml` and no I/O, and says in as many words that ARQ-02 does not bind layer 3. This record extends that decision to the two layer-3 crates that follow.

## Decision

`gafira-render` and `gafira-media` take the copy of `clippy.toml` and are therefore pure in the ARQ-02 sense. They read and write only through `ports::FileSystem`, take time only from `ports::Clock`, receive the render pool as a parameter and never create one, and never call `std::fs`, `std::thread::spawn`, `std::time::Instant::now` or `std::env`.

`gafira-media` is pull-based: the caller owns the decode threads of 02 §6.3 and calls into the crate. `gafira-render` receives the `rayon::ThreadPool` it schedules tiles on.

## Consequences

The two crates are testable without a disk, a clock or a thread, which is what the testkit doubles of ADR 0013 are for. The cost is that a layer-3 crate cannot be self-sufficient: the cache root, the RAM budget and the pool all arrive as parameters, and the composition root has to supply them. That is the same trade layer 2 already made for `gafira-raster` and `gafira-audio`.

The ban also reaches the timing and the counting inside these crates: a bench that needs a clock uses the `Clock` double, and a benchmark of the frame budget measures the budget's arithmetic, not the wall clock. A relaxation for layer 3 would mean a second lint set and a second CI surface, which is why it is not taken.

## Alternatives considered

- No `clippy.toml` copy for the layer-3 crates. Rejected: the V12 step fails for a crate in the V5 table without a copy, and V5 fails for a crate with a copy and no row. The two steps force the issue either way.
- A relaxed copy for layer 3, with the I/O bans lifted. Rejected: two canonical lint sets to keep in sync, and the layer-3 crates would then differ from the rest of the workspace in a way that only the lint set records.
- Let the layer-3 crates own their threads and their file access. Rejected: it contradicts the spirit of ARQ-02 and of the layer-2 precedent, and it would make the crates untestable without a real disk.
- Put the threads in `gafira-ports`. Rejected: a port is a trait, not a runtime.
