#!/usr/bin/env python3

import argparse
import ast
import asyncio
import hashlib
import sys
import types
from dataclasses import dataclass
from pathlib import Path

from outcome import CoreDriver, Status, require_accepted, require_equal, require_status


ROLLS_12 = (
    "4143635634262235413523365216435254463322212645361116426144546546266665636"
)
ROLLS_24 = (
    "3636354421253156325664453445554151616553311413251345312415365644616142264"
    "2356225562624436543341651646353163143145256446142313532636225565"
)


def grouped_rolls(groups: list[tuple[str, int]]) -> str:
    return "".join(face * count for face, count in groups)


DISTRIBUTION_CASES = [
    (12, grouped_rolls([("1", 15), ("2", 7), ("3", 7), ("4", 7), ("5", 7), ("6", 7)]), True),
    (12, grouped_rolls([("1", 16), ("2", 7), ("3", 7), ("4", 7), ("5", 7), ("6", 6)]), False),
    (12, grouped_rolls([("1", 15), ("2", 8), ("3", 7), ("4", 7), ("5", 7), ("6", 7)]), True),
    (12, grouped_rolls([("1", 16), ("2", 7), ("3", 7), ("4", 7), ("5", 7), ("6", 7)]), False),
    (24, grouped_rolls([("1", 29), ("2", 14), ("3", 14), ("4", 14), ("5", 14), ("6", 14)]), True),
    (24, grouped_rolls([("1", 30), ("2", 14), ("3", 14), ("4", 14), ("5", 14), ("6", 13)]), False),
    (24, grouped_rolls([("1", 30), ("2", 14), ("3", 14), ("4", 14), ("5", 14), ("6", 14)]), True),
    (24, grouped_rolls([("1", 31), ("2", 14), ("3", 14), ("4", 14), ("5", 14), ("6", 13)]), False),
]


@dataclass(frozen=True)
class FirmwareOutcome:
    accepted: bool
    entropy: str | None = None


class Firmware:
    def __init__(self, source: str) -> None:
        seed_path = Path(source) / "shared" / "seed.py"
        tree = ast.parse(seed_path.read_text(), filename=str(seed_path))
        functions = {
            node.name: node
            for node in tree.body
            if isinstance(node, ast.AsyncFunctionDef)
        }
        try:
            self._function = functions["add_dice_rolls"]
            self._approve_word_list = functions["approve_word_list"]
        except KeyError as error:
            raise RuntimeError(f"Coldcard function not found: {error.args[0]}") from error
        self._filename = str(seed_path)

    def generate(self, rolls: str, words: int) -> FirmwareOutcome:
        namespace = self._namespace(rolls)
        ux = types.ModuleType("ux")
        ux.ux_dice_rolling = lambda: (lambda *_: None)
        previous_ux = sys.modules.get("ux")
        sys.modules["ux"] = ux
        try:
            exec(
                compile(
                    ast.Module(body=[self._function], type_ignores=[]),
                    self._filename,
                    "exec",
                ),
                namespace,
            )
            count, digest = asyncio.run(
                namespace["add_dice_rolls"](0, b"", True, words, True)
            )
        finally:
            if previous_ux is None:
                del sys.modules["ux"]
            else:
                sys.modules["ux"] = previous_ux
        if count == 0:
            return FirmwareOutcome(accepted=False)
        if count != len(rolls):
            raise AssertionError(
                f"Coldcard consumed {count} of {len(rolls)} supplied rolls"
            )
        return FirmwareOutcome(True, self._select_entropy(digest, words).hex())

    def _select_entropy(self, digest: bytes, words: int) -> bytes:
        class CapturedEntropy(Exception):
            def __init__(self, entropy: bytes) -> None:
                self.entropy = entropy

        class Bip39:
            @staticmethod
            def b2a_words(entropy: bytes) -> str:
                raise CapturedEntropy(bytes(entropy))

        namespace = {"bip39": Bip39()}
        exec(
            compile(
                ast.Module(body=[self._approve_word_list], type_ignores=[]),
                self._filename,
                "exec",
            ),
            namespace,
        )
        try:
            asyncio.run(namespace["approve_word_list"](digest, words))
        except CapturedEntropy as captured:
            return captured.entropy
        raise AssertionError("Coldcard approve_word_list did not encode entropy")

    @staticmethod
    def _namespace(rolls: str) -> dict[str, object]:
        class FirmwareSha256:
            def __init__(self, value: bytes = b"") -> None:
                self._hash = hashlib.sha256(value)

            def update(self, value: str | bytes) -> None:
                self._hash.update(
                    value.encode() if isinstance(value, str) else value
                )

            def digest(self) -> bytes:
                return self._hash.digest()

        class PressRelease:
            def __init__(self) -> None:
                self._keys = iter(rolls + "y")

            async def wait(self) -> str:
                return next(self._keys)

        async def reject_story(*_args) -> str:
            return "x"

        return {
            "sha256": FirmwareSha256,
            "PressRelease": PressRelease,
            "b2a_hex": lambda value: value.hex().encode(),
            "KEY_CANCEL": "x",
            "KEY_ENTER": "y",
            "OK": "y",
            "X": "x",
            "ux_show_story": reject_story,
            "ux_confirm": reject_story,
        }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--core", required=True)
    parser.add_argument("--source", required=True)
    args = parser.parse_args()
    core = CoreDriver(args.core)
    firmware = Firmware(args.source)

    for words, minimum, observations in [
        (12, 50, ROLLS_12),
        (24, 99, ROLLS_24),
    ]:
        below = observations[: minimum - 1]
        if firmware.generate(below, words).accepted:
            raise AssertionError(f"Coldcard {words}-word minimum - 1 was accepted")
        require_status(
            f"Coldcard {words}-word minimum - 1",
            core.calculate("coldcard-v1", words, below),
            Status.INVALID,
            "observation-count",
            (str(minimum), str(minimum - 1)),
        )

        for count in [minimum, minimum + 1, len(observations)]:
            rolls = observations[:count]
            reference = firmware.generate(rolls, words)
            if not reference.accepted or reference.entropy is None:
                raise AssertionError(
                    f"Coldcard {words}-word {count}-roll input was rejected"
                )
            actual = require_accepted(
                f"Coldcard {words}-word {count}-roll input",
                core.calculate("coldcard-v1", words, rolls),
            )
            require_equal(
                f"Coldcard {words}-word {count}-roll entropy",
                reference.entropy,
                actual.entropy,
            )
    for words, rolls, accepted in DISTRIBUTION_CASES:
        label = f"Coldcard {words}-word {len(rolls)}-roll distribution"
        reference = firmware.generate(rolls, words)
        actual = core.calculate("coldcard-v1", words, rolls)
        if accepted:
            if not reference.accepted or reference.entropy is None:
                raise AssertionError(f"{label}: firmware unexpectedly rejected")
            require_equal(
                f"{label} entropy",
                reference.entropy,
                require_accepted(label, actual).entropy,
            )
        else:
            if reference.accepted:
                raise AssertionError(f"{label}: firmware unexpectedly accepted")
            require_status(
                label,
                actual,
                Status.REJECTED,
                "dice-distribution",
            )
    print("validated Coldcard dice capture hashing against core")


if __name__ == "__main__":
    main()
