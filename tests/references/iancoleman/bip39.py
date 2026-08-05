#!/usr/bin/env python3

import argparse
from pathlib import Path

from outcome import CoreDriver, require_accepted, require_equal
from reference import IanColeman


EXACT_12_BOUNDARY = "61262262611466652634263242642635642166662444332122"
EXACT_24_BOUNDARY = (
    "62633655465242253144132535222554424333361342521346"
    "25323511345453215331453612416422433556351254153322"
)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--core", required=True)
    parser.add_argument("--node", required=True)
    parser.add_argument("--runner", required=True, type=Path)
    parser.add_argument("--source", required=True)
    args = parser.parse_args()
    core = CoreDriver(args.core)
    iancoleman = IanColeman(args.node, args.runner, args.source)

    vectors = [
        (
            12,
            "1" * 50,
            "00" * 16,
            " ".join(["abandon"] * 11 + ["about"]),
        ),
        (
            12,
            EXACT_12_BOUNDARY,
            "ff" * 16,
            " ".join(["zoo"] * 11 + ["wrong"]),
        ),
        (
            24,
            "1" * 100,
            "00" * 32,
            " ".join(["abandon"] * 23 + ["art"]),
        ),
        (
            24,
            EXACT_24_BOUNDARY,
            "ff" * 32,
            " ".join(["zoo"] * 23 + ["vote"]),
        ),
    ]
    for words, rolls, entropy, mnemonic in vectors:
        label = f"Ian Coleman {words}-word BIP-39"
        actual = require_accepted(label, core.calculate("exact-v1", words, rolls))
        require_equal(f"{label} core entropy", entropy, actual.entropy)
        require_equal(f"{label} core mnemonic", mnemonic, actual.mnemonic)

        reference = iancoleman.from_entropy(entropy)
        require_equal(f"{label} reference entropy", entropy, reference.entropy)
        require_equal(f"{label} reference mnemonic", mnemonic, reference.mnemonic)
    print("validated Ian Coleman BIP-39 against core")


if __name__ == "__main__":
    main()
