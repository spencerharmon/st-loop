# st-loop

An 8-track × 8-scene live audio looper. Part of the
[st-suite](https://github.com/spencerharmon/st-suite) live performance rig.

`st-loop` is a JACK audio client controlled live via MIDI Program-Change
messages. It records, plays back, and arranges audio loops across 8 mono/stereo
tracks organized into 8 scenes (think Ableton's session view, or a
Mobius/Sooperlooper-style live looper). Loops are bar-quantized, aligned to
the sample-accurate beat-frame stream from [`st-conductor`](../st-conductor)
via [`st-sync`](../st-sync).

Project state is persisted via [NSM](https://new-session-manager.jackaudio.org/)
(Non Session Manager).

## Usage

```sh
st-loop
```

`st-loop` is intended to be launched by an NSM-compatible session manager
(e.g. `nsm-legacy-gui`, `agordejo`, or `RaySession`). It will announce itself
via the `NSM_URL` environment variable and respond to Save / Open messages by
reading/writing a `config.yaml` in the session directory.

Runtime requirements:

- A running JACK server.
- A running [`st-conductor`](../st-conductor) (provides JACK timebase and the
  `st-sync` server on `127.0.0.1:6142`).
- An NSM session.

## JACK ports

- **Audio in:** 8 stereo input pairs (one per track).
- **Audio out:** 8 stereo output pairs (one per track).
- **MIDI in:** 1 control port (consumes the Program-Change vocabulary below).

## Control: MIDI Program Changes

| PC #  | Action                |
| ----- | --------------------- |
| 0–7   | Start scene 1–8       |
| 28    | Undo                  |
| 29    | Stop                  |
| 30    | Clear                 |
| 31    | Go (arm / commit)     |

Commands are buffered and applied on the next bar boundary, so the looper
always stays in sync with the conductor.

See `src/midi_control.rs` for the canonical mapping.

## Concepts

- **Track** — one of 8 audio channels with its own input/output ports.
- **Sequence** — one recorded loop on one track. Multiple sequences on the
  same track are mixed (overdub-style).
- **Scene** — an ordered list of sequence IDs. Activating a scene starts
  playback of its sequences.

## Session persistence

When NSM sends Save, `st-loop` writes:

- `config.yaml` — scene definitions and a list of sequences (track, length in
  beats, WAV filename).
- One WAV file per recorded sequence (via `hound`).

On Open, the inverse: it reads `config.yaml`, loads the WAVs, and rebuilds
scenes and sequences.

## Status

Early / experimental. Many of the warnings the compiler emits are known and
not yet cleaned up. The realtime path is solid; the control surface and
session format may still change.
