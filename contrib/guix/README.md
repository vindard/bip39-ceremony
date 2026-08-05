# Guix release-build prototype

This directory is an auditable prototype, not yet the canonical release path.
It does not establish a reproducible-release or attestation claim.

## Inputs

- `channels.scm` authenticates the official Guix history from its documented
  channel introduction and pins commit
  `86813d5779253bb50002d79ab791eeda5a8b4729`.
- `package.scm` selects Guix's Rust 1.94.1 and lists every registry package from
  `Cargo.lock` as a crates.io origin with the same SHA-256 digest.
- `just guix-lock-lint` rejects missing, stale, or mismatched crate origins.

The Guix daemon acquires fixed-output sources before the package build. Guix's
Cargo build system then removes `Cargo.lock`, constructs a vendor directory only
from the declared origins, regenerates the graph with `--offline`, and compiles
with `--offline`. The lock lint is therefore the control that requires the Guix
origins to match `Cargo.lock` exactly; passing `--frozen` here would conflict with
Guix's standard lock-file handling rather than add an independent check.
The source is the Git-tracked repository content selected by `git-predicate`.
Consequently a release wrapper must reject dirty and untracked worktrees before
building.

## Evaluation

With Guix installed and its daemon running, use the repository-owned gate:

```sh
just guix-validate
```

It builds through the pinned channel without graft substitutions and runs the
installed binary through a pseudo-terminal smoke test. The equivalent build
command is:

```sh
guix time-machine -C contrib/guix/channels.scm -- \
  build -f contrib/guix/package.scm --no-grafts
```

The renamed package completed this gate on 2026-08-05 using Guix 1.5.0: all 271
workspace tests passed, the root workspace executable was installed, and the
pseudo-terminal smoke passed. Official signed substitutes were allowed for
practical bootstrapping. This validates the recipe at that revision, not a
future release candidate or cross-machine reproducibility.

## Validation cadence

Merge a packaging correction after normal review and a successful gate; do not
hold a known fix until release. Run `just guix-validate` before merging changes
to Rust sources, workspace manifests, `Cargo.lock`, `contrib/guix/**`, or
release/build scripts when a daemon is available. Keep `guix-lock-lint` and
`guix-syntax-lint` in the ordinary local gate, but do not present those static
checks as a replacement for a daemon build.

Repeat the complete gate from the exact clean release candidate or signed tag.
Record the pinned channel, output path and hash, host architecture, and whether
substitutes were allowed. A second independent builder must produce and compare
the release output before making a reproducibility claim.

Allowing authorized substitutes is practical for routine validation and does
not change the package derivation, but it trusts the build farm for substituted
bytes. `--no-substitutes` provides stronger source-build evidence at much higher
cost and still does not make the initial Guix binary bootstrap source-built.
Choose and record that policy explicitly for release validation.

A static-musl cross build remains a separate experiment. It is deliberately not
called a release command until Guix cross-target semantics and the resulting
artifact have been independently validated.

## Current limitations

- The prototype emits a package output, not normalized release distribution
  bytes, attestations, or provenance.
- Only one current native build has been recorded; no independent output
  comparison or reproducibility claim exists.
- The native result does not validate the intended static-musl cross build.
