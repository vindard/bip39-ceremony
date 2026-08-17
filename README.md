<div align="center">

# 🎲 BIP-39 Ceremony

**A glass-box Rust TUI for turning physical dice rolls and coin flips into
standard 12- or 24-word English BIP-39 mnemonics.**

### _“Don't trust, verify.”_

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
&nbsp;[![Rust](https://img.shields.io/badge/Rust-1.94%2B-orange?logo=rust&logoColor=white)](https://www.rust-lang.org)
&nbsp;[![unsafe: forbidden](https://img.shields.io/badge/unsafe-forbidden-success)](Cargo.toml)
&nbsp;[![BIP-39](https://img.shields.io/badge/BIP--39-compatible-f7931a?logo=bitcoin&logoColor=white)](https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki)
&nbsp;[![Built with Nix](https://img.shields.io/badge/built%20with-Nix-5277C3?logo=nixos&logoColor=white)](flake.nix)

[![CI](https://github.com/vindard/bip39-ceremony/actions/workflows/ci.yml/badge.svg)](https://github.com/vindard/bip39-ceremony/actions/workflows/ci.yml)
&nbsp;[![Security](https://github.com/vindard/bip39-ceremony/actions/workflows/security.yml/badge.svg)](https://github.com/vindard/bip39-ceremony/actions/workflows/security.yml)
&nbsp;[![Reproducibility](https://github.com/vindard/bip39-ceremony/actions/workflows/reproducibility.yml/badge.svg)](https://github.com/vindard/bip39-ceremony/actions/workflows/reproducibility.yml)
&nbsp;[![OpenSSF Scorecard](https://api.securityscorecards.dev/projects/github.com/vindard/bip39-ceremony/badge)](https://securityscorecards.dev/viewer/?uri=github.com/vindard/bip39-ceremony)

<br>

<img src="docs/img/ceremony.gif" alt="Animated walkthrough of the ceremony: setup, safety preflight, dice capture, reveal, and the derivation pipeline" width="820">

<sub><i>Setup → safety → dice capture → reveal → derivation. (COLDCARD with example dice; the shown mnemonic is a throwaway demo seed — never use it.)</i></sub>

</div>

## 🔥 Why this exists

Seed **entropy** is the one step of self-custody almost nobody can verify — you
trust the device's random number generator. This tool makes it a **glass box**:
entropy you generate physically (dice, coins) and watch become checksum,
indices, and words. Nothing to trust, all of it to verify — and the output is
standard BIP-39, compatible with the wallets you already use.

> [!IMPORTANT]
> **July 2026 — that trust broke.** A Coldcard firmware bug shipped in March
> 2021 and unnoticed for four years fed seed generation from a deterministic
> software PRNG instead of the hardware RNG, collapsing effective key strength to
> as little as 40 bits — brute-forceable without ever touching the wallet. By
> early August 2026, roughly **1,367 BTC ($89M)** had been drained from thousands
> of addresses (forensics firms later put the total above $100M). The owners who
> were spared had mixed in their **own dice-roll entropy**, refusing to trust the
> device's RNG alone.
>
> <sub>Sources, Jul–Aug 2026: [The Hacker News](https://thehackernews.com/2026/08/coldcard-hardware-wallet-flaw-linked-to.html) · [TRM Labs](https://www.trmlabs.com/resources/blog/the-largest-hardware-wallet-exploit-of-2026-inside-the-usd-116-million-coldcard-hack) · [TechCrunch](https://techcrunch.com/2026/08/04/hackers-steal-over-130-million-by-exploiting-bug-in-offline-hardware-wallets/)</sub>

## 🎯 Purpose

The primary goal is to make the entropy pipeline understandable and
independently inspectable—not merely to generate a mnemonic. The ceremony shows
how an ordered physical outcome sequence is converted into entropy, checksum
bits, word indices, and final words. It keeps the entropy accounting explicit:
conversion and hashing can transform or condition the observations, but cannot
create entropy.

The implementation follows the same goal. Calculations are isolated in the
independent [`bip39-ceremony-core`](crates/bip39-ceremony-core/) crate so they are
easy to read, test, verify, and reuse without the terminal interface.

## 🔍 What you can inspect

After deliberate reveal, the TUI exposes:

- the entered roll or flip sequence and protocol-specific grouping;
- conversion details, including rejection decisions where applicable;
- the resulting entropy bits and BIP-39 checksum;
- each 11-bit word index and its corresponding English word.

<table>
<tr>
<td width="50%">
<img src="docs/img/roll-capture.png" alt="Roll-capture workspace with a live SHA-256 preview and zero-padded roll ledger">
<br><sub><i>Physical capture: numbered positions, a live running hash, and the
zero-padded ledger (hidden by default; toggle with <code>h</code>).</i></sub>
</td>
<td width="50%">
<img src="docs/img/reveal.png" alt="Reveal screen showing the numbered 12-word grid and transcription-check guidance">
<br><sub><i>Deliberate reveal: the numbered word grid gates behind an explicit
step, with transcription and backup-verification guidance.</i></sub>
</td>
</tr>
</table>

## 🎲 How you throw

Every protocol here is deterministic, so all of the unpredictability has to come
from the dice — and that is the one part no software can check. Press `t` on the
safety checklist or during capture for the figures; the short version:

- **The die barely matters.** The largest bias measured in ordinary dice is about
  1.4% ([Labby 2009][labby], 315,672 rolls; [Iversen et al. 1971][iversen],
  4,380,000 throws). At 24 words a 2% bias costs 2.9 bits of 255.9. Buying casino
  dice does not buy a better seed.
- **The throw matters a lot.** A die dropped without bouncing ends up with the
  same face down as it started **54.8%** of the time; with 4–5 bounces, 19.9%,
  against 16.7% for a fair die ([Kapitaniak et al. 2012][kapitaniak]). Fair by
  symmetry is not fair by dynamics.
- **So: give every roll a thorough tumble.** A closed box with room to shake
  bounces the dice off the walls and each other — the part a hand throw has to
  get right and often does not.
- **More rolls do not help.** The roll count fills the seed rather than
  exceeding a floor: 99 fair rolls carry 255.9 of 256 bits. For protocols that
  hash every roll, extra rolls make a *different* seed, not a stronger one.
- **Face-frequency guards prove nothing.** COLDCARD's 30%-per-face rejection and
  Shannon-entropy meters see *which* faces appeared, not what order. A repeating
  `1 2 3 4 5 6 …` tape has a flat distribution and passes every such check while
  being entirely predictable.

[kapitaniak]: https://doi.org/10.1063/1.4746038
[labby]: https://doi.org/10.1080/09332480.2009.10722977
[iversen]: https://doi.org/10.1007/BF02291418

## 🛡️ Security boundary

Rolls, flips, entropy, derivations, and words are wallet-secret material —
prefer a trusted offline computer and local terminal.

| ✅ The program does | 🚫 It cannot protect against |
| --- | --- |
| Persists nothing — no writes to disk | A compromised OS or malware |
| Zeroizes owned secret buffers | Swap / hibernation paging memory to disk |
| Keeps secrets in memory for the ceremony only | Terminal recording, scrollback, screenshots |
| Gates every derived secret behind a deliberate reveal | Cameras, shoulder-surfing, other observers |

## 🚀 Quick start

The repository pins Rust and audit tooling through Nix:

```sh
direnv allow       # automatic `nix develop`
# or: nix develop
just hooks         # install the repository pre-commit hook
just run
```

## 📦 Install & verify

Run it straight from source with Nix (no download to trust):

```sh
nix run github:vindard/bip39-ceremony
```

Or download the reproducible static-`musl` Linux binary (x86_64, runs on any
distro) from a
[release](https://github.com/vindard/bip39-ceremony/releases). Releases carry
checksums, a maintainer **PGP signature** on a hardware key (fingerprint
`F1E2 DF8F 56F8 8D73 4181  1B92 1B00 5D83 8F95 D90A`), cosign keyless signatures,
and SLSA build provenance — and the build is bit-for-bit reproducible from this
tag. See [**Verifying releases**](docs/verifying-releases.md).

## 🎲 Conversion protocols

| Protocol | 🎯 Target | Input | Basis & notes |
| --- | :---: | --- | --- |
| **COLDCARD** | 12 · 24 | D6 | SHA-256 over ASCII roll digits, compatible for an identical ordered sequence. Minimums are 50 rolls for 12 words and COLDCARD's documented 99 rolls for 24 words; additional rolls are accepted. |
| **Word-by-word Exact** | 12 · 24 | D6 | Exact localized rejection over six-roll 11-bit candidates and a final entropy-tail candidate. Minimums are 70 or 140 rolls; rejected groups never discard accepted positions. |
| **Exact** | 12 · 24 | D6 | Exact base-6 rejection mapping. Uniform under independent fair dice; ≈16% of 50-roll and ≈11% of 100-roll attempts reject. |
| **Keystone legacy dice** | 24 | D6 | 24-word compatibility profile that maps face `6` to ASCII `0`, permits completion from 50 mapped rolls, recommends continuing to 99, and uses the full digest. |
| **Jade direct words** | 12 · 24 | D16/D8 | D16/D16/D8 triples select the first 11 or 23 BIP-39 indices using [Blockstream's published table order][jade-guide]. A final D16/D8 or D8 roll supplies the remaining entropy bits before checksum calculation. |
| **BitBox02 Diceware** | 12 · 24 | D6 + coin | Reproduces BitBox02's external printed-table workflow; the firmware does not capture the dice rolls. Five D6 faces accepted only in `1`–`4` plus one coin side select each direct word using the [published table][bitbox-guide]. Rejected `5`/`6` retry locally; final coins supply the entropy tail before the words are entered on the device. |
| **Krux D20** | 12 · 24 | D20 | At least 30 or 60 D20 rolls serialized as hyphen-separated decimal faces, matching [Krux's documented implementation][krux-guide]. Additional rolls are accepted before hashing with SHA-256. |
| **Coin + four-D6 direct words** | 12 | D6 + coin | One coin and four ordered D6 faces select each direct index using the pinned [Bip39-diceware table][coin-four-d6-table]. Whole rejected candidates retry; 12 accepted candidates provide 128 entropy bits before deterministic checksum replacement. |
| **SeedSigner coin flips** | 12 · 24 | Coin | Exactly 128 or 256 physical flips serialized as ASCII `0`/`1`, hashed with SHA-256, then truncated for 12 words when needed. |
| **bitcoinlib base-6 (no hash)** | 12 · 24 | D6 | Reads exactly 50 or 99 rolls as one base-6 integer and uses it directly as entropy — no SHA-256 and no rejection sampling — matching [RooSoft/bitcoinlib][bitcoinlib-base6]. Not uniform. A reading whose minimal big-endian encoding is not exactly 16 or 32 bytes is rejected rather than padded or truncated; roughly three in five 50-roll tapes reject. |
| **BlueWallet bit packing** | 12 · 24 | D6 | Packs faces straight into entropy bits with no hash, matching [BlueWallet][bluewallet-bitpack]: faces `1`–`4` give two bits (`00`–`11`), faces `5`/`6` give one (`0`/`1`), ≈1.67 bits per roll. The tape length varies with the faces rolled — 64–128 rolls for 12 words — and the roll that overshoots keeps only its leading bits. Requires the full width; BlueWallet fills a shortfall from the phone's RNG, which is not reproducible. |

[jade-guide]: https://help.blockstream.com/blockstream-jade/add-more-security-functionality/create-a-recovery-phrase-using-dice
[bitbox-guide]: https://blog.bitbox.swiss/en/roll-the-dice-generate-your-own-seed/
[krux-guide]: https://selfcustody.github.io/krux/getting-started/usage/generating-a-mnemonic/
[coin-four-d6-table]: https://github.com/taelfrinn/Bip39-diceware/blob/5320c9978fe89b5e068f6c0cafe45effe900e74c/README.md
[bitcoinlib-base6]: https://github.com/RooSoft/bitcoinlib/blob/a998a61caad66d074772ec4a10ba5268aa65ca40/lib/key/hd/entropy.ex#L41-L79
[bluewallet-bitpack]: https://github.com/BlueWallet/BlueWallet/blob/8.0.1/screen/wallets/ProvideEntropy.tsx#L103-L125

All selectable protocols produce ordinary BIP-39 output. The protocol matters
only when reproducing a mnemonic from its original rolls.

### 🧪 Group compare — one entropy set, many wallets

Press `g` on the protocol screen to enter **group compare**. You roll a single
physical D6 tape, and every D6-native protocol is replayed against that *same*
entropy so you can line up the seeds each wallet's method produces from
identical rolls.

A protocol only belongs to a set if it consumes **exactly** those rolls, all of
them, with none rejected. Because the protocols complete at different lengths,
results group into **entropy sets at roll-count checkpoints** — the current tape
length plus each length where a fixed-count protocol completes (`100` for the
strict protocols, `140` for `Word-by-word Exact` at 24 words, and `99` for the
unhashed base-6 reading), plus wherever `BlueWallet bit packing` fills the
entropy width on *this* tape. That checkpoint depends on the faces rolled rather
than only the target. A fair 166-roll tape **gives five sets** — `166`, `153`,
`140`, `100`, and `99`:

| Set | Protocols that use exactly those rolls |
| --- | --- |
| **166** | `COLDCARD`, `Keystone` |
| **153** | `BlueWallet bit packing`, `COLDCARD`, `Keystone` |
| **140** | `Word-by-word Exact`, `COLDCARD`, `Keystone` |
| **100** | `Exact`, `COLDCARD`, `Keystone` |
| **99** | `bitcoinlib base-6`, `COLDCARD`, `Keystone` |

The `99` set is the one worth staring at: `COLDCARD` and `bitcoinlib base-6` both
complete on the same 99 rolls and produce mnemonics with no words in common. The
roll count that signals a hashed construction does not identify one.

`BlueWallet bit packing` is the one checkpoint that moves: an all-`1`–`4` tape
fills 128 bits in 64 rolls where an all-`5`/`6` tape needs 128, so the same tape
length can put it in a different set.

The open-ended protocols (`COLDCARD`, `Keystone`) hash every roll, so they appear
in every set at or above their minimum — with a *different* seed each time, since
roll count is part of their entropy. A protocol never appears against a length it
doesn't cleanly consume: `Exact` shows up only at `100`, `Word-by-word Exact`
only at `140`. Accepted seeds reveal with `r`; a content **rejection** (`Exact`
out-of-range, `COLDCARD` >30% one face, or a rejected `Word-by-word Exact`
candidate) can't be fixed by rolling more, so the remedy is `n`, a fresh set —
earlier captures stay for comparison. The same rolls usually yield *different*
seeds per wallet, because entropy provenance is protocol-dependent.

## 🏗️ Architecture and testing

| Layer | Owns | Boundary |
| --- | --- | --- |
| [`bip39-ceremony-core`](crates/bip39-ceremony-core/) · publishable | capture values, versioned conversions, SHA-256 conditioning, BIP-39 encoding, structured calculation evidence | no ceremony, presentation, or terminal dependencies; crypto is concrete so a compatibility label cannot front an incompatible hash |
| Application · unpublished | event-sourced `Ceremony`, safety and reveal policy, explanatory wording, terminal interface | consumes core only |

Unit tests cover pure calculation and domain paths, while consumer-style vector
tests verify the core crate's public boundary.

## 🔬 Reference validation

![upstream oracles](https://img.shields.io/badge/upstream%20oracles-7%20pinned-2aa198)

Every wallet-compatible profile is cross-checked against the wallet's own
**executable** code — not just its published example — pinned by revision and
content hash in `flake.lock`:

**COLDCARD · SeedSigner · Krux · BitBox02 · Keystone · Jade · Ian Coleman**

| Upstream (pinned) | Executed and compared at | `just` check |
| --- | --- | --- |
| **COLDCARD firmware** | dice count, 30% distribution rejection, ASCII SHA-256, target truncation | `reference-coldcard` |
| **SeedSigner** (+ `embit 0.8.0`) | `generate_mnemonic_from_coin_flips` coin-flip helper | `reference-seedsigner` |
| **Krux v26.08.0** | `DiceEntropy.new_key`: minimum gate, decimal-hyphen serialization, SHA-256, truncation | `reference-krux` |
| **BitBox02 firmware v9.26.4** | checksum-completion candidates + entropy-tail ordering | `reference-bitbox` |
| **Keystone** (legacy Android) | 50/98/99-roll completion branches, face `6`→`0` mapping, SHA-256 | `reference-keystone` |
| **Jade firmware 1.0.40** (+ pinned libwally) | `valid_final_words` checksum + final-word ordering | `reference-jade` |
| **Ian Coleman BIP-39** | entropy → English encoder (shared BIP-39 boundary) | `reference-iancoleman` |

Only executable upstream code counts as a reference. Project-owned protocols
without an upstream oracle (Exact, Word-by-word Exact, and the static Coin +
four-D6 table) are instead held to pinned in-repo vectors under
[`crates/bip39-ceremony-core/tests/`](crates/bip39-ceremony-core/tests/).

The suite is grouped by upstream authority under
[`tests/references/`](tests/references/); each implementation owns its loading,
vectors, and assertions, while a shared harness holds the core driver and its
typed accepted/rejected/invalid/error wire protocol. Nix exposes each check
independently and composes them in `reference-implementations`. Run the
aggregate with `just reference-validation` (also part of the host-wide
`just release-feasibility`), or a single comparison with `just reference-jade`
and its `reference-coldcard`, `reference-seedsigner`, `reference-krux`,
`reference-bitbox`, `reference-keystone`, `reference-iancoleman` siblings.

## 🧰 Development environment

Useful commands inside `nix develop` or direnv:

```sh
just                     # list commands
just check               # format, Clippy, tests, package and privacy lint
just security            # RustSec, licenses, sources and duplicate versions
just precommit           # complete local gate
just build               # debug binary
just release             # local optimized build; not a release artifact
just reference-validation # compare against pinned upstream implementations
just release-gnu         # Nix GNU/Linux feasibility artifact and PTY smoke
just release-musl        # Nix static-musl feasibility artifact and PTY smoke
just release-feasibility # build and test all host feasibility outputs
just guix-lock-lint      # compare Guix crate origins with Cargo.lock
just deps                # reviewed normal dependency graph
```

| Platform | Support | How to get it |
| --- | --- | --- |
| 🐧 **Linux** | ✅ Reproducible-build target | `gnu` (dynamic glibc) & `musl` (static PIE) from hash-checked `Cargo.lock`, with the test suite run during the build; independently cross-checked by Guix ([`contrib/guix/`](contrib/guix/)) |
| 🍎 **macOS** | 🔨 Build from source | `nix run` or `cargo` — CI runs the full test gate, but no signed or reproducible artifact yet |
| 🪟 **Windows** | 🚫 Not native | Run under **WSL2**, which uses the Linux build |

<sub>Linux outputs are release-feasibility probes, not yet a finalized reproducibility claim. The static-musl profile and the Guix checks are Linux-only; the terminal layer ([`termion`]) targets Unix TTYs.</sub>

[`termion`]: https://docs.rs/termion
