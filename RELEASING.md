# Releasing

Maintainer checklist for cutting a release. Releases are **draft-first**: CI
builds, verifies, and signs; a maintainer then signs the checksums on a hardware
key and publishes. See [`docs/verifying-releases.md`](docs/verifying-releases.md)
for the consumer side.

## 0. Dry run first — do not skip

**Before tagging anything, run the release workflow as a dry run** and confirm it
goes green:

- GitHub → **Actions → Release → Run workflow** (`workflow_dispatch`), or
  `gh workflow run release.yml`.

A dispatch run does everything except create the release: build `gnu`/`musl`,
the **`nix --rebuild` reproducibility gate**, the PTY smoke, and the cosign +
provenance signing. It exists to catch **non-determinism before you tag** — if
the reproducibility gate fails, the build is not bit-for-bit reproducible and
must be fixed first (a tagged run would fail the same way).

## 1. Pre-flight

- [ ] `main` is green.
- [ ] `Cargo.toml` `version` is the version you're releasing.
- [ ] The dry run in step 0 passed.

## 2. Tag

```sh
git tag v0.1.0          # annotated is fine: git tag -s v0.1.0 -m v0.1.0
git push origin v0.1.0
```

## 3. Approve the gate

The `release` environment requires your approval — approve the deployment in the
run. CI then produces a **draft** release with the binaries, `SHA256SUMS`,
`*.cosign.bundle` signatures, and SLSA provenance.

## 4. Verify + sign locally (YubiKey)

Sign on the hardware key that **never touches CI**:

```sh
# reproduce and confirm the published hashes come from source
nix build "github:vindard/bip39-ceremony/v0.1.0#musl" --rebuild
sha256sum result/bin/bip39-ceremony            # compare to SHA256SUMS (musl)

# download SHA256SUMS from the draft, then sign it (YubiKey touch)
gpg --detach-sign --armor SHA256SUMS           # -> SHA256SUMS.asc
gpg --verify SHA256SUMS.asc SHA256SUMS
```

Upload `SHA256SUMS.asc` to the draft release.

## 5. Publish

- [ ] Draft has: two binaries, `SHA256SUMS`, `SHA256SUMS.asc`, `*.cosign.bundle`.
- [ ] Publish the draft.

## 6. Post-release

- [ ] From the **published** release, re-verify as a consumer would
      (`sha256sum -c`, `gpg --verify`, `cosign verify-blob`,
      `gh attestation verify`) — see `docs/verifying-releases.md`.
- [ ] Announce.

> Scope today: x86_64 Linux (`gnu`, `musl`). arm64, macOS, and a crates.io
> publish of `bip39-ceremony-core` are follow-ups.
