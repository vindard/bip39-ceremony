#!/usr/bin/env python3

import argparse
import subprocess

from outcome import CoreDriver, Status, require_accepted, require_equal, require_status


def legacy_entropy(java: str, classes: str, rolls: str) -> str:
    return subprocess.check_output(
        [java, "-cp", classes, "SetupVaultAdapter", rolls], text=True
    ).strip()


def completion_policy(java: str, classes: str, count: int) -> str:
    return subprocess.check_output(
        [java, "-cp", classes, "RollingDiceAdapter", str(count)], text=True
    ).strip()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--core", required=True)
    parser.add_argument("--java", required=True)
    parser.add_argument("--classes", required=True)
    args = parser.parse_args()

    core = CoreDriver(args.core)
    for count, expected in [(49, "blocked"), (50, "confirm"), (98, "confirm"), (99, "direct")]:
        require_equal(
            f"Keystone legacy completion policy at {count} rolls",
            expected,
            completion_policy(args.java, args.classes, count),
        )
    require_status(
        "Keystone legacy short capture",
        core.calculate("keystone-legacy-v1", 24, "1" * 49),
        Status.INVALID,
        "observation-count",
        ("50", "49"),
    )

    vectors = [
        "123456" * 8 + "12",
        ("615243" * 16) + "61",
        ("654321" * 16) + "654",
        ("162534" * 22) + "16253",
    ]
    for rolls in vectors:
        actual = require_accepted(
            f"Keystone legacy conversion with {len(rolls)} rolls",
            core.calculate("keystone-legacy-v1", 24, rolls),
        )
        require_equal(
            f"Keystone legacy entropy with {len(rolls)} rolls",
            legacy_entropy(args.java, args.classes, rolls),
            actual.entropy,
        )

    print("validated Keystone legacy dice mapping and hashing against core")


if __name__ == "__main__":
    main()
