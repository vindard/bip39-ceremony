#!/usr/bin/env python3
"""Enforce immutable dependency declarations and complete trust records."""

from __future__ import annotations

import argparse
import io
import json
import re
import subprocess
import sys
import tarfile
import tempfile
import tomllib
from collections import deque
from pathlib import Path
from typing import Any

USES_RE = re.compile(
    r"^\s*(?:-\s*)?(?:uses|[\"']uses[\"'])\s*:\s*([^\s#]+)", re.MULTILINE
)
FLOW_USES_RE = re.compile(
    r"(?:^|[{,])\s*(?:uses|[\"']uses[\"'])\s*:\s*([^,}\s#]+)", re.MULTILINE
)
IMAGE_RE = re.compile(r"^\s*image:\s*([^\s#]+)", re.MULTILINE)
FLOW_IMAGE_RE = re.compile(r"(?:^|[{,])\s*image\s*:\s*([^,}\s#]+)", re.MULTILINE)
SCALAR_CONTAINER_RE = re.compile(r"^\s*container:\s*([^\s#{]+)", re.MULTILINE)
NIX_FETCH_RE = re.compile(
    r"\b(?:builtins\.)?fetch(?:Git|Tarball|url)|\bfetch(?:FromGitHub|git|zip)\b"
)
FLAKE_INPUT_RE = re.compile(r"^    ([A-Za-z0-9_-]+)(?:\.url)?\s*=", re.MULTILINE)
SHA_RE = re.compile(r"[0-9a-f]{40}")
DEPENDENCY_SECTIONS = ("dependencies", "dev-dependencies", "build-dependencies")
REQUIRED_TRUST_FIELDS = {
    "class",
    "signature",
    "signer",
    "history",
    "reviewed",
    "verification",
}
REQUIRED_SOURCE_FIELDS = {
    "Cargo package": {"source", "checksum", "registry"},
    "Action": {"repository"},
    "flake input": {"source", "source-type"},
    "Guix channel": {"repository"},
}
ALLOWED_CLASSES = {
    "Cargo package": {
        "security-runtime",
        "runtime-adapter",
        "runtime-transitive",
        "runtime-and-build-time",
        "build-time",
        "development",
    },
    "Action": {"ci", "release"},
    "flake input": {"build-system", "reference-oracle"},
    "Guix channel": {"build-system"},
}
ALLOWED_VERIFICATION = {
    "github-verified",
    "trusted-publishing",
    "authenticated-history",
    "registry-integrity",
    "unsigned",
    "unknown-key",
}
COMPENSATING_REQUIRED = {"registry-integrity", "unsigned", "unknown-key"}
ALLOWED_CARGO_SOURCE = "registry+https://github.com/rust-lang/crates.io-index"


def workspace_manifests(root: Path) -> list[Path]:
    manifest = tomllib.loads((root / "Cargo.toml").read_text())
    paths = [root / "Cargo.toml"]
    paths.extend(root / member / "Cargo.toml" for member in manifest["workspace"]["members"])
    return paths


def dependency_tables(manifest: dict[str, Any]) -> list[dict[str, Any]]:
    tables = [manifest[name] for name in DEPENDENCY_SECTIONS if name in manifest]
    workspace = manifest.get("workspace", {})
    tables.extend(workspace[name] for name in DEPENDENCY_SECTIONS if name in workspace)
    for target in manifest.get("target", {}).values():
        tables.extend(target[name] for name in DEPENDENCY_SECTIONS if name in target)
    return tables


def rust_dependencies(root: Path, failures: list[str]) -> dict[str, str]:
    dependencies: dict[str, str] = {}
    for path in workspace_manifests(root):
        manifest = tomllib.loads(path.read_text())
        location = path.relative_to(root)
        if manifest.get("patch"):
            failures.append(f"{location}: Cargo source patches are not allowed")
        if manifest.get("replace"):
            failures.append(f"{location}: Cargo source replacements are not allowed")
        for table in dependency_tables(manifest):
            for alias, declaration in table.items():
                if not isinstance(declaration, dict):
                    failures.append(f"{location}: {alias} must use an explicit dependency table")
                    continue
                package = declaration.get("package", alias)
                if not isinstance(package, str) or not package:
                    failures.append(f"{location}: {alias} has an invalid package name")
                    continue
                if "git" in declaration:
                    failures.append(f"{location}: {package} uses a Git dependency")
                if "path" in declaration:
                    continue
                version = declaration.get("version")
                if not isinstance(version, str) or not re.fullmatch(
                    r"=\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?", version
                ):
                    failures.append(f"{location}: {package} must use an exact external version")
                    continue
                if declaration.get("default-features") is not False:
                    failures.append(f"{location}: {package} must disable default features")
                normalized = version.removeprefix("=")
                previous = dependencies.setdefault(package, normalized)
                if previous != normalized:
                    failures.append(
                        f"{location}: {package} has versions {previous} and {normalized}"
                    )
    return dependencies


def cargo_packages(
    root: Path, failures: list[str]
) -> dict[str, tuple[str, str, str]]:
    lock = tomllib.loads((root / "Cargo.lock").read_text())
    packages: dict[str, tuple[str, str, str]] = {}
    for package in lock["package"]:
        if "checksum" not in package:
            continue
        source = package.get("source", "")
        if source != ALLOWED_CARGO_SOURCE:
            failures.append(
                f"Cargo package {package['name']} uses unapproved source {source!r}"
            )
        packages[package["name"]] = (
            package["version"],
            package["checksum"],
            source,
        )
    return packages


def local_action_file(root: Path, reference: str) -> Path | None:
    target = (root / reference).resolve()
    try:
        target.relative_to(root.resolve())
    except ValueError:
        return None
    if target.is_file():
        return target
    for name in ("action.yml", "action.yaml"):
        candidate = target / name
        if candidate.is_file():
            return candidate
    return None


def action_files(root: Path) -> list[Path]:
    files = list((root / ".github" / "workflows").glob("*.y*ml"))
    action_root = root / ".github" / "actions"
    if action_root.exists():
        files.extend(action_root.rglob("action.yml"))
        files.extend(action_root.rglob("action.yaml"))
    files.extend(path for path in (root / "action.yml", root / "action.yaml") if path.exists())
    return sorted(set(files))


def actions(root: Path, failures: list[str]) -> dict[str, tuple[str, str]]:
    declared: dict[str, tuple[str, str]] = {}
    pending = deque(action_files(root))
    visited: set[Path] = set()
    while pending:
        path = pending.popleft()
        if path in visited:
            continue
        visited.add(path)
        content = path.read_text()
        references = USES_RE.findall(content) + FLOW_USES_RE.findall(content)
        for reference in references:
            if reference.startswith("./"):
                local = local_action_file(root, reference)
                if local is None:
                    failures.append(
                        f"{path.relative_to(root)}: local Action {reference} was not found"
                    )
                else:
                    pending.append(local)
                continue
            if reference.startswith("docker://"):
                failures.append(
                    f"{path.relative_to(root)}: Docker Action {reference} requires explicit policy support"
                )
                continue
            if "@" not in reference:
                failures.append(f"{path.relative_to(root)}: invalid Action reference {reference}")
                continue
            name, revision = reference.rsplit("@", 1)
            if not SHA_RE.fullmatch(revision):
                failures.append(
                    f"{path.relative_to(root)}: {name} must use a full 40-character commit SHA"
                )
                continue
            repository = "/".join(name.split("/")[:2])
            previous = declared.setdefault(name, (repository, revision))
            if previous != (repository, revision):
                failures.append(f"{name} is pinned to more than one revision")
        images = (
            IMAGE_RE.findall(content)
            + FLOW_IMAGE_RE.findall(content)
            + SCALAR_CONTAINER_RE.findall(content)
        )
        for image in images:
            failures.append(
                f"{path.relative_to(root)}: container image {image} requires explicit policy support"
            )
    return declared


def reject_direct_nix_fetches(root: Path, failures: list[str]) -> None:
    for path in root.rglob("*.nix"):
        if any(part in {".git", ".claude", "target"} for part in path.parts):
            continue
        if NIX_FETCH_RE.search(path.read_text()):
            failures.append(
                f"{path.relative_to(root)}: direct Nix fetches require explicit policy support"
            )


def flake_inputs(
    root: Path, failures: list[str]
) -> dict[str, tuple[str, str, str]]:
    lock = json.loads((root / "flake.lock").read_text())
    declared_section = (root / "flake.nix").read_text().split("  outputs", 1)[0]
    declared = set(FLAKE_INPUT_RE.findall(declared_section))
    locked_names = set(lock["nodes"]["root"].get("inputs", {}))
    for name in sorted(declared - locked_names):
        failures.append(f"flake input {name} is declared but not locked")
    for name in sorted(locked_names - declared):
        failures.append(f"flake input {name} is locked but not declared")
    inputs: dict[str, tuple[str, str, str]] = {}
    for name, node in lock["nodes"].items():
        locked = node.get("locked")
        if not locked or name == "root":
            continue
        source_type = locked["type"]
        if source_type == "path":
            failures.append(f"flake input {name} uses a path source")
            continue
        if source_type == "github":
            source = f'{locked["owner"]}/{locked["repo"]}'
        else:
            source = locked.get("url", "")
        revision = locked.get("rev") or locked.get("narHash", "")
        inputs[name] = (source_type, source, revision)
    return inputs


def scheme_channel_forms(text: str) -> list[str]:
    forms: list[str] = []
    cursor = 0
    while (start := text.find("(channel", cursor)) != -1:
        depth = 0
        quoted = False
        escaped = False
        comment = False
        for index in range(start, len(text)):
            character = text[index]
            if comment:
                if character == "\n":
                    comment = False
                continue
            if quoted:
                if escaped:
                    escaped = False
                elif character == "\\":
                    escaped = True
                elif character == '"':
                    quoted = False
                continue
            if character == ";":
                comment = True
            elif character == '"':
                quoted = True
            elif character == "(":
                depth += 1
            elif character == ")":
                depth -= 1
                if depth == 0:
                    forms.append(text[start : index + 1])
                    cursor = index + 1
                    break
        else:
            break
    return forms


def guix_channels(root: Path, failures: list[str]) -> dict[str, tuple[str]]:
    text = (root / "contrib" / "guix" / "channels.scm").read_text()
    text = re.sub(r"#\{channel\}#", "channel", text)
    channels: dict[str, tuple[str]] = {}
    for form in scheme_channel_forms(text):
        name = re.search(r"\(name '([^\s)]+)\)", form)
        revision = re.search(r'\(commit "([0-9a-f]{40})"\)', form)
        if name is None or revision is None:
            failures.append("Guix channel must have a symbolic name and exact commit")
            continue
        channels[name.group(1)] = (revision.group(1),)
    return channels


def records_by_name(
    records: list[dict[str, Any]], kind: str, failures: list[str]
) -> dict[str, dict[str, Any]]:
    indexed: dict[str, dict[str, Any]] = {}
    for record in records:
        name = record.get("name")
        if not isinstance(name, str) or not name:
            failures.append(f"{kind} trust record has no name")
            continue
        if name in indexed:
            failures.append(f"duplicate {kind} trust record: {name}")
            continue
        required = REQUIRED_TRUST_FIELDS | REQUIRED_SOURCE_FIELDS[kind]
        missing = sorted(field for field in required if not record.get(field))
        if missing:
            failures.append(f"{kind} trust record {name} lacks {', '.join(missing)}")
        classification = record.get("class")
        if classification not in ALLOWED_CLASSES[kind]:
            failures.append(f"{kind} trust record {name} has invalid class {classification!r}")
        verification = record.get("verification")
        if verification not in ALLOWED_VERIFICATION:
            failures.append(
                f"{kind} trust record {name} has invalid verification {verification!r}"
            )
        elif verification in COMPENSATING_REQUIRED and not record.get(
            "compensating-controls"
        ):
            failures.append(
                f"{kind} trust record {name} requires compensating-controls"
            )
        indexed[name] = record
    return indexed


def compare_inventory(
    kind: str,
    actual: dict[str, tuple[str, ...]],
    recorded: dict[str, dict[str, Any]],
    fields: tuple[str, ...],
    failures: list[str],
) -> None:
    for name in sorted(actual.keys() - recorded.keys()):
        failures.append(f"missing {kind} trust record: {name}")
    for name in sorted(recorded.keys() - actual.keys()):
        failures.append(f"stale {kind} trust record: {name}")
    for name in sorted(actual.keys() & recorded.keys()):
        expected = actual[name]
        found = tuple(str(recorded[name].get(field, "")) for field in fields)
        if expected != found:
            failures.append(
                f"stale {kind} trust record for {name}: expected {expected}, found {found}"
            )


def dependency_inventory(
    root: Path, failures: list[str]
) -> dict[str, dict[str, tuple[str, ...]]]:
    reject_direct_nix_fetches(root, failures)
    return {
        "Rust direct": {
            name: (version,)
            for name, version in rust_dependencies(root, failures).items()
        },
        "Cargo package": cargo_packages(root, failures),
        "Action": actions(root, failures),
        "flake input": flake_inputs(root, failures),
        "Guix channel": guix_channels(root, failures),
    }


def changed_dependencies(root: Path, base_ref: str) -> list[str]:
    archive = subprocess.check_output(
        ["git", "-C", str(root), "archive", "--format=tar", base_ref]
    )
    with tempfile.TemporaryDirectory() as directory:
        base_root = Path(directory)
        with tarfile.open(fileobj=io.BytesIO(archive), mode="r:") as contents:
            contents.extractall(base_root)
        current_failures: list[str] = []
        base_failures: list[str] = []
        current = dependency_inventory(root, current_failures)
        base = dependency_inventory(base_root, base_failures)
        failures = current_failures + base_failures
        if failures:
            raise ValueError("; ".join(failures))
        changed_by_kind: dict[str, list[str]] = {}
        for kind in current:
            names = current[kind].keys() | base[kind].keys()
            changed_by_kind[kind] = sorted(
                name for name in names if current[kind].get(name) != base[kind].get(name)
            )
        changed = [
            f"{kind}:{name}"
            for kind, names in changed_by_kind.items()
            if kind != "Cargo package"
            for name in names
        ]
        if not changed_by_kind["Rust direct"]:
            changed.extend(
                f"Cargo package:{name}" for name in changed_by_kind["Cargo package"]
            )
        return sorted(changed)


def lint(root: Path, base_ref: str | None = None) -> list[str]:
    failures: list[str] = []
    trust = tomllib.loads((root / "supply-chain" / "trust.toml").read_text())
    inventory = dependency_inventory(root, failures)
    direct = inventory["Rust direct"]
    cargo = inventory["Cargo package"]
    action = inventory["Action"]
    flake = inventory["flake input"]
    guix = inventory["Guix channel"]

    cargo_records = records_by_name(
        trust.get("cargo-package", []), "Cargo package", failures
    )
    action_records = records_by_name(trust.get("action", []), "Action", failures)
    flake_records = records_by_name(trust.get("flake-input", []), "flake input", failures)
    guix_records = records_by_name(trust.get("guix-channel", []), "Guix channel", failures)

    compare_inventory(
        "Cargo package",
        cargo,
        cargo_records,
        ("version", "checksum", "registry"),
        failures
    )
    for name, (version,) in direct.items():
        record = cargo_records.get(name)
        if record is None:
            failures.append(f"missing Cargo package trust record: {name}")
        elif record.get("version") != version:
            failures.append(
                f"direct Rust dependency {name} expects {version}, trust record has {record.get('version')}"
            )
    compare_inventory(
        "Action", action, action_records, ("repository", "revision"), failures
    )
    compare_inventory(
        "flake input",
        flake,
        flake_records,
        ("source-type", "source", "revision"),
        failures,
    )
    compare_inventory("Guix channel", guix, guix_records, ("revision",), failures)
    if base_ref is not None:
        try:
            changed = changed_dependencies(root, base_ref)
        except (subprocess.CalledProcessError, ValueError) as error:
            failures.append(f"could not compare dependency changes with {base_ref}: {error}")
        else:
            if len(changed) > 1:
                failures.append(
                    "dependency updates must be isolated; changed " + ", ".join(changed)
                )
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--base-ref")
    arguments = parser.parse_args()
    root = arguments.root.resolve()
    failures = lint(root, arguments.base_ref)
    if failures:
        print("dependency policy lint failed:", file=sys.stderr)
        print("\n".join(failures), file=sys.stderr)
        return 1

    trust = tomllib.loads((root / "supply-chain" / "trust.toml").read_text())
    print(
        "dependency policy passed: "
        f"{len(trust.get('cargo-package', []))} Cargo package, "
        f"{len(trust.get('action', []))} Action, "
        f"{len(trust.get('flake-input', []))} flake input, "
        f"{len(trust.get('guix-channel', []))} Guix channel records"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
