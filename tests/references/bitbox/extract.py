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
    parser.add_argument("--bip39", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()

    firmware = Path(args.source)
    mnemonic_source = (
        firmware
        / "src"
        / "rust"
        / "bitbox02-rust"
        / "src"
        / "workflow"
        / "mnemonic.rs"
    ).read_text()
    function = extract_function(mnemonic_source, "fn lastword_choices(")
    if "Sha256::digest(&seed)" not in function:
        raise RuntimeError("unexpected BitBox02 checksum implementation")

    output = Path(args.output)
    (output / "src").mkdir(parents=True)
    (output / "Cargo.toml").write_text(
        f'''[package]\nname = "bitbox-lastword-adapter"\nversion = "0.1.0"\nedition = "2024"\n\n[dependencies]\nbip39 = {{ path = "{args.bip39}", features = ["std"] }}\nsha2 = "0.10"\nzeroize = {{ version = "1", features = ["alloc"] }}\n'''
    )
    (output / "src" / "main.rs").write_text(
        '''use sha2::{Digest, Sha256};
use std::vec::Vec;

mod bip39 {
    use zeroize::Zeroizing;

    pub fn get_word(idx: u16) -> Result<Zeroizing<String>, ()> {
        Ok(Zeroizing::new(
            ::bip39::Language::English
                .word_list()
                .get(idx as usize)
                .ok_or(())?
                .to_string(),
        ))
    }

    pub fn mnemonic_to_seed(mnemonic: &str) -> Result<Zeroizing<Vec<u8>>, ()> {
        let mnemonic = ::bip39::Mnemonic::parse_in_normalized(
            ::bip39::Language::English,
            mnemonic,
        )
        .map_err(|_| ())?;
        let (seed, seed_len) = mnemonic.to_entropy_array();
        Ok(Zeroizing::new(seed[..seed_len].to_vec()))
    }
}

'''
        + function
        + '''

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let entered_words: Vec<&str> = arguments.iter().map(String::as_str).collect();
    let choices = lastword_choices(&entered_words);
    println!(
        "{}",
        choices
            .iter()
            .map(|index| {
                let word = bip39::get_word(*index).unwrap();
                format!("{index}:{}", word.as_str())
            })
            .collect::<Vec<_>>()
            .join(",")
    );
}
'''
    )


if __name__ == "__main__":
    main()
