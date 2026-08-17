#!/usr/bin/env python3
"""Compares both iancoleman dice paths with the tool's own function.

One dropdown decides which construction a tape gets. With a fixed word count the
tool hashes the rewritten digits; left on its default "raw" setting it uses the
packed bits and keeps only whole 32-bit groups, counted from the end. Both are
executed here, so the check also pins that they disagree.
"""

import argparse
import subprocess

from outcome import CoreDriver, Status, require_accepted, require_equal, require_status


def cycled(count: int) -> str:
    return "".join("123456"[index % 6] for index in range(count))


HASH_TAPES = [cycled(50), "1" * 50, "6" * 50, cycled(63)]
HASH_TAPES_24 = [cycled(100), "6" * 100, cycled(137)]

# 10 bits per six cycled rolls, so 77 rolls land exactly on 128 and 78 spills
# two bits that shift the whole result.
RAW_TAPES_12 = [cycled(77), cycled(78), cycled(90), "4" * 128, "5" * 128, "1" * 64]
RAW_TAPES_24 = [cycled(154), "4" * 256, "1" * 128]


def upstream(node: str, runner: str, source: str, adapter: str, setting: str, rolls: str) -> str:
    return subprocess.check_output(
        [node, runner, source, adapter, setting, rolls], text=True
    ).strip()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--core", required=True)
    parser.add_argument("--node", required=True)
    parser.add_argument("--runner", required=True)
    parser.add_argument("--source", required=True)
    parser.add_argument("--adapter", required=True)
    args = parser.parse_args()

    core = CoreDriver(args.core)

    def oracle(setting: str, rolls: str) -> str:
        return upstream(
            args.node, args.runner, args.source, args.adapter, setting, rolls
        )

    for words, tapes in ((12, HASH_TAPES), (24, HASH_TAPES_24)):
        for rolls in tapes:
            label = f"iancoleman {words}-word hash of {len(rolls)} rolls"
            actual = require_accepted(
                label, core.calculate("iancoleman-dice-v1", words, rolls)
            )
            require_equal(label, oracle(str(words), rolls), actual.mnemonic)

    for words, tapes in ((12, RAW_TAPES_12), (24, RAW_TAPES_24)):
        for rolls in tapes:
            label = f"iancoleman raw packing of {len(rolls)} rolls at {words} words"
            actual = require_accepted(
                label, core.calculate("iancoleman-raw-v1", words, rolls)
            )
            require_equal(label, oracle("raw", rolls), actual.mnemonic)

    # A tape carrying another whole 32-bit group is a different word count
    # upstream, so core declines rather than silently picking a window.
    overrun = cycled(96)
    require_status(
        "iancoleman raw overrun declined",
        core.calculate("iancoleman-raw-v1", 12, overrun),
        Status.INVALID,
        "observation-count",
        ("128", "160"),
    )
    require_equal(
        "iancoleman raw overrun is a longer phrase upstream",
        15,
        len(oracle("raw", overrun).split(" ")),
    )

    # The dropdown, not the tape, decides the construction.
    divergent = cycled(77)
    hashed = require_accepted(
        "hash path", core.calculate("iancoleman-dice-v1", 12, divergent)
    )
    packed = require_accepted(
        "raw path", core.calculate("iancoleman-raw-v1", 12, divergent)
    )
    if hashed.mnemonic == packed.mnemonic:
        raise AssertionError("the two iancoleman paths must not agree on one tape")

    print("validated both iancoleman dice paths against core")


if __name__ == "__main__":
    main()
