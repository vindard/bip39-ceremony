# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Physical-entropy guidance (`t` on the safety checklist or during capture): the
  measured cost of dice bias, how often a die lands on the face it started on
  when it is not tumbled, what to do about it, how roll-count effects differ by
  method, and why a face-frequency guard cannot see a patterned tape. Every
  figure is cited.
- A "Throwing method fixed" safety attestation, so the throw is something the
  operator commits to before rolling rather than improvises mid-ceremony.
- `bitcoinlib-base6-v1`: a compatibility profile for implementations that read
  the roll tape as one base-6 integer and use it directly as BIP-39 entropy,
  with no SHA-256 and no rejection sampling, pinned to `RooSoft/bitcoinlib`.
  Reproduces that library's own 50-roll and 99-roll test vectors. It demands
  exactly 50 or 99 rolls — the counts that elsewhere signal a hashed
  construction — so it is included in group compare, where the same 99 rolls
  now show COLDCARD and this reading producing mnemonics with no words in
  common.
- Group compare covers eight protocols and checkpoints at 99 rolls.
- `bluewallet-bitpack-v1`: a compatibility profile for implementations that pack
  dice faces straight into entropy bits with no hash, pinned to BlueWallet
  8.0.1. Faces `1`-`4` carry two bits and `5`/`6` carry one, so the tape length
  depends on the faces rolled — 64 to 128 rolls for 12 words — and the roll that
  overshoots the width keeps only its leading bits. Vectors were reproduced from
  an independent transcription of the source reducer.
- Group compare's roll-count checkpoints now include wherever bit packing fills
  the entropy width on the tape in hand, which is the first checkpoint derived
  from the faces rolled rather than from the target.
- `iancoleman-dice-v1` and `iancoleman-raw-v1`, the two constructions the web
  tool produces from one dice tape depending on its length dropdown. Choosing a
  word count hashes the digits after the 6-to-0 rewrite — the same construction
  Keystone's legacy application uses, but offered at 12 words as well, which
  Keystone never was. Leaving the dropdown on its default packs base-6 bits and
  keeps only the trailing whole 32-bit groups, so the word count follows the
  tape and the first rolls can fall out of the seed entirely.
- `reference-iancoleman-dice` executes the tool's own `setMnemonicFromEntropy`
  for both branches and compares with the core driver.
- Both Ian Coleman dice profiles are included in group compare. The fixed-length
  hash appears at every set above its target minimum; raw mode appears at its
  face-dependent exact-width checkpoint. Later raw tapes are omitted because
  dropping a leading remainder would violate the rule that every set roll
  contributes.

### Changed

- The setup menus drop the blank line between rows once a list exceeds nine
  entries, so the whole protocol catalog still fits the minimum supported
  terminal without scrolling.

### Fixed

- Roll capture no longer advertises a next roll once the capture is closed. A
  fixed-count protocol that had met its count still showed `NEXT · #051` and
  "Roll once, observe, then press…", inviting an entry the ceremony would
  refuse. Open-ended captures, which really do accept more, are unchanged.
- The attempt-rejection screen wrapped its text to a fixed width instead of the
  terminal's, so on narrower terminals the card clipped every line. At the
  minimum supported 52 columns that included the remedy itself, which read
  "re-roll all 100 physical…". It now wraps to the available width like every
  other phase.

## [0.2.0] - 2026-08-08

Adds a group-compare mode for studying how the same entropy becomes different
seeds across wallets, sharpens the derivation view, and removes the
non-standard native-hash protocol from `bip39-ceremony-core`. No changes to the
conversion math of the protocols that remain.

### Added

- Group compare: roll ONE physical D6 tape, then replay it across the four
  dice protocols (Exact, COLDCARD, Keystone legacy, Word-by-word Exact) to see
  which accept the same entropy and what seed each produces. Results group into
  entropy sets at roll-count checkpoints — e.g. 166 rolls yields sets at
  {166, 140, 100} — because protocols complete at different lengths, and a
  protocol appears in a set only if it consumes exactly those rolls.
- Protocol details overlay (`[e]`) in group compare: the canonical, target-
  specific explanation for each of the four protocols, stepped through with
  `←/→`.
- Per-seed derivation overlay (`[d]`) in group compare: the same numbered
  BIP-39 breakdown as the single-protocol view (canonical input → entropy →
  checksum → 11-bit word indices → recovery words) for every accepted seed.

### Changed

- Derivation canonical-input display: the encoding is now shown as its own
  bold line above the raw input (e.g. `base-6 (0-5), msb-first (left-to-right),
  global rejection`) instead of a `label:content` prefix, making explicit that
  the encoding label is an annotation and never part of the hashed input.

### Removed

- `bip39-ceremony-core`: the non-standard **native-hash** protocol
  (`ConversionProtocol::NativeHashV1`, id `native-hash-v1`) and its supporting
  public `CanonicalInput` / `CanonicalInputKind` conditioning variants. It
  matched no external wallet and had no reference oracle, so it could not be
  cross-verified against an upstream implementation like the shipped protocols.
  **Breaking** for direct `bip39-ceremony-core` API consumers. The ceremony TUI
  never listed it as a selectable protocol, so end users are unaffected.

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

[Unreleased]: https://github.com/vindard/bip39-ceremony/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/vindard/bip39-ceremony/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/vindard/bip39-ceremony/releases/tag/v0.1.0
