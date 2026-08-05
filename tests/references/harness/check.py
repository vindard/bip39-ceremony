#!/usr/bin/env python3

import argparse

from outcome import CoreDriver, Outcome, Status, require_accepted


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--core", required=True)
    args = parser.parse_args()

    accepted = Outcome.parse("accepted\t00ff\tabandon about\n")
    if accepted != Outcome(
        Status.ACCEPTED, entropy="00ff", mnemonic="abandon about"
    ):
        raise AssertionError("accepted core record did not round-trip")

    invalid = Outcome.parse("invalid\tobservation-count\t50\t49\n")
    if invalid != Outcome(
        Status.INVALID,
        code="observation-count",
        details=("50", "49"),
    ):
        raise AssertionError("invalid core record did not round-trip")

    try:
        Outcome.parse("accepted\tonly-one-field\n")
    except ValueError:
        pass
    else:
        raise AssertionError("malformed accepted record was not rejected")

    rejected = Outcome.parse("rejected\texact-range\n")
    if rejected != Outcome(Status.REJECTED, code="exact-range"):
        raise AssertionError("rejected core record did not round-trip")

    core = CoreDriver(args.core)
    require_accepted(
        "core driver acceptance",
        core.calculate("exact-v1", 12, "1" * 50),
    )
    print("validated core reference harness")


if __name__ == "__main__":
    main()
