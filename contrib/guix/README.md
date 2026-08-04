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

The Guix daemon acquires fixed-output sources before the package build. Cargo's
build itself uses the generated vendor directory, `--frozen`, and no network.
The source is the Git-tracked repository content selected by `git-predicate`.
Consequently a release wrapper must reject dirty and untracked worktrees before
building.

## Evaluation

With Guix installed and its daemon running:

```sh
guix time-machine -C contrib/guix/channels.scm -- \
  build -f contrib/guix/package.scm --no-grafts
```

A static-musl cross build is the intended next experiment, but is deliberately
not wrapped as a release command until the native definition above has been
built through the pinned time machine and the Guix cross-target semantics have
been verified.

## Current limitations

- This host has no system Guix daemon. The Scheme definition and Cargo/Guix lock
  correspondence can be reviewed locally, but a successful pinned Guix build is
  still required.
- Substitute keys and no-substitute builder policy remain to be recorded.
- The prototype emits a package output, not normalized release distribution
  bytes, attestations, or provenance.
- No reproducibility claim follows from this definition alone.
