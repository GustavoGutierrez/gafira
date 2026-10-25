# Spike: arithmetic claims for `gafira-time`

Scratch experiments run outside the repository: Python 3.10 (exact integers) for the time arithmetic, and a throwaway Cargo project for the serde shape. No repository file was modified by these runs.

## C1: one tick divides every required rate exactly

Command (exact integer division):

```text
python3 -c 'f=705_600_000
for d in [24,25,30,48,50,60,120,44_100,48_000,96_000]:
    print(d, divmod(f, d))'
```

Output (abridged):

```text
24 (29400000, 0)
25 (28224000, 0)
30 (23520000, 0)
48 (14700000, 0)
50 (14112000, 0)
60 (11760000, 0)
120 (5880000, 0)
44100 (16000, 0)
48000 (14700, 0)
96000 (7350, 0)
```

Verdict: **confirmed**. 705,600,000 = 2^9 x 3^2 x 5^5 x 7^2, which contains every factor of 24, 25, 30, 48, 50, 60, 120, 44,100, 48,000 and 96,000.

## C2: the NTSC rates are rational and their frame durations are whole flicks

Command: for each rate `num/den`, compute `divmod(705_600_000 * den, num)`.

Output:

```text
24000/1001 frame duration 29429400 rem 0
24/1       frame duration 29400000 rem 0
25/1       frame duration 28224000 rem 0
30000/1001 frame duration 23543520 rem 0
30/1       frame duration 23520000 rem 0
48/1       frame duration 14700000 rem 0
50/1       frame duration 14112000 rem 0
60000/1001 frame duration 11771760 rem 0
60/1       frame duration 11760000 rem 0
120/1      frame duration 5880000 rem 0
```

Rejected cases:

```text
11/1      integral False rem 6
2398/100  integral False rem 1040
1/2       integral True, below 1 fps
```

Verdict: **confirmed**. Integrality is testable as `(705_600_000 * den) % num == 0`; 23.976, 29.97 and 59.94 are exact; 11 fps and 23.98 are not and must be rejected.

## C3: overflow headroom of the exact products

Output:

```text
705600000 * u32::MAX = 3030528923352000000  (i64::MAX = 9223372036854775807)
i64::MAX flicks in seconds = 13071672387.8  (about 414 years)
```

Verdict: **confirmed**. A product of the tick by any `u32` denominator fits `i64`; an `i128` intermediate is still used for the validation to avoid relying on the bound. `i64` flicks span about ±414 years, enough for any media timeline.

## C4: drop-frame label mapping

Formulas measured over 20 ten-minute blocks for 29.97 (nominal 30, drop 2) and 59.94 (nominal 60, drop 4):

```text
label -> real: real = label - drop * (total_minutes - total_minutes // 10)
real -> label: block = real // frames_per_10min; rem = real % frames_per_10min
              frames_per_10min = nominal * 600 - drop * 9
              frames_per_min   = nominal * 60 - drop
              if rem < nominal * 60: local = rem
              else: r = rem - nominal * 60
                    minute = 1 + r // frames_per_min
                    local = minute * nominal * 60 + (r % frames_per_min) + drop
              label = block * nominal * 600 + local
dropped label: total_minutes % 10 != 0 and seconds == 0 and frames < drop
```

Output:

```text
nominal 30: real->label->real mismatches=0, label->real->label mismatches=0 over 359640 valid labels
nominal 60: real->label->real mismatches=0, label->real->label mismatches=0 over 719280 valid labels
skipped in 10 min: 18 (29.97), 36 (59.94)
```

Reference vectors:

```text
29.97 real      0 -> 00:00:00;00 -> real      0
29.97 real   1800 -> 00:01:00;02 -> real   1800   (60.0600 s)
29.97 real  17982 -> 00:10:00;00 -> real  17982   (599.9994 s)
29.97 real 107892 -> 01:00:00;00 -> real 107892   (3599.9964 s)
59.94 real   3600 -> 00:01:00;04 -> real   3600   (60.0600 s)
59.94 real  35964 -> 00:10:00;00 -> real  35964
59.94 real 215784 -> 01:00:00;00 -> real 215784
```

Non-drop drift at 29.97, for contrast:

```text
NDF  1800 frames -> 60.0600 s
NDF 108000 frames -> 3603.6000 s
DF  107892 frames -> 3599.9964 s
```

The first run of the experiment counted a skipped label at every second (`ff < drop`) instead of only at second zero; it reported 1,080 and 2,160 "skipped" labels and 360/720 false mismatches. Adding the `seconds == 0` condition (the rule in the standard) produced zero mismatches. The corrected rule is the one recorded above and the one the spec states.

Verdict: **confirmed** with the corrected rule.

## C5: serde validation shapes compile and round-trip

Throwaway Cargo project with `serde` 1 (derive) and `serde_json` 1, using `#[serde(try_from = "...", into = "...")]` on validated types.

Output:

```text
frame rate: {"num":30000,"den":1001}
back: FrameRate { num: 30000, den: 1001 }
zero den: true
range: {"start":705600000,"duration":1411200000}
back: TimeRange { start: 705600000, duration: 1411200000 }
negative: true
overflow: true
```

Verdict: **confirmed**. The wire shapes in the spec are produced by derive plus private wire structs, and invalid JSON is rejected by the conversion, not by a post-load check.

## Summary

| Claim | Verdict |
|---|---|
| C1 tick divisibility | confirmed |
| C2 frame durations integral, 11 fps and 23.98 rejected | confirmed |
| C3 i64 headroom | confirmed |
| C4 drop-frame mapping and reference vectors | confirmed (after correcting the skipped-label rule) |
| C5 serde validation shape | confirmed |

No claim was refuted; no spec or design value needs to change.
