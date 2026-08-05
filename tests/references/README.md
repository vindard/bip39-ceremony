# Upstream reference validation

This suite compares `bip39-ceremony-core` with pinned upstream implementations.
The `harness` directory contains only the machine adapter and shared outcome
protocol. Each other directory owns the loading, vectors, and assertions for the
upstream implementation named by that directory.

Checks are independently addressable:

- `reference-harness`
- `reference-coldcard`
- `reference-seedsigner`
- `reference-iancoleman-bip39`
- `reference-iancoleman-legacy-dice`

`reference-implementations` composes the four upstream comparisons; the harness
remains an independently targeted infrastructure check. Ian Coleman's legacy
dice pipeline validates the documented compatibility transformation; it does
not claim to execute Keystone firmware.

Protocol requirements without an upstream oracle remain in
`crates/bip39-ceremony-core/tests/` rather than this suite.
