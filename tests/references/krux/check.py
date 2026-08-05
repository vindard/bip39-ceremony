#!/usr/bin/env python3

import argparse
import importlib.util
import sys
import types
from contextlib import contextmanager
from pathlib import Path
from typing import Iterator

from outcome import CoreDriver, Status, require_accepted, require_equal, require_status


KNOWN_MNEMONICS = {
    12: "erupt remain ride bleak year cabin orange sure ghost gospel husband oppose",
    24: (
        "fun island vivid slide cable pyramid device tuition only essence thought gain "
        "silk jealous eternal anger response virus couple faculty ozone test key vocal"
    ),
}


class KruxDiceEntropy:
    def __init__(self, source: str) -> None:
        source_path = (
            Path(source)
            / "src"
            / "krux"
            / "pages"
            / "new_mnemonic"
            / "dice_rolls.py"
        )
        with krux_dependencies():
            spec = importlib.util.spec_from_file_location(
                "krux.pages.new_mnemonic.dice_rolls", source_path
            )
            if spec is None or spec.loader is None:
                raise RuntimeError("unable to load Krux dice implementation")
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
        self._dice_entropy = module.DiceEntropy
        self.minimums = {
            12: module.D20_12W_MIN_ROLLS,
            24: module.D20_24W_MIN_ROLLS,
        }

    def capture(
        self, rolls: list[int], words: int, premature_go_at: int | None = None
    ) -> tuple[bytes, int, list[str]]:
        ctx = types.SimpleNamespace(
            display=Display(), input=types.SimpleNamespace(wait_for_button=lambda: None)
        )
        instance = self._dice_entropy(ctx, is_d20=True)
        expected_states = [str(value) for value in range(1, 21)]
        if instance.roll_states != expected_states:
            raise AssertionError(f"unexpected Krux D20 states: {instance.roll_states}")
        instance.choose_len_mnemonic = lambda: words
        prompts = []

        def approve(prompt: str, *_args, **_kwargs) -> bool:
            prompts.append(prompt)
            return True

        instance.prompt = approve
        observations = [str(roll) for roll in rolls]
        if premature_go_at is not None:
            observations.insert(premature_go_at, "")
        observations.append("")
        entered = iter(observations)
        instance.capture_from_keypad = lambda *_args, **_kwargs: next(entered)
        flashes = []
        instance.flash_text = lambda text: flashes.append(text)

        with krux_dependencies():
            entropy = instance.new_key()
        if not isinstance(entropy, bytes):
            raise AssertionError("Krux did not return captured entropy")
        return entropy, len(flashes), prompts


class Display:
    def draw_hcentered_text(self, *_args, **_kwargs) -> int:
        return 0

    def draw_centered_text(self, *_args, **_kwargs) -> None:
        pass

    def clear(self) -> None:
        pass


@contextmanager
def krux_dependencies() -> Iterator[None]:
    class Page:
        def __init__(self, ctx: object, _menu: object = None) -> None:
            self.ctx = ctx

    class Menu:
        def __init__(self, *_args, **_kwargs) -> None:
            pass

        def run_loop(self) -> tuple[int, None]:
            return 1, None

    pages = package("krux.pages")
    pages.Page = Page
    pages.Menu = Menu
    pages.MENU_EXIT = 0
    pages.ESC_KEY = "escape"

    modules = {
        "krux": package("krux"),
        "krux.pages": pages,
        "krux.pages.new_mnemonic": package("krux.pages.new_mnemonic"),
        "krux.themes": module(
            "krux.themes",
            theme=types.SimpleNamespace(
                highlight_color=0, error_color=0, info_bg_color=0
            ),
        ),
        "krux.krux_settings": module("krux.krux_settings", t=lambda text: text),
        "krux.display": module(
            "krux.display",
            DEFAULT_PADDING=0,
            FONT_HEIGHT=1,
            TOTAL_LINES=20,
            BOTTOM_PROMPT_LINE=0,
        ),
        "krux.kboard": module(
            "krux.kboard", kboard=types.SimpleNamespace(has_minimal_display=False)
        ),
        "krux.settings": module("krux.settings", ELLIPSIS="..."),
    }
    previous = {name: sys.modules.get(name) for name in modules}
    sys.modules.update(modules)
    try:
        yield
    finally:
        for name, old_module in previous.items():
            if old_module is None:
                del sys.modules[name]
            else:
                sys.modules[name] = old_module


def package(name: str) -> types.ModuleType:
    value = types.ModuleType(name)
    value.__path__ = []
    return value


def module(name: str, **attributes: object) -> types.ModuleType:
    value = types.ModuleType(name)
    for key, attribute in attributes.items():
        setattr(value, key, attribute)
    return value


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--core", required=True)
    parser.add_argument("--source", required=True)
    args = parser.parse_args()

    core = CoreDriver(args.core)
    krux = KruxDiceEntropy(args.source)
    require_equal("Krux 12-word minimum", "30", str(krux.minimums[12]))
    require_equal("Krux 24-word minimum", "60", str(krux.minimums[24]))

    def varied_rolls(count: int) -> list[int]:
        state = 0xC0FFEE
        rolls = []
        for _ in range(count):
            state = (1_103_515_245 * state + 12_345) % (2**31)
            rolls.append((state >> 16) % 20 + 1)
        return rolls

    vectors = [
        (12, [1] * 30, KNOWN_MNEMONICS[12], 29),
        (12, varied_rolls(43), None, None),
        (24, [1] * 60, KNOWN_MNEMONICS[24], 59),
        (24, varied_rolls(77), None, None),
    ]
    for words, rolls, known_mnemonic, premature_go_at in vectors:
        observations = ",".join(str(roll) for roll in rolls)
        actual = require_accepted(
            f"Krux {words}-word D20 conversion",
            core.calculate("krux-d20-v1", words, observations),
        )
        entropy, flashes, prompts = krux.capture(rolls, words, premature_go_at)
        require_equal(
            f"Krux {words}-word entropy",
            entropy.hex(),
            actual.entropy,
        )
        if premature_go_at is not None and flashes != 1:
            raise AssertionError(
                f"Krux {words}-word minimum gate flashed {flashes} times"
            )
        if known_mnemonic is None and len(prompts) != 1:
            raise AssertionError(
                f"Krux {words}-word varied capture unexpectedly warned: {prompts}"
            )
        if known_mnemonic is not None:
            require_equal(
                f"Krux {words}-word known mnemonic",
                known_mnemonic,
                actual.mnemonic,
            )

    require_status(
        "Krux short 12-word capture",
        core.calculate("krux-d20-v1", 12, ",".join(["1"] * 29)),
        Status.INVALID,
        "observation-count",
        ("30", "29"),
    )
    require_status(
        "Krux short 24-word capture",
        core.calculate("krux-d20-v1", 24, ",".join(["1"] * 59)),
        Status.INVALID,
        "observation-count",
        ("60", "59"),
    )
    print("validated Krux D20 capture conversion against core")


if __name__ == "__main__":
    main()
