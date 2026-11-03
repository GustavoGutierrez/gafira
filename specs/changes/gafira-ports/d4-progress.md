## D4 progress (tdd-apply)

Branch `feat/gafira-ports-d4`. Files: `crates/gafira-ports/src/media.rs`, `crates/gafira-ports/tests/media.rs`. Nothing shared was touched.

### Done

- `media`: `Container`, `Demuxer`, `Muxer`, `Codec`, `Decoder`, `Encoder` (all object-safe), `MediaError` with `ToMessage`, `CodecId`, `CodecCaps`, `CodecParams`, `EncoderConfig`, `StreamInfo`, `StreamIdx`, `MuxConfig`, `MuxReport`, `SeekMode`, `Packet`, `Frame`, `VideoFrame`, `AudioBuf`, `CodecRegistry` (over `Registry<dyn Codec>`).
- `CodecRegistry::best_decoder`: eligible = same `CodecId`, `decode`, not `Encumbered`; rank hardware, `OsLicensed`, `RoyaltyFree`; `min_by_key` keeps the first on a tie.
- `AudioBuf::new`: first channel sets `expected`; the first differing channel is named.

### RED evidence

Skeleton with wrong bodies (`best_decoder` always `None`, `AudioBuf::new` unchecked, `to_message` one key). Failing assertions:

| ID | Test | RED |
|---|---|---|
| ARQ-77 | `arq_77_encumbered_is_never_chosen` | left None, right Some("rf") |
| ARQ-77 | `arq_77_encode_only_is_not_chosen` | left None, right Some("dec") |
| ARQ-77 | `arq_77_other_codec_id_is_not_chosen` | left None, right Some("av1") |
| ARQ-77 | `arq_77_order_is_hardware_then_os_then_royalty_free` | left None, right Some("hw") |
| ARQ-77 | `arq_77_tie_takes_the_first_registered` | left None, right Some("a") |
| ARQ-102 | `arq_102_media_error_messages` | key left `error.media` right `error.media.unsupported_codec` |
| P8 | `p8_audio_buf_names_the_first_bad_channel` | unwrap_err on Ok |
| ARQ-31, V-5, P8 empty/equal channels, dyn-compat | passed on the skeleton (data and delegation only) | n/a |

### Hand mutants (all with `RUSTFLAGS="--cfg wt_d4"`)

| # | Mutant | Killed by |
|---|---|---|
| 1 | `Encumbered` decoder is eligible | `arq_77_encumbered_is_never_chosen` |
| 2 | Encode-only codec is eligible | `arq_77_encode_only_is_not_chosen` |
| 3 | `RoyaltyFree` ranks above hardware | `arq_77_order_is_hardware_then_os_then_royalty_free` |
| 3b | `RoyaltyFree` ranks above `OsLicensed` | same |
| 3c | Hardware flag ignored | same |
| 4 | A tie keeps the last registered | `arq_77_tie_takes_the_first_registered` |
| 5 | Another `CodecId` matches | `arq_77_other_codec_id_is_not_chosen` |
| 6 | `AudioBuf::new` checks only the first two channels | `p8_audio_buf_names_the_first_bad_channel` |
| 7 | `ChannelMismatch` names the last bad channel | `p8_audio_buf_names_the_first_bad_channel` |
| 8 | `Cancelled` does not delegate | `arq_102_media_error_messages` |

### Deviations

- P4 leaves the shapes of `StreamInfo`, `CodecParams`, `EncoderConfig`, `MuxConfig`, `MuxReport`, `SeekMode`, `CodecId` open; fixed here as the smallest plain data (`CodecId` has `as_str` for the `codec` message argument). `gafira-media` may extend them.
- `ChannelMismatch` fields are `usize`, sent as `Count`.
- `CodecRegistry` is a struct wrapping `Registry<dyn Codec>` (not an alias) so `best_decoder` has a home. No alias for `Registry<dyn Container>` was added (not in P4).
- The shared target dir let sibling worktrees overwrite the `gafira-ports` artifact; the first runs linked foreign code. Every result above was taken (or retaken) with `--cfg wt_d4`.
- ARQ-77 registry walk of the full build stays pending (needs adapters).
