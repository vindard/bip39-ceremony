# Screenshot & GIF generation

The README's hero GIF and the "What you can inspect" stills are generated
headlessly from the real TUI — no manual screen-capture. This directory holds
the harness so they can be reproduced on a fresh machine.

## How it works

- **`tui_shot.py`** — drives the debug binary in a pseudo-terminal, replays
  keystrokes, and reconstructs each frame with [`pyte`](https://pypi.org/project/pyte/)
  (a terminal emulator) so the `ember` 256-color palette is captured exactly.
  Frames are rendered with [`Pillow`](https://pypi.org/project/Pillow/) and a
  monospace font, then assembled into a GIF (shared palette) or cropped PNGs.
- **`generate.py`** — the deterministic recipe: it produces `ceremony.gif`,
  `reveal.png`, and `roll-capture.png` into [`docs/img/`](../../docs/img/).

## Design choices (why the demo looks the way it does)

- **COLDCARD protocol** with a **balanced 54-roll sequence** — each face nine
  times (16.7%), comfortably under COLDCARD's **30% distribution guard**. This
  shows an honest, guard-passing capture, not a degenerate all-one-face input.
- The rolls are a fixed `random.Random(42)` shuffle, so output is reproducible.
- The generated mnemonic is a **throwaway demo seed** — it is reconstructable
  from the rolls shown in the derivation frame. **Never use it for real funds.**
- The GIF ends on a dice title card that marks the loop boundary.

## Requirements

- The debug binary: `cargo build` (yields `target/debug/bip39-ceremony`).
- Python 3 with `pyte` and `Pillow`.
- A monospace font (default: JetBrains Mono Nerd Font Mono). Any monospace
  works; box-drawing and dice-pip glyphs render best with a Nerd Font.

## Run

```sh
cargo build
python3 -m venv .venv && . .venv/bin/activate
pip install pyte Pillow
python3 contrib/screenshots/generate.py
```

Overrides (optional):

```sh
export BIP39_BIN=/path/to/bip39-ceremony            # non-default binary
export SHOT_FONT_REG="$(fc-match -f '%{file}' 'JetBrainsMono Nerd Font Mono:style=Regular')"
export SHOT_FONT_BOLD="$(fc-match -f '%{file}' 'JetBrainsMono Nerd Font Mono:style=Bold')"
```

The harness is a documentation tool only — it is not part of the crate build,
the test suite, or the reproducibility gates.
