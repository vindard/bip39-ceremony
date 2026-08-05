#!/usr/bin/env python3

import argparse
from pathlib import Path

from outcome import CoreDriver, require_accepted, require_equal
from reference import IanColeman


ROLLS = (
    "6162523635662652513121532336265621541251462434643353211512451266533116426"
    "262632524344312132312356234"
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

    for count in [99, 100]:
        label = f"Ian Coleman legacy dice with {count} rolls"
        actual = require_accepted(
            label,
            core.calculate("keystone-legacy-v1", 24, ROLLS[:count]),
        )
        reference = iancoleman.from_legacy_dice(ROLLS[:count])
        require_equal(f"{label} entropy", reference.entropy, actual.entropy)
        require_equal(f"{label} mnemonic", reference.mnemonic, actual.mnemonic)
    print("validated Ian Coleman legacy dice pipeline against core")


if __name__ == "__main__":
    main()
