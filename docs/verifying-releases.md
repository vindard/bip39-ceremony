# Verifying releases

Every release ships several **independent** integrity layers. You don't need all
of them, but each answers a different question — and the strongest is that you
can rebuild the binary from source yourself.

| Layer | Answers | Verify with |
| --- | --- | --- |
| `SHA256SUMS` | did the download arrive intact? | `sha256sum -c` |
| Maintainer PGP (`.asc`) | does the maintainer (hardware key) vouch? | `gpg --verify` |
| cosign keyless (`.cosign.bundle`) | did this repo's release workflow build it? | `cosign verify-blob` |
| SLSA provenance | built from which commit, by which workflow? | `gh attestation verify` |
| Reproducible build | does the binary match the source? | `nix build … --rebuild` |

Download the artifacts and `SHA256SUMS`, `SHA256SUMS.asc`, and the
`*.cosign.bundle` files from the release, then:

## 1. Checksums

```sh
sha256sum -c SHA256SUMS --ignore-missing
```

## 2. Maintainer PGP signature (recommended)

Signed on a hardware key (YubiKey) that never touches CI.

Fingerprint: **`F1E2 DF8F 56F8 8D73 4181  1B92 1B00 5D83 8F95 D90A`**

```sh
# import the key (any one of these)
curl -fsSL https://keybase.io/vindard/pgp_keys.asc | gpg --import
# gpg --locate-keys pgp@arvinda.me

gpg --verify SHA256SUMS.asc SHA256SUMS
```

A `Good signature` from the fingerprint above, over a `SHA256SUMS` whose entries
match your downloads, is the maintainer vouching for exactly those bytes.

## 3. cosign keyless signature

Proves the artifact was produced by this repository's release workflow (sigstore
identity, recorded in the Rekor transparency log — no key to trust).

```sh
cosign verify-blob \
  --bundle bip39-ceremony-<version>-x86_64-linux-musl.cosign.bundle \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  --certificate-identity-regexp '^https://github.com/vindard/bip39-ceremony/\.github/workflows/release\.yml@' \
  bip39-ceremony-<version>-x86_64-linux-musl
```

## 4. Build provenance (SLSA)

Ties the binary to the exact source commit and builder.

```sh
gh attestation verify bip39-ceremony-<version>-x86_64-linux-musl \
  --repo vindard/bip39-ceremony
```

## 5. Reproduce it yourself (the real "verify")

Signatures vouch for a hash; reproducibility lets you confirm that hash comes
from auditable source. Rebuild the exact release from the tag and compare:

```sh
# builds and asserts bit-for-bit determinism against the cached output
nix build "github:vindard/bip39-ceremony/v<version>#musl" --rebuild

# compare against the published checksum
sha256sum result/bin/bip39-ceremony
grep musl SHA256SUMS
```

Or just run it without downloading a binary at all:

```sh
nix run github:vindard/bip39-ceremony
```
