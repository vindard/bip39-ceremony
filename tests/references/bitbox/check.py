#!/usr/bin/env python3

import argparse
import subprocess

from outcome import CoreDriver, require_accepted, require_equal


def observations(words: int, tail: int, varied: bool) -> str:
    direct_words = words - 1
    tokens = []
    for position in range(direct_words):
        faces = (
            [((position * 3 + offset) % 4) + 1 for offset in range(5)]
            if varied
            else [1] * 5
        )
        tokens.extend(f"d{face}" for face in faces)
        selector = position % 2 if varied else 0
        tokens.append(f"c{1 - selector}")

    tail_bits = 7 if words == 12 else 3
    tokens.extend(
        f"c{1 - ((tail >> shift) & 1)}" for shift in range(tail_bits - 1, -1, -1)
    )
    return ",".join(tokens)


def candidates(adapter: str, entered_words: list[str]) -> list[tuple[int, str]]:
    record = subprocess.check_output([adapter, *entered_words], text=True).strip()
    result = []
    for candidate in record.split(","):
        index, separator, word = candidate.partition(":")
        if not separator:
            raise AssertionError(f"invalid BitBox02 adapter record: {record!r}")
        result.append((int(index), word))
    return result


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--core", required=True)
    parser.add_argument("--adapter", required=True)
    args = parser.parse_args()

    core = CoreDriver(args.core)
    vectors = [(12, tail, False) for tail in range(128)]
    vectors.extend([(12, 85, True)])
    vectors.extend((24, tail, False) for tail in range(8))
    vectors.extend([(24, 5, True)])
    candidate_cache: dict[tuple[str, ...], list[tuple[int, str]]] = {}
    for words, tail, varied in vectors:
        actual = require_accepted(
            f"BitBox02 {words}-word checksum completion",
            core.calculate(
                "bitbox02-direct-v1",
                words,
                observations(words, tail, varied),
            ),
        )
        mnemonic = actual.mnemonic.split()
        prefix = tuple(mnemonic[:-1])
        if prefix not in candidate_cache:
            candidate_cache[prefix] = candidates(args.adapter, list(prefix))
        choices = candidate_cache[prefix]
        expected_count = 128 if words == 12 else 8
        require_equal(
            f"BitBox02 {words}-word candidate count",
            str(expected_count),
            str(len(choices)),
        )
        if len(set(choices)) != expected_count:
            raise AssertionError(f"BitBox02 {words}-word candidates are not unique")
        require_equal(
            f"BitBox02 {words}-word final word at tail rank {tail}",
            mnemonic[-1],
            choices[tail][1],
        )

    print("validated BitBox02 checksum completion against core")


if __name__ == "__main__":
    main()
