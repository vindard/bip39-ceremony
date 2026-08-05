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


@dataclass(frozen=True)
class FirmwareOutcome:
    accepted: bool
    entropy: str | None = None


class Firmware:
    def __init__(self, source: str) -> None:
        seed_path = Path(source) / "shared" / "seed.py"
        tree = ast.parse(seed_path.read_text(), filename=str(seed_path))
        try:
            self._function = next(
                node
                for node in tree.body
                if isinstance(node, ast.AsyncFunctionDef)
                and node.name == "add_dice_rolls"
            )
        except StopIteration as error:
            raise RuntimeError("Coldcard add_dice_rolls function not found") from error
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
        entropy_bytes = 16 if words == 12 else 32
        return FirmwareOutcome(True, digest[:entropy_bytes].hex())

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

        for count in [minimum, minimum + 1]:
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
    print("validated Coldcard firmware against core")


if __name__ == "__main__":
    main()
