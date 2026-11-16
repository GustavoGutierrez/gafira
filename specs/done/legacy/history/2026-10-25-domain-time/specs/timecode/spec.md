# Timecode Specification

## Purpose

SMPTE-style `HH:MM:SS:FF` timecode over the exact time model in `time-model`: non-drop and drop-frame labels, parsing, formatting and lossless conversion to and from flicks. A timecode is a frame label for display and interchange; flicks remain the source of truth. Drop-frame is a labeling rule, not a second time base.

Applies only to frame rates accepted by `time-model`. Scenario types: unit, property, manual. Delivery: D4.

## ADDED Requirements

### Requirement: A timecode is a validated frame label over flicks (D4)

A timecode MUST pair a label of whole frames with the frame rate and the labeling mode (non-drop or drop-frame) it belongs to. Only labels valid for that rate and mode MUST be representable; a constructor MUST reject an invalid label instead of repairing it. Conversion from flicks MUST floor to the frame that contains the instant, following `time-model`. Conversion back to flicks MUST be exact. `Timecode` MUST NOT be serialized by this crate, because it is a derived display value.

#### Scenario: Label to time and back (property)
- GIVEN an arbitrary valid label for a format and its exact time
- WHEN the label is converted to time and back to a label
- THEN the original label is returned

#### Scenario: Time inside a frame floors to the containing label (unit)
- GIVEN one flick less than the start of the next frame
- WHEN it is converted to a timecode
- THEN the label of the containing frame is returned

#### Scenario: Invalid label is not representable (unit)
- GIVEN a frame field at or above the nominal frame rate, and a drop-frame label that names a skipped frame number
- WHEN such a label is constructed
- THEN each construction is rejected and no timecode exists

### Requirement: Non-drop timecode (D4)

Non-drop timecode MUST format as `HH:MM:SS:FF`, with the field widths of "Timecode text is canonical", no 24-hour wrap on hours, minutes and seconds below 60, and frames below the nominal integer frame rate. The nominal integer frame rate MUST be the declared rate rounded to the nearest integer, with ties rounded up; it is 24 for 24000/1001, 30 for 30000/1001 and 60 for 60000/1001. The separator before the frames field MUST be `:`. Parsing MUST accept exactly the canonical text, with a leading `-` only for negative time, and MUST reject out-of-range fields, missing or extra fields, non-digit characters and the drop-frame separator. Non-drop labels count frames at the declared rational rate: at 30000/1001 fps the label drifts against wall-clock time by design and MUST NOT be silently compensated.

#### Scenario: Nominal frame rate (unit)
- GIVEN 24000/1001, 30000/1001, 60000/1001 and 24.5 fps
- WHEN the nominal integer frame rate is used for a label
- THEN it is 24, 30, 60 and 25 respectively, and a frames field equal to it is rejected

#### Scenario: Formatting (unit)
- GIVEN label 90,000 at 25 fps non-drop
- WHEN it is formatted
- THEN it reads `01:00:00:00`

#### Scenario: Parse round trip (property)
- GIVEN an arbitrary representable non-drop label at an accepted rate
- WHEN it is formatted and parsed back with the same format
- THEN the same label and the same time are returned

#### Scenario: Invalid fields rejected (unit)
- GIVEN `00:60:00:00`, `00:00:60:00`, `00:00:00:24` at 24 fps, `00:00:00`, `00:00:00:00:00` and `aa:00:00:00`
- WHEN each is parsed as non-drop
- THEN each is rejected

#### Scenario: Negative timecode (unit)
- GIVEN exactly one frame before zero at 24 fps
- WHEN it is formatted non-drop
- THEN it reads `-00:00:00:01`, and parsing that text returns the same time

#### Scenario: Hours beyond 24 do not wrap (unit)
- GIVEN a label of 25 hours at 24 fps
- WHEN it is formatted
- THEN it reads `25:00:00:00`

#### Scenario: Non-drop 29.97 drift is explicit (unit)
- GIVEN the label `01:00:00:00` at 30000/1001 fps non-drop
- WHEN it is converted to flicks
- THEN the time is 3,603.6 s exactly (108,000 frames at 1001/30000 s), not 3,600 s

### Requirement: Drop-frame timecode for 29.97 and 59.94 only (D4)

Drop-frame MUST be supported exactly for 30000/1001 and 60000/1001 fps and MUST be rejected for every other rate. It MUST skip frame numbers 00 and 01 at the start of every minute whose minute number is not a multiple of 10 (29.97 fps), and frame numbers 00 to 03 for the same minutes (59.94 fps). Minutes divisible by 10 MUST NOT skip any frame number. The separator before the frames field MUST be `;`. Parsing MUST require that separator and MUST reject a label that names a skipped frame number. The mapping between valid labels and frame times MUST be bijective: every valid label maps to exactly one instant and back.

#### Scenario: Reference labels at 29.97 (unit)
- GIVEN 30000/1001 fps drop-frame
- WHEN zero, real frame 1,800, real frame 17,982 and real frame 107,892 are formatted
- THEN the labels are `00:00:00;00`, `00:01:00;02`, `00:10:00;00` and `01:00:00;00`

#### Scenario: Reference times at 29.97 (unit)
- GIVEN those four drop-frame labels
- WHEN each is converted to flicks
- THEN the times are 0 s, 60.06 s, 599.9994 s (17,982 frames) and 3,599.9964 s (107,892 frames)

#### Scenario: Reference labels at 59.94 (unit)
- GIVEN 60000/1001 fps drop-frame
- WHEN real frame 3,600 is formatted
- THEN the label is `00:01:00;04`

#### Scenario: Skipped labels rejected (unit)
- GIVEN `00:01:00;00`, `00:01:00;01` and `00:09:00;01` at 29.97 fps, and `00:01:00;03` at 59.94 fps
- WHEN each is parsed as drop-frame
- THEN each is rejected

#### Scenario: Tenth minutes do not skip (unit)
- GIVEN `00:10:00;00` and `00:20:00;00` at 29.97 fps
- WHEN each is parsed
- THEN each is accepted and maps to an exact frame time

#### Scenario: Drop-frame rejected for other rates (unit)
- GIVEN 24 fps, 25 fps, 30 fps, 60 fps and 120 fps, and 24000/1001 fps
- WHEN drop-frame is requested
- THEN each request is rejected

#### Scenario: Drop-frame round trip (property)
- GIVEN an arbitrary real frame index over at least twenty ten-minute blocks and a supported drop-frame rate
- WHEN it is converted to a label and back to flicks
- THEN the original frame index is recovered, and no produced label is a skipped one

### Requirement: Separators are part of the format (D4)

A timecode format MUST fix the rate and the mode, and parsing MUST reject text whose frames separator contradicts the mode: `:` for non-drop and `;` for drop-frame. Formatting MUST emit the separator of the mode. Round-tripping a formatted timecode MUST preserve the mode and the rate.

#### Scenario: Contradictory separator rejected (unit)
- GIVEN `00:00:00;00` parsed as non-drop at 24 fps and `00:00:00:00` parsed as drop-frame at 29.97 fps
- WHEN each is parsed
- THEN each is rejected

#### Scenario: Formatting uses the mode separator (unit)
- GIVEN a non-drop timecode and a drop-frame timecode
- WHEN each is formatted
- THEN the non-drop text has `:` before the frames field and the drop-frame text has `;`

### Requirement: Timecode text is canonical (D4)

Each label MUST have exactly one canonical text in both modes, and formatting MUST produce it. The minutes and seconds fields MUST have exactly two digits. The hours and frames fields MUST have at least two digits and no leading zero beyond that minimum: a value below 10 has one leading zero, and a value of 10 or more has none. A leading `-` MUST appear only before a label whose frame count is not zero. Parsing MUST reject any text that formatting cannot produce for that format, so formatting a parsed text returns the input text unchanged.

#### Scenario: Frames field width at 120 fps (unit)
- GIVEN 120 fps non-drop
- WHEN labels 5, 99, 100 and 119 are formatted
- THEN they read `00:00:00:05`, `00:00:00:99`, `00:00:00:100` and `00:00:00:119`, and parsing each text returns the same label

#### Scenario: Extra leading zeros rejected (unit)
- GIVEN `00:00:00:005` at 120 fps; `00:00:00:005`, `00:00:00:0023`, `000:00:00:00` and `001:00:00:00` at 24 fps; `00:00:00;005` at 30000/1001 fps drop-frame; and `0:00:00:00`, `00:0:00:00` and `00:00:000:00` at 24 fps
- WHEN each is parsed with that format
- THEN each is rejected

#### Scenario: Hours of 100 or more have no padding (unit)
- GIVEN a label of 100 hours at 24 fps non-drop
- WHEN it is formatted
- THEN it reads `100:00:00:00`, and parsing that text returns the same label

#### Scenario: Negative zero rejected (unit)
- GIVEN `-00:00:00:00` at 24 fps non-drop and `-00:00:00;00` at 30000/1001 fps drop-frame
- WHEN each is parsed
- THEN each is rejected, while `-00:00:00:01` at 24 fps is accepted as one frame before zero

#### Scenario: Accepted text is canonical (property)
- GIVEN an arbitrary valid label and a format at an accepted rate (non-drop at any accepted rate, including rates of 100 fps or more, or drop-frame at 30000/1001 or 60000/1001), and a text derived from the formatted label by one arbitrary edit: inserting extra leading zeros into any field, adding or removing a leading `-`, or inserting, deleting or replacing one character from `0-9`, `:`, `;` and `-`
- WHEN the derived text is parsed with that format
- THEN either parsing is rejected, or it succeeds and formatting the result returns the derived text unchanged; inserting extra leading zeros and adding `-` to a zero label are always rejected

## Non-goals

- A second time base: drop-frame changes labels only, never the underlying flicks.
- Time-of-day, dates, wall clocks or 24-hour rollover.
- Drop-frame at rates other than 30000/1001 and 60000/1001.
- Pulldown, interlaced field timecode, or 23.976 timecode declared as drop-frame.
- Localized separators, digits or labels.
- Serialization of timecode; consumers persist flicks.
- A default display mode for the interface (owned by the visual and interaction document, 03).

## Edge cases

| Case | Required behavior |
|---|---|
| Negative time | Formatted with a leading `-`; the magnitude is labeled and parsed back to the same negative flicks |
| Hours at or above 24 | Displayed as a running hour count; never wrapped |
| Skipped drop-frame numbers | Rejected by parsing and by label construction; never repaired silently |
| Tenth-minute boundary | No frame numbers are skipped at minutes divisible by 10 |
| Rate without drop-frame support | Drop-frame requested for it is rejected |
| Zero time | `00:00:00:00` non-drop, `00:00:00;00` drop-frame |
| Contradictory separator | Parsing fails |
| Out-of-range fields | Parsing fails (minutes or seconds 60 or more, frames at or above the nominal rate) |
| Non-canonical field width | Parsing fails (extra leading zeros in hours or frames, minutes or seconds not exactly two digits); formatting never emits it |
| Negative zero | A `-` before a zero label is rejected by parsing; formatting never emits it |
| Overflowing conversion | Return an overflow error; never wrap |
| Non-drop 29.97 drift | Expected and documented; never compensated |

## Open questions

None. The crate exposes both labeling modes and fixes no product default; the interface default belongs to the visual and interaction document (03) when the timeline exists. No owner decision in `docs/specs/00-decisiones.md` is touched: D-03 and D-08 do not constrain timecode labels. The SMPTE drop-frame rule for 29.97 and 59.94 is fixed by the standard and measured in `spike.md`; no alternative is open.
