#!/usr/bin/env python3
"""Compares `bluewallet-bitpack-v1` with BlueWallet's own extracted functions.

The oracle reports how many bits it packed, how many rolls actually contributed,
and the resulting bytes. Those three answer the whole protocol: the face-to-bits
split, where the tape stops counting, and the entropy itself.
"""

import argparse
import subprocess

from outcome import CoreDriver, Status, require_accepted, require_equal, require_status

PROTOCOL = "bluewallet-bitpack-v1"


def cycled(count: int) -> str:
    return "".join("123456"[index % 6] for index in range(count))


# Tapes covering both face widths, both targets, the exact fill, a tape that
# stops short, a tape with unused surplus, and the overshoot boundary where the
# last roll contributes only its leading bit.
TAPES_12 = [
    "1" * 64,
    "4" * 64,
    "5" * 128,
    "6" * 128,
    cycled(76),
    cycled(77),
    "1" * 63,
    "1" * 70,
    "5" * 127 + "4",
    "5" * 127 + "1",
    "3" * 64,
]
TAPES_24 = [
    "1" * 128,
    "6" * 256,
    cycled(153),
    cycled(160),
    "2" * 127,
]


def oracle(node: str, runner: str, adapter: str, bignumber: str, words: int, rolls: str):
    record = subprocess.check_output(
        [node, runner, adapter, bignumber, str(words), rolls], text=True
    ).strip()
    bits, consumed, entropy = record.split("\t")
    return int(bits), int(consumed), entropy


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--core", required=True)
    parser.add_argument("--node", required=True)
    parser.add_argument("--runner", required=True)
    parser.add_argument("--adapter", required=True)
    parser.add_argument("--bignumber", required=True)
    args = parser.parse_args()

    core = CoreDriver(args.core)

    for words, tapes in ((12, TAPES_12), (24, TAPES_24)):
        limit = 128 if words == 12 else 256
        for rolls in tapes:
            bits, consumed, entropy = oracle(
                args.node, args.runner, args.adapter, args.bignumber, words, rolls
            )
            outcome = core.calculate(PROTOCOL, words, rolls)
            label = f"BlueWallet {words}-word packing of {len(rolls)} rolls"

            if bits == limit and consumed == len(rolls):
                actual = require_accepted(label, outcome)
                require_equal(f"{label} entropy", entropy, actual.entropy)
                continue

            # Upstream would top the shortfall up from the phone's RNG, or
            # silently ignore the surplus. Neither is reproducible, so core
            # declines rather than inventing the remainder.
            short = bits < limit
            reason = "short of the width" if short else "past the width"
            expected = len(rolls) + 1 if short else consumed
            require_status(
                f"{label} declined, {reason}",
                outcome,
                Status.INVALID,
                "observation-count",
                (str(expected), str(len(rolls))),
            )

    print("validated BlueWallet bit packing and tape length against core")


if __name__ == "__main__":
    main()
