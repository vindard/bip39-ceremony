# Upstream reference validation

This suite compares `bip39-ceremony-core` with pinned upstream implementations.
The `harness` directory contains only the machine adapter and shared outcome
protocol. Each other directory owns the loading, vectors, and assertions for the
upstream implementation named by that directory.

## Sources and validation boundaries

`flake.lock` is authoritative for every revision and Nix content hash. The
current pinned executable boundaries are:

- **Coldcard firmware**
  ([source](https://github.com/Coldcard/firmware/tree/c849c4e04a978335937a0fd0c96e76f5bd70bbb6)):
  executes `shared/seed.py::add_dice_rolls` and `approve_word_list` for count,
  strict 30% distribution rejection, ASCII hashing, and target truncation.
- **SeedSigner**
  ([source](https://github.com/SeedSigner/seedsigner/tree/1fb2956322ea978428a6a96b955baa93e965c877)):
  executes `generate_mnemonic_from_coin_flips` with SeedSigner's declared
  `embit==0.8.0` dependency
  ([source](https://github.com/diybitcoinhardware/embit/tree/84cce66fb831fa6d625fb73f28e03605f3c04e28)).
- **Krux v26.08.0**
  ([source](https://github.com/selfcustody/krux/tree/ec058d862b00a62bd06ec7932b31f007fc2b77e3)):
  executes `DiceEntropy.new_key` for D20 states, minimum gating, decimal-hyphen
  serialization, optional rolls, SHA-256, and target truncation after simulated
  keypad input.
- **BitBox02 firmware v9.26.4**
  ([source](https://github.com/BitBoxSwiss/bitbox02-firmware/tree/6c18aa9cebcc457c3c5cd2c36ce58268a16bede5)):
  executes the exact extracted `lastword_choices` function with its pinned
  `rust-bip39` revision
  ([source](https://github.com/BitBoxSwiss/rust-bip39/tree/d69f68c837ee7962a26619316fb7a725e2e8d44c)).
  It exhaustively validates the 128/8 checksum candidates and entropy-tail
  ordering, not the external printed dice table.
- **Legacy Keystone Android application**
  ([source](https://github.com/KeystoneHQ/Keystone-cold-app/tree/34e638fa57aed6a54051f9fe065d501c3e129581)):
  executes the exact extracted `RollingDiceFragment.onCompleteClick`,
  `SetupVaultViewModel.generateMnemonicFromDiceRolls`, and `HashUtil.sha256`
  methods. This covers the 50-roll gate, confirmation through 98, the 99-roll
  recommendation boundary, face `6` mapping to ASCII `0`, and hashing.
- **Jade firmware 1.0.40**
  ([source](https://github.com/Blockstream/Jade/tree/6f858f39a19f89ff7fd4580c5b2db72cfe1dc0af)):
  executes the exact extracted `valid_final_words` function against Jade's
  pinned libwally
  ([source](https://github.com/ElementsProject/libwally-core/tree/43b97bed2e5b6347a909bfd1113242528826a8a2))
  and secp256k1
  ([source](https://github.com/BlockstreamResearch/secp256k1-zkp/tree/6152622613fdf1c5af6f31f74c427c4e9ee120ce))
  revisions. It exhaustively validates checksum candidates and final-word
  ordering, not the external D16/D8 table.
- **RooSoft/bitcoinlib**
  ([source](https://github.com/RooSoft/bitcoinlib/tree/a998a61caad66d074772ec4a10ba5268aa65ca40)):
  executes `BitcoinLib.Key.HD.Entropy.from_dice_rolls/1` under Elixir. That
  module depends on nothing outside the standard library, so the adapter loads
  it alone. Upstream returns an integer and stops there, so the check asserts
  the two things this project adds: accepted entropy is that integer in
  big-endian bytes, and a value whose minimal encoding is not exactly the target
  width is rejected rather than padded or truncated.
- **Ian Coleman BIP-39 tool**
  ([source](https://github.com/iancoleman/bip39/tree/de71c22328b24e0848bbe1bd12ac8974ca83b5b8)):
  executes its entropy-to-English-mnemonic encoder using independently fixed
  entropy and mnemonic vectors. It validates the shared BIP-39 boundary, not a
  physical-capture protocol.

## Checks

Checks are independently addressable:

- `reference-harness`
- `reference-coldcard`
- `reference-seedsigner`
- `reference-krux`
- `reference-bitbox-checksum`
- `reference-keystone-legacy`
- `reference-jade-checksum`
- `reference-iancoleman-bip39`
- `reference-bitcoinlib-base6`

`reference-implementations` composes the eight upstream comparisons; the harness
remains an independently targeted infrastructure check. Profiles without an
executable upstream artifact are not included as reference validations.

## Boundaries without an executable oracle

Protocol requirements without an upstream oracle remain in
`crates/bip39-ceremony-core/tests/` rather than this suite. In particular:

- Jade and BitBox02 firmware do not implement their published physical-dice
  mappings. Their checks are deliberately limited to executable
  checksum-completion and final-word ordering functions. The Jade adapter links
  the exact libwally and secp256k1 revisions pinned by Jade.
- The pinned Coin + four-D6 project contains a static table, not executable
  conversion code.
- The Keystone legacy check executes its Android UI completion branches at
  49/50/98/99 rolls, then separately executes the post-capture dice mapping and
  hashing method.
- Exact and Word Exact are project-owned protocols without an independent
  upstream implementation.
