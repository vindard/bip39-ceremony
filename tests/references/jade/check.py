#!/usr/bin/env python3

import argparse
import subprocess

from outcome import CoreDriver, require_accepted, require_equal


def observations(words: int, tail: int, varied: bool) -> str:
    tokens = []
    for position in range(words - 1):
        if varied:
            first = position % 16 + 1
            second = (position * 5 + 3) % 16 + 1
            third = (position * 3 + 1) % 8 + 1
        else:
            first = second = third = 1
        tokens.extend([f"a{first}", f"a{second}", f"b{third}"])

    if words == 12:
        tokens.extend([f"a{tail // 8 + 1}", f"b{tail % 8 + 1}"])
    else:
        tokens.append(f"b{tail + 1}")
    return ",".join(tokens)


def candidates(adapter: str, prefix: list[str]) -> list[tuple[int, str]]:
    record = subprocess.check_output([adapter, *prefix], text=True).strip()
    result = []
    for candidate in record.split(","):
        index, separator, word = candidate.partition(":")
        if not separator:
            raise AssertionError(f"invalid Jade adapter record: {record!r}")
        result.append((int(index), word))
    return result


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--core", required=True)
    parser.add_argument("--adapter", required=True)
    args = parser.parse_args()

    vectors = [(12, tail, False) for tail in range(128)]
    vectors.extend([(12, 73, True)])
    vectors.extend((24, tail, False) for tail in range(8))
    vectors.extend([(24, 6, True)])

    core = CoreDriver(args.core)
    candidate_cache: dict[tuple[str, ...], list[tuple[int, str]]] = {}
    for words, tail, varied in vectors:
        actual = require_accepted(
            f"Jade {words}-word checksum completion",
            core.calculate("jade-direct-v1", words, observations(words, tail, varied)),
        )
        mnemonic = actual.mnemonic.split()
        prefix = tuple(mnemonic[:-1])
        if prefix not in candidate_cache:
            candidate_cache[prefix] = candidates(args.adapter, list(prefix))
        choices = candidate_cache[prefix]
        expected_count = 128 if words == 12 else 8
        require_equal(
            f"Jade {words}-word candidate count",
            str(expected_count),
            str(len(choices)),
        )
        if choices != sorted(choices):
            raise AssertionError("Jade final-word candidates are not index ordered")
        require_equal(
            f"Jade {words}-word final word at tail rank {tail}",
            mnemonic[-1],
            choices[tail][1],
        )

    print("validated Jade checksum completion against core")


if __name__ == "__main__":
    main()
