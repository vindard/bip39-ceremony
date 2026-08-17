#!/usr/bin/env python3
"""Extracts BlueWallet's dice entropy functions verbatim from pinned source.

`ProvideEntropy.tsx` imports React Native, so the file cannot be loaded as is.
The pure pieces that decide the protocol are lifted out with their bodies
byte-for-byte unchanged: the face-to-bits split, the accumulator that packs and
truncates, and the byte conversion. Nothing is rewritten here — the emitted file
is still TypeScript, and esbuild erases the annotations, so what runs is
upstream's logic rather than a restatement of it.
"""

import argparse
from pathlib import Path

BINDINGS = (
    "const initialState",
    "const shiftLeft = ",
    "const shiftRight = ",
    "export const eReducer = ",
    "export const getEntropy = ",
    "export const convertToBuffer = ",
)

PREAMBLE = """// Supplied by the runner so the extracted bodies keep their own reference.
declare const globalThis: any;
const BN = globalThis.__BN;
"""

EXPORTS = "module.exports = { EActionType, eReducer, getEntropy, convertToBuffer };\n"


def extract_binding(source: str, signature: str) -> str:
    """Takes one top-level `const x = ...;` binding, balancing brackets."""
    start = source.index(signature)
    depth = 0
    for position in range(start, len(source)):
        character = source[position]
        if character in "{([":
            depth += 1
        elif character in "})]":
            depth -= 1
        elif character == ";" and depth == 0:
            return source[start : position + 1]
    raise RuntimeError(f"unterminated upstream binding: {signature}")


def extract_enum(source: str) -> str:
    start = source.index("export enum EActionType")
    end = source.index("}", start) + 1
    return source[start:end]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()

    text = Path(args.source, "screen/wallets/ProvideEntropy.tsx").read_text(encoding="utf-8")

    parts = [PREAMBLE, extract_enum(text).replace("export enum", "enum")]
    for signature in BINDINGS:
        parts.append(extract_binding(text, signature).replace("export const", "const"))
    parts.append(EXPORTS)

    Path(args.output).write_text("\n\n".join(parts), encoding="utf-8")


if __name__ == "__main__":
    main()
