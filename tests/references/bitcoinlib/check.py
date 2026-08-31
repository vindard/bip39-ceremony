#!/usr/bin/env python3
"""Compares `bitcoinlib-base6-v1` with RooSoft/bitcoinlib's own reading.

The upstream function returns an integer and stops there; turning that integer
into BIP-39 entropy is where implementations diverge. So the oracle supplies the
integer and this check asserts the two things core claims about it: the accepted
entropy is that integer in big-endian bytes, and a value whose minimal encoding
is not exactly the target width is rejected rather than padded or truncated.
"""

import argparse
import subprocess

from outcome import CoreDriver, Status, require_accepted, require_equal, require_status

# Tapes chosen to land on both sides of the width rule at both targets: the two
# vectors bitcoinlib tests itself, the all-ones and all-sixes extremes, and
# tapes whose leading digits push the value narrow or wide.
TAPES_12 = [
    ("123456" * 8) + "12",
    "1" * 50,
    "6" * 50,
    ("135246" * 8) + "34",
    ("246135" * 8) + "51",
    "1" + ("543216" * 8) + "1",
]
TAPES_24 = [
    ("123456" * 16) + "123",
    "1" * 99,
    "6" * 99,
    ("614253" * 16) + "362",
    "1" + ("362514" * 16) + "24",
]


def upstream_value(elixir: str, adapter: str, source: str, rolls: str) -> int | None:
    record = subprocess.check_output(
        [elixir, adapter, source, rolls], text=True
    ).strip()
    status, _, payload = record.partition("\t")
    if status == "error":
        return None
    return int(payload)


def minimal_byte_width(value: int) -> int:
    return (value.bit_length() + 7) // 8


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--core", required=True)
    parser.add_argument("--elixir", required=True)
    parser.add_argument("--adapter", required=True)
    parser.add_argument("--source", required=True)
    args = parser.parse_args()

    core = CoreDriver(args.core)

    for words, tapes in ((12, TAPES_12), (24, TAPES_24)):
        target_bytes = 16 if words == 12 else 32
        for rolls in tapes:
            value = upstream_value(args.elixir, args.adapter, args.source, rolls)
            if value is None:
                raise AssertionError(f"upstream refused a {len(rolls)}-roll tape")
            outcome = core.calculate("bitcoinlib-base6-v1", words, rolls)
            width = minimal_byte_width(value)
            label = f"bitcoinlib {words}-word reading of {len(rolls)} rolls"

            if width == target_bytes:
                actual = require_accepted(label, outcome)
                require_equal(
                    f"{label} entropy",
                    value.to_bytes(target_bytes, "big").hex(),
                    actual.entropy,
                )
            else:
                require_status(
                    f"{label} off-width rejection ({width} bytes)",
                    outcome,
                    Status.REJECTED,
                    "base6-width",
                )

    # Upstream demands exactly 50 or 99 rolls and so does core.
    for words, count, expected in ((12, 49, "50"), (24, 100, "99")):
        rolls = "3" * count
        if upstream_value(args.elixir, args.adapter, args.source, rolls) is not None:
            raise AssertionError(f"upstream accepted {count} rolls")
        require_status(
            f"bitcoinlib {words}-word count gate at {count} rolls",
            core.calculate("bitcoinlib-base6-v1", words, rolls),
            Status.INVALID,
            "observation-count",
            (expected, str(count)),
        )

    print("validated bitcoinlib base-6 reading and width policy against core")


if __name__ == "__main__":
    main()
