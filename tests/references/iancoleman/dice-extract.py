#!/usr/bin/env python3
"""Extracts iancoleman's `setMnemonicFromEntropy` verbatim from pinned source.

That function is where the tool decides what a dice tape becomes: with a fixed
word count it hashes the cleaned digits, and with the default "raw" setting it
uses the packed bits and takes the trailing whole 32-bit groups. Both branches
matter here, and neither is reachable without the function, so it is lifted out
byte-for-byte and the runner stubs the DOM around it.
"""

import argparse
from pathlib import Path

SIGNATURE = "    function setMnemonicFromEntropy() {"


def extract_function(source: str, signature: str) -> str:
    start = source.index(signature)
    depth = 0
    saw_body = False
    for position in range(start, len(source)):
        character = source[position]
        if character == "{":
            depth += 1
            saw_body = True
        elif character == "}":
            depth -= 1
            if saw_body and depth == 0:
                return source[start : position + 1]
    raise RuntimeError(f"unterminated upstream function: {signature}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()

    text = Path(args.source, "src/js/index.js").read_text(encoding="utf-8")
    body = extract_function(text, SIGNATURE)
    Path(args.output).write_text(body + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
