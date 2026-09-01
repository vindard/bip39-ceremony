#!/usr/bin/env python3

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SOURCE_ROOT = Path(__file__).resolve().parents[2]
LINT = SOURCE_ROOT / "scripts" / "dependency-policy-lint.py"


def isolated_git_env() -> dict[str, str]:
    """Environment for fixture subprocesses, stripped of ambient GIT_* vars.

    A git hook (e.g. this suite running under pre-commit) sets GIT_DIR,
    GIT_WORK_TREE, and GIT_INDEX_FILE for its own repository. Those vars
    override an explicit `git -C <path>`, so any inherited copy silently
    redirects fixture git commands onto the caller's real repo/index
    instead of the temp repo they were given.
    """
    return {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}


class DependencyPolicyLintTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        for path in (
            "Cargo.toml",
            "Cargo.lock",
            "flake.nix",
            "flake.lock",
            "crates/bip39-ceremony-core/Cargo.toml",
            "tests/references/harness/core-driver/Cargo.toml",
            "supply-chain/trust.toml",
            "contrib/guix/channels.scm",
        ):
            source = SOURCE_ROOT / path
            destination = self.root / path
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, destination)
        shutil.copytree(
            SOURCE_ROOT / ".github" / "workflows",
            self.root / ".github" / "workflows",
        )
        env = isolated_git_env()
        subprocess.run(["git", "init", "-q", str(self.root)], check=True, env=env)
        subprocess.run(["git", "-C", str(self.root), "add", "."], check=True, env=env)
        subprocess.run(
            [
                "git",
                "-C",
                str(self.root),
                "-c",
                "user.name=Policy Test",
                "-c",
                "user.email=policy@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-qm",
                "baseline",
            ],
            check=True,
            env=env,
        )
        self.base = subprocess.check_output(
            ["git", "-C", str(self.root), "rev-parse", "HEAD"], text=True, env=env
        ).strip()

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def run_lint(self, compare_base: bool = False) -> subprocess.CompletedProcess[str]:
        command = [sys.executable, str(LINT), "--root", str(self.root)]
        if compare_base:
            command.extend(["--base-ref", self.base])
        return subprocess.run(
            command,
            text=True,
            capture_output=True,
            check=False,
            env=isolated_git_env(),
        )

    def replace(self, path: str, old: str, new: str) -> None:
        target = self.root / path
        content = target.read_text()
        self.assertIn(old, content)
        target.write_text(content.replace(old, new, 1))

    def assert_rejected(self, message: str) -> None:
        result = self.run_lint()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn(message, result.stderr)

    def test_repository_policy_passes(self) -> None:
        result = self.run_lint()
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_fixture_ignores_inherited_git_hook_environment(self) -> None:
        # Reproduce a git hook's ambient GIT_DIR/GIT_WORK_TREE/GIT_INDEX_FILE
        # pointing at an unrelated repo, and confirm the fixture still
        # targets its own temp repo instead of mutating the decoy.
        with tempfile.TemporaryDirectory() as decoy_name:
            decoy_root = Path(decoy_name)
            # This suite may itself be running inside the real pre-commit
            # hook, so the *bootstrap* of the decoy must not inherit that
            # hook's own GIT_DIR either, or it would recurse into the real
            # repo before the scenario under test is even set up.
            bootstrap_env = isolated_git_env()
            subprocess.run(
                ["git", "init", "-q", str(decoy_root)], check=True, env=bootstrap_env
            )
            subprocess.run(
                [
                    "git",
                    "-C",
                    str(decoy_root),
                    "-c",
                    "user.name=Decoy",
                    "-c",
                    "user.email=decoy@example.invalid",
                    "-c",
                    "commit.gpgsign=false",
                    "commit",
                    "-q",
                    "--allow-empty",
                    "-m",
                    "decoy baseline",
                ],
                check=True,
                env=bootstrap_env,
            )
            decoy_head_before = subprocess.check_output(
                ["git", "-C", str(decoy_root), "rev-parse", "HEAD"],
                text=True,
                env=bootstrap_env,
            ).strip()
            decoy_index = decoy_root / ".git" / "index"
            decoy_index_before = decoy_index.read_bytes()

            hook_env = dict(os.environ)
            hook_env["GIT_DIR"] = str(decoy_root / ".git")
            hook_env["GIT_WORK_TREE"] = str(decoy_root)
            hook_env["GIT_INDEX_FILE"] = str(decoy_index)

            result = subprocess.run(
                [
                    sys.executable,
                    "-m",
                    "unittest",
                    "scripts.tests.test_dependency_policy_lint"
                    ".DependencyPolicyLintTests.test_repository_policy_passes",
                ],
                cwd=str(SOURCE_ROOT),
                env=hook_env,
                text=True,
                capture_output=True,
            )

            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            decoy_head_after = subprocess.check_output(
                ["git", "-C", str(decoy_root), "rev-parse", "HEAD"],
                text=True,
                env=bootstrap_env,
            ).strip()
            self.assertEqual(decoy_head_after, decoy_head_before)
            self.assertEqual(decoy_index.read_bytes(), decoy_index_before)

    def test_non_exact_cargo_version_is_rejected(self) -> None:
        self.replace("Cargo.toml", 'version = "=0.14.0"', 'version = "^0.14.0"')
        self.assert_rejected("must use an exact external version")

    def test_enabled_default_features_are_rejected(self) -> None:
        self.replace("Cargo.toml", "default-features = false", "default-features = true")
        self.assert_rejected("must disable default features")

    def test_cargo_package_alias_needs_its_own_trust_record(self) -> None:
        self.replace(
            "crates/bip39-ceremony-core/Cargo.toml",
            'bip39 = { version = "=2.2.2"',
            'bip39 = { package = "attacker-bip39", version = "=2.2.2"',
        )
        self.assert_rejected("missing Cargo package trust record: attacker-bip39")

    def test_cargo_source_patch_is_rejected(self) -> None:
        path = self.root / "Cargo.toml"
        path.write_text(
            path.read_text()
            + '\n[patch.crates-io]\nbip39 = { git = "https://example.invalid/bip39" }\n'
        )
        self.assert_rejected("Cargo source patches are not allowed")

    def test_lock_package_update_needs_a_trust_record(self) -> None:
        self.replace(
            "Cargo.lock",
            'name = "zeroize_derive"\nversion = "1.5.0"',
            'name = "zeroize_derive"\nversion = "1.5.1"',
        )
        self.assert_rejected("stale Cargo package trust record for zeroize_derive")

    def test_unapproved_lock_registry_is_rejected(self) -> None:
        old = "registry+https://github.com/rust-lang/crates.io-index"
        new = "registry+https://example.invalid/index"
        self.replace("Cargo.lock", old, new)
        self.replace("supply-chain/trust.toml", old, new)
        self.assert_rejected("uses unapproved source")

    def test_mutable_action_reference_is_rejected(self) -> None:
        self.replace(
            ".github/workflows/ci.yml",
            "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1",
            "actions/checkout@v7",
        )
        self.assert_rejected("must use a full 40-character commit SHA")

    def test_quoted_uses_key_is_scanned(self) -> None:
        self.replace(
            ".github/workflows/ci.yml",
            "uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1",
            '"uses": actions/checkout@v7',
        )
        self.assert_rejected("must use a full 40-character commit SHA")

    def test_flow_mapping_action_is_scanned(self) -> None:
        workflow = self.root / ".github" / "workflows" / "flow.yml"
        workflow.write_text("steps:\n  - {uses: actions/checkout@v7}\n")
        self.assert_rejected("must use a full 40-character commit SHA")

    def test_local_composite_action_dependencies_are_scanned(self) -> None:
        action = self.root / ".github" / "actions" / "example" / "action.yml"
        action.parent.mkdir(parents=True)
        action.write_text(
            "name: example\nruns:\n  using: composite\n  steps:\n    - uses: actions/setup-python@v5\n"
        )
        self.assert_rejected("must use a full 40-character commit SHA")

    def test_referenced_local_actions_are_scanned_recursively(self) -> None:
        workflow = self.root / ".github" / "workflows" / "local.yml"
        workflow.write_text("steps:\n  - uses: ./tools/actions/outer\n")
        outer = self.root / "tools" / "actions" / "outer" / "action.yml"
        outer.parent.mkdir(parents=True)
        outer.write_text(
            "name: outer\nruns:\n  using: composite\n  steps:\n    - uses: ./tools/actions/inner\n"
        )
        inner = self.root / "tools" / "actions" / "inner" / "action.yml"
        inner.parent.mkdir(parents=True)
        inner.write_text(
            "name: inner\nruns:\n  using: composite\n  steps:\n    - uses: actions/setup-python@v5\n"
        )
        self.assert_rejected("must use a full 40-character commit SHA")

    def test_uninventoried_docker_action_is_rejected(self) -> None:
        workflow = self.root / ".github" / "workflows" / "docker.yml"
        workflow.write_text("steps:\n  - uses: docker://alpine:3.22\n")
        self.assert_rejected("requires explicit policy support")

    def test_scalar_job_container_is_rejected(self) -> None:
        workflow = self.root / ".github" / "workflows" / "container.yml"
        workflow.write_text("jobs:\n  test:\n    container: alpine:latest\n")
        self.assert_rejected(
            "container image alpine:latest requires explicit policy support"
        )

    def test_path_flake_input_is_rejected(self) -> None:
        path = self.root / "flake.lock"
        lock = json.loads(path.read_text())
        lock["nodes"]["nixpkgs"]["locked"] = {
            "type": "path",
            "path": "/tmp/unreviewed",
            "narHash": "sha256-example",
        }
        path.write_text(json.dumps(lock))
        self.assert_rejected("flake input nixpkgs uses a path source")

    def test_non_github_flake_input_needs_a_trust_record(self) -> None:
        path = self.root / "flake.lock"
        lock = json.loads(path.read_text())
        lock["nodes"]["external-git"] = {
            "locked": {
                "type": "git",
                "url": "https://example.com/source.git",
                "rev": "0" * 40,
                "narHash": "sha256-example",
            }
        }
        path.write_text(json.dumps(lock))
        self.assert_rejected("missing flake input trust record: external-git")

    def test_unlocked_flake_declaration_is_rejected(self) -> None:
        self.replace(
            "flake.nix",
            "  outputs =",
            '    unreviewed.url = "github:attacker/unreviewed";\n  outputs =',
        )
        self.assert_rejected("flake input unreviewed is declared but not locked")

    def test_direct_nix_fetch_is_rejected(self) -> None:
        self.replace(
            "flake.nix",
            "  outputs =",
            '    fetched = builtins.fetchGit { url = "https://example.invalid"; };\n  outputs =',
        )
        self.assert_rejected("direct Nix fetches require explicit policy support")

    def test_every_guix_channel_needs_a_trust_record(self) -> None:
        path = self.root / "contrib" / "guix" / "channels.scm"
        path.write_text(
            path.read_text()
            + "\n(#{channel}# (url \"https://example.com\") (name 'unreviewed) "
            + f'(commit "{"0" * 40}"))\n'
        )
        self.assert_rejected("missing Guix channel trust record: unreviewed")

    def test_invalid_verification_status_is_rejected(self) -> None:
        self.replace(
            "supply-chain/trust.toml",
            'verification = "registry-integrity"',
            'verification = "invalid"',
        )
        self.assert_rejected("has invalid verification 'invalid'")

    def test_cargo_source_evidence_is_required(self) -> None:
        self.replace(
            "supply-chain/trust.toml",
            'source = "crates.io / bluss/arrayvec"',
            'source = ""',
        )
        self.assert_rejected("Cargo package trust record arrayvec lacks source")

    def test_pr_workflow_runs_base_comparison(self) -> None:
        workflow = (self.root / ".github" / "workflows" / "ci.yml").read_text()
        self.assertIn(
            'dependency-policy-lint.py --base-ref "${{ github.event.pull_request.base.sha }}"',
            workflow,
        )

    def test_multiple_direct_updates_are_rejected(self) -> None:
        revisions = {
            "3d3c42e5aac5ba805825da76410c181273ba90b1": "a" * 40,
            "043fb46d1a93c77aae656e7c1c64a875d1fc6a0a": "b" * 40,
        }
        for workflow in (self.root / ".github" / "workflows").glob("*.yml"):
            content = workflow.read_text()
            for old, new in revisions.items():
                content = content.replace(old, new)
            workflow.write_text(content)
        trust = self.root / "supply-chain" / "trust.toml"
        content = trust.read_text()
        for old, new in revisions.items():
            content = content.replace(old, new)
        trust.write_text(content)

        result = self.run_lint(compare_base=True)
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("dependency updates must be isolated", result.stderr)


if __name__ == "__main__":
    unittest.main()
