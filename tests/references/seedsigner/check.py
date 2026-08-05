#!/usr/bin/env python3

import argparse
import importlib.util
import sys
import types
from contextlib import contextmanager
from pathlib import Path
from typing import Iterator

from outcome import CoreDriver, require_accepted, require_equal


COINS_12 = (
    "1101100111010111010100111111000000010101011110011110001111100100110010111"
    "0111110110001101000011011101001101001111110000100010001"
)
COINS_24 = (
    "0101100010100011100101000100010001101110110101000111000100010111000011011"
    "0010101000101100011100110000101000101000001110001111111100011011100111011"
    "0110000101111011011001110011110000000110011000010011010101101100000100101"
    "1110110011100100001100011001010001100"
)


class SeedSigner:
    def __init__(self, source: str, embit_source: str) -> None:
        requirements = [
            line.partition("#")[0].strip()
            for line in (Path(source) / "requirements.txt").read_text().splitlines()
        ]
        embit_requirements = [
            requirement
            for requirement in requirements
            if requirement.lower().startswith("embit")
        ]
        require_equal(
            "SeedSigner embit requirement",
            "embit==0.8.0",
            "\n".join(embit_requirements),
        )
        helper_path = (
            Path(source)
            / "src"
            / "seedsigner"
            / "helpers"
            / "mnemonic_generation.py"
        )
        with isolated_import(str(Path(embit_source) / "src"), "embit"):
            from embit.wordlists.bip39 import WORDLIST

            settings = types.ModuleType("seedsigner.models.settings_definition")

            class SettingsConstants:
                WORDLIST_LANGUAGE__ENGLISH = "en"

            settings.SettingsConstants = SettingsConstants
            seed = types.ModuleType("seedsigner.models.seed")

            class Seed:
                @staticmethod
                def get_wordlist(_language: str = "en"):
                    return WORDLIST

            seed.Seed = Seed
            with temporary_modules(
                {
                    settings.__name__: settings,
                    seed.__name__: seed,
                }
            ):
                spec = importlib.util.spec_from_file_location(
                    "reference_mnemonic_generation", helper_path
                )
                if spec is None or spec.loader is None:
                    raise RuntimeError("unable to load SeedSigner mnemonic helper")
                module = importlib.util.module_from_spec(spec)
                spec.loader.exec_module(module)
        self._module = module

    def from_coin_flips(self, flips: str) -> str:
        return " ".join(self._module.generate_mnemonic_from_coin_flips(flips))


@contextmanager
def isolated_import(path: str, prefix: str) -> Iterator[None]:
    matches = lambda name: name == prefix or name.startswith(f"{prefix}.")
    previous = {name: module for name, module in sys.modules.items() if matches(name)}
    for name in previous:
        del sys.modules[name]
    sys.path.insert(0, path)
    try:
        yield
    finally:
        sys.path.remove(path)
        for name in [name for name in sys.modules if matches(name)]:
            del sys.modules[name]
        sys.modules.update(previous)


@contextmanager
def temporary_modules(modules: dict[str, types.ModuleType]) -> Iterator[None]:
    missing = object()
    previous = {name: sys.modules.get(name, missing) for name in modules}
    sys.modules.update(modules)
    try:
        yield
    finally:
        for name, old_module in previous.items():
            if old_module is missing:
                del sys.modules[name]
            else:
                sys.modules[name] = old_module


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--core", required=True)
    parser.add_argument("--source", required=True)
    parser.add_argument("--embit", required=True)
    args = parser.parse_args()
    core = CoreDriver(args.core)
    seedsigner = SeedSigner(args.source, args.embit)

    for words, flips in [(12, COINS_12), (24, COINS_24)]:
        actual = require_accepted(
            f"SeedSigner {words}-word coin conversion",
            core.calculate("seedsigner-coins-v1", words, flips),
        )
        require_equal(
            f"SeedSigner {words}-word mnemonic",
            seedsigner.from_coin_flips(flips),
            actual.mnemonic,
        )
    print("validated SeedSigner and embit against core")


if __name__ == "__main__":
    main()
