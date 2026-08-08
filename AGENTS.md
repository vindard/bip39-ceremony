# Agent instructions

## Crate boundaries — where new changes go

`bip39-ceremony-core` is strictly the conversion math: turning physical capture into entropy and entropy into BIP-39 mnemonics (rolls/flips → entropy → checksum → words), plus the input-requirement metadata that describes each conversion (e.g. `minimum_observations`, `strict_roll_count`, `supports_target`). Keep it lean, simple, and readable by someone auditing the math, because that is where the security lives. It must take on **no** non-security responsibilities — no UX, presentation, comparison, orchestration, ceremony, or terminal logic — and no dependency on the app crate. The intent is for core to eventually be a standalone, broadly reusable crate: a secure, verified reference for how different projects convert entropy into mnemonics.

Litmus test before adding anything to core: **"does this help someone validate the entropy→mnemonic math?"** If no, it belongs in the app layer (`src/`), which already consumes core's public API. Product, presentation, comparison, and policy features live in the app — even when they are pure and could compile inside core. Example: a multi-protocol comparison harness that replays one capture across several protocols is curation built on public core primitives, so it belongs in the app, not core.

## Guix validation

Read `contrib/guix/README.md` before changing Guix packaging, dependencies, workspace layout, or release workflow.

Run `just guix-validate` before marking a Guix-related change ready to merge when a daemon is available. Changes to Rust sources, `Cargo.toml`, `Cargo.lock`, `contrib/guix/**`, or release/build scripts require this validation unless the pull request explicitly records why it could not run. Static lock and Scheme checks are not substitutes for a daemon build.

Do not defer a validated packaging fix until release. Merge it through the normal review process, then repeat validation from the exact clean release candidate or signed tag. Do not claim reproducibility from one successful build; that requires comparison with an independent build. Record whether substitutes were allowed and keep binary-bootstrap trust distinct from source and output reproducibility.
