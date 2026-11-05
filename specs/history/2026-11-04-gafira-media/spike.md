# Spike: layer-3 claims XP1 to XP7 (explorer round)

Branch `docs/layer-3-explore`, cut from `feat/layer-3-media` at `c1ba024`. Date 2026-11-04.
Claims under test: the "Needs explorer" table of `specs/changes/layer-3/design.md:315-325` (XP1 to XP7).
Governing specs: `specs/02-arquitectura-software.md` §6.2, §6.3, §6.5, §6.7 and `specs/05-funcionalidad-video-y-distribucion.md` FV-11, FV-12, FV-22.

Every experiment ran outside the repo, in scratch cargo projects under `/tmp`, with
`CARGO_TARGET_DIR=/tmp/gafira-expl-target` and `CARGO_INCREMENTAL=0`. The crates of the worktree
were used as **path dependencies, unmodified**; the tree of RE1 was exported read-only with
`git archive feat/layer-3-render`. Nothing in the repository was changed except this file.

## Result

| Claim | Verdict |
|---|---|
| XP5 — the prefetch policy keeps the hit rate high enough for `bench_scrub_1080` | **confirmed**, with a scope note (the claim holds while the head is dragged; it does not hold when the head jumps) |
| XP6 — the keyframe scan holds at ≤ 1 s per minute of 1080p material | **confirmed** |
| XP7 — pull-based and single-threaded is enough for the eight decode streams | **confirmed** for the contention; the decode-order policy itself does not exist yet |
| XP2 — the content key of every tile of a 100-layer page fits in the 2 ms of XP1 | **confirmed in part** (the cost of one key and its extrapolation to 100 layers; the per-tile multiplication needs the planner of RE2) |
| XP4 — the byte accounting of the tile cache matches reality | **confirmed in part** (the arithmetic, with the real `ImageBuf`; the tile cache of RE3 does not exist) |
| XP1 — the 02 §6.2 split holds for the graph of this cut | **not verified — blocked by RE2** |
| XP3 — the 256×256 grid and the mip levels keep the visible tile count inside the budget | **not verified — blocked by RE2** |

Counts: **3 confirmed, 2 confirmed in part, 0 refuted, 2 not verified.**

XP8 to XP11 are out of this round: the design assigns XP8 and XP9 to the window of `SC1`, XP10 to
no delivery in this cut and XP11 to `RE1`.

## The material under test

A real 1080p file of the machine, not a synthetic one:

```
/home/meridian/Downloads/Introducción a la arquitectura de microservicios/
  1. Introducción a Microservicios/1 presentación.mp4
```

```
$ ffprobe -v error -select_streams v:0 \
    -show_entries stream=width,height,r_frame_rate,nb_frames,duration -of json "<the file>"
    "width": "1920", "height": "1080", "r_frame_rate": "30/1",
    "duration": "349.366667", "nb_frames": "10481"
```

Its packet table was dumped with `ffprobe -show_packets` (10 481 packets, 67 keyframes, 12 282 569
bytes of payload; the median packet is 660 B and the largest is 76 808 B) and replayed by a demuxer
double that implements `gafira_ports::media::Demuxer`.

The file is a screen recording: a 5.3 s GOP (160 frames) and a low bit rate. Both facts matter and
are stated with every number below.

---

## XP5 — the prefetch policy and the hit rate of the frame cache

**Claim.** "The prefetch policy of 02 §6.5 ('in the direction of the playhead') keeps the frame cache
hit rate high enough for `bench_scrub_1080` (≥ 24 updates/s, cache search under 50 ms). Unmeasured:
the policy is a heuristic and the hit rate depends on the material."

**Method.** A simulation over the real clip against the cache of ME2
(`crates/gafira-media/src/cache.rs`): `lookup`, `prefetch`, `put` and the LRU eviction are the code
under test. The head of the scrub asks for a frame; on a miss the decoder does what 02 §6.5
prescribes (seek to the previous keyframe, decode forward) and stores what it decodes. Three
workloads and three policies:

- workloads: `drag` (the head moves 1 to 8 frames at a time), `jump` (the head jumps to a random
  frame), `mixed` (half and half);
- policies: `no prefetch`, `prefetch 8` and `prefetch 24` — the last two call
  `FrameCache::prefetch` (`crates/gafira-media/src/cache.rs:266`) with the direction of the head.

The budget is the default of the crate, 1 GiB (`crates/gafira-media/src/cache.rs:20`), and a frame
is 1920×1080 `Rgba8`, so the cache holds **129 frames** — less than the 160-frame GOP of this clip.

**Command.**

```
$ /tmp/gafira-expl-target/release/xp5_scrub \
    /tmp/gafira-expl/data/packets_full.json /tmp/gafira-expl/data/stream.json 60 1000
```

**Output** (real, verbatim; `near` is the distance in frames to the nearest cached frame, `lookup
p99` the p99 of the `lookup` call in µs, `empty` the requests the cache answered with nothing):

```
clip: 1920x1080 @ 30 fps, 1800 frames, 12 keyframes (GOP 160 frames), budget Bytes(1073741824)
frame: 1920x1080 = 8294400 bytes (7.91 MiB) -> the budget holds 129 frames
requests per run: 1000

workload   policy             hits hit rate   decodes    dec/req  near p50  near p95  near max lookup p99     empty
drag       no prefetch         808     80.8%     76846      76.85      3.99      8.01      9.00    20.386         1
drag       prefetch 8          932     93.2%     78772      78.77      3.99      8.01      9.00    15.011         1
drag       prefetch 24         906     90.6%     84032      84.03      3.99      8.01      9.00    18.934         1
jump       no prefetch          48      4.8%     80952      80.95    276.00    999.00   1531.98    35.064         1
jump       prefetch 8           50      5.0%     86016      86.02    291.99   1035.99   1566.99    25.261         1
jump       prefetch 24          54      5.4%     94704      94.70    308.01   1078.98   1566.99    21.485         1
mixed      no prefetch         274     27.4%     81629      81.63    105.00    924.00   1386.99    28.387         1
mixed      prefetch 8          400     40.0%     88133      88.13    201.00   1092.00   1551.99    32.548         1
mixed      prefetch 24         407     40.7%     99909      99.91    228.00   1092.00   1551.99    33.393         1
```

The run is deterministic (the same seed gives the same hits, misses, decodes and distances); the
`lookup p99` column is the only noisy one, because it is a wall-clock measurement.

**Verdict: confirmed, with a scope note.**

What the numbers say:

- The policy does what the claim says on the workload the claim is about (dragging the head):
  **80.8 % → 93.2 %** of the requests are an exact hit with `prefetch 8`. The window of 8 frames
  is the best of the three: 24 frames costs more than it returns, because the prefetched frames
  evict the frames of the GOP that the budget cannot hold (129 frames held against a 160-frame GOP).
- The cache search is far inside the 50 ms of CAL-14: the p99 of `lookup` is **15 to 35 µs** over
  the whole simulation, and the isolated cost of the search is in the next section (1.5 µs at the
  129-frame occupancy). The margin is three orders of magnitude.
- The half of the claim that says "≥ 24 updates/s" **cannot be measured in this cut**: there is no
  decoder (the codecs are layer 4; `gafira-codec-free` and `gafira-codec-os` are out of the cut),
  so the program counts the frames a decoder would have to produce and never claims a decode time.
  The number the coordinator needs is therefore the decoder load the hit rate implies: at 30 fps and
  a 93.2 % hit rate, **2.0 decodes per second**; at 24 updates/s of UI, **1.6 decodes/s**.
- The scope note: when the head **jumps** (the user clicks on the timeline), the hit rate is 4.8 %
  with no prefetch and 5.0 % / 5.4 % with it, and the nearest cached frame is **276 frames away at
  the median** (9.2 s at 30 fps). The prefetch does not help there — the jump lands outside any
  window — and the "nearest frame in the cache" that FV-11 shows while the exact frame is decoded
  is then a frame several seconds away. The claim is true for the drag and false for the jump; the
  hit rate depends on the material and on the gesture, which is what the design already warned
  about ("the policy is a heuristic and the hit rate depends on the material").
- `dec/req` is the number of frames the decoder produced per request, under the rule of 02 §6.5
  (seek to the previous keyframe, decode forward). It is high (77 to 100) because the cache cannot
  hold a GOP of this clip at 1080p `Rgba8`. That is a property of the *decoder policy* and of the
  budget, not of the cache's hit rate.

**Sensitivity: the same run with 540p frames** (960×540, 1.98 MiB per frame, so the same 1 GiB
budget holds 518 frames — more than three GOPs):

```
$ /tmp/gafira-expl-target/release/xp5_scrub \
    /tmp/gafira-expl/data/packets_full.json /tmp/gafira-expl/data/stream.json 60 600 960 540

clip: 1920x1080 @ 30 fps, 1800 frames, 12 keyframes (GOP 160 frames), budget Bytes(1073741824)
frame: 960x540 = 2073600 bytes (1.98 MiB) -> the budget holds 518 frames
requests per run: 600

workload   policy             hits hit rate   decodes    dec/req  near p50  near p95  near max lookup p99     empty
drag       no prefetch         561     93.5%     48346      80.58      3.99      8.01      8.01    10.602         1
drag       prefetch 8          599     99.8%     48794      81.32      0.00      0.00      0.00     5.863         1
drag       prefetch 24         599     99.8%     49370      82.28      0.00      0.00      0.00     6.490         1
jump       no prefetch         162     27.0%     47777      79.63     67.98    299.01    759.00   104.278         1
jump       prefetch 8          162     27.0%     50477      84.13     72.00    310.98    759.00   117.010         1
jump       prefetch 24         171     28.5%     56830      94.72     84.00    360.00    759.00    92.035         1
mixed      no prefetch         261     43.5%     50213      83.69     32.01    303.00    681.00    96.732         1
mixed      prefetch 8          330     55.0%     53875      89.79     55.98    340.02    681.00    71.472         1
mixed      prefetch 24         336     56.0%     60479     100.80     57.00    404.01    681.00    85.580         1
```

The size of the frame decides how much the cache holds and therefore how much the hit rate
depends on the material: at 540p the drag reaches **99.8 %** of exact hits with a nearest distance
of **0 frames**, and the jump goes from 4.8 % to 27.0 % of hits with a nearest frame 68 frames away
instead of 276. Nothing in the crate changes; only the bytes per frame.

---

## XP6 — the keyframe scan of a real 1080p file

**Claim.** "The keyframe scan of 02 §6.5 (≤ 1 s per minute of 1080p material) holds with the ports'
`Demuxer`, whose `read_packet` allocates a `Vec<u8>` per packet."

**Method.** `gafira_media::KeyframeIndex::build` (`crates/gafira-media/src/index.rs:116`) runs
against a demuxer double that replays the packet table of the real file, in three variants of the
per-packet reservation: none (`Vec::new()`), the reservation alone (`vec![0u8; size]`), and the
reservation with the write a real demuxer does (`Vec::with_capacity` + `resize`). The budget of the
crate is 1 s per minute (`crates/gafira-media/src/index.rs:20`) and the report of the scan says
whether it was inside.

**Command.**

```
$ /tmp/gafira-expl-target/release/xp6_scan \
    /tmp/gafira-expl/data/packets_full.json /tmp/gafira-expl/data/stream.json 60 4
```

**Output** (real, verbatim; 21 runs each, median shown):

```
clip: 1920x1080 @ 30 fps, 1800 packets, 60 s
clip: 12 keyframes, 1812355 bytes of compressed payload, pool of 4

reservation: None
  packets read      : 1800
  keyframes found   : 12
  bytes reserved    : 0
  scan wall time    : min 60.537µs / median 67.47µs / max 252.589µs (21 runs)
  scan reported     : min 60.075µs / median 66.854µs / max 251.83µs (21 runs, injected clock)
  budget            : 1s -> within budget: true
  per packet        : 0.037 us (median)

reservation: Reserve
  packets read      : 1800
  keyframes found   : 12
  bytes reserved    : 1812355
  scan wall time    : min 141.757µs / median 154.041µs / max 193.495µs (21 runs)
  scan reported     : min 141.59µs / median 153.714µs / max 193.274µs (21 runs, injected clock)
  budget            : 1s -> within budget: true
  per packet        : 0.086 us (median)

reservation: ReserveAndWrite
  packets read      : 1800
  keyframes found   : 12
  bytes reserved    : 1812355
  scan wall time    : min 97.313µs / median 100.731µs / max 151.334µs (21 runs)
  scan reported     : min 97.06µs / median 100.615µs / max 151.176µs (21 runs, injected clock)
  budget            : 1s -> within budget: true
  per packet        : 0.056 us (median)
```

The whole real file (349.4 s, 10 481 packets, 67 keyframes):

```
$ /tmp/gafira-expl-target/release/xp6_scan \
    /tmp/gafira-expl/data/packets_full.json /tmp/gafira-expl/data/stream.json 0 4

reservation: Reserve
  scan wall time    : min 712.189µs / median 1.195712ms / max 1.437097ms (21 runs)
  budget            : 5.822783333s -> within budget: true
  per packet        : 0.114 us (median)

reservation: ReserveAndWrite
  scan wall time    : min 772.988µs / median 859.581µs / max 892.33µs (21 runs)
  budget            : 5.822783333s -> within budget: true
  per packet        : 0.082 us (median)
```

**Verdict: confirmed.**

- One minute of real 1080p material scans in **154 µs** (median, with the per-packet reservation)
  against a budget of 1 s: a margin of about **6 500×**. The whole file (349 s) scans in 1.20 ms
  against a budget of 5.82 s.
- The `Vec<u8>` per packet costs **33 to 87 µs over the 1800 packets** of that minute, that is
  **18 to 48 ns per packet** on this material (86.6 µs for the reservation alone and 33.3 µs for
  the reservation with the write, against 67.5 µs of scan without any reservation), i.e. it is the
  larger half of the scan itself. It is a linear cost in the packet size: this clip's packets are
  small (median 660 B); a file with 100 KB packets would pay roughly 150× more in the memset.
- The pool matters more than the reservation: with four workers the scan pays about **50 µs of
  thread spawn** inside the measured window (67 µs with four workers against 15.6 µs with one).
  A pool of one scans the minute in 15.6 µs without the reservation and 92 µs with it.
- **Scope:** the measurement covers the crate's scan loop and the reservation of the port. It does
  **not** cover the container parsing nor the disk read: no real demuxer exists in this cut (the
  containers are layer 4), so the end-to-end cost of importing a file is not measured here. The
  figure that 02 §6.5 fixes (≤ 1 s per minute) is a product-level figure about the import, and
  only its crate-side half is settled.

---

## XP7 — eight concurrent decoders on one frame cache

**Claim.** "`gafira-media` being pull-based and single-threaded (P3) is enough for the eight decode
streams of 02 §6.3. The thread count belongs to the caller, but the policy of which stream to
decode next lives in this crate."

**Method.** `FrameCache` has no lock of its own: the caller owns the threads, so the prototype
shares one cache behind a `Mutex` and measures the wait for the lock, the time held and the
throughput, for 1, 2, 4 and 8 threads. The decode happens outside the lock (the decoder owns the
frame; the cache only stores it). The frames are 1920×1080 `Rgba8` and the budget is the default
1 GiB. A second part measures the cost of the cache search alone against the occupancy.

**Command.**

```
$ /tmp/gafira-expl-target/release/xp7_contention 2000
```

**Output** (real, verbatim):

```
`FrameCache` is Send + Sync: the caller owns the threads and the lock.

=== contention on the shared cache (1920x1080 frames, budget 1 GiB) ===
1 threads / 8 media | 2000 ops in 3.489398036s | 573 ops/s | hits 0.0% | lock wait: p99 3.495µs max 5.708µs | lock hold: p99 101.905µs max 3.248926ms | wait share 2.7%
2 threads / 8 media | 4000 ops in 3.718214744s | 1076 ops/s | hits 0.0% | lock wait: p99 4.274µs max 2.406441ms | lock hold: p99 101.019µs max 8.410106ms | wait share 11.2%
4 threads / 8 media | 8000 ops in 4.981483214s | 1606 ops/s | hits 0.0% | lock wait: p99 553.172µs max 11.583169ms | lock hold: p99 111.22µs max 11.667331ms | wait share 39.4%
8 threads / 8 media | 16000 ops in 9.05251882s | 1767 ops/s | hits 0.0% | lock wait: p99 4.356417ms max 11.614772ms | lock hold: p99 1.472385ms max 20.540455ms | wait share 69.5%
8 threads / 1 media | 16000 ops in 9.197458664s | 1740 ops/s | hits 7.5% | lock wait: p99 3.384206ms max 15.279195ms | lock hold: p99 1.088534ms max 15.238231ms | wait share 61.2%

=== the cost of the cache search against the occupancy (one medium) ===
     1 frames | miss (nearest walk): p50     0.06 us  p99     0.06 us  max     0.83 us | hit: p50  0.055 us  p99  0.057 us
    16 frames | miss (nearest walk): p50     0.24 us  p99     0.30 us  max     3.22 us | hit: p50  0.115 us  p99  0.160 us
    64 frames | miss (nearest walk): p50     0.74 us  p99     0.79 us  max     3.65 us | hit: p50  0.161 us  p99  0.236 us
   129 frames | miss (nearest walk): p50     1.50 us  p99     2.36 us  max    75.93 us | hit: p50  0.237 us  p99  0.623 us
   512 frames | miss (nearest walk): p50     5.35 us  p99     8.64 us  max    85.31 us | hit: p50  0.285 us  p99  0.940 us
  2048 frames | miss (nearest walk): p50    26.00 us  p99    47.05 us  max   284.68 us | hit: p50  0.441 us  p99  1.447 us
```

**Verdict: confirmed for the contention; the decode-order policy does not exist yet.**

- The eight streams are served. At eight threads the cache does **1 767 operations per second**
  against the 240 frames per second that eight 30 fps streams need: a headroom of about 7×.
- The lock is the scaling limit and it shows: the share of the time spent waiting for it grows
  from 2.7 % (one thread) to 69.5 % (eight threads). The p99 of the wait is 4.4 ms and the worst
  wait is 15.3 ms, inside the 16 ms of a UI frame but not by much.
- The reason is in the shape of the API: `FrameCache::lookup` takes `&mut self`
  (`crates/gafira-media/src/cache.rs:247`), so the *search* — the walk of `nearest`
  (`crates/gafira-media/src/cache.rs:296`), which is O(the frames of that medium) — runs under the
  exclusive lock, even though `nearest` itself only needs `&self`. Eight streams therefore
  serialize their searches. Splitting the read path (`nearest`, `&self`) from the write path
  (`touch`, the LRU order) would let the eight streams search in parallel; that is a change to
  `crates/gafira-media/src/cache.rs:247` and it belongs to ME2 or to ME5, not to this round.
- The search itself is cheap: **1.5 µs** at the occupancy the default budget reaches at 1080p
  (129 frames) and 26 µs at 2 048 frames, both far inside the 50 ms of CAL-14.
- **The second half of the claim is not verified.** XP7 says the policy of which stream to decode
  next lives in this crate; `gafira-media` has no such policy today: `FrameCache` knows only
  `prefetch(head, direction, count)` and the crate exposes no queue of streams, no priority and no
  "next" decision. The prototype can only measure the contention, not the policy. That API is
  still to be written (ME3 for the proxies, ME5 for the exporter).
- P3 itself holds: `grep -rn "thread::spawn\|Instant::now\|SystemTime::now\|std::fs::" crates/gafira-media/src/`
  returns nothing, and the crate's `clippy.toml` bans those paths (ADR 0016).

---

## XP2 (in part) — the cost of the content key

**Claim.** "The content key can be computed for every tile of a 100-layer page inside the 2 ms of
XP1. Unmeasured: BLAKE3 over a canonical description of every node, every frame."

**Method.** The key derivation of RE1 (`gafira_render::content_key`, exported from
`crates/gafira-render/src/key.rs`) is timed over a chain of 100 shape nodes, each one taking the
key of the previous as its input, with the six parameters of a real node (quality, geometry, fill,
transform, opacity, blend mode). The tree of RE1 was exported read-only with
`git archive feat/layer-3-render` into `/tmp/gafira-exp-proj/re-tree` and used as a path
dependency.

**Command.**

```
$ /tmp/gafira-expl-target/release/xp2_key 100 200
```

**Output** (real, verbatim):

```
=== the description of one node ===
canonical description of a shape with 2 inputs and 6 params: 136 bytes

=== the cost of one key (RE1) ===
canonical_description: median 0.290 us  (min 0.283, max 1.843)
content_key (description + BLAKE3): median 0.544 us  (min 0.526, max 5.513)

=== a page of 100 layers, one key each ===
total for 100 keys: median 0.0512 ms  (min 0.0459, max 0.1311)
share of the 2 ms of XP1: 2.561 %
keys per millisecond: 1953 -> the keys of 3905 tiles of a 100-layer page fit in 2 ms
```

**Verdict: confirmed in part.**

- One key costs **0.544 µs** (description 0.29 µs, BLAKE3 the rest) on this machine with
  `blake3 1.8.7` (the version the repo locks).
- The 100 keys of a 100-layer page cost **0.051 ms**, 2.6 % of the 2 ms of XP1. The budget of 2 ms
  therefore holds about **3 900 tile-keys**, which for a 100-layer page means **about 39 tiles per
  layer**.
- **What is not measured:** the claim is about *every tile* of the page, and the number of tiles
  per layer is the planner's (RE2), which does not exist. The extrapolation above is what the
  per-key cost allows; it is not a measurement of the plan. XP2 stays open until the planner lands
  and the tile count of a real page is known.

---

## XP4 (in part) — the byte accounting of a tile

**Claim.** "The tile cache byte accounting matches reality. A 256×256 tile is 256 KiB in `Rgba8` and
1 MiB in `RgbaF32`, and a cached tile may hold mips too."

**Method.** The real `ImageBuf` of `gafira-raster` is built at the tile size of the crate
(`TILE_SIZE = 256`, `crates/gafira-raster/src/tile.rs:4`) and its bytes are counted from the type of
its elements. The same buffers are then put in the only cache that exists today, the frame cache of
ME2, and `bytes()` is read back. The mip chain is built with the real `build_mips` of
`gafira-raster` (`crates/gafira-raster/src/mip.rs:162`).

**Command.**

```
$ /tmp/gafira-expl-target/release/xp4_bytes
```

**Output** (real, verbatim):

```
TILE_SIZE of gafira-raster: 256 px

=== one 256x256 tile ===
Rgba8   : 262144 bytes = 256 KiB (claim: 256 KiB) -> match
RgbaF32 : 1048576 bytes = 1 MiB (claim: 1 MiB) -> match
Alpha8  : 65536 bytes = 64 KiB (one channel)

=== what the frame cache of ME2 counts ===
Rgba8   : 262144 bytes (cache) vs 262144 bytes (buffer) -> match
RgbaF32 : 1048576 bytes (cache) vs 1048576 bytes (buffer) -> match

=== the same tile with its mip levels (build_mips of gafira-raster) ===
mip levels of a 256x256 tile: 9
Rgba8: 9 levels, 349524 bytes = 341 KiB (level 0 alone: 262144 bytes)
   level 0: 256x256 -> 262144 bytes
   level 1: 128x128 -> 65536 bytes
   level 2: 64x64 -> 16384 bytes
   level 3: 32x32 -> 4096 bytes
   level 4: 16x16 -> 1024 bytes
   level 5: 8x8 -> 256 bytes
   level 6: 4x4 -> 64 bytes
   level 7: 2x2 -> 16 bytes
   level 8: 1x1 -> 4 bytes
RgbaF32: 9 levels, 1398096 bytes = 1365 KiB (level 0 alone: 1048576 bytes)
   level 0: 256x256 -> 1048576 bytes
   level 1: 128x128 -> 262144 bytes
   level 2: 64x64 -> 65536 bytes
   level 3: 32x32 -> 16384 bytes
   level 4: 16x16 -> 1024 bytes
   level 5: 8x8 -> 256 bytes
   level 6: 4x4 -> 64 bytes
   level 7: 2x2 -> 16 bytes
   level 8: 1x1 -> 4 bytes
```

**Verdict: confirmed in part.**

- The arithmetic of the claim is right: 256×256 in `Rgba8` is **262 144 bytes (256 KiB)** and in
  `RgbaF32` it is **1 048 576 bytes (1 MiB)**, counted from the real `ImageBuf` and from the type
  of its elements.
- The accounting of the frame cache of ME2 (`frame_size`, `crates/gafira-media/src/cache.rs:356`)
  agrees with the buffer **exactly** in both formats.
- The half of the claim about the mips: a 256×256 tile with its chain is **9 levels and 349 524
  bytes (341 KiB)** in `Rgba8`, that is 1.33× the level 0, and 1 398 096 bytes (1 365 KiB) in
  `RgbaF32`. A cache that holds a tile with its mips must count the whole chain: the level 0
  alone is not the size of the tile.
- **What is not measured:** the tile cache of RE3 does not exist. `crates/gafira-render/src/` holds
  only `lib.rs` in this cut, so the accounting of a cache that holds tiles with mips is not
  verified. What is verified is the arithmetic that cache will have to use, with the real types.

---

## XP1 and XP3 — not verified, blocked by RE2

Neither claim was measured, on purpose: both need the planner of RE2, which does not exist.

```
$ ls -la crates/gafira-render/src/
-rw-rw-r-- 1 meridian meridian  146 nov  4 12:55 lib.rs

$ git ls-tree -r --name-only feat/layer-3-render crates/gafira-render/src/
crates/gafira-render/src/graph.rs
crates/gafira-render/src/key.rs
crates/gafira-render/src/lib.rs
crates/gafira-render/src/pool.rs

$ grep -rn "planner\|Planner\|RenderPlan\|lod\|Lod" crates/gafira-render/src/
crates/gafira-render/src/lib.rs:1://! The render engine of Gafira: the render graph, the planner, the caches and
```

**XP1** (the 02 §6.2 split: ≤ 2 ms of graph and planning, ≤ 8 ms of visible render) needs the
planner to produce the plan whose two stages are timed; the graph of RE1 exists but the planning
stage does not, so the 2 ms cannot be attributed to anything that exists.

**XP3** (the 256×256 grid and the mip levels keep the visible tile count inside the budget at
3200 % of zoom) needs the planner's viewport clipping and LOD rule to know *which* tiles are
visible. Two facts are already fixed outside the planner and are worth recording:
`crates/gafira-raster/src/tile.rs:4` fixes `TILE_SIZE = 256` and `crates/gafira-raster/src/mip.rs:10`
fixes the levels (`⌊log2(max(w, h))⌋ + 1`). What is missing is the count of visible tiles, and that
is the planner's.

Both stay **not verified, blocked by RE2**.

---

## What the coordinator should take from this round

1. **Nothing is refuted.** No claim of XP1 to XP7 broke a requirement of 02 §6.2, §6.3, §6.5 or
   §6.7, and no decision of `specs/changes/layer-3/design.md` has to change.
2. **XP5 needs a scope note in the design.** The policy works while the head is dragged (80.8 % →
   93.2 % of exact hits) and does not work when the head jumps (4.8 % → 5.4 %, with the nearest
   cached frame 276 frames away at the median). The design says "the hit rate depends on the
   material"; it also depends on the gesture and on the size of the frame. The row to amend is
   `specs/changes/layer-3/design.md:319`.
3. **The cache search figure of CAL-14 is safe.** The search costs 1.5 µs at the occupancy the
   default budget reaches at 1080p, against the 50 ms the spec fixes.
4. **The decode-order policy of XP7 does not exist.** `gafira-media` has no "which stream next"
   API today; the claim cannot be closed until ME3 or ME5 writes it.
5. **The lock of the frame cache is the scaling limit of the eight streams.** `lookup` takes
   `&mut self` (`crates/gafira-media/src/cache.rs:247`), so the search runs under the exclusive
   lock; at eight threads the wait share is 69.5 % and the worst wait is 15.3 ms. Splitting the
   read path from the LRU update would let the streams search in parallel. This is a change in
   `crates/gafira-media/src/cache.rs`, in ME2 or ME5.
6. **The scan figure of 02 §6.5 is safe on the crate side** (154 µs for a minute of real 1080p
   against 1 s), but only the crate's half was measured: no real demuxer exists in this cut.
7. **The frame size decides the hit rate.** At 540p the same budget holds 518 frames instead of
   129, the drag reaches 99.8 % of exact hits and the jump goes from 4.8 % to 27.0 %. Any decision
   about the preview resolution (FV-11 offers 100 %, 50 % and 25 %) moves the hit rate more than
   the prefetch policy does.

## How the experiments were run

The scratch projects, outside the repository:

```
/tmp/gafira-exp-proj/media/     # XP4, XP5, XP6, XP7 (path deps on /tmp/gafira-wt/expl/crates/*)
/tmp/gafira-exp-proj/render/    # XP2 (path dep on the exported tree of RE1)
/tmp/gafira-exp-proj/re-tree/   # git archive feat/layer-3-render, read-only
/tmp/gafira-expl/data/          # the ffprobe dumps of the real clip
CARGO_TARGET_DIR=/tmp/gafira-expl-target  CARGO_INCREMENTAL=0
```

Toolchain 1.93.1, release profile of the repo (`opt-level = 3`, `lto = "thin"`,
`codegen-units = 4`). The crates of the worktree were never edited: they are path dependencies and
`git status` in `/tmp/gafira-wt/expl` is clean apart from this file.
