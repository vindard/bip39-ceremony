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

    for words, rolls in [
        (12, "1" * 50),
        (12, EXACT_12_BOUNDARY),
        (24, "1" * 100),
        (24, EXACT_24_BOUNDARY),
    ]:
        label = f"Ian Coleman {words}-word BIP-39"
        actual = require_accepted(label, core.calculate("exact-v1", words, rolls))
        reference = iancoleman.from_entropy(actual.entropy)
        require_equal(f"{label} mnemonic", reference.mnemonic, actual.mnemonic)
    print("validated Ian Coleman BIP-39 against core")


if __name__ == "__main__":
    main()
