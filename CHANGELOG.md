# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-08-06

First release — a personal tool built for two reasons: to actually understand
how the entropy behind a seed is produced (and how differently each wallet does
it), and to have something I'd trust on an offline, throwaway or air-gapped
machine to generate a hardware-wallet seed from dice rolls — where I supply the
entropy and watch it become words, instead of trusting a device RNG I can't see.

Deliberately a "works, not polished" first version. The goal wasn't a finished
product; it was something that works, is well tested, and is released so you can
verify it.

### Added

- Glass-box BIP-39 ceremony TUI: physical dice/coin outcomes → entropy →
  checksum → 11-bit word indices → mnemonic, with explicit entropy accounting
  (conversion and hashing can condition observations but never manufacture
  entropy).
- Nine conversion protocols: COLDCARD, Word-by-word Exact, Exact, Keystone
  legacy dice, Jade direct words, BitBox02 Diceware, Krux D20, Coin + four-D6,
  and SeedSigner coin flips.
- Reference-validation suite that cross-checks each wallet-compatible protocol
  against the wallet's own upstream code — COLDCARD, SeedSigner, Krux, BitBox02,
  Keystone, Jade — plus the Ian Coleman BIP-39 boundary, so "compatible" means
  verified, not asserted.
- Reproducible, verifiable release: a static-PIE `musl` Linux binary, a
  `SHA256SUMS` manifest signed with a hardware (YubiKey) PGP key, cosign keyless
  signatures, and SLSA build provenance — byte-for-byte rebuildable from source
  with Nix. See [`docs/verifying-releases.md`](docs/verifying-releases.md).

### Notes

- Personal tool, offered as-is. Run it on a trusted offline machine and treat
  everything it displays as wallet-secret material.
- Linux `x86_64` only for now (static `musl`); macOS builds from source
  (`nix run`); arm64 binaries and a crates.io publish of `bip39-ceremony-core`
  are follow-ups.

[Unreleased]: https://github.com/vindard/bip39-ceremony/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/vindard/bip39-ceremony/releases/tag/v0.1.0
