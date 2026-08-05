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

With Guix installed and its daemon running:

```sh
guix time-machine -C contrib/guix/channels.scm -- \
  build -f contrib/guix/package.scm --no-grafts
```

The predecessor recipe was successfully built before the implementation was
split into this repository: all 206 workspace tests and the installed binary's
pseudo-terminal smoke passed in a clean Debian container backed by a chrooting
Guix daemon. That validates the original recipe correction, not this renamed
repository state or cross-machine reproducibility.

A static-musl cross build is the intended next experiment, but is deliberately
not wrapped as a release command until this native definition has been rebuilt
through the pinned time machine and Guix cross-target semantics have been
verified.

## Current limitations

- This host has no system Guix daemon. Lock correspondence and Scheme parsing
  pass, but the renamed package has not yet completed a daemon build.
- Substitute authorization and a no-substitute builder policy remain to be
  recorded. The predecessor validation used official Guix substitutes where
  available.
- The prototype emits a package output, not normalized release distribution
  bytes, attestations, or provenance.
- No reproducibility claim follows from this definition alone.
