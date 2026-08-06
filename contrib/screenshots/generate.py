#!/usr/bin/env python3
"""Regenerate the README ceremony GIF and stills into docs/img/.

Everything here is deterministic. It drives the debug binary through a PTY with
the COLDCARD protocol and a balanced 54-roll sequence (each face nine times,
16.7% — comfortably under COLDCARD's 30% distribution guard), so the demo shows
an honest, guard-passing capture rather than a degenerate all-one-face input.

The resulting mnemonic is a throwaway demo seed (reconstructable from the rolls
shown in the derivation frame) — never use it for real funds.

Usage (see README.md in this directory for full setup):
    cargo build                      # produces target/debug/bip39-ceremony
    python3 -m venv .venv && . .venv/bin/activate && pip install pyte Pillow
    python3 contrib/screenshots/generate.py
Override the binary or font with BIP39_BIN / SHOT_FONT_REG / SHOT_FONT_BOLD.
"""
from __future__ import annotations

import importlib.util
import os
import random

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
BIN = os.environ.get("BIP39_BIN", os.path.join(ROOT, "target", "debug", "bip39-ceremony"))
OUT = os.path.join(ROOT, "docs", "img")

_spec = importlib.util.spec_from_file_location("tui_shot", os.path.join(HERE, "tui_shot.py"))
t = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(t)

ROWS, COLS = 40, 100


def coldcard_rolls() -> str:
    """54 balanced, shuffled D6 rolls (deterministic; passes the 30% guard)."""
    rng = random.Random(42)
    faces = list("123456" * 9)
    rng.shuffle(faces)
    return "".join(faces)


def still(script, name, fontsize=15):
    data = t.drive(BIN, ROWS, COLS, [(k.encode(), w) for k, w in script])
    t.render(data, ROWS, COLS, os.path.join(OUT, name), scale=2, fontsize=fontsize)


def gif():
    rolls = coldcard_rolls()
    a, b, c = rolls[:18], rolls[18:36], rolls[36:]
    # COLDCARD is the first protocol, so no cursor moves are needed to pick it.
    # (keys, settle_seconds, hold_ms) — one frame captured per step.
    steps = [
        ("", 0.4, 1700),      # mnemonic-length select
        ("\r", 0.3, 1600),    # -> protocol list (COLDCARD highlighted)
        ("\r", 0.3, 1400),    # select COLDCARD -> safety
        ("c", 0.3, 1300),     # check all acknowledgements
        ("\r", 0.3, 900),     # -> roll capture
        (a, 0.3, 650),        # rolls 1-18
        (b, 0.3, 650),        # rolls 19-36
        (c, 0.3, 1000),       # rolls 37-54 (ready)
        ("\r", 0.4, 700),     # generate
        ("r", 0.4, 2200),     # reveal words
        ("d", 0.5, 2600),     # derivation pipeline
    ]
    t.animate(BIN, ROWS, COLS, steps, os.path.join(OUT, "ceremony.gif"),
              scale=2, fontsize=14, target_w=820, spacer_ms=1700)


def reveal_still():
    rolls = coldcard_rolls()
    still([("\r", .3), ("\r", .3), ("c", .2), ("\r", .3), (rolls, .5), ("\r", .5), ("r", .5)],
          "reveal.png")


def roll_capture_still():
    rolls = coldcard_rolls()[:30]
    still([("\r", .3), ("\r", .3), ("c", .2), ("\r", .3), (rolls, .5), ("h", .3)],
          "roll-capture.png")


if __name__ == "__main__":
    if not os.path.isfile(BIN):
        raise SystemExit(f"binary not found: {BIN}\nRun `cargo build` or set BIP39_BIN.")
    os.makedirs(OUT, exist_ok=True)
    gif()
    reveal_still()
    roll_capture_still()
    print(f"wrote ceremony.gif, reveal.png, roll-capture.png -> {OUT}")
