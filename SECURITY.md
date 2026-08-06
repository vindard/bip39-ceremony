# Security Policy

## Scope and threat model

This project turns physical dice rolls and coin flips into standard BIP-39
mnemonics. **Rolls, flips, entropy, intermediate derivations, and the mnemonic
are wallet-secret material.**

The [README](README.md)'s "Security boundary" section describes what the
program does and does not defend against. In short: it persists nothing,
zeroizes owned secret buffers, keeps secrets in memory only, and gates every
derived secret behind a deliberate reveal — but it **cannot** protect against a
compromised OS, memory paged to swap/hibernation, terminal recording or
scrollback, screenshots, cameras, or bystanders. Run it on a trusted, offline
computer and local terminal.

## Supported versions

The project is pre-1.0 and under active development. Only the latest `main` is
supported; fixes land there.

| Version | Supported |
| --- | --- |
| `main` (latest) | ✅ |
| Older commits / tags | ❌ |

## Reporting a vulnerability

**Please report privately — do not open a public issue for a security problem.**

Use GitHub's private vulnerability reporting:

- Open the repository's **Security** tab → **Report a vulnerability**, or go
  directly to
  <https://github.com/vindard/bip39-ceremony/security/advisories/new>.

Please include, where possible:

- affected commit or tag,
- a description of the issue and its impact (especially any path that could
  expose or weaken secret material),
- steps to reproduce, and
- any suggested remediation.

We aim to acknowledge reports on a best-effort basis, typically within a few
days, and will coordinate a fix and disclosure with you. Fixes are developed
privately when warranted and released on `main`, with a published advisory once
users can update.

## Cryptography boundaries

Protocol-mandated cryptography (SHA-256 conditioning, BIP-39 encoding, and the
per-wallet conversion profiles) is intentionally concrete so a compatibility
label cannot front an incompatible hash. Reports that a profile diverges from
its documented upstream behavior are in scope; see the reference-validation
suite under [`tests/references/`](tests/references/).
