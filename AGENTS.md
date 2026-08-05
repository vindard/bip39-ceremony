# Agent instructions

## Guix validation

Read `contrib/guix/README.md` before changing Guix packaging, dependencies, workspace layout, or release workflow.

Run `just guix-validate` before marking a Guix-related change ready to merge when a daemon is available. Changes to Rust sources, `Cargo.toml`, `Cargo.lock`, `contrib/guix/**`, or release/build scripts require this validation unless the pull request explicitly records why it could not run. Static lock and Scheme checks are not substitutes for a daemon build.

Do not defer a validated packaging fix until release. Merge it through the normal review process, then repeat validation from the exact clean release candidate or signed tag. Do not claim reproducibility from one successful build; that requires comparison with an independent build. Record whether substitutes were allowed and keep binary-bootstrap trust distinct from source and output reproducibility.
