A reproducible static-PIE Linux binary for `x86_64` (`musl` — no interpreter or
glibc-version dependency, runs on any x86_64 Linux), built with Nix and verified
bit-for-bit reproducible in the release workflow. Nix users can also
`nix run github:vindard/bip39-ceremony`.

> **Draft:** this release is finalized only after a maintainer attaches a
> hardware-key PGP signature over the checksums (`SHA256SUMS.asc`). Do not use a
> draft/unsigned build.

### Artifacts

- `bip39-ceremony-vX.Y.Z-x86_64-linux-musl` — static PIE (no libc dependency)
- `SHA256SUMS` (+ maintainer `SHA256SUMS.asc`)
- `*.cosign.bundle` — sigstore keyless signatures
- SLSA build provenance (verify with `gh attestation verify`)

### Verify (short)

```sh
# 1. checksums
sha256sum -c SHA256SUMS --ignore-missing
# 2. maintainer PGP signature (fingerprint F1E2 DF8F 56F8 8D73 4181 1B92 1B00 5D83 8F95 D90A)
gpg --verify SHA256SUMS.asc SHA256SUMS
# 3. build provenance
gh attestation verify bip39-ceremony-*-x86_64-linux-musl --repo vindard/bip39-ceremony
```

Full instructions — including cosign verification and reproducing the build from
source — are in
[`docs/verifying-releases.md`](https://github.com/vindard/bip39-ceremony/blob/main/docs/verifying-releases.md).

No Nix? You can also run straight from source:
`nix run github:vindard/bip39-ceremony`.
