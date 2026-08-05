# BIP-39 Ceremony

A glass-box Rust TUI for turning physical dice rolls or coin flips into standard
12- or 24-word English BIP-39 mnemonics.

## Purpose

The primary goal is to make the entropy pipeline understandable and
independently inspectable—not merely to generate a mnemonic. The ceremony shows
how an ordered physical outcome sequence is converted into entropy, checksum
bits, word indices, and final words. It keeps the entropy accounting explicit:
conversion and hashing can transform or condition the observations, but cannot
create entropy.

The implementation follows the same goal. Calculations are isolated in the
independent [`bip39-ceremony-core`](crates/bip39-ceremony-core/) crate so they are
easy to read, test, verify, and reuse without the terminal interface. The
in-memory event history also makes every logical step of a ceremony available
for inspection without changing its live result.

## What you can inspect

After deliberate reveal, the TUI exposes:

- the entered roll or flip sequence and protocol-specific grouping;
- conversion details, including rejection decisions where applicable;
- the resulting entropy bits and BIP-39 checksum;
- each 11-bit word index and its corresponding English word; and
- the ceremony state and phase path at every event-prefix snapshot.

## Security boundary

Rolls, flips, entropy, intermediate derivations, and mnemonic words are
wallet-secret material. Prefer a trusted offline computer and local terminal.
The program avoids persistence and zeroizes owned secret buffers, but cannot
protect a compromised OS, swap, terminal recording, screenshots, or observers.

## Quick start

The repository pins Rust and audit tooling through Nix:

```sh
direnv allow       # automatic `nix develop`
# or: nix develop
just hooks         # install the repository pre-commit hook
just run
```

## Conversion protocols

- **COLDCARD:** SHA-256 over ASCII roll digits, compatible for an identical
  ordered sequence. Minimums are 50 rolls for 12 words and COLDCARD's documented
  99 rolls for 24 words; additional rolls are accepted.
- **Word-by-word Exact:** exact localized rejection over six-roll 11-bit
  candidates and a final entropy-tail candidate. Minimums are 70 or 140 rolls;
  rejected groups never discard accepted positions.
- **Exact:** exact base-6 rejection mapping. Uniform under independent fair dice;
  approximately 16% of 50-roll and 11% of 100-roll attempts reject.
- **Keystone legacy dice:** 24-word compatibility profile that maps face `6` to
  ASCII `0`, hashes at least 99 mapped rolls, and uses the full digest.
- **Jade direct words:** D16/D16/D8 triples select the first 11 or 23 BIP-39
  indices using [Blockstream's published table order][jade-guide]. A final D16/D8 or D8 roll
  supplies the remaining entropy bits before checksum calculation.
- **BitBox02 Diceware:** five D6 faces accepted only in `1`–`4` plus one coin
  side select each direct word using [BitBox02's published table][bitbox-guide].
  Rejected `5`/`6` attempts retry locally; final coins supply the entropy tail.
- **Krux D20:** at least 30 or 60 D20 rolls serialized as hyphen-separated
  decimal faces, matching [Krux's documented implementation][krux-guide].
  Additional rolls are accepted before hashing with SHA-256.
- **Coin + four-D6 direct words:** one coin and four ordered D6 faces select
  each direct index using the pinned [Bip39-diceware table][coin-four-d6-table].
  Whole rejected candidates retry; 12 accepted candidates provide 128 entropy
  bits before deterministic checksum replacement. Available for 12 words only.
- **SeedSigner coin flips:** exactly 128 or 256 physical flips serialized as
  ASCII `0`/`1`, hashed with SHA-256, then truncated for 12 words when needed.

[jade-guide]: https://help.blockstream.com/blockstream-jade/add-more-security-functionality/create-a-recovery-phrase-using-dice
[bitbox-guide]: https://blog.bitbox.swiss/en/roll-the-dice-generate-your-own-seed/
[krux-guide]: https://selfcustody.github.io/krux/getting-started/usage/generating-a-mnemonic/
[coin-four-d6-table]: https://github.com/taelfrinn/Bip39-diceware/blob/5320c9978fe89b5e068f6c0cafe45effe900e74c/README.md

All selectable protocols produce ordinary BIP-39 output. The protocol matters
only when reproducing a mnemonic from its original rolls. The frozen
`native-hash-v1` implementation and vectors remain in the source for protocol
compatibility, but it is no longer offered as a ceremony choice.

## Interaction

The ceremony is event sourced in memory. Press `Tab` or `i` to replace the live
workspace with a read-only ceremony inspection without changing the live result.
Moving its cursor reprojects the workspace and phase path at that historical
state:

- Left/Right or Home/End: navigate event-prefix snapshots
- Up/Down or Page Up/Page Down: scroll longer detail views
- `s`: return from a detail view to the ceremony projection
- `t`: semantic event timeline
- `d`: open the rolls-to-entropy-to-words derivation after mnemonic reveal
- `?`: explanation and controls
- `Tab`, `i`, or Escape: return to live state

The concealed-result phase gates every equivalent derived secret together:
entropy, checksum, indices, derivation words, and the mnemonic become inspectable
only after deliberate reveal.

Choice menus use Up/Down or `j`/`k` to move the highlighted row. Left/Right or
`h`/`l` move between mnemonic-length and conversion-protocol setup steps; Left
or `h` also returns from safety to protocol selection. On protocol selection,
`e` explains whichever protocol is highlighted, including wallet compatibility
profiles and target-limited rows. Up/Down scrolls one row, Page
Up/Page Down scrolls a page, and a right-border rail shows position. The
Safety preflight uses Up/Down to select an acknowledgement, Space to toggle it,
and `c` to check all; Enter advances after all five are checked. Numeric shortcuts
are intentionally not accepted.

Physical input uses a fixed capture workspace. D6 protocols accept `1` through
`6`; SeedSigner coin capture accepts `0` for tails and `1` for heads. Jade's
mixed-dice capture accepts `1`–`9` and uppercase `A`–`G` for D16 faces 1–16,
then `1`–`8` for D8 faces. BitBox02 capture alternates stage-aware D6 keys with
`0` tails / `1` heads; its lookup selector maps heads to 0 and tails to 1.
Coin + four-D6 capture requests `0` tails / `1` heads, then four `1`–`6`
faces; a rejected tails tuple retries the complete five-outcome candidate.
Krux D20 accepts `1`–`9` and uppercase `A`–`K` for faces 10–20. Backspace
removes only the latest observation, while
the next position and remaining count
stay stationary. Inputs are secret, so prior values are hidden by default while
the numbered latest outcome remains visible; `h` toggles the complete zero-padded
ledger. Repeats and patterns are explicitly valid. Generation and mnemonic
reveal are separate. The 52×40 minimum layout keeps contextual controls visible
and presents all 24 words in stable numbered columns without scrolling. After
manual comparison, `v` records that transcription was marked checked; it does not
validate the backup. On the reveal screen, `h` immediately replaces the words
with a quick-hidden view; `h` or Escape restores them. Cancelling or finishing
requires confirmation and persists nothing.

## Visual themes

The warm `ember` palette is the sole visual theme. It preserves the terminal's
background and uses gold, amber, and coral for semantic emphasis. Run
`just run-plain`, set `BIP39_CEREMONY_THEME=plain`, or use a non-empty `NO_COLOR`
for the structurally equivalent colorless presentation. `TERM=dumb` also selects
plain mode. Labels, symbols, selection markers, and secret warnings remain
complete without color.

The interface normally uses one full-height workspace card. Inspection, help,
timelines, and derivation replace that card on demand. Protocol explanation is
the deliberate exception: the selected protocol list remains in an upper card
while its scrollable explanation opens below, with a visible return control.
After reveal, opening inspection replaces the mnemonic with an explicit
concealment view. Historical projections remain read-only. Overflow stays inside
the workspace and is navigated with Page Up/Page Down. The phase path uses `✓`,
`●`, and `○` for complete, current, and future stages.

## Architecture and testing

Reusable capture values, versioned conversions, SHA-256 conditioning, BIP-39
encoding, and structured calculation evidence live in the publishable
[`bip39-ceremony-core`](crates/bip39-ceremony-core/) workspace library. It has no
ceremony, presentation, or terminal dependencies. Protocol-mandated
cryptography is concrete so callers cannot substitute an incompatible hash
while retaining a compatibility label.

The unpublished application owns the event-sourced `Ceremony`, safety and reveal
policy, explanatory wording, and terminal interface. Unit tests cover pure
calculation and domain paths, while consumer-style vector tests verify the core
crate's public boundary.

## Development environment

Useful commands inside `nix develop` or direnv:

```sh
just                     # list commands
just check               # format, Clippy, tests, package and privacy lint
just security            # RustSec, licenses, sources and duplicate versions
just precommit           # complete local gate
just build               # debug binary
just release             # local optimized build; not a release artifact
just release-gnu         # Nix GNU/Linux feasibility artifact and PTY smoke
just release-musl        # Nix static-musl feasibility artifact and PTY smoke
just release-feasibility # build and test all host feasibility outputs
just guix-lock-lint      # compare Guix crate origins with Cargo.lock
just deps                # reviewed normal dependency graph
```

The supported first-iteration platforms are Linux and macOS. The flake's package
outputs are release-feasibility probes, not an accepted release pipeline or an
independent reproducibility claim. On Linux, `gnu` tests dynamic glibc linkage
and `musl` tests static PIE linkage; both consume hash-checked Cargo inputs from
`Cargo.lock` and run the real test suite during their Nix builds.
