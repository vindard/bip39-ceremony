#!/usr/bin/env python3

import argparse
from pathlib import Path


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

    source = Path(args.source) / "main/process/mnemonic.c"
    function = extract_function(source.read_text(), "static size_t valid_final_words(")
    if "bip39_mnemonic_validate(NULL, buf)" not in function:
        raise RuntimeError("unexpected Jade final-word implementation")

    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        '''#include <assert.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <wally_bip39.h>

#define JADE_ASSERT(value) assert(value)
#define MNEMONIC_BUFLEN 216
#define BIP39_WORDLIST_LEN 2048

'''
        + function
        + '''

int main(int argc, char** argv)
{
    JADE_ASSERT(argc == 12 || argc == 24);
    const size_t num_words = (size_t)argc - 1;
    size_t choices[128];
    const size_t count = valid_final_words(
        (const char**)&argv[1], num_words, choices, sizeof(choices) / sizeof(choices[0]));
    for (size_t i = 0; i < count; ++i) {
        const char* word = bip39_get_word_by_index(NULL, choices[i]);
        JADE_ASSERT(word);
        printf("%s%zu:%s", i ? "," : "", choices[i], word);
    }
    putchar('\\n');
    return 0;
}
'''
    )


if __name__ == "__main__":
    main()
