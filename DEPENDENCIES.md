# Dependency policy

Dependencies are executable trust relationships. This policy covers Rust crates,
GitHub Actions, release tools, Nix and Guix inputs, and upstream implementations
used as reference oracles.

## Default: own trivial functionality

Implement functionality in this repository when it is small, stable, fully
auditable, expressible with `std` or an existing dependency, and free of
cryptographic, standards, portability, or operating-system hazards. Evaluate the
whole transitive closure and long-term maintenance cost, not just the direct
package or the amount of code avoided.

Do not replace mature cryptographic primitives, BIP-39 encoding, secret
zeroization, protocol standards, or complex platform integration with casual
in-house implementations. In those cases, a narrowly configured and well-vetted
dependency can reduce risk. New dependencies must disable default features and
enable only the capabilities the use case requires.

A dependency is admitted only when its PR explains:

1. why existing code and dependencies cannot reasonably provide the behavior;
2. why implementing it locally would be riskier or materially harder to audit;
3. the complete dependency closure and execution capabilities it introduces;
4. its maintenance, ownership, license, provenance, and security posture; and
5. how it can later be updated, replaced, or removed.

## Trust classes

| Class | Examples | Required emphasis |
| --- | --- | --- |
| Security/runtime | `bip39`, `bitcoin_hashes`, `zeroize` | calculation correctness, secret handling, source review |
| Runtime adapter | `termion` | platform behavior, unsafe/native boundary, PTY evidence |
| Build-time | build scripts and procedural macros | compiler-time filesystem, process, and network access |
| CI | GitHub Actions | token permissions, secrets, OIDC, network access, immutable revision |
| Release | signing, provenance, and publishing tools | artifact identity, exercised release path, environment protections |
| Build system | Rust, Nixpkgs, and the Guix channel | compiler closure, authenticated history, reproducibility |
| Reference oracle | wallet implementations in `flake.lock` | semantic drift in the external behavior used as evidence |

“Development-only” is not a low-risk classification. Build scripts and
procedural macros execute during compilation. A GitHub Action can read source,
use the job token, access configured secrets, or alter artifacts.

## Update review

One direct dependency is updated per PR. Its necessary transitive lock changes
stay in the same PR. One Action revision may be changed everywhere that Action
is used. Major upgrades and reference-oracle changes are never grouped. CI
compares direct Cargo, Action, flake-input, and Guix-channel pins with the PR base
and rejects updates to more than one identity.

Review the source diff, not only release notes. Record:

- old and new immutable revisions, package checksums, and upstream repository;
- direct and transitive additions, removals, and duplicate versions;
- new features, build scripts, procedural macros, native code, unsafe code, or
  network behavior;
- API, MSRV, platform, serialization, protocol, and secret-lifecycle changes;
- RustSec advisories, yanked or unmaintained status, licenses, and source changes;
- Action permissions, secrets, OIDC claims, outbound endpoints, and artifact
  handling;
- the tests that exercise the dependency's real boundary; and
- required Cargo lock, Nix lock, Guix origin, and trust-ledger updates.

Automation may open an update but never approves or merges it.

## Signatures and signer trust

For every update, identify the object being authenticated: commit, tag, crate,
artifact, or provenance statement. Record the verification mechanism, signer,
full key fingerprint or Sigstore OIDC subject when available, and the relationship
between the signed object and the exact source or artifact being consumed.

Build a signer profile from recent history. Compare at least the previous three
releases or twelve months of releases, whichever is larger enough to establish a
pattern. Check maintainer or automation role, key age and status, documented
rotation, revocation, changes in release automation, and unexplained gaps in a
project that normally signs releases.

Decision rules:

- an invalid or mismatched signature blocks the update;
- a new signer, unexplained key rotation, or unexpectedly unsigned release is
  held until verified through a second channel;
- ecosystems without usable artifact signing require compensating evidence:
  registry checksum, publisher history, source correspondence, cooling period,
  source review, tests, and reproducible packaging; and
- a valid signature with established continuity is positive evidence, not proof
  that the code is safe.

A Dependabot signature authenticates Dependabot's commit, not the upstream
revision. A GitHub “Verified” record is also insufficient by itself: GitHub keeps
historical verification records after a key is revoked or expires. Review current
key state and signer continuity.

For Sigstore, verify the expected issuer, repository, workflow identity, ref,
environment, and transparency-log inclusion. Any valid Sigstore identity is not
an acceptable substitute for the expected identity.

Current baselines live in [`supply-chain/trust.toml`](supply-chain/trust.toml).
Every registry package in `Cargo.lock`, including build-time and transitive code,
has a record. Each record names its risk class, verification status, signer
evidence, history, and compensating controls where signatures are absent or unverifiable. The lint
checks that records are complete and synchronized; it does not turn free-form
trust evidence into an automated cryptographic decision. The reviewer verifies
that evidence independently. Updating a direct Rust dependency, Action, external
flake input, or Guix channel requires updating its trust record in the same PR. Cargo source patches and replacements
are prohibited; a reviewed registry release must be used instead. Direct Nix
fetch primitives, path flake inputs, Docker Actions, and workflow container
images are rejected until the policy and trust ledger explicitly model them.

## Timing

Routine updates use a cooling period measured from upstream publication:

| Change | Minimum delay |
| --- | ---: |
| Patch | 7 days |
| Minor | 14 days |
| Major | 30 days |

A confirmed security fix bypasses the cooldown. Its PR must identify the
advisory, affected surface, exposure, urgency, and any compensating controls.
Security urgency does not bypass signature, source identity, or boundary tests.

Cargo updates are checked monthly and GitHub Actions weekly. Dependabot is
limited to one open version-update PR per ecosystem so reviews remain isolated.
Security updates are not delayed by Dependabot cooldowns. GitHub Actions use a
14-day automation delay because that ecosystem does not support SemVer-specific
cooldowns; reviewers enforce the 30-day major delay. Cooling periods for manual
updates are also reviewer-enforced because the local lint deliberately performs
no network-dependent release-date lookup.

## Required evidence

Every dependency change runs:

```sh
just check
just security
```

Changes to Rust manifests, `Cargo.lock`, Rust sources, Nix or Guix packaging,
release scripts, or build workflows also run:

```sh
just guix-validate
```

Record whether Guix substitutes were allowed. Static lock and syntax checks are
not substitutes for a daemon build. The ordinary GitHub-hosted CI gate does not
run a Guix daemon, so this evidence is produced separately before merge.

Additional evidence follows ownership:

- calculation dependencies: protocol vectors and reference validation;
- terminal dependencies: PTY smoke and supported viewport/platform checks;
- CI Actions: affected workflows with least-privilege permissions;
- release tools: a test that actually executes the changed tag-only path; and
- reference inputs: the corresponding pinned upstream oracle comparison.

A green general CI run does not validate a conditionally skipped release step.

## Removal and incidents

Remove dependencies that become unnecessary, unmaintained, unverifiable, or
replaceable by simpler owned code. Re-evaluate admitted dependencies when their
ownership, signing identity, source repository, license, build behavior, or
transitive closure changes.

For a suspected compromise, stop updates and releases, preserve the relevant
hashes and provenance, identify affected builds, rotate exposed credentials,
remove or pin away from the dependency, and document why the selected safe
revision is trusted before resuming publication.

## Basis

- [GitHub secure use: immutable Action SHAs and source audit](https://docs.github.com/en/actions/reference/security/secure-use)
- [GitHub Dependabot cooldown behavior](https://docs.github.com/en/code-security/reference/supply-chain-security/dependabot-options-reference#cooldown)
- [Cargo manifests and lockfiles](https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html)
- [Rust procedural-macro security](https://doc.rust-lang.org/reference/procedural-macros.html)
- [RustSec advisory database](https://rustsec.org/)
- [Sigstore identity and transparency model](https://docs.sigstore.dev/about/overview/)
